use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use asrbox_mod_widgets::{
    load_weather_config, normalize_forecast_days, save_weather_config, WeatherData,
    WeatherWidgetConfig, MAX_WEATHER_HEIGHT, MAX_WEATHER_WIDTH, MIN_WEATHER_HEIGHT,
    MIN_WEATHER_WIDTH,
};
use asrbox_platform_api::{AppearanceConfig, WidgetMenuVisibility};
use slint::{ComponentHandle, Timer, TimerMode};

use crate::generated::ui::WeatherWindow;
use crate::settings_state::apply_theme_to_window;
use crate::vibrancy::{
    apply_acrylic_vibrancy, apply_window_opacity, configure_desktop_window, enforce_skip_taskbar,
    install_bounds_tracker, move_window_to, resize_window_to, set_window_layer, set_window_visible,
    start_window_drag, start_window_resize, window_hwnd_raw, DesktopWindowSpec,
};
use crate::widgets_menu::{show_widget_context_menu, WidgetMenuChoice, WidgetMenuParams};
use crate::widgets_weather_data::{fetch_weather_blocking, sync_weather_ui, WeatherFetchPayload};
use crate::win_os::{emit_config_changed, emit_open_settings, emit_worker_log, open_text_file};

thread_local! {
    static WEATHER_SLOT: RefCell<Option<SharedWeather>> = const { RefCell::new(None) };
}

pub struct WeatherSlintState {
    pub ui: WeatherWindow,
    pub cfg_path: PathBuf,
    pub cfg: WeatherWidgetConfig,
    pub data: WeatherData,
    pub is_hybrid: bool,
    pub error: Option<String>,
    pub loading: bool,
    pub appearance: AppearanceConfig,
    pub vis: WidgetMenuVisibility,
    pub dark: bool,
    pub window_shown: bool,
    pub ignore_bounds_until: Instant,
    pub refresh_timer: Timer,
}

pub type SharedWeather = Rc<RefCell<WeatherSlintState>>;

pub fn create_weather_controller(
    cfg_path: PathBuf,
    appearance: AppearanceConfig,
    vis: WidgetMenuVisibility,
    safe_mode: bool,
) -> Result<SharedWeather, String> {
    let ui = WeatherWindow::new().map_err(|e| format!("create WeatherWindow: {e}"))?;
    let dark = apply_theme_to_window(&ui, appearance);
    let cfg = load_weather_config(&cfg_path).unwrap_or_default();
    let state = Rc::new(RefCell::new(WeatherSlintState {
        ui,
        cfg_path,
        cfg,
        data: WeatherData::default(),
        is_hybrid: false,
        error: None,
        loading: false,
        appearance,
        vis,
        dark,
        window_shown: false,
        ignore_bounds_until: Instant::now() + Duration::from_millis(800),
        refresh_timer: Timer::default(),
    }));
    WEATHER_SLOT.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&state)));
    wire_weather_callbacks(&state);
    apply_weather_window_state(&state, safe_mode, true);
    Ok(state)
}

pub fn clear_weather_slot() {
    WEATHER_SLOT.with(|slot| *slot.borrow_mut() = None);
}

pub fn reload_weather(
    state: &SharedWeather,
    appearance: AppearanceConfig,
    vis: WidgetMenuVisibility,
    safe_mode: bool,
) {
    let (city_changed, prov_changed, need_fetch) = {
        let mut s = state.borrow_mut();
        let prev_city = s.cfg.city.clone();
        let prev_prov = s.cfg.provider;
        if let Ok(fresh) = load_weather_config(&s.cfg_path) {
            s.cfg = fresh;
        }
        s.appearance = appearance;
        s.vis = vis;
        let dark = apply_theme_to_window(&s.ui, appearance);
        if dark != s.dark {
            s.dark = dark;
            apply_acrylic_vibrancy(s.ui.window(), dark);
        }
        (
            s.cfg.city != prev_city,
            s.cfg.provider != prev_prov,
            !s.data.has_live_data,
        )
    };
    apply_weather_window_state(state, safe_mode, city_changed || prov_changed || need_fetch);
}

