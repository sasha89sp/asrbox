use std::cell::RefCell;
use std::ffi::c_void;
use std::path::{Path, PathBuf};

use asrbox_platform_api::{FluentPalette, IconProvider, IconRequest, PixelPoint, PixelRect};
use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, ShowWindow,
    TranslateMessage, MSG, SW_SHOW, WA_INACTIVE, WM_ACTIVATE, WM_CANCELMODE, WM_KEYDOWN,
    WM_KILLFOCUS, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONUP, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::fluent_menu_hook::{
    force_foreground, is_fluent_menu_open, MenuOutsideHookGuard, WM_MENU_DISMISS,
};
use crate::fluent_menu_win::monitor_work_area_at;
use crate::folder_popup_render::{
    clamp_popup_origin, compute_folder_popup_layout, render_folder_popup, FolderEntry,
    FolderPopupLayout,
};
use crate::icons::WindowsIconProvider;
use crate::present_layered_bgra;
use crate::window::{lparam_point, WM_MOUSELEAVE};

const MAX_FOLDER_ENTRIES: usize = 24;

struct FolderPopupSession {
    hwnd: HWND,
    folder_path: PathBuf,
    title: String,
    entries: Vec<FolderEntry>,
    layout: FolderPopupLayout,
    origin: PixelPoint,
    palette: FluentPalette,
    hovered_tile: Option<usize>,
    hovered_explorer: bool,
    tracking_leave: bool,
    result: Option<PathBuf>,
    done: bool,
}

thread_local! {
    static FOLDER_SESSION: RefCell<Option<FolderPopupSession>> = const { RefCell::new(None) };
}

pub(crate) fn show_folder_popup(
    hinstance: HINSTANCE,
    folder_path: &Path,
    anchor_rect: PixelRect,
    palette: FluentPalette,
) -> Result<Option<PathBuf>, String> {
    if is_fluent_menu_open() {
        return Ok(None);
    }
    // SAFETY: Releasing active mouse capture before running a modal folder popup.
    unsafe {
        let _ = ReleaseCapture();
    }
    let entries = load_folder_entries(folder_path);
    let title = folder_path
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| folder_path.display().to_string());
    let anchor_center = PixelPoint {
        x: anchor_rect.x + (anchor_rect.width as i32) / 2,
        y: anchor_rect.y + (anchor_rect.height as i32) / 2,
    };
    let work_area = monitor_work_area_at(anchor_center);
    let layout = compute_folder_popup_layout(&entries);
    let origin = clamp_popup_origin(anchor_rect, &layout, work_area);

    let ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
    // SAFETY: Creating a layered popup window using registered class AsrboxFolderPopupWindow.
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            w!("AsrboxFolderPopupWindow"),
            w!(""),
            WS_POPUP,
            origin.x,
            origin.y,
            layout.total_w as i32,
            layout.total_h as i32,
            None,
            None,
            Some(hinstance),
            None,
        )
        .map_err(|e| format!("CreateWindowExW(folder_popup) failed: {e}"))?
    };

    let frame = render_folder_popup(&title, &entries, &layout, &palette, None, false)?;
    let win_rect = PixelRect {
        x: origin.x,
        y: origin.y,
        width: layout.total_w,
        height: layout.total_h,
    };
    present_layered_bgra(hwnd, win_rect, &frame)?;

    let panel_rect = PixelRect {
        x: origin.x + layout.panel_x,
        y: origin.y + layout.panel_y,
        width: layout.panel_w.max(0) as u32,
        height: layout.panel_h.max(0) as u32,
    };
    let _hook_guard = MenuOutsideHookGuard::install(hwnd, panel_rect);

    let prev = FOLDER_SESSION.with(|cell| {
        cell.borrow_mut().replace(FolderPopupSession {
            hwnd,
            folder_path: folder_path.to_path_buf(),
            title,
            entries,
            layout,
            origin,
            palette,
            hovered_tile: None,
            hovered_explorer: false,
            tracking_leave: false,
            result: None,
            done: false,
        })
    });
    if let Some(old) = prev {
        // SAFETY: Destroying stale popup window on UI thread.
        unsafe {
            let _ = DestroyWindow(old.hwnd);
        }
    }

    // SAFETY: Showing our layered folder popup and moving keyboard/mouse focus to it.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
    }
    force_foreground(hwnd);
    run_folder_modal_loop();

    let session = FOLDER_SESSION.with(|cell| cell.borrow_mut().take());
    if let Some(s) = session {
        let chosen = s.result;
        // SAFETY: Destroying owned popup HWND after leaving modal loop.
        unsafe {
            let _ = DestroyWindow(s.hwnd);
        }
        Ok(chosen)
    } else {
        Ok(None)
    }
}

