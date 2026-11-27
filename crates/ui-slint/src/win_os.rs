use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use asrbox_core::{encode_worker_msg, WorkerMsg};
use asrbox_platform_api::{
    AppearanceConfig, DockMenuAction, DockMenuState, FluentMenuSpec, HttpFetchResult, IconRequest,
    LocalDateTime, PixelRect, RgbaImage, WidgetLayer,
};

pub fn emit_worker_msg(msg: &WorkerMsg) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{}", encode_worker_msg(msg));
    let _ = out.flush();
}

pub fn emit_worker_log(level: &str, module: &str, message: impl Into<String>) {
    emit_worker_msg(&WorkerMsg::Log {
        level: level.to_owned(),
        module: module.to_owned(),
        message: message.into(),
    });
}

pub fn emit_open_settings(tab: &str) {
    emit_worker_msg(&WorkerMsg::OpenSettings {
        tab: tab.to_owned(),
    });
}

pub fn emit_config_changed(module: &str) {
    emit_worker_msg(&WorkerMsg::ConfigChanged {
        module: module.to_owned(),
    });
}

pub fn layer_to_idx(layer: WidgetLayer) -> i32 {
    match layer {
        WidgetLayer::Wallpaper => 0,
        WidgetLayer::Normal => 1,
        WidgetLayer::Topmost => 2,
    }
}

pub fn idx_to_layer(idx: i32) -> WidgetLayer {
    match idx {
        1 => WidgetLayer::Normal,
        2 => WidgetLayer::Topmost,
        _ => WidgetLayer::Wallpaper,
    }
}

#[cfg(windows)]
pub fn is_parent_alive(pid: u32) -> bool {
    pid == 0 || asrbox_platform_windows::is_process_alive(pid)
}

#[cfg(not(windows))]
pub fn is_parent_alive(_pid: u32) -> bool {
    true
}

#[cfg(windows)]
pub fn local_time_now() -> LocalDateTime {
    asrbox_platform_windows::query_local_time()
}

#[cfg(not(windows))]
pub fn local_time_now() -> LocalDateTime {
    LocalDateTime {
        year: 2026,
        month: 1,
        day: 1,
        hour: 12,
        minute: 0,
        second: 0,
        utc_offset_min: 0,
    }
}

#[cfg(windows)]
pub fn primary_work_area() -> PixelRect {
    asrbox_platform_windows::primary_work_area().unwrap_or(PixelRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
    })
}

#[cfg(not(windows))]
pub fn primary_work_area() -> PixelRect {
    PixelRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
    }
}

#[cfg(windows)]
pub fn cred_load(target: &str) -> Result<Option<(String, String)>, String> {
    asrbox_platform_windows::cred_load(target)
}

#[cfg(not(windows))]
pub fn cred_load(_target: &str) -> Result<Option<(String, String)>, String> {
    Ok(None)
}

#[cfg(windows)]
pub fn http_get(url: &str, headers: &[(String, String)]) -> HttpFetchResult {
    asrbox_platform_windows::winhttp_fetch(url, headers)
}

#[cfg(not(windows))]
pub fn http_get(_url: &str, _headers: &[(String, String)]) -> HttpFetchResult {
    HttpFetchResult {
        status_code: 0,
        body: Vec::new(),
        error: Some("WinHTTP unavailable on non-Windows".to_owned()),
    }
}

#[cfg(windows)]
pub fn open_text_file(path: &Path) -> Result<(), String> {
    asrbox_platform_windows::open_in_notepad(path)
}

#[cfg(not(windows))]
pub fn open_text_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn open_url(url: &str) -> Result<(), String> {
    asrbox_platform_windows::open_url_str(url)
}

#[cfg(not(windows))]
pub fn open_url(_url: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn launch_system_tool(cmd: &str, args: &str) -> Result<(), String> {
    asrbox_platform_windows::launch_system_tool(cmd, args)
}

#[cfg(not(windows))]
pub fn launch_system_tool(_cmd: &str, _args: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn open_item_location(path: &Path) -> Result<(), String> {
    asrbox_platform_windows::open_item_location(path)
}

#[cfg(not(windows))]
pub fn open_item_location(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn pick_icon_file() -> Result<Option<PathBuf>, String> {
    asrbox_platform_windows::pick_icon_file()
}

#[cfg(not(windows))]
pub fn pick_icon_file() -> Result<Option<PathBuf>, String> {
    Ok(None)
}

#[cfg(windows)]
pub fn activate_or_launch(path: &Path, focus_running: bool) -> Result<bool, String> {
    asrbox_platform_windows::activate_or_launch_target(path, focus_running)
}

#[cfg(not(windows))]
pub fn activate_or_launch(_path: &Path, _focus_running: bool) -> Result<bool, String> {
    Ok(false)
}

#[cfg(windows)]
pub fn running_exe_names() -> HashSet<String> {
    asrbox_platform_windows::query_running_exe_names()
}

#[cfg(not(windows))]
pub fn running_exe_names() -> HashSet<String> {
    HashSet::new()
}

#[cfg(windows)]
pub fn resolve_exe_name(path: &Path) -> Option<String> {
    asrbox_platform_windows::resolve_target_exe_name(path)
}

#[cfg(not(windows))]
pub fn resolve_exe_name(_path: &Path) -> Option<String> {
    None
}

#[cfg(windows)]
pub fn load_shell_icon(request: &IconRequest, target_px: u32) -> Result<RgbaImage, String> {
    use asrbox_platform_api::IconProvider;
    let provider = asrbox_platform_windows::WindowsIconProvider::new();
    provider.load_icon(request, target_px)
}

#[cfg(not(windows))]
pub fn load_shell_icon(_request: &IconRequest, _target_px: u32) -> Result<RgbaImage, String> {
    Err("icons unavailable on non-Windows".to_owned())
}

#[cfg(windows)]
pub fn show_dock_native_menu(
    hwnd_raw: isize,
    state: &DockMenuState,
    appearance: AppearanceConfig,
) -> Result<Option<DockMenuAction>, String> {
    asrbox_platform_windows::show_dock_native_menu(hwnd_raw, state, appearance)
}

#[cfg(not(windows))]
pub fn show_dock_native_menu(
    _hwnd_raw: isize,
    _state: &DockMenuState,
    _appearance: AppearanceConfig,
) -> Result<Option<DockMenuAction>, String> {
    Ok(None)
}

#[cfg(windows)]
pub fn show_folder_native_popup(
    folder_path: &Path,
    appearance: AppearanceConfig,
) -> Result<Option<PathBuf>, String> {
    asrbox_platform_windows::show_folder_native_popup(folder_path, appearance)
}

#[cfg(not(windows))]
pub fn show_folder_native_popup(
    _folder_path: &Path,
    _appearance: AppearanceConfig,
) -> Result<Option<PathBuf>, String> {
    Ok(None)
}

#[cfg(windows)]
pub fn show_native_fluent_menu(
    hwnd_raw: isize,
    spec: &FluentMenuSpec,
    appearance: AppearanceConfig,
) -> Result<Option<u32>, String> {
    asrbox_platform_windows::show_native_fluent_menu(hwnd_raw, spec, appearance)
}

#[cfg(not(windows))]
pub fn show_native_fluent_menu(
    _hwnd_raw: isize,
    _spec: &FluentMenuSpec,
    _appearance: AppearanceConfig,
) -> Result<Option<u32>, String> {
    Ok(None)
}