pub fn toggle_weather_visible(state: &SharedWeather) {
    {
        let mut s = state.borrow_mut();
        s.cfg.visible = !s.cfg.visible;
        let _ = save_weather_config(&s.cfg_path, &s.cfg);
    }
    apply_weather_window_state(state, false, false);
    emit_config_changed("widgets");
}

pub fn trigger_weather_refresh(state: &SharedWeather, geocode_first: bool) {
    let (city, lat, lon, provider) = {
        let mut s = state.borrow_mut();
        if !s.window_shown {
            return;
        }
        s.loading = true;
        s.ui.set_loading(true);
        (s.cfg.city.clone(), s.cfg.lat, s.cfg.lon, s.cfg.provider)
    };
    thread::spawn(move || {
        let payload = fetch_weather_blocking(city, lat, lon, provider, geocode_first);
        let _ = slint::invoke_from_event_loop(move || {
            WEATHER_SLOT.with(|slot| {
                if let Some(rc) = slot.borrow().as_ref().cloned() {
                    apply_fetch_result(&rc, payload);
                }
            });
        });
    });
}

fn apply_weather_window_state(state: &SharedWeather, safe_mode: bool, force_fetch: bool) {
    let (should_show, needs_fetch, interval_min) = {
        let mut s = state.borrow_mut();
        let show = !safe_mode && s.cfg.visible;
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
                        min_w: MIN_WEATHER_WIDTH,
                        min_h: MIN_WEATHER_HEIGHT,
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
            sync_weather_ui(&s);
        } else if s.window_shown {
            set_window_visible(s.ui.window(), false);
            s.window_shown = false;
            s.refresh_timer.stop();
        }
        let fetch = show && (force_fetch || !s.data.has_live_data);
        (show, fetch, s.cfg.refresh_interval_min)
    };
    if should_show {
        arm_weather_timer(state, interval_min);
        if needs_fetch {
            trigger_weather_refresh(state, force_fetch);
        }
    }
}

fn arm_weather_timer(state: &SharedWeather, interval_min: u32) {
    let secs = u64::from(interval_min.clamp(5, 360)) * 60;
    let st_weak = Rc::downgrade(state);
    state
        .borrow()
        .refresh_timer
        .start(TimerMode::Repeated, Duration::from_secs(secs), move || {
            if let Some(rc) = st_weak.upgrade() {
                trigger_weather_refresh(&rc, false);
            }
        });
}

fn wire_weather_callbacks(state: &SharedWeather) {
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
    ui.on_open_settings(|| emit_open_settings("weather"));
    let st = Rc::clone(state);
    ui.on_refresh_now(move || trigger_weather_refresh(&st, false));
    wire_weather_menu_callbacks(state);
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
                let _ = save_weather_config(&s.cfg_path, &s.cfg);
            }
        },
        move |w, h| {
            let mut s = st_r.borrow_mut();
            let cw = w.clamp(MIN_WEATHER_WIDTH, MAX_WEATHER_WIDTH);
            let ch = h.clamp(MIN_WEATHER_HEIGHT, MAX_WEATHER_HEIGHT);
            if s.window_shown
                && Instant::now() >= s.ignore_bounds_until
                && (s.cfg.width != cw || s.cfg.height != ch)
            {
                s.cfg.width = cw;
                s.cfg.height = ch;
                let _ = save_weather_config(&s.cfg_path, &s.cfg);
            }
        },
    );
}