fn load_folder_entries(folder_path: &Path) -> Vec<FolderEntry> {
    let Ok(rd) = std::fs::read_dir(folder_path) else {
        return Vec::new();
    };
    let mut raw: Vec<(bool, String, PathBuf)> = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        let Some(name_os) = path.file_name() else {
            continue;
        };
        let name = name_os.to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if name.starts_with('.') || lower == "desktop.ini" || lower == "thumbs.db" {
            continue;
        }
        let is_dir = path.is_dir();
        raw.push((is_dir, name, path));
    }
    raw.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.to_ascii_lowercase().cmp(&b.1.to_ascii_lowercase()))
    });
    raw.truncate(MAX_FOLDER_ENTRIES);

    let icons = WindowsIconProvider::new();
    raw.into_iter()
        .map(|(is_dir, name, path)| {
            let icon = icons
                .load_icon(&IconRequest::AutoShell(path.clone()), 64)
                .ok();
            FolderEntry {
                name,
                path,
                is_dir,
                icon,
            }
        })
        .collect()
}

fn run_folder_modal_loop() {
    let mut msg = MSG::default();
    loop {
        let finished = FOLDER_SESSION.with(|c| {
            c.try_borrow()
                .map(|g| g.as_ref().is_none_or(|s| s.done))
                .unwrap_or(true)
        });
        if finished {
            break;
        }
        // SAFETY: Standard Win32 modal message dispatch while folder popup is open.
        let status = unsafe { GetMessageW(&mut msg, None, 0, 0).0 };
        if status <= 0 {
            break;
        }
        // SAFETY: Translating and dispatching valid MSG.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

pub(crate) unsafe extern "system" fn folder_popup_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_MOUSEMOVE => {
            on_mouse_move(hwnd, lparam_point(lparam));
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            on_mouse_leave();
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            on_left_click(lparam_point(lparam));
            LRESULT(0)
        }
        WM_RBUTTONUP | WM_CANCELMODE | WM_MENU_DISMISS => {
            close_popup(None);
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 as u16 == VK_ESCAPE.0 => {
            close_popup(None);
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            let other = HWND(wparam.0 as *mut c_void);
            if other != hwnd {
                close_popup(None);
            }
            LRESULT(0)
        }
        WM_ACTIVATE => {
            if (wparam.0 & 0xFFFF) as u32 == WA_INACTIVE {
                let other = HWND(lparam.0 as *mut c_void);
                if other != hwnd {
                    close_popup(None);
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn on_mouse_move(hwnd: HWND, pt: PixelPoint) {
    FOLDER_SESSION.with(|c| {
        let Ok(mut guard) = c.try_borrow_mut() else {
            return;
        };
        let Some(s) = guard.as_mut() else {
            return;
        };
        if !s.tracking_leave {
            s.tracking_leave = true;
            let mut tme = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            // SAFETY: Valid TRACKMOUSEEVENT for our active popup HWND.
            unsafe {
                let _ = TrackMouseEvent(&mut tme);
            }
        }
        let next_exp = rect_contains(s.layout.explorer_btn, pt);
        let next_tile = s
            .layout
            .tiles
            .iter()
            .find(|t| rect_contains(t.rect, pt))
            .map(|t| t.entry_index);
        if s.hovered_explorer != next_exp || s.hovered_tile != next_tile {
            s.hovered_explorer = next_exp;
            s.hovered_tile = next_tile;
            repaint_popup(s);
        }
    });
}

fn on_mouse_leave() {
    FOLDER_SESSION.with(|c| {
        let Ok(mut guard) = c.try_borrow_mut() else {
            return;
        };
        let Some(s) = guard.as_mut() else {
            return;
        };
        s.tracking_leave = false;
        if s.hovered_explorer || s.hovered_tile.is_some() {
            s.hovered_explorer = false;
            s.hovered_tile = None;
            repaint_popup(s);
        }
    });
}

fn on_left_click(pt: PixelPoint) {
    let chosen = FOLDER_SESSION.with(|c| {
        let Ok(guard) = c.try_borrow() else {
            return None;
        };
        let s = guard.as_ref()?;
        if rect_contains(s.layout.explorer_btn, pt) {
            return Some(s.folder_path.clone());
        }
        s.layout
            .tiles
            .iter()
            .find(|t| rect_contains(t.rect, pt))
            .and_then(|t| s.entries.get(t.entry_index))
            .map(|e| e.path.clone())
    });
    close_popup(chosen);
}

fn repaint_popup(s: &FolderPopupSession) {
    if let Ok(frame) = render_folder_popup(
        &s.title,
        &s.entries,
        &s.layout,
        &s.palette,
        s.hovered_tile,
        s.hovered_explorer,
    ) {
        let rect = PixelRect {
            x: s.origin.x,
            y: s.origin.y,
            width: s.layout.total_w,
            height: s.layout.total_h,
        };
        let _ = present_layered_bgra(s.hwnd, rect, &frame);
    }
}

fn close_popup(chosen: Option<PathBuf>) {
    FOLDER_SESSION.with(|c| {
        if let Ok(mut guard) = c.try_borrow_mut() {
            if let Some(s) = guard.as_mut() {
                s.result = chosen;
                s.done = true;
            }
        }
    });
}

fn rect_contains(r: PixelRect, pt: PixelPoint) -> bool {
    pt.x >= r.x && pt.y >= r.y && pt.x < r.x + r.width as i32 && pt.y < r.y + r.height as i32
}
