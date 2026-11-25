use std::path::PathBuf;

use asrbox_platform_api::{
    FluentPalette, HttpFetchResult, PixelPoint, PixelRect, WidgetLayer, WidgetMenuAction,
    WidgetMenuState, WidgetMenuVisibility, WidgetPlatform, WidgetSizeBounds,
};

use crate::storage::{MAX_OPACITY_PERCENT, MIN_OPACITY_PERCENT};
use crate::weather_api::{
    build_forecast_url, build_geocode_url, parse_open_meteo_forecast, parse_open_meteo_geocode,
    DailySlot, WeatherData,
};
use crate::weather_config::{
    load_weather_config, normalize_forecast_days, save_weather_config, WeatherProvider,
    WeatherWidgetConfig, MAX_WEATHER_HEIGHT, MAX_WEATHER_WIDTH, MIN_WEATHER_HEIGHT,
    MIN_WEATHER_WIDTH, YANDEX_WEATHER_CRED_TARGET,
};
use crate::weather_render::{
    hit_test_weather, render_weather_widget, WeatherHitTarget, WeatherRenderInput, WEATHER_HEADER_H,
};
use crate::weather_yandex::{build_yandex_forecast_url, parse_yandex_forecast};

pub const WEATHER_WIDGET_ID: u64 = 3;
const WEATHER_REQ_BASE: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingKind {
    Forecast(WeatherProvider),
    YandexFallbackDays,
    Geocode,
}

