use std::path::PathBuf;

use asrbox_platform_api::{
    FluentPalette, HttpFetchResult, PixelPoint, PixelRect, WidgetLayer, WidgetMenuAction,
    WidgetMenuState, WidgetMenuVisibility, WidgetPlatform, WidgetSizeBounds,
};

use crate::storage::{MAX_OPACITY_PERCENT, MIN_OPACITY_PERCENT};
use crate::weather_api::{
    build_forecast_url, build_geocode_url, parse_open_meteo_forecast, parse_open_meteo_geocode,
    WeatherData,
};
use crate::weather_config::{
    load_weather_config, save_weather_config, WeatherWidgetConfig, MAX_WEATHER_HEIGHT,
    MAX_WEATHER_WIDTH, MIN_WEATHER_HEIGHT, MIN_WEATHER_WIDTH,
};
use crate::weather_render::{
    hit_test_weather, render_weather_widget, WeatherHitTarget, WeatherRenderInput, WEATHER_HEADER_H,
};

pub const WEATHER_WIDGET_ID: u64 = 3;
const WEATHER_REQ_BASE: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingKind {
    Forecast,
    Geocode,
}

pub struct WeatherWidget {
    config_path: PathBuf,
    config: WeatherWidgetConfig,
    window_open: bool,
    hovered: WeatherHitTarget,
    last_tooltip: Option<String>,
    data: WeatherData,
    error: Option<String>,
    pending_req: Option<(u64, PendingKind)>,
    next_req_id: u64,
    palette: FluentPalette,
    menu_vis: WidgetMenuVisibility,
}

impl WeatherWidget {
    pub fn load(config_path: PathBuf) -> Self {
        let config = load_weather_config(&config_path).unwrap_or_default();
        Self {
            config_path,
            config,
            window_open: false,
            hovered: WeatherHitTarget::None,
            last_tooltip: None,
            data: WeatherData::default(),
            error: None,
            pending_req: None,
            next_req_id: WEATHER_REQ_BASE,
            palette: FluentPalette::default(),
            menu_vis: WidgetMenuVisibility::default(),
        }
    }

    pub fn set_menu_visibility(&mut self, vis: WidgetMenuVisibility) {
        self.menu_vis = vis;
    }

    pub fn set_palette(&mut self, host: &mut dyn WidgetPlatform, palette: FluentPalette) {
        self.palette = palette;
        if self.window_open {
            self.redraw(host);
        }
    }

    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    pub fn config(&self) -> &WeatherWidgetConfig {
        &self.config
    }

    pub fn status_summary(&self) -> String {
        format!(
            "visible={}, city={}, {}x{}, interval={}m",
            self.config.visible,
            self.config.city,
            self.config.width,
            self.config.height,
            self.config.refresh_interval_min
        )
    }

    pub fn sync_window(
        &mut self,
        host: &mut dyn WidgetPlatform,
        safe_mode: bool,
    ) -> Result<(), String> {
        if !safe_mode && self.config.visible {
            if !self.window_open {
                host.create_widget_window(
                    WEATHER_WIDGET_ID,
                    "Погода",
                    self.window_rect(),
                    self.config.layer,
                    WEATHER_HEADER_H,
                )?;
                let _ = host.set_widget_resize_bounds(
                    WEATHER_WIDGET_ID,
                    WidgetSizeBounds {
                        min_w: MIN_WEATHER_WIDTH,
                        min_h: MIN_WEATHER_HEIGHT,
                        max_w: MAX_WEATHER_WIDTH,
                        max_h: MAX_WEATHER_HEIGHT,
                    },
                );
                self.window_open = true;
            }
            let _ = host.set_widget_layer(WEATHER_WIDGET_ID, self.config.layer);
            let sec = self.config.refresh_interval_min.saturating_mul(60).max(300);
            let _ = host.set_widget_timers(WEATHER_WIDGET_ID, sec, false);
            self.redraw(host);
            if !self.data.has_live_data && self.pending_req.is_none() {
                self.trigger_forecast(host);
            }
        } else if self.window_open {
            let _ = host.destroy_widget_window(WEATHER_WIDGET_ID);
            self.window_open = false;
        }
        Ok(())
    }

    pub fn toggle_visible(&mut self, host: &mut dyn WidgetPlatform, safe_mode: bool) {
        self.config.visible = !self.config.visible;
        self.persist();
        let _ = self.sync_window(host, safe_mode);
    }

    pub fn reload_config(&mut self, host: &mut dyn WidgetPlatform, safe_mode: bool) -> String {
        let prev_city = self.config.city.clone();
        match load_weather_config(&self.config_path) {
            Ok(cfg) => self.config = cfg,
            Err(e) => return format!("error: reload weather.cfg: {e}"),
        }
        let _ = self.sync_window(host, safe_mode);
        if self.window_open {
            if self.config.city != prev_city {
                self.trigger_geocode(host);
            } else {
                self.trigger_forecast(host);
            }
        }
        format!("reload weather.cfg: {}", self.status_summary())
    }

