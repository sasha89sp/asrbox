use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use asrbox_mod_widgets::{
    load_sysmon_config, save_sysmon_config, SysMonitorConfig, MAX_REFRESH_MS, MAX_SYSMON_HEIGHT,
    MAX_SYSMON_WIDTH, MIN_REFRESH_MS, MIN_SYSMON_HEIGHT, MIN_SYSMON_WIDTH,
};
use asrbox_platform_api::{AppearanceConfig, SysTelemetry, WidgetMenuVisibility};
use slint::ComponentHandle;

use crate::generated::ui::SysMonitorWindow;
use crate::settings_state::apply_theme_to_window;
use crate::vibrancy::{
    apply_acrylic_vibrancy, configure_desktop_window, install_bounds_tracker, resize_window_to,
    set_window_layer, set_window_visible, start_resize_se, start_window_drag,
};
use crate::widgets_sysmon_view::sync_sysmon_ui;
use crate::win_os::{
    emit_config_changed, emit_open_settings, idx_to_layer, launch_system_tool, open_text_file,
};

thread_local! {
    static SYSMON_SLOT: RefCell<Option<SharedSysMon>> = const { RefCell::new(None) };
}

pub struct SysMonSlintState {
    pub ui: SysMonitorWindow,
    pub cfg_path: PathBuf,
    pub cfg: SysMonitorConfig,
    pub data: SysTelemetry,
    pub vis: WidgetMenuVisibility,
    pub dark: bool,
    pub window_shown: bool,
    pub sampler_active: Arc<AtomicBool>,
    pub sampler_stop: Arc<AtomicBool>,
    pub sampler_ms: Arc<AtomicU32>,
    pub sampler_kick: Arc<AtomicBool>,
}

pub type SharedSysMon = Rc<RefCell<SysMonSlintState>>;

pub fn create_sysmon_controller(
    cfg_path: PathBuf,
    appearance: AppearanceConfig,
    vis: WidgetMenuVisibility,
    safe_mode: bool,
) -> Result<SharedSysMon, String> {
    let ui = SysMonitorWindow::new().map_err(|e| format!("create SysMonitorWindow: {e}"))?;
    let dark = apply_theme_to_window(&ui, appearance);
    let cfg = load_sysmon_config(&cfg_path).unwrap_or_default();
    let sampler_active = Arc::new(AtomicBool::new(false));
    let sampler_stop = Arc::new(AtomicBool::new(false));
    let sampler_ms = Arc::new(AtomicU32::new(cfg.refresh_interval_ms));
    let sampler_kick = Arc::new(AtomicBool::new(true));

    let state = Rc::new(RefCell::new(SysMonSlintState {
        ui,
        cfg_path,
        cfg,
        data: SysTelemetry::default(),
        vis,
        dark,
        window_shown: false,
        sampler_active,
        sampler_stop,
        sampler_ms,
        sampler_kick,
    }));
    SYSMON_SLOT.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&state)));
    wire_sysmon_callbacks(&state);
    spawn_sysmon_sampler(&state);
    apply_sysmon_window_state(&state, safe_mode);
    Ok(state)
}

pub fn clear_sysmon_slot() {
    SYSMON_SLOT.with(|slot| *slot.borrow_mut() = None);
}

pub fn reload_sysmon(
    state: &SharedSysMon,
    appearance: AppearanceConfig,
    vis: WidgetMenuVisibility,
    safe_mode: bool,
) {
    {
        let mut s = state.borrow_mut();
        if let Ok(fresh) = load_sysmon_config(&s.cfg_path) {
            s.cfg = fresh;
        }
        s.vis = vis;
        s.sampler_ms
            .store(s.cfg.refresh_interval_ms, Ordering::Relaxed);
        let dark = apply_theme_to_window(&s.ui, appearance);
        if dark != s.dark {
            s.dark = dark;
            apply_acrylic_vibrancy(s.ui.window(), dark);
        }
    }
    apply_sysmon_window_state(state, safe_mode);
}

pub fn toggle_sysmon_visible(state: &SharedSysMon) {
    {
        let mut s = state.borrow_mut();
        s.cfg.visible = !s.cfg.visible;
        let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
    }
    apply_sysmon_window_state(state, false);
    emit_config_changed("widgets");
}

pub fn kick_sysmon_sample(state: &SharedSysMon) {
    state.borrow().sampler_kick.store(true, Ordering::Relaxed);
}

pub fn stop_sysmon_sampler(state: &SharedSysMon) {
    state.borrow().sampler_stop.store(true, Ordering::Relaxed);
}

fn apply_sysmon_window_state(state: &SharedSysMon, safe_mode: bool) {
    let mut s = state.borrow_mut();
    let show = !safe_mode && s.cfg.visible;
    s.sampler_active.store(show, Ordering::Relaxed);
    if show {
        if !s.window_shown {
            let _ = s.ui.show();
            configure_desktop_window(
                s.ui.window(),
                s.cfg.x,
                s.cfg.y,
                s.cfg.width,
                s.cfg.height,
                s.cfg.layer,
                s.dark,
            );
            s.window_shown = true;
        } else {
            set_window_visible(s.ui.window(), true);
            set_window_layer(s.ui.window(), s.cfg.layer);
            resize_window_to(s.ui.window(), s.cfg.width, s.cfg.height);
        }
        s.sampler_kick.store(true, Ordering::Relaxed);
        sync_sysmon_ui(&s);
    } else if s.window_shown {
        set_window_visible(s.ui.window(), false);
        s.window_shown = false;
    }
}