pub struct WeatherWidget {
    config_path: PathBuf,
    config: WeatherWidgetConfig,
    window_open: bool,
    hovered: WeatherHitTarget,
    last_tooltip: Option<String>,
    data: WeatherData,
    yandex_native_days: usize,
    is_hybrid_fallback: bool,
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
            yandex_native_days: 0,
            is_hybrid_fallback: false,
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
        let prev_provider = self.config.provider;
        match load_weather_config(&self.config_path) {
            Ok(cfg) => self.config = cfg,
            Err(e) => return format!("error: reload weather.cfg: {e}"),
        }
        if self.config.forecast_days == 7 && self.config.height < 460 {
            self.config.height = 460;
            self.persist();
        }
        let _ = self.sync_window(host, safe_mode);
        if self.window_open {
            if self.config.city != prev_city {
                self.trigger_geocode(host);
            } else if self.config.provider != prev_provider || !self.data.has_live_data {
                self.trigger_forecast(host);
            } else {
                self.redraw(host);
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
            WeatherHitTarget::HorizonDays(d) => Some(format!("Показать прогноз на {d} дн.")),
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
            WeatherHitTarget::HorizonDays(days) => {
                let norm = normalize_forecast_days(days);
                self.config.forecast_days = norm;
                if norm == 7 && self.config.height < 460 {
                    self.config.height = 460;
                }
                self.persist();
                self.redraw(host);
                format!("weather horizon={norm}d")
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
            if kind == PendingKind::YandexFallbackDays {
                self.redraw(host);
                return Some((false, format!("weather hybrid fallback skipped: {err}")));
            }
            self.error = Some(format!("Нет сети: {err}"));
            self.redraw(host);
            return Some((true, format!("weather http error: {err}")));
        }
        if result.status_code != 200 {
            if kind == PendingKind::YandexFallbackDays {
                self.redraw(host);
                return Some((
                    false,
                    format!("weather hybrid fallback HTTP {}", result.status_code),
                ));
            }
            self.error = Some(format!("HTTP {}", result.status_code));
            self.redraw(host);
            return Some((true, format!("weather http status {}", result.status_code)));
        }
        match kind {
            PendingKind::Geocode => self.finish_geocode(host, &result.body),
            PendingKind::Forecast(provider) => self.finish_forecast(host, provider, &result.body),
            PendingKind::YandexFallbackDays => self.finish_yandex_fallback(host, &result.body),
        }
    }

    fn finish_geocode(
        &mut self,
        host: &mut dyn WidgetPlatform,
        body: &[u8],
    ) -> Option<(bool, String)> {
        if let Some(geo) = parse_open_meteo_geocode(body) {
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

    fn finish_forecast(
        &mut self,
        host: &mut dyn WidgetPlatform,
        provider: WeatherProvider,
        body: &[u8],
    ) -> Option<(bool, String)> {
        let now = host.local_time_now();
        let updated = format!("{:02}:{:02}", now.hour, now.minute);
        let parsed = match provider {
            WeatherProvider::OpenMeteo => parse_open_meteo_forecast(body, now.hour, updated),
            WeatherProvider::Yandex => parse_yandex_forecast(body, now.hour, updated),
        };
        match parsed {
            Ok(data) => {
                let native_days = data.daily.len();
                self.is_hybrid_fallback = false;
                self.yandex_native_days = if provider == WeatherProvider::Yandex {
                    native_days
                } else {
                    0
                };
                self.data = data;
                self.error = None;
                if provider == WeatherProvider::Yandex && native_days < 7 {
                    self.trigger_yandex_fallback_days(host);
                } else {
                    self.redraw(host);
                }
                Some((
                    false,
                    format!(
                        "weather updated ({}): {} {}°C, days={}, body={}B",
                        provider.as_str(),
                        self.config.city,
                        self.data.temp_c,
                        native_days,
                        body.len()
                    ),
                ))
            }
            Err(e) => {
                self.error = Some(format!("Ошибка ответа {}", provider.display_name()));
                self.redraw(host);
                Some((true, format!("weather parse error: {e}")))
            }
        }
    }

    fn finish_yandex_fallback(
        &mut self,
        host: &mut dyn WidgetPlatform,
        body: &[u8],
    ) -> Option<(bool, String)> {
        let now = host.local_time_now();
        let updated = self.data.updated_label.clone();
        if let Ok(extra) = parse_open_meteo_forecast(body, now.hour, updated) {
            let added = merge_hybrid_days(&mut self.data.daily, &extra.daily);
            if added > 0 {
                self.is_hybrid_fallback = true;
            }
            self.redraw(host);
            Some((
                false,
                format!(
                    "weather hybrid days appended: +{added} (total={})",
                    self.data.daily.len()
                ),
            ))
        } else {
            self.redraw(host);
            Some((false, "weather hybrid fallback parse skipped".to_owned()))
        }
    }

    fn trigger_yandex_fallback_days(&mut self, host: &mut dyn WidgetPlatform) {
        let id = self.next_req_id;
        self.next_req_id = self.next_req_id.saturating_add(1);
        let url = build_forecast_url(self.config.lat, self.config.lon);
        if host.start_http_get(id, &url).is_ok() {
            self.pending_req = Some((id, PendingKind::YandexFallbackDays));
        }
        self.redraw(host);
    }

    pub fn trigger_forecast(&mut self, host: &mut dyn WidgetPlatform) {
        let id = self.next_req_id;
        self.next_req_id = self.next_req_id.saturating_add(1);
        let provider = self.config.provider;
        let send_res = match provider {
            WeatherProvider::OpenMeteo => {
                let url = build_forecast_url(self.config.lat, self.config.lon);
                host.start_http_get(id, &url)
            }
            WeatherProvider::Yandex => match host.read_secret(YANDEX_WEATHER_CRED_TARGET) {
                Ok(Some(key)) if !key.trim().is_empty() => {
                    let url = build_yandex_forecast_url(self.config.lat, self.config.lon);
                    host.start_http_get_with_headers(
                        id,
                        &url,
                        &[("X-Yandex-Weather-Key".to_owned(), key.trim().to_owned())],
                    )
                }
                Ok(_) => {
                    self.error = Some("Укажите API-ключ Яндекс.Погоды в настройках".to_owned());
                    self.redraw(host);
                    return;
                }
                Err(e) => Err(format!("Credential Manager: {e}")),
            },
        };
        match send_res {
            Ok(()) => {
                self.pending_req = Some((id, PendingKind::Forecast(provider)));
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
            provider: self.config.provider,
            forecast_days: self.config.forecast_days,
            city: &self.config.city,
            now: host.local_time_now(),
            data: &self.data,
            yandex_native_days: self.yandex_native_days,
            is_hybrid_fallback: self.is_hybrid_fallback,
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

pub(crate) fn merge_hybrid_days(base: &mut Vec<DailySlot>, extra: &[DailySlot]) -> usize {
    let mut added = 0usize;
    let last_date = base.last().map(|d| d.date_iso.clone()).unwrap_or_default();
    for day in extra {
        if base.len() >= 7 {
            break;
        }
        let is_newer = if last_date.is_empty() {
            !base.iter().any(|b| b.date_iso == day.date_iso)
        } else {
            day.date_iso.as_str() > last_date.as_str()
                && !base.iter().any(|b| b.date_iso == day.date_iso)
        };
        if is_newer {
            base.push(day.clone());
            added += 1;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(date: &str, min_c: i32, max_c: i32) -> DailySlot {
        DailySlot {
            date_iso: date.to_owned(),
            min_c,
            max_c,
            wmo_code: 3,
        }
    }

    #[test]
    fn hybrid_merge_appends_missing_days_up_to_seven() {
        let mut yandex = vec![mk("2026-11-25", 0, 5), mk("2026-11-26", -1, 4)];
        let open_meteo = vec![
            mk("2026-11-25", 1, 6),
            mk("2026-11-26", 0, 5),
            mk("2026-11-27", -2, 3),
            mk("2026-11-28", -3, 2),
            mk("2026-11-29", -4, 1),
            mk("2026-11-30", -1, 4),
            mk("2026-12-01", 0, 5),
            mk("2026-12-02", 1, 6),
        ];
        let added = merge_hybrid_days(&mut yandex, &open_meteo);
        assert_eq!(added, 5);
        assert_eq!(yandex.len(), 7);
        assert_eq!(yandex[0].max_c, 5);
        assert_eq!(yandex[1].max_c, 4);
        assert_eq!(yandex[2].date_iso, "2026-11-27");
        assert_eq!(yandex[6].date_iso, "2026-12-01");
    }
}
