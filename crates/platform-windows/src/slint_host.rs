use std::cell::Cell;
use std::ffi::c_void;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Once;

use asrbox_platform_api::{
    AppearanceConfig, DockMenuAction, DockMenuState, FluentMenuSpec, FluentPalette, PixelPoint,
    PixelRect,
};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, ERROR_CLASS_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{ITaskbarList, TaskbarList};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetCursorPos, GetWindowLongPtrW, IsWindowVisible, LoadCursorW,
    RegisterClassExW, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, IDC_ARROW,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE,
    SW_SHOWNOACTIVATE, WINDOW_LONG_PTR_INDEX, WNDCLASSEXW, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

use crate::app_icon::{apply_window_icon, large_app_icon, small_app_icon};
use crate::dock_menu::show_dock_context_menu;
use crate::fluent_menu::{fluent_menu_wnd_proc, show_fluent_menu};
use crate::folder_popup::{folder_popup_wnd_proc, show_folder_popup};

const GWLP_OWNER_INDEX: WINDOW_LONG_PTR_INDEX = WINDOW_LONG_PTR_INDEX(-8);

static POPUP_CLASSES_ONCE: Once = Once::new();

thread_local! {
    static HIDDEN_OWNER_HWND: Cell<isize> = const { Cell::new(0) };
}

fn ensure_hidden_owner_hwnd() -> isize {
    HIDDEN_OWNER_HWND.with(|cell| {
        let cur = cell.get();
        if cur != 0 {
            return cur;
        }
        // SAFETY: Creating a never-shown 0x0 WS_EX_TOOLWINDOW owner using built-in STATIC class.
        let created = unsafe {
            let hmod = GetModuleHandleW(None).ok();
            let hinst = hmod.map(HINSTANCE::from);
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                hinst,
                None,
            )
            .ok()
        };
        let raw = created.map_or(0, |h| h.0 as isize);
        cell.set(raw);
        raw
    })
}

pub fn apply_window_app_icon(hwnd_raw: isize) {
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    apply_window_icon(hwnd);
}

fn delete_taskbar_tab(hwnd: HWND) {
    // SAFETY: Standard STA COM call to ITaskbarList::DeleteTab on UI thread for live HWND.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let created: Result<ITaskbarList, _> =
            CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER);
        if let Ok(list) = created {
            if list.HrInit().is_ok() {
                let _ = list.DeleteTab(hwnd);
            }
        }
    }
}

