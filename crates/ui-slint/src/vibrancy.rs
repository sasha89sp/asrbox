use slint::winit_030::WinitWindowAccessor;

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

pub fn start_window_drag(window: &slint::Window) {
    let _ = window.with_winit_window(|w| {
        let _ = w.drag_window();
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
