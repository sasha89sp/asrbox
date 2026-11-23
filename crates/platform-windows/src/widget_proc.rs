use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::time::Instant;

use asrbox_platform_api::{PixelPoint, PlatformEvent, WidgetLayer, WidgetSizeBounds};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetCursorPos, GetWindowLongPtrW, GetWindowRect, LoadCursorW, SetCursor,
    SetWindowPos, GWLP_USERDATA, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WINDOWPOS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_RBUTTONUP, WM_SETCURSOR, WM_TIMER, WM_WINDOWPOSCHANGING,
};

use crate::widget_win::{compute_resized_rect, cursor_for_edges, hit_resize_edges};
use crate::window::{lparam_point, push_event, WM_MOUSELEAVE};

pub(crate) const WIDGET_MINUTE_TIMER_ID: usize = 101;
pub(crate) const WIDGET_REFRESH_TIMER_ID: usize = 102;

#[derive(Clone, Copy)]
struct WidgetDragMeta {
    layer: WidgetLayer,
    drag_header_h: i32,
    bounds: Option<WidgetSizeBounds>,
    pressing_header: bool,
    dragging: bool,
    resize_edges: u8,
    press_screen: POINT,
    win_start: RECT,
    last_resize_emit: Option<Instant>,
    last_emitted_rect: RECT,
}

thread_local! {
    static WIDGET_META: RefCell<HashMap<u64, WidgetDragMeta>> = RefCell::new(HashMap::new());
}

pub(crate) fn register_widget_meta(widget_id: u64, layer: WidgetLayer, drag_header_h: i32) {
    WIDGET_META.with(|m| {
        m.borrow_mut().insert(
            widget_id,
            WidgetDragMeta {
                layer,
                drag_header_h,
                bounds: None,
                pressing_header: false,
                dragging: false,
                resize_edges: 0,
                press_screen: POINT::default(),
                win_start: RECT::default(),
                last_resize_emit: None,
                last_emitted_rect: RECT::default(),
            },
        );
    });
}

pub(crate) fn set_widget_bounds_meta(widget_id: u64, bounds: WidgetSizeBounds) {
    WIDGET_META.with(|m| {
        if let Some(meta) = m.borrow_mut().get_mut(&widget_id) {
            meta.bounds = Some(bounds);
        }
    });
}

pub(crate) fn update_widget_layer_meta(widget_id: u64, layer: WidgetLayer) {
    WIDGET_META.with(|m| {
        if let Some(meta) = m.borrow_mut().get_mut(&widget_id) {
            meta.layer = layer;
        }
    });
}

pub(crate) fn remove_widget_meta(widget_id: u64) {
    WIDGET_META.with(|m| {
        m.borrow_mut().remove(&widget_id);
    });
}