pub fn set_window_skip_taskbar(hwnd_raw: isize, skip: bool) {
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    let tool_bit = WS_EX_TOOLWINDOW.0 as isize;
    let app_bit = WS_EX_APPWINDOW.0 as isize;
    let owner_target = if skip { ensure_hidden_owner_hwnd() } else { 0 };
    // SAFETY: `hwnd_raw` is a live top-level Win32 HWND owned by the caller on the UI thread.
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let next_ex = if skip {
            (ex | tool_bit) & !app_bit
        } else {
            (ex & !tool_bit) | app_bit
        };
        let cur_owner = GetWindowLongPtrW(hwnd, GWLP_OWNER_INDEX);
        let need_owner = skip && owner_target != 0 && cur_owner != owner_target;
        if next_ex != ex || need_owner {
            let was_visible = IsWindowVisible(hwnd).as_bool();
            if was_visible {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            if need_owner {
                SetWindowLongPtrW(hwnd, GWLP_OWNER_INDEX, owner_target);
            }
            if next_ex != ex {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_ex);
            }
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
            if was_visible {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
    }
    if skip {
        delete_taskbar_tab(hwnd);
    }
}

pub fn move_window_phys(hwnd_raw: isize, x: i32, y: i32) {
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    let flags = SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
    // SAFETY: `hwnd_raw` is a valid Win32 HWND; SetWindowPos moves without changing size/z-order.
    unsafe {
        let _ = SetWindowPos(hwnd, None, x, y, 0, 0, flags);
    }
}

pub fn set_window_bounds_phys(hwnd_raw: isize, x: i32, y: i32, width: i32, height: i32) {
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    let flags = SWP_NOZORDER | SWP_NOACTIVATE;
    let w = width.max(1);
    let h = height.max(1);
    // SAFETY: `hwnd_raw` is a valid Win32 HWND; SetWindowPos applies exact physical rect.
    unsafe {
        let _ = SetWindowPos(hwnd, None, x, y, w, h, flags);
    }
}

pub fn cursor_screen_pos() -> PixelPoint {
    let mut pt = POINT::default();
    // SAFETY: `pt` is valid writable storage for GetCursorPos.
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    PixelPoint { x: pt.x, y: pt.y }
}

pub fn show_dock_native_menu(
    hwnd_raw: isize,
    state: &DockMenuState,
    appearance: AppearanceConfig,
) -> Result<Option<DockMenuAction>, String> {
    let hinstance = ensure_popup_classes_registered()?;
    let hwnd = HWND(hwnd_raw as *mut c_void);
    let pt = cursor_screen_pos();
    let palette = palette_from_appearance(appearance);
    show_dock_context_menu(hinstance, hwnd, pt, state, palette)
}

pub fn show_folder_native_popup(
    folder_path: &Path,
    appearance: AppearanceConfig,
) -> Result<Option<PathBuf>, String> {
    let hinstance = ensure_popup_classes_registered()?;
    let pt = cursor_screen_pos();
    let anchor = PixelRect {
        x: pt.x.saturating_sub(24),
        y: pt.y.saturating_sub(24),
        width: 48,
        height: 48,
    };
    let palette = palette_from_appearance(appearance);
    show_folder_popup(hinstance, folder_path, anchor, palette)
}

pub fn show_native_fluent_menu(
    hwnd_raw: isize,
    spec: &FluentMenuSpec,
    appearance: AppearanceConfig,
) -> Result<Option<u32>, String> {
    let hinstance = ensure_popup_classes_registered()?;
    let hwnd = HWND(hwnd_raw as *mut c_void);
    let pt = cursor_screen_pos();
    let palette = palette_from_appearance(appearance);
    show_fluent_menu(hinstance, hwnd, pt, spec, palette)
}

fn palette_from_appearance(app: AppearanceConfig) -> FluentPalette {
    FluentPalette::from_appearance(app.design_theme, app.theme, app.accent)
}

fn ensure_popup_classes_registered() -> Result<HINSTANCE, String> {
    // SAFETY: Querying current module handle and registering popup window classes once.
    let hmod =
        unsafe { GetModuleHandleW(None) }.map_err(|e| format!("GetModuleHandleW failed: {e}"))?;
    let hinstance = HINSTANCE::from(hmod);
    POPUP_CLASSES_ONCE.call_once(|| {
        let _ = register_popup_class(
            hinstance,
            w!("AsrboxFluentMenuWindow"),
            Some(fluent_menu_wnd_proc),
        );
        let _ = register_popup_class(
            hinstance,
            w!("AsrboxFolderPopupWindow"),
            Some(folder_popup_wnd_proc),
        );
    });
    Ok(hinstance)
}

fn register_popup_class(
    hinstance: HINSTANCE,
    class_name: PCWSTR,
    wnd_proc: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>,
) -> Result<(), String> {
    // SAFETY: Registering a Win32 popup window class for this module; duplicate registration is ok.
    unsafe {
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let wc = WNDCLASSEXW {
            cbSize: u32::try_from(mem::size_of::<WNDCLASSEXW>()).unwrap_or(0),
            lpfnWndProc: wnd_proc,
            hInstance: hinstance,
            hIcon: large_app_icon(),
            hIconSm: small_app_icon(),
            hCursor: cursor,
            hbrBackground: HBRUSH::default(),
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
            return Err("RegisterClassExW(popup) failed".to_owned());
        }
    }
    Ok(())
}
