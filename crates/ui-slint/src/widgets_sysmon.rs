use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use asrbox_mod_widgets::{
    load_sysmon_config, save_sysmon_config, SysMonitorConfig, MAX_REFRESH_MS, MAX_SYSMON_HEIGHT,
    MAX_SYSMON_WIDTH, MIN_REFRESH_MS, MIN_SYSMON_HEIGHT, MIN_SYSMON_WIDTH,
};
use asrbox_platform_api::{AppearanceConfig, SysTelemetry, WidgetMenuVisibility};
use slint::{ComponentHandle, Timer};

use crate::generated::ui::SysMonitorWindow;
use crate::settings_state::apply_theme_to_window;
use crate::vibrancy::{
    apply_acrylic_vibrancy, apply_window_opacity, configure_desktop_window, enforce_skip_taskbar,
    install_bounds_tracker, move_window_to, resize_window_to, set_window_layer, set_window_visible,
    start_window_drag, start_window_resize, window_hwnd_raw, DesktopWindowSpec,
};
use crate::widgets_menu::{show_widget_context_menu, WidgetMenuChoice, WidgetMenuParams};
use crate::widgets_sysmon_view::sync_sysmon_ui;
use crate::win_os::{emit_config_changed, emit_open_settings, launch_system_tool, open_text_file};

thread_local! {
    static SYSMON_SLOT: RefCell<Option<SharedSysMon>> = const { RefCell::new(None) };
}

pub struct SysMonSlintState {
    pub ui: SysMonitorWindow,
    pub cfg_path: PathBuf,
    pub cfg: SysMonitorConfig,
    pub data: SysTelemetry,
    pub appearance: AppearanceConfig,
    pub vis: WidgetMenuVisibility,
    pub dark: bool,
    pub window_shown: bool,
    pub ignore_bounds_until: Instant,
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
        appearance,
        vis,
        dark,
        window_shown: false,
        ignore_bounds_until: Instant::now() + Duration::from_millis(800),
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
        s.appearance = appearance;
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
        s.ignore_bounds_until = Instant::now() + Duration::from_millis(700);
        if !s.window_shown {
            let (saved_x, saved_y) = (s.cfg.x, s.cfg.y);
            let _ = s.ui.show();
            configure_desktop_window(
                s.ui.window(),
                DesktopWindowSpec {
                    x: saved_x,
                    y: saved_y,
                    width: s.cfg.width,
                    height: s.cfg.height,
                    min_w: MIN_SYSMON_WIDTH.min(380),
                    min_h: MIN_SYSMON_HEIGHT.min(360),
                    resizable: true,
                    layer: s.cfg.layer,
                    dark: s.dark,
                    opacity_percent: s.cfg.opacity_percent,
                },
            );
            let weak = s.ui.as_weak();
            Timer::single_shot(Duration::from_millis(60), move || {
                if let Some(u) = weak.upgrade() {
                    move_window_to(u.window(), saved_x, saved_y);
                    enforce_skip_taskbar(u.window());
                }
            });
            s.window_shown = true;
        } else {
            set_window_visible(s.ui.window(), true);
            set_window_layer(s.ui.window(), s.cfg.layer);
            apply_window_opacity(s.ui.window(), s.cfg.opacity_percent);
            resize_window_to(s.ui.window(), s.cfg.width, s.cfg.height);
            move_window_to(s.ui.window(), s.cfg.x, s.cfg.y);
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
    let (w1, w2) = (ui.as_weak(), ui.as_weak());
    ui.on_drag_window(move || {
        if let Some(u) = w1.upgrade() {
            start_window_drag(u.window());
        }
    });
    ui.on_resize_window(move |dir| {
        if let Some(u) = w2.upgrade() {
            start_window_resize(u.window(), dir);
        }
    });
    ui.on_open_settings(|| emit_open_settings("sysmon"));
    let st = Rc::clone(state);
    ui.on_refresh_now(move || kick_sysmon_sample(&st));
    ui.on_open_section_tool(|idx| {
        let (cmd, args) = match idx {
            1 => ("ms-settings:display", ""),
            2 => ("perfmon.exe", "/res"),
            3 => ("ncpa.cpl", ""),
            _ => ("taskmgr.exe", ""),
        };
        thread::spawn(move || {
            let _ = launch_system_tool(cmd, args);
        });
    });
    wire_sysmon_menu_callbacks(state);
    let (st_m, st_r) = (Rc::clone(state), Rc::clone(state));
    install_bounds_tracker(
        state.borrow().ui.window(),
        move |x, y| {
            let mut s = st_m.borrow_mut();
            if s.window_shown
                && Instant::now() >= s.ignore_bounds_until
                && (s.cfg.x != x || s.cfg.y != y)
            {
                s.cfg.x = x;
                s.cfg.y = y;
                let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
            }
        },
        move |w, h| {
            let mut s = st_r.borrow_mut();
            let cw = w.clamp(MIN_SYSMON_WIDTH, MAX_SYSMON_WIDTH);
            let ch = h.clamp(MIN_SYSMON_HEIGHT, MAX_SYSMON_HEIGHT);
            if s.window_shown
                && Instant::now() >= s.ignore_bounds_until
                && (s.cfg.width != cw || s.cfg.height != ch)
            {
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
    ui.on_open_context_menu(move || open_sysmon_context_menu(&st));
}

fn open_sysmon_context_menu(state: &SharedSysMon) {
    const INTERVALS: [(u32, &str); 4] = [
        (1000, "1 сек"),
        (2000, "2 сек"),
        (3000, "3 сек"),
        (5000, "5 сек"),
    ];
    let choice = {
        let s = state.borrow();
        let hwnd = window_hwnd_raw(s.ui.window());
        show_widget_context_menu(&WidgetMenuParams {
            hwnd_raw: hwnd,
            appearance: s.appearance,
            vis: s.vis,
            refresh_label: "Обновить датчики",
            settings_label: "Настройки монитора",
            config_label: "Открыть sysmon.cfg",
            layer: s.cfg.layer,
            opacity_percent: s.cfg.opacity_percent,
            interval_title: "Частота опроса",
            interval_options: &INTERVALS,
            current_interval: s.cfg.refresh_interval_ms,
        })
    };
    let Some(act) = choice else {
        return;
    };
    match act {
        WidgetMenuChoice::Refresh => kick_sysmon_sample(state),
        WidgetMenuChoice::OpenSettings => emit_open_settings("sysmon"),
        WidgetMenuChoice::OpenConfig => {
            let path = state.borrow().cfg_path.clone();
            let _ = open_text_file(&path);
        }
        WidgetMenuChoice::SetLayer(layer) => {
            let mut s = state.borrow_mut();
            s.cfg.layer = layer;
            set_window_layer(s.ui.window(), layer);
            let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
            emit_config_changed("widgets");
        }
        WidgetMenuChoice::SetOpacity(pct) => {
            let mut s = state.borrow_mut();
            s.cfg.opacity_percent = pct;
            apply_window_opacity(s.ui.window(), pct);
            let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
            emit_config_changed("widgets");
        }
        WidgetMenuChoice::SetInterval(ms) => {
            let mut s = state.borrow_mut();
            let clamped = ms.clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
            s.cfg.refresh_interval_ms = clamped;
            s.sampler_ms.store(clamped, Ordering::Relaxed);
            let _ = save_sysmon_config(&s.cfg_path, &s.cfg);
            emit_config_changed("widgets");
        }
        WidgetMenuChoice::HideWidget => toggle_sysmon_visible(state),
    }
}