fn wire_weather_menu_callbacks(state: &SharedWeather) {
    let ui = &state.borrow().ui;
    let st = Rc::clone(state);
    ui.on_set_forecast_days(move |days| {
        let mut s = st.borrow_mut();
        let norm = normalize_forecast_days(days.clamp(1, 7) as u8);
        s.cfg.forecast_days = norm;
        if norm == 7 && s.cfg.height < 460 {
            s.ignore_bounds_until = Instant::now() + Duration::from_millis(500);
            s.cfg.height = 460;
            resize_window_to(s.ui.window(), s.cfg.width, s.cfg.height);
        }
        let _ = save_weather_config(&s.cfg_path, &s.cfg);
        sync_weather_ui(&s);
        emit_config_changed("widgets");
    });
    let st = Rc::clone(state);
    ui.on_open_context_menu(move || open_weather_context_menu(&st));
}

fn open_weather_context_menu(state: &SharedWeather) {
    const INTERVALS: [(u32, &str); 3] = [(15, "15 минут"), (30, "30 минут"), (60, "60 минут")];
    let choice = {
        let s = state.borrow();
        let hwnd = window_hwnd_raw(s.ui.window());
        show_widget_context_menu(&WidgetMenuParams {
            hwnd_raw: hwnd,
            appearance: s.appearance,
            vis: s.vis,
            refresh_label: "Обновить прогноз",
            settings_label: "Настройки погоды",
            config_label: "Открыть weather.cfg",
            layer: s.cfg.layer,
            opacity_percent: s.cfg.opacity_percent,
            interval_title: "Обновление",
            interval_options: &INTERVALS,
            current_interval: s.cfg.refresh_interval_min,
        })
    };
    let Some(act) = choice else {
        return;
    };
    match act {
        WidgetMenuChoice::Refresh => trigger_weather_refresh(state, false),
        WidgetMenuChoice::OpenSettings => emit_open_settings("weather"),
        WidgetMenuChoice::OpenConfig => {
            let path = state.borrow().cfg_path.clone();
            let _ = open_text_file(&path);
        }
        WidgetMenuChoice::SetLayer(layer) => {
            let mut s = state.borrow_mut();
            s.cfg.layer = layer;
            set_window_layer(s.ui.window(), layer);
            let _ = save_weather_config(&s.cfg_path, &s.cfg);
            emit_config_changed("widgets");
        }
        WidgetMenuChoice::SetOpacity(pct) => {
            let mut s = state.borrow_mut();
            s.cfg.opacity_percent = pct;
            apply_window_opacity(s.ui.window(), pct);
            let _ = save_weather_config(&s.cfg_path, &s.cfg);
            emit_config_changed("widgets");
        }
        WidgetMenuChoice::SetInterval(mins) => {
            let clamped = mins.clamp(5, 360);
            {
                let mut s = state.borrow_mut();
                s.cfg.refresh_interval_min = clamped;
                let _ = save_weather_config(&s.cfg_path, &s.cfg);
            }
            arm_weather_timer(state, clamped);
            emit_config_changed("widgets");
        }
        WidgetMenuChoice::HideWidget => toggle_weather_visible(state),
    }
}

fn apply_fetch_result(state: &SharedWeather, payload: WeatherFetchPayload) {
    let mut s = state.borrow_mut();
    s.loading = false;
    if let Some((name, lat, lon)) = payload.city_override {
        s.cfg.city = name;
        s.cfg.lat = lat;
        s.cfg.lon = lon;
        let _ = save_weather_config(&s.cfg_path, &s.cfg);
    }
    if let Some(data) = payload.data {
        emit_worker_log(
            "INFO",
            "http",
            format!(
                "Погода ({}): {} {}°C, дней={}",
                s.cfg.provider.as_str(),
                s.cfg.city,
                data.temp_c,
                data.daily.len()
            ),
        );
        s.data = data;
        s.is_hybrid = payload.is_hybrid;
        s.error = None;
    } else if let Some(err) = payload.error {
        emit_worker_log("ERROR", "http", format!("Погода: {err}"));
        s.error = Some(err);
    }
    sync_weather_ui(&s);
}