    pub fn trigger_geocode_for_city(&mut self, host: &mut dyn WidgetPlatform, city: &str) {
        let trimmed = city.trim();
        if trimmed.is_empty() {
            return;
        }
        self.config.city = trimmed.to_owned();
        self.persist();
        self.trigger_geocode(host);
    }

    pub fn on_pointer_move(&mut self, host: &mut dyn WidgetPlatform, pt: PixelPoint) {
        let hit = hit_test_weather(pt, self.config.width, self.config.height);
        if hit == self.hovered {
            return;
        }
        self.hovered = hit;
        let tip = match hit {
            WeatherHitTarget::SettingsBtn => Some("Настройки виджета Погоды".to_owned()),
            WeatherHitTarget::RefreshBtn => Some("Обновить прогноз сейчас".to_owned()),
            _ => None,
        };
        if tip != self.last_tooltip {
            let _ = host.set_widget_tooltip(WEATHER_WIDGET_ID, tip.as_deref());
            self.last_tooltip = tip;
        }
        self.redraw(host);
    }

    pub fn on_pointer_leave(&mut self, host: &mut dyn WidgetPlatform) {
        if self.last_tooltip.take().is_some() {
            let _ = host.set_widget_tooltip(WEATHER_WIDGET_ID, None);
        }
        if self.hovered != WeatherHitTarget::None {
            self.hovered = WeatherHitTarget::None;
            self.redraw(host);
        }
    }

    pub fn on_left_click(&mut self, host: &mut dyn WidgetPlatform, pt: PixelPoint) -> String {
        match hit_test_weather(pt, self.config.width, self.config.height) {
            WeatherHitTarget::SettingsBtn => "open_settings_weather".to_owned(),
            WeatherHitTarget::RefreshBtn => {
                self.trigger_forecast(host);
                "weather manual refresh".to_owned()
            }
            WeatherHitTarget::HeaderDrag | WeatherHitTarget::None => String::new(),
        }
    }

