use asrbox_platform_api::{PixelRect, WidgetLayer};
use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::WindowEvent;
use winit::window::{ResizeDirection, WindowLevel};

pub fn select_winit_backend() -> Result<(), String> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()
        .map_err(|e| format!("slint winit backend: {e}"))
}

#[cfg(windows)]
fn extract_hwnd(w: &winit::window::Window) -> isize {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match w.window_handle() {
        Ok(handle) => match handle.as_raw() {
            RawWindowHandle::Win32(win32) => win32.hwnd.get(),
            _ => 0,
        },
        Err(_) => 0,
    }
}

#[cfg(windows)]
pub fn apply_window_vibrancy(window: &slint::Window, dark: bool) {
    let _ = window.with_winit_window(|w| {
        w.set_resizable(true);
        let hwnd = extract_hwnd(w);
        asrbox_platform_windows::apply_dwm_frameless_style(hwnd, dark, false);
        asrbox_platform_windows::force_redraw_hwnd(hwnd);
    });
    window.request_redraw();
}

#[cfg(not(windows))]
pub fn apply_window_vibrancy(window: &slint::Window, _dark: bool) {
    window.request_redraw();
}

#[cfg(windows)]
pub fn apply_acrylic_vibrancy(window: &slint::Window, dark: bool) {
    let _ = window.with_winit_window(|w| {
        let hwnd = extract_hwnd(w);
        asrbox_platform_windows::apply_dwm_frameless_style(hwnd, dark, false);
        asrbox_platform_windows::force_redraw_hwnd(hwnd);
    });
    window.request_redraw();
}

#[cfg(not(windows))]
pub fn apply_acrylic_vibrancy(window: &slint::Window, _dark: bool) {
    window.request_redraw();
}

#[cfg(windows)]
pub fn apply_window_opacity(window: &slint::Window, opacity_percent: u8) {
    let _ = window.with_winit_window(|w| {
        let hwnd = extract_hwnd(w);
        asrbox_platform_windows::set_window_opacity_alpha(hwnd, opacity_percent);
    });
}

#[cfg(not(windows))]
pub fn apply_window_opacity(_window: &slint::Window, _opacity_percent: u8) {}

#[cfg(windows)]
pub fn force_window_repaint(window: &slint::Window) {
    let _ = window.with_winit_window(|w| {
        asrbox_platform_windows::force_redraw_hwnd(extract_hwnd(w));
    });
    window.request_redraw();
}

#[cfg(not(windows))]
pub fn force_window_repaint(window: &slint::Window) {
    window.request_redraw();
}

pub fn center_window_in_work_area(
    window: &slint::Window,
    work_area: PixelRect,
    width: u32,
    height: u32,
    min_w: u32,
    min_h: u32,
) {
    let _ = window.with_winit_window(|w| {
        w.set_resizable(true);
        w.set_min_inner_size(Some(LogicalSize::new(min_w, min_h)));
        let _ = w.request_inner_size(LogicalSize::new(width.max(min_w), height.max(min_h)));
        let sf = w.scale_factor().max(1.0);
        let phys_w = (f64::from(width.max(min_w)) * sf).round() as i32;
        let phys_h = (f64::from(height.max(min_h)) * sf).round() as i32;
        let x = (work_area.x + (work_area.width as i32 - phys_w) / 2).max(work_area.x + 8);
        let y = (work_area.y + (work_area.height as i32 - phys_h) / 2).max(work_area.y + 8);
        w.set_outer_position(PhysicalPosition::new(x, y));
    });
}

pub struct DesktopWindowSpec {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub min_w: u32,
    pub min_h: u32,
    pub resizable: bool,
    pub layer: WidgetLayer,
    pub dark: bool,
    pub opacity_percent: u8,
}

pub fn configure_desktop_window(window: &slint::Window, spec: DesktopWindowSpec) {
    let _ = window.with_winit_window(|w| {
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowExtWindows;
            w.set_skip_taskbar(true);
        }
        w.set_resizable(spec.resizable);
        if spec.resizable {
            w.set_min_inner_size(Some(LogicalSize::new(spec.min_w, spec.min_h)));
        }
        w.set_outer_position(PhysicalPosition::new(spec.x, spec.y));
        let lw = spec.width.max(spec.min_w);
        let lh = spec.height.max(spec.min_h);
        let _ = w.request_inner_size(LogicalSize::new(lw, lh));
        w.set_window_level(to_winit_level(spec.layer));
        #[cfg(windows)]
        {
            let hwnd = extract_hwnd(w);
            asrbox_platform_windows::apply_dwm_frameless_style(hwnd, spec.dark, false);
            asrbox_platform_windows::set_window_opacity_alpha(hwnd, spec.opacity_percent);
            asrbox_platform_windows::force_redraw_hwnd(hwnd);
        }
        #[cfg(not(windows))]
        let _ = (spec.dark, spec.opacity_percent);
    });
    window.request_redraw();
}

