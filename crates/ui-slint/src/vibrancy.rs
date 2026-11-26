use asrbox_platform_api::WidgetLayer;
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
pub fn apply_window_vibrancy(window: &slint::Window, dark: bool) {
    let _ = window.with_winit_window(|w| {
        if window_vibrancy::apply_mica(w, Some(dark)).is_err() {
            let tint = if dark {
                Some((24, 27, 32, 224))
            } else {
                Some((243, 245, 248, 235))
            };
            let _ = window_vibrancy::apply_acrylic(w, tint);
        }
    });
}

#[cfg(not(windows))]
pub fn apply_window_vibrancy(_window: &slint::Window, _dark: bool) {}

#[cfg(windows)]
pub fn apply_acrylic_vibrancy(window: &slint::Window, dark: bool) {
    let _ = window.with_winit_window(|w| {
        let tint = if dark {
            Some((24, 27, 32, 200))
        } else {
            Some((243, 245, 248, 218))
        };
        let _ = window_vibrancy::apply_acrylic(w, tint);
    });
}

#[cfg(not(windows))]
pub fn apply_acrylic_vibrancy(_window: &slint::Window, _dark: bool) {}

pub fn configure_desktop_window(
    window: &slint::Window,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    layer: WidgetLayer,
    dark: bool,
) {
    let _ = window.with_winit_window(|w| {
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowExtWindows;
            w.set_skip_taskbar(true);
        }
        w.set_outer_position(PhysicalPosition::new(x, y));
        let _ = w.request_inner_size(LogicalSize::new(width.max(64), height.max(48)));
        w.set_window_level(to_winit_level(layer));
    });
    apply_acrylic_vibrancy(window, dark);
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
        w.set_window_level(WindowLevel::AlwaysOnTop);
        w.set_outer_position(PhysicalPosition::new(x, y));
        let _ = w.request_inner_size(LogicalSize::new(width.max(120), height.max(80)));
        w.set_visible(true);
        w.focus_window();
    });
    apply_acrylic_vibrancy(window, dark);
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
}

pub fn start_window_drag(window: &slint::Window) {
    let _ = window.with_winit_window(|w| {
        let _ = w.drag_window();
    });
}

pub fn start_resize_se(window: &slint::Window) {
    let _ = window.with_winit_window(|w| {
        let _ = w.drag_resize_window(ResizeDirection::SouthEast);
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
    let _ = window.on_winit_window_event(move |w, ev| {
        match ev {
            WindowEvent::Moved(pos) => on_move(pos.x, pos.y),
            WindowEvent::Resized(size) => {
                let sf = w.scale_factor().max(1.0);
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
    let _ = window.on_winit_window_event(move |_w, ev| {
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
    let _ = window.on_winit_window_event(move |_w, ev| {
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