pub(crate) fn apply_widget_layer(hwnd: HWND, layer: WidgetLayer) {
    let insert_after = match layer {
        WidgetLayer::Wallpaper => HWND_BOTTOM,
        WidgetLayer::Normal => HWND_NOTOPMOST,
        WidgetLayer::Topmost => HWND_TOPMOST,
    };
    // SAFETY: Updating z-order on an owned widget HWND on the UI thread.
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(insert_after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

pub(crate) unsafe extern "system" fn widget_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let widget_id = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as u64;
    match msg {
        WM_SETCURSOR => on_widget_set_cursor(hwnd, widget_id, wparam, lparam),
        WM_LBUTTONDOWN => on_widget_lbutton_down(hwnd, widget_id, lparam),
        WM_MOUSEMOVE => on_widget_mouse_move(hwnd, widget_id, lparam),
        WM_LBUTTONUP => on_widget_lbutton_up(hwnd, widget_id, lparam),
        WM_MOUSELEAVE => {
            push_event(PlatformEvent::WidgetPointerLeave { widget_id });
            LRESULT(0)
        }
        WM_RBUTTONUP => on_widget_rbutton_up(hwnd, widget_id, lparam),
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
            push_event(PlatformEvent::WidgetWheel { widget_id, delta });
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == WIDGET_MINUTE_TIMER_ID => {
            push_event(PlatformEvent::WidgetMinuteTick { widget_id });
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == WIDGET_REFRESH_TIMER_ID => {
            push_event(PlatformEvent::WidgetRefreshTick { widget_id });
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGING => on_widget_pos_changing(hwnd, widget_id, lparam),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_widget_set_cursor(
    hwnd: HWND,
    widget_id: u64,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let mut cursor = POINT::default();
    let mut wr = RECT::default();
    if GetCursorPos(&mut cursor).is_ok() && GetWindowRect(hwnd, &mut wr).is_ok() {
        let local = PixelPoint {
            x: cursor.x - wr.left,
            y: cursor.y - wr.top,
        };
        let w = wr.right - wr.left;
        let h = wr.bottom - wr.top;
        let edges = WIDGET_META.with(|m| {
            let map = m.borrow();
            let meta = map.get(&widget_id)?;
            if meta.resize_edges != 0 {
                return Some(meta.resize_edges);
            }
            meta.bounds.map(|_| hit_resize_edges(local, w, h))
        });
        if let Some(e) = edges {
            if let Some(id) = cursor_for_edges(e) {
                if let Ok(hc) = LoadCursorW(None, id) {
                    SetCursor(Some(hc));
                    return LRESULT(1);
                }
            }
        }
    }
    DefWindowProcW(hwnd, WM_SETCURSOR, wparam, lparam)
}

unsafe fn on_widget_lbutton_down(hwnd: HWND, widget_id: u64, lparam: LPARAM) -> LRESULT {
    let pt = lparam_point(lparam);
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    let mut wr = RECT::default();
    let _ = GetWindowRect(hwnd, &mut wr);
    let w = wr.right - wr.left;
    let h = wr.bottom - wr.top;
    let captured = WIDGET_META.with(|m| {
        if let Some(meta) = m.borrow_mut().get_mut(&widget_id) {
            if meta.bounds.is_some() {
                let edges = hit_resize_edges(pt, w, h);
                if edges != 0 {
                    meta.resize_edges = edges;
                    meta.pressing_header = false;
                    meta.dragging = false;
                    meta.press_screen = cursor;
                    meta.win_start = wr;
                    meta.last_emitted_rect = wr;
                    meta.last_resize_emit = None;
                    return true;
                }
            }
            if pt.y < meta.drag_header_h {
                meta.pressing_header = true;
                meta.dragging = false;
                meta.resize_edges = 0;
                meta.press_screen = cursor;
                meta.win_start = wr;
                return true;
            }
        }
        false
    });
    if captured {
        let _ = SetCapture(hwnd);
    }
    LRESULT(0)
}

unsafe fn on_widget_mouse_move(hwnd: HWND, widget_id: u64, lparam: LPARAM) -> LRESULT {
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);

    enum MoveAction {
        Resize(RECT),
        Drag(i32, i32),
        Hover,
    }
    let action = WIDGET_META.with(|m| {
        let mut map = m.borrow_mut();
        let Some(meta) = map.get_mut(&widget_id) else {
            return MoveAction::Hover;
        };
        if meta.resize_edges != 0 {
            if let Some(bounds) = meta.bounds {
                let next = compute_resized_rect(
                    meta.win_start,
                    meta.resize_edges,
                    cursor.x - meta.press_screen.x,
                    cursor.y - meta.press_screen.y,
                    bounds,
                );
                let due = meta
                    .last_resize_emit
                    .is_none_or(|t| t.elapsed().as_millis() >= 24);
                let changed = next.left != meta.last_emitted_rect.left
                    || next.top != meta.last_emitted_rect.top
                    || next.right != meta.last_emitted_rect.right
                    || next.bottom != meta.last_emitted_rect.bottom;
                if due && changed {
                    meta.last_resize_emit = Some(Instant::now());
                    meta.last_emitted_rect = next;
                    return MoveAction::Resize(next);
                }
                return MoveAction::Resize(RECT::default());
            }
        }
        if meta.pressing_header {
            let dx = cursor.x - meta.press_screen.x;
            let dy = cursor.y - meta.press_screen.y;
            if !meta.dragging && dx.abs() + dy.abs() >= 4 {
                meta.dragging = true;
            }
            if meta.dragging {
                return MoveAction::Drag(meta.win_start.left + dx, meta.win_start.top + dy);
            }
        }
        MoveAction::Hover
    });

    match action {
        MoveAction::Resize(r) => {
            if r.right > r.left && r.bottom > r.top {
                push_event(PlatformEvent::WidgetResized {
                    widget_id,
                    x: r.left,
                    y: r.top,
                    width: (r.right - r.left) as u32,
                    height: (r.bottom - r.top) as u32,
                    finished: false,
                });
            }
            return LRESULT(0);
        }
        MoveAction::Drag(nx, ny) => {
            let _ = SetWindowPos(hwnd, None, nx, ny, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
            return LRESULT(0);
        }
        MoveAction::Hover => {}
    }

    let mut tme = TRACKMOUSEEVENT {
        cbSize: u32::try_from(mem::size_of::<TRACKMOUSEEVENT>()).unwrap_or(0),
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    let _ = TrackMouseEvent(&mut tme);
    push_event(PlatformEvent::WidgetPointerMove {
        widget_id,
        point: lparam_point(lparam),
    });
    LRESULT(0)
}

unsafe fn on_widget_lbutton_up(hwnd: HWND, widget_id: u64, lparam: LPARAM) -> LRESULT {
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    let outcome = WIDGET_META.with(|m| {
        if let Some(meta) = m.borrow_mut().get_mut(&widget_id) {
            if meta.resize_edges != 0 {
                let edges = meta.resize_edges;
                meta.resize_edges = 0;
                meta.last_resize_emit = None;
                if let Some(bounds) = meta.bounds {
                    let final_r = compute_resized_rect(
                        meta.win_start,
                        edges,
                        cursor.x - meta.press_screen.x,
                        cursor.y - meta.press_screen.y,
                        bounds,
                    );
                    return (false, Some(final_r));
                }
            }
            let d = meta.dragging;
            meta.pressing_header = false;
            meta.dragging = false;
            return (d, None);
        }
        (false, None)
    });
    let _ = ReleaseCapture();
    if let Some(r) = outcome.1 {
        push_event(PlatformEvent::WidgetResized {
            widget_id,
            x: r.left,
            y: r.top,
            width: (r.right - r.left).max(1) as u32,
            height: (r.bottom - r.top).max(1) as u32,
            finished: true,
        });
    } else if outcome.0 {
        let mut wr = RECT::default();
        if GetWindowRect(hwnd, &mut wr).is_ok() {
            push_event(PlatformEvent::WidgetMoved {
                widget_id,
                x: wr.left,
                y: wr.top,
            });
        }
    } else {
        push_event(PlatformEvent::WidgetLeftClick {
            widget_id,
            point: lparam_point(lparam),
        });
    }
    LRESULT(0)
}

unsafe fn on_widget_rbutton_up(hwnd: HWND, widget_id: u64, lparam: LPARAM) -> LRESULT {
    let pt = lparam_point(lparam);
    let mut screen = POINT { x: pt.x, y: pt.y };
    let _ = ClientToScreen(hwnd, &mut screen);
    push_event(PlatformEvent::WidgetRightClick {
        widget_id,
        screen_point: PixelPoint {
            x: screen.x,
            y: screen.y,
        },
    });
    LRESULT(0)
}

unsafe fn on_widget_pos_changing(hwnd: HWND, widget_id: u64, lparam: LPARAM) -> LRESULT {
    let keep_bottom = WIDGET_META.with(|m| {
        m.borrow()
            .get(&widget_id)
            .is_some_and(|meta| meta.layer == WidgetLayer::Wallpaper)
    });
    if keep_bottom && lparam.0 != 0 {
        let wp = lparam.0 as *mut WINDOWPOS;
        (*wp).hwndInsertAfter = HWND_BOTTOM;
    }
    DefWindowProcW(hwnd, WM_WINDOWPOSCHANGING, WPARAM(0), lparam)
}