pub fn configure_popup_window(
    window: &slint::Window,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    dark: bool,
) {
    let _ = window.with_winit_window(|w| {
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowExtWindows;
            w.set_skip_taskbar(true);
        }
        w.set_resizable(false);
        w.set_window_level(WindowLevel::AlwaysOnTop);
        let _ = w.request_inner_size(LogicalSize::new(width.max(120), height.max(80)));
        w.set_outer_position(PhysicalPosition::new(x, y));
        #[cfg(windows)]
        {
            let hwnd = extract_hwnd(w);
            asrbox_platform_windows::apply_dwm_frameless_style(hwnd, dark, false);
            asrbox_platform_windows::force_redraw_hwnd(hwnd);
        }
        #[cfg(not(windows))]
        let _ = dark;
        w.set_visible(true);
        w.focus_window();
    });
    window.request_redraw();
}

pub fn set_window_layer(window: &slint::Window, layer: WidgetLayer) {
    let _ = window.with_winit_window(|w| {
        w.set_window_level(to_winit_level(layer));
    });
}

pub fn move_window_to(window: &slint::Window, x: i32, y: i32) {
    let _ = window.with_winit_window(|w| {
        w.set_outer_position(PhysicalPosition::new(x, y));
    });
}

pub fn resize_window_to(window: &slint::Window, width: u32, height: u32) {
    let _ = window.with_winit_window(|w| {
        let _ = w.request_inner_size(LogicalSize::new(width.max(64), height.max(48)));
    });
}

pub fn set_window_visible(window: &slint::Window, visible: bool) {
    let _ = window.with_winit_window(|w| {
        w.set_visible(visible);
    });
    if visible {
        window.request_redraw();
    }
}

pub fn start_window_drag(window: &slint::Window) {
    let _ = window.with_winit_window(|w| {
        let _ = w.drag_window();
    });
}

pub fn start_window_resize(window: &slint::Window, dir: i32) {
    let direction = match dir {
        0 => ResizeDirection::North,
        1 => ResizeDirection::South,
        2 => ResizeDirection::West,
        3 => ResizeDirection::East,
        4 => ResizeDirection::NorthWest,
        5 => ResizeDirection::NorthEast,
        6 => ResizeDirection::SouthWest,
        _ => ResizeDirection::SouthEast,
    };
    let _ = window.with_winit_window(|w| {
        let _ = w.drag_resize_window(direction);
    });
}

pub fn minimize_window(window: &slint::Window) {
    let _ = window.with_winit_window(|w| {
        w.set_minimized(true);
    });
}

pub fn toggle_maximize_window(window: &slint::Window) {
    let _ = window.with_winit_window(|w| {
        w.set_maximized(!w.is_maximized());
    });
}

pub fn install_bounds_tracker<FMove, FResize>(
    window: &slint::Window,
    mut on_move: FMove,
    mut on_resize: FResize,
) where
    FMove: FnMut(i32, i32) + 'static,
    FResize: FnMut(u32, u32) + 'static,
{
    window.on_winit_window_event(move |w, ev| {
        match ev {
            WindowEvent::Moved(pos) => on_move(pos.x, pos.y),
            WindowEvent::Resized(size) => {
                let sf = f64::from(w.scale_factor().max(1.0));
                let lw = (f64::from(size.width) / sf).round().max(64.0) as u32;
                let lh = (f64::from(size.height) / sf).round().max(48.0) as u32;
                on_resize(lw, lh);
            }
            _ => {}
        }
        EventResult::Propagate
    });
}

pub fn install_focus_lost_hook<F>(window: &slint::Window, mut on_blur: F)
where
    F: FnMut() + 'static,
{
    window.on_winit_window_event(move |_w, ev| {
        if let WindowEvent::Focused(false) = ev {
            on_blur();
        }
        EventResult::Propagate
    });
}

pub fn install_dock_window_hooks<FMove, FDrop>(
    window: &slint::Window,
    mut on_move: FMove,
    mut on_drop: FDrop,
) where
    FMove: FnMut(i32, i32) + 'static,
    FDrop: FnMut(std::path::PathBuf) + 'static,
{
    window.on_winit_window_event(move |_w, ev| {
        match ev {
            WindowEvent::Moved(pos) => on_move(pos.x, pos.y),
            WindowEvent::DroppedFile(path) => on_drop(path.clone()),
            _ => {}
        }
        EventResult::Propagate
    });
}

fn to_winit_level(layer: WidgetLayer) -> WindowLevel {
    match layer {
        WidgetLayer::Wallpaper => WindowLevel::AlwaysOnBottom,
        WidgetLayer::Normal => WindowLevel::Normal,
        WidgetLayer::Topmost => WindowLevel::AlwaysOnTop,
    }
}