    pub fn on_right_click(&mut self, host: &mut dyn WidgetPlatform, screen: PixelPoint) -> String {
        let state = WidgetMenuState {
            layer: self.config.layer,
            opacity_percent: self.config.opacity_percent,
            refresh_ms: None,
            sync_interval_sec: Some(self.config.refresh_interval_min.saturating_mul(60)),
            menu_vis: self.menu_vis,
        };
        let Ok(Some(action)) = host.show_widget_menu(WEATHER_WIDGET_ID, screen, &state) else {
            return String::new();
        };
        match action {
            WidgetMenuAction::Refresh => {
                self.trigger_forecast(host);
                "weather refresh".to_owned()
            }
            WidgetMenuAction::OpenSettings => "open_settings_weather".to_owned(),
            WidgetMenuAction::OpenConfig => {
                if !self.config_path.exists() {
                    self.persist();
                }
                match host.open_text_file(&self.config_path) {
                    Ok(()) => "open weather.cfg".to_owned(),
                    Err(e) => format!("error: open weather.cfg: {e}"),
                }
            }
            WidgetMenuAction::SetLayerWallpaper => self.set_layer(host, WidgetLayer::Wallpaper),
            WidgetMenuAction::SetLayerNormal => self.set_layer(host, WidgetLayer::Normal),
            WidgetMenuAction::SetLayerTopmost => self.set_layer(host, WidgetLayer::Topmost),
            WidgetMenuAction::SetOpacity(pct) => {
                self.config.opacity_percent = pct.clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT);
                self.persist();
                self.redraw(host);
                format!("weather opacity={}%", self.config.opacity_percent)
            }
            WidgetMenuAction::SetSyncIntervalSec(sec) => {
                self.config.refresh_interval_min = (sec / 60).clamp(5, 360);
                self.persist();
                let _ = host.set_widget_timers(
                    WEATHER_WIDGET_ID,
                    self.config.refresh_interval_min * 60,
                    false,
                );
                format!("weather interval={}m", self.config.refresh_interval_min)
            }
            WidgetMenuAction::HideWidget => {
                self.config.visible = false;
                self.persist();
                let _ = self.sync_window(host, false);
                "weather hidden".to_owned()
            }
            WidgetMenuAction::SetRefreshMs(_) => String::new(),
        }
    }

    pub fn on_moved(&mut self, x: i32, y: i32) {
        if self.config.x != x || self.config.y != y {
            self.config.x = x;
            self.config.y = y;
            self.persist();
        }
    }

    pub fn on_resized(
        &mut self,
        host: &mut dyn WidgetPlatform,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        finished: bool,
    ) {
        self.config.x = x;
        self.config.y = y;
        self.config.width = width.clamp(MIN_WEATHER_WIDTH, MAX_WEATHER_WIDTH);
        self.config.height = height.clamp(MIN_WEATHER_HEIGHT, MAX_WEATHER_HEIGHT);
        self.redraw(host);
        if finished {
            self.persist();
        }
    }

    pub fn on_refresh_tick(&mut self, host: &mut dyn WidgetPlatform) {
        if self.window_open {
            self.trigger_forecast(host);
        }
    }

    pub fn on_http_done(
        &mut self,
        host: &mut dyn WidgetPlatform,
        request_id: u64,
        result: HttpFetchResult,
    ) -> Option<(bool, String)> {
        let (pending_id, kind) = self.pending_req?;
        if pending_id != request_id {
            return None;
        }
        self.pending_req = None;
        if let Some(err) = result.error {
            self.error = Some(format!("Нет сети: {err}"));
            self.redraw(host);
            return Some((true, format!("weather http error: {err}")));
        }
        if result.status_code != 200 {
            self.error = Some(format!("HTTP {}", result.status_code));
            self.redraw(host);
            return Some((true, format!("weather http status {}", result.status_code)));
        }
        match kind {
            PendingKind::Geocode => {
                if let Some(geo) = parse_open_meteo_geocode(&result.body) {
                    self.config.city = geo.name;
                    self.config.lat = geo.lat;
                    self.config.lon = geo.lon;
                    self.persist();
                    self.error = None;
                    self.trigger_forecast(host);
                    Some((false, format!("weather geocode ok: {}", self.config.city)))
                } else {
                    self.error = Some("Город не найден".to_owned());
                    self.redraw(host);
                    Some((true, "weather geocode: city not found".to_owned()))
                }
            }
            PendingKind::Forecast => {
                let now = host.local_time_now();
                let updated = format!("{:02}:{:02}", now.hour, now.minute);
                match parse_open_meteo_forecast(&result.body, now.hour, updated) {
                    Ok(data) => {
                        self.data = data;
                        self.error = None;
                        self.redraw(host);
                        Some((
                            false,
                            format!(
                                "weather updated: {} {}°C",
                                self.config.city, self.data.temp_c
                            ),
                        ))
                    }
                    Err(e) => {
                        self.error = Some("Ошибка ответа Open-Meteo".to_owned());
                        self.redraw(host);
                        Some((true, format!("weather parse error: {e}")))
                    }
                }
            }
        }
    }

    pub fn trigger_forecast(&mut self, host: &mut dyn WidgetPlatform) {
        let id = self.next_req_id;
        self.next_req_id = self.next_req_id.saturating_add(1);
        let url = build_forecast_url(self.config.lat, self.config.lon);
        match host.start_http_get(id, &url) {
            Ok(()) => {
                self.pending_req = Some((id, PendingKind::Forecast));
                self.error = None;
            }
            Err(e) => self.error = Some(format!("HTTP: {e}")),
        }
        self.redraw(host);
    }

    fn trigger_geocode(&mut self, host: &mut dyn WidgetPlatform) {
        let id = self.next_req_id;
        self.next_req_id = self.next_req_id.saturating_add(1);
        let url = build_geocode_url(&self.config.city);
        match host.start_http_get(id, &url) {
            Ok(()) => {
                self.pending_req = Some((id, PendingKind::Geocode));
                self.error = None;
            }
            Err(e) => self.error = Some(format!("HTTP: {e}")),
        }
        self.redraw(host);
    }

    fn set_layer(&mut self, host: &mut dyn WidgetPlatform, layer: WidgetLayer) -> String {
        self.config.layer = layer;
        let _ = host.set_widget_layer(WEATHER_WIDGET_ID, layer);
        self.persist();
        format!("weather layer={layer:?}")
    }

    fn redraw(&self, host: &mut dyn WidgetPlatform) {
        if !self.window_open {
            return;
        }
        let frame = render_weather_widget(&WeatherRenderInput {
            width: self.config.width,
            height: self.config.height,
            opacity_percent: self.config.opacity_percent,
            city: &self.config.city,
            now: host.local_time_now(),
            data: &self.data,
            loading: self.pending_req.is_some(),
            error: self.error.as_deref(),
            hovered: self.hovered,
            palette: self.palette,
        });
        let _ = host.present_widget_frame(WEATHER_WIDGET_ID, self.window_rect(), &frame);
    }

    fn window_rect(&self) -> PixelRect {
        PixelRect {
            x: self.config.x,
            y: self.config.y,
            width: self.config.width,
            height: self.config.height,
        }
    }

    fn persist(&self) {
        let _ = save_weather_config(&self.config_path, &self.config);
    }
}
