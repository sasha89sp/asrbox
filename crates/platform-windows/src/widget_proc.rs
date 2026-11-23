use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem;

use asrbox_platform_api::{
    PixelPoint, PlatformEvent, WidgetLayer, WidgetMenuAction, WidgetMenuState,
};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
    DWM_SYSTEMBACKDROP_TYPE,
};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DefWindowProcW, DestroyMenu, GetCursorPos, GetWindowLongPtrW, GetWindowRect,
    PostMessageW, SetForegroundWindow, SetWindowPos, TrackPopupMenuEx, GWLP_USERDATA, HWND_BOTTOM,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOWPOS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_NULL, WM_RBUTTONUP, WM_TIMER, WM_WINDOWPOSCHANGING,
};

use crate::tray::{add_item, add_sep};
use crate::window::{lparam_point, push_event, WM_MOUSELEAVE};

pub(crate) const WIDGET_MINUTE_TIMER_ID: usize = 101;
pub(crate) const WIDGET_REFRESH_TIMER_ID: usize = 102;

const CMD_REFRESH: u32 = 401;
const CMD_OPEN_CFG: u32 = 402;
const CMD_LAYER_WALLPAPER: u32 = 403;
const CMD_LAYER_NORMAL: u32 = 404;
const CMD_LAYER_TOPMOST: u32 = 405;
const CMD_TOGGLE_ACRYLIC: u32 = 406;
const CMD_HIDE: u32 = 407;

#[derive(Clone, Copy)]
struct WidgetDragMeta {
    layer: WidgetLayer,
    drag_header_h: i32,
    pressing_header: bool,
    dragging: bool,
    press_screen: POINT,
    win_origin: POINT,
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
                pressing_header: false,
                dragging: false,
                press_screen: POINT::default(),
                win_origin: POINT::default(),
            },
        );
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

pub(crate) fn apply_widget_layer(hwnd: HWND, layer: WidgetLayer, acrylic: bool) {
    let insert_after = match layer {
        WidgetLayer::Wallpaper => HWND_BOTTOM,
        WidgetLayer::Normal => HWND_NOTOPMOST,
        WidgetLayer::Topmost => HWND_TOPMOST,
    };
    let backdrop: DWM_SYSTEMBACKDROP_TYPE = if acrylic {
        DWMSBT_TRANSIENTWINDOW
    } else {
        DWMSBT_NONE
    };
    // SAFETY: Setting z-order and optional Windows 11 backdrop attribute on an owned HWND.
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
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const DWM_SYSTEMBACKDROP_TYPE as *const c_void,
            u32::try_from(mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>()).unwrap_or(4),
        );
    }
}

pub(crate) fn show_widget_context_menu(
    hwnd: HWND,
    screen_pt: PixelPoint,
    state: &WidgetMenuState,
) -> Result<Option<WidgetMenuAction>, String> {
    // SAFETY: Popup menu is created, tracked, and destroyed synchronously on UI thread.
    unsafe {
        let menu = CreatePopupMenu().map_err(|err| format!("CreatePopupMenu failed: {err}"))?;
        add_item(menu, CMD_REFRESH, "Обновить календарь сейчас", false, true);
        add_item(menu, CMD_OPEN_CFG, "Открыть настройки", false, true);
        add_sep(menu);
        let is_wall = state.layer == WidgetLayer::Wallpaper;
        let is_norm = state.layer == WidgetLayer::Normal;
        let is_top = state.layer == WidgetLayer::Topmost;
        add_item(menu, CMD_LAYER_WALLPAPER, "Слой: На обоях", is_wall, true);
        add_item(menu, CMD_LAYER_NORMAL, "Слой: Обычное окно", is_norm, true);
        add_item(menu, CMD_LAYER_TOPMOST, "Слой: Поверх всех", is_top, true);
        add_sep(menu);
        let acr = state.acrylic;
        add_item(menu, CMD_TOGGLE_ACRYLIC, "Размытие Acrylic", acr, true);
        add_item(menu, CMD_HIDE, "Скрыть виджет", false, true);

        let _ = SetForegroundWindow(hwnd);
        let flags = (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0;
        let raw = TrackPopupMenuEx(menu, flags, screen_pt.x, screen_pt.y, hwnd, None);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        Ok(match raw.0 as u32 {
            CMD_REFRESH => Some(WidgetMenuAction::Refresh),
            CMD_OPEN_CFG => Some(WidgetMenuAction::OpenConfig),
            CMD_LAYER_WALLPAPER => Some(WidgetMenuAction::SetLayerWallpaper),
            CMD_LAYER_NORMAL => Some(WidgetMenuAction::SetLayerNormal),
            CMD_LAYER_TOPMOST => Some(WidgetMenuAction::SetLayerTopmost),
            CMD_TOGGLE_ACRYLIC => Some(WidgetMenuAction::ToggleAcrylic),
            CMD_HIDE => Some(WidgetMenuAction::HideWidget),
            _ => None,
        })
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

unsafe fn on_widget_lbutton_down(hwnd: HWND, widget_id: u64, lparam: LPARAM) -> LRESULT {
    let pt = lparam_point(lparam);
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    let mut wr = RECT::default();
    let _ = GetWindowRect(hwnd, &mut wr);
    let captured = WIDGET_META.with(|m| {
        if let Some(meta) = m.borrow_mut().get_mut(&widget_id) {
            if pt.y < meta.drag_header_h {
                meta.pressing_header = true;
                meta.dragging = false;
                meta.press_screen = cursor;
                meta.win_origin = POINT {
                    x: wr.left,
                    y: wr.top,
                };
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
    let drag_target = WIDGET_META.with(|m| {
        let mut map = m.borrow_mut();
        let meta = map.get_mut(&widget_id)?;
        if !meta.pressing_header {
            return None;
        }
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let dx = cursor.x - meta.press_screen.x;
        let dy = cursor.y - meta.press_screen.y;
        if !meta.dragging && dx.abs() + dy.abs() >= 4 {
            meta.dragging = true;
        }
        if meta.dragging {
            Some((meta.win_origin.x + dx, meta.win_origin.y + dy))
        } else {
            None
        }
    });
    if let Some((nx, ny)) = drag_target {
        let _ = SetWindowPos(hwnd, None, nx, ny, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        return LRESULT(0);
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
    let was_drag = WIDGET_META.with(|m| {
        if let Some(meta) = m.borrow_mut().get_mut(&widget_id) {
            let d = meta.dragging;
            meta.pressing_header = false;
            meta.dragging = false;
            return d;
        }
        false
    });
    let _ = ReleaseCapture();
    if was_drag {
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