fn spawn_sysmon_sampler(state: &SharedSysMon) {
    let active = Arc::clone(&state.borrow().sampler_active);
    let stop = Arc::clone(&state.borrow().sampler_stop);
    let interval_ms = Arc::clone(&state.borrow().sampler_ms);
    let kick = Arc::clone(&state.borrow().sampler_kick);

    thread::spawn(move || {
        #[cfg(windows)]
        let mut collector = asrbox_platform_windows::SysTelemetryCollector::new();
        while !stop.load(Ordering::Relaxed) {
            if !active.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(300));
                continue;
            }
            kick.store(false, Ordering::Relaxed);
            #[cfg(windows)]
            let snap = collector.sample();
            #[cfg(not(windows))]
            let snap = SysTelemetry::default();
            let _ = slint::invoke_from_event_loop(move || {
                SYSMON_SLOT.with(|slot| {
                    if let Some(rc) = slot.borrow().as_ref().cloned() {
                        let mut s = rc.borrow_mut();
                        s.data = snap;
                        if s.window_shown {
                            sync_sysmon_ui(&s);
                        }
                    }
                });
            });
            let wait_ms = interval_ms
                .load(Ordering::Relaxed)
                .clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
            let mut slept = 0u32;
            while slept < wait_ms
                && !stop.load(Ordering::Relaxed)
                && !kick.load(Ordering::Relaxed)
                && active.load(Ordering::Relaxed)
            {
                let step = (wait_ms - slept).min(200);
                thread::sleep(Duration::from_millis(u64::from(step)));
                slept += step;
            }
        }
    });
}

fn wire_sysmon_callbacks(state: &SharedSysMon) {
    let ui = &state.borrow().ui;
    let w = ui.as_weak();
    ui.on_drag_window(move || {
        if let Some(u) = w.upgrade() {
            start_window_drag(u.window());
        }
    });
    let w = ui.as_weak();
    ui.on_start_resize(move || {
        if let Some(u) = w.upgrade() {
            start_resize_se(u.window());
        }
    });
    ui.on_open_settings(|| emit_open_settings("sysmon"));
    let st = Rc::clone(state);
    ui.on_refresh_now(move || kick_sysmon_sample(&st));
    ui.on_open_section_tool(|idx| {
        let (cmd, args) = match idx {
            1 => ("dxdiag.exe", ""),
            2 => ("perfmon.exe", "/res"),
            3 => ("ncpa.cpl", ""),
            _ => ("taskmgr.exe", ""),
        };
        let _ = launch_system_tool(cmd, args);
    });
    wire_sysmon_menu_callbacks(state);
    let st_m = Rc::clone(state);
    let st_r = Rc::clone(state);
    install_bounds_tracker(
        state.borrow().ui.window(),
        move |x, y| {
            let mut s = st_m.borrow_mut();
            if s.window_shown && (s.cfg.x != x || s.cfg.y != y) {
                s.cfg.x = x;
                s.cfg.y = y;
                let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
            }
        },
        move |w, h| {
            let mut s = st_r.borrow_mut();
            let cw = w.clamp(MIN_SYSMON_WIDTH, MAX_SYSMON_WIDTH);
            let ch = h.clamp(MIN_SYSMON_HEIGHT, MAX_SYSMON_HEIGHT);
            if s.window_shown && (s.cfg.width != cw || s.cfg.height != ch) {
                s.cfg.width = cw;
                s.cfg.height = ch;
                let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
            }
        },
    );
}

fn wire_sysmon_menu_callbacks(state: &SharedSysMon) {
    let ui = &state.borrow().ui;
    let st = Rc::clone(state);
    ui.on_set_layer(move |idx| {
        let mut s = st.borrow_mut();
        s.cfg.layer = idx_to_layer(idx);
        set_window_layer(s.ui.window(), s.cfg.layer);
        let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
        sync_sysmon_ui(&s);
        emit_config_changed("widgets");
    });
    let st = Rc::clone(state);
    ui.on_set_opacity(move |pct| {
        let mut s = st.borrow_mut();
        s.cfg.opacity_percent = pct.clamp(20, 100) as u8;
        let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
        sync_sysmon_ui(&s);
        emit_config_changed("widgets");
    });
    let st = Rc::clone(state);
    ui.on_set_refresh_ms(move |ms| {
        let mut s = st.borrow_mut();
        let clamped = (ms.max(500) as u32).clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
        s.cfg.refresh_interval_ms = clamped;
        s.sampler_ms.store(clamped, Ordering::Relaxed);
        let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
        sync_sysmon_ui(&s);
        emit_config_changed("widgets");
    });
    let st = Rc::clone(state);
    ui.on_open_config(move || {
        let path = st.borrow().cfg_path.clone();
        let _ = open_text_file(&path);
    });
    let st = Rc::clone(state);
    ui.on_hide_widget(move || toggle_sysmon_visible(&st));
}
