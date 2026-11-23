use std::collections::HashMap;
use std::path::PathBuf;

use asrbox_platform_api::{
    HttpFetchResult, LocalDateTime, PixelPoint, PixelRect, WidgetLayer, WidgetMenuAction,
    WidgetMenuState, WidgetPlatform,
};

use crate::calendar_view::{
    build_display_rows, build_footer_status, contains_vcalendar, filter_events, parse_all_sources,
    tooltip_for_target,
};
use crate::format::event_day_url;
use crate::ics::CalendarEvent;
use crate::render::{
    hit_test_widget, render_calendar_widget, widget_height, RenderView, WidgetHitTarget,
    HEADER_HEIGHT,
};
use crate::storage::{
    load_cached_ics, load_widget_config, save_cached_ics, save_widget_config, CalendarWidgetConfig,
    DateFormatMode,
};

pub const CALENDAR_WIDGET_ID: u64 = 1;
const GOOGLE_CAL_HOME: &str = "https://calendar.google.com/calendar/r";
const GOOGLE_CAL_NEW_EVENT: &str = "https://calendar.google.com/calendar/r/eventedit";

pub struct CalendarWidget {
    config_path: PathBuf,
    cache_dir: PathBuf,
    config: CalendarWidgetConfig,
    window_open: bool,
    all_events: Vec<CalendarEvent>,
    cached_by_source: HashMap<String, Vec<u8>>,
    pending_requests: HashMap<u64, String>,
    next_request_id: u64,
    scroll_offset: usize,
    hovered: WidgetHitTarget,
    last_tooltip: Option<String>,
    last_updated: Option<String>,
    status_error: Option<String>,
    busy_warning: bool,
}

impl CalendarWidget {
    pub fn load(config_path: PathBuf, cache_dir: PathBuf) -> Self {
        let config = load_widget_config(&config_path).unwrap_or_default();
        let mut widget = Self {
            config_path,
            cache_dir,
            config,
            window_open: false,
            all_events: Vec::new(),
            cached_by_source: HashMap::new(),
            pending_requests: HashMap::new(),
            next_request_id: 1,
            scroll_offset: 0,
            hovered: WidgetHitTarget::None,
            last_tooltip: None,
            last_updated: None,
            status_error: None,
            busy_warning: false,
        };
        widget.load_disk_caches();
        widget
    }

    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    pub fn sync_window(
        &mut self,
        host: &mut dyn WidgetPlatform,
        safe_mode: bool,
    ) -> Result<(), String> {
        let show = !safe_mode && self.config.visible;
        if show {
            let now = host.local_time_now();
            self.reparse_all_sources(now);
            if !self.window_open {
                let rect = self.window_rect();
                host.create_widget_window(
                    CALENDAR_WIDGET_ID,
                    &self.config.title,
                    rect,
                    self.config.layer,
                    self.config.acrylic,
                    HEADER_HEIGHT,
                )?;
                self.window_open = true;
            }
            self.sync_timers(host);
            self.redraw(host);
            if self.pending_requests.is_empty() && !self.config.sources.is_empty() {
                self.trigger_network_refresh(host);
            }
        } else if self.window_open {
            let _ = host.destroy_widget_window(CALENDAR_WIDGET_ID);
            self.window_open = false;
        }
        Ok(())
    }

    pub fn toggle_visible(&mut self, host: &mut dyn WidgetPlatform, safe_mode: bool) {
        self.config.visible = !self.config.visible;
        self.persist();
        let _ = self.sync_window(host, safe_mode);
    }

    pub fn open_config_file(&self, host: &dyn WidgetPlatform) {
        let _ = save_widget_config(&self.config_path, &self.config);
        let path = self.config_path.to_string_lossy();
        let _ = host.open_url(&path);
    }

    pub fn on_pointer_move(&mut self, host: &mut dyn WidgetPlatform, point: PixelPoint) {
        let now = host.local_time_now();
        let rows = build_display_rows(&self.all_events, &self.config, now, self.scroll_offset);
        let target = hit_test_widget(
            point,
            self.config.width,
            self.config.visible_rows,
            rows.len(),
        );
        let tip = tooltip_for_target(target, &rows);
        if tip != self.last_tooltip {
            let _ = host.set_widget_tooltip(CALENDAR_WIDGET_ID, tip.as_deref());
            self.last_tooltip = tip;
        }
        if target != self.hovered {
            self.hovered = target;
            self.redraw(host);
        }
    }

    pub fn on_pointer_leave(&mut self, host: &mut dyn WidgetPlatform) {
        if self.last_tooltip.is_some() {
            let _ = host.set_widget_tooltip(CALENDAR_WIDGET_ID, None);
            self.last_tooltip = None;
        }
        if self.hovered != WidgetHitTarget::None {
            self.hovered = WidgetHitTarget::None;
            self.redraw(host);
        }
    }

    pub fn on_left_click(&mut self, host: &mut dyn WidgetPlatform, point: PixelPoint) {
        let now = host.local_time_now();
        let filtered = filter_events(&self.all_events, &self.config, now);
        let max_rows = usize::from(self.config.visible_rows);
        let shown = filtered
            .len()
            .saturating_sub(self.scroll_offset)
            .min(max_rows);
        let hit = hit_test_widget(point, self.config.width, self.config.visible_rows, shown);
        match hit {
            WidgetHitTarget::TitleLink => {
                let _ = host.open_url(GOOGLE_CAL_HOME);
            }
            WidgetHitTarget::AddEventBtn => {
                let _ = host.open_url(GOOGLE_CAL_NEW_EVENT);
            }
            WidgetHitTarget::SettingsBtn => self.open_config_file(host),
            WidgetHitTarget::RefreshBtn => self.reload_config_and_refresh(host),
            WidgetHitTarget::FilterTab(period) => {
                self.config.period = period;
                self.scroll_offset = 0;
                self.persist();
                self.redraw(host);
            }
            WidgetHitTarget::FormatCycleBtn => {
                self.config.date_format = self.config.date_format.next();
                self.persist();
                self.sync_timers(host);
                self.redraw(host);
            }
            WidgetHitTarget::ScrollUpBtn => self.scroll_by(host, -1),
            WidgetHitTarget::ScrollDownBtn => self.scroll_by(host, 1),
            WidgetHitTarget::EventRow(idx) => {
                if let Some(ev) = filtered.get(self.scroll_offset + idx) {
                    let _ = host.open_url(&event_day_url(ev));
                }
            }
            WidgetHitTarget::HeaderDrag | WidgetHitTarget::None => {}
        }
    }

    pub fn on_right_click(&mut self, host: &mut dyn WidgetPlatform, screen: PixelPoint) {
        let state = WidgetMenuState {
            layer: self.config.layer,
            acrylic: self.config.acrylic,
        };
        let Ok(Some(action)) = host.show_widget_menu(CALENDAR_WIDGET_ID, screen, &state) else {
            return;
        };
        match action {
            WidgetMenuAction::Refresh => self.reload_config_and_refresh(host),
            WidgetMenuAction::OpenConfig => self.open_config_file(host),
            WidgetMenuAction::SetLayerWallpaper => self.set_layer(host, WidgetLayer::Wallpaper),
            WidgetMenuAction::SetLayerNormal => self.set_layer(host, WidgetLayer::Normal),
            WidgetMenuAction::SetLayerTopmost => self.set_layer(host, WidgetLayer::Topmost),
            WidgetMenuAction::ToggleAcrylic => {
                self.config.acrylic = !self.config.acrylic;
                let _ = host.set_widget_layer(
                    CALENDAR_WIDGET_ID,
                    self.config.layer,
                    self.config.acrylic,
                );
                self.persist();
                self.redraw(host);
            }
            WidgetMenuAction::HideWidget => {
                self.config.visible = false;
                self.persist();
                let _ = self.sync_window(host, false);
            }
        }
    }

    pub fn on_wheel(&mut self, host: &mut dyn WidgetPlatform, delta: i16) {
        self.scroll_by(host, if delta > 0 { -1 } else { 1 });
    }

    pub fn on_moved(&mut self, x: i32, y: i32) {
        if self.config.x != x || self.config.y != y {
            self.config.x = x;
            self.config.y = y;
            self.persist();
        }
    }

    pub fn on_minute_tick(&mut self, host: &mut dyn WidgetPlatform) {
        if self.window_open && self.config.date_format == DateFormatMode::Countdown {
            self.redraw(host);
        }
    }

    pub fn on_refresh_tick(&mut self, host: &mut dyn WidgetPlatform) {
        if self.window_open {
            self.trigger_network_refresh(host);
        }
    }

    pub fn on_http_done(
        &mut self,
        host: &mut dyn WidgetPlatform,
        request_id: u64,
        result: HttpFetchResult,
    ) {
        let Some(source_id) = self.pending_requests.remove(&request_id) else {
            return;
        };
        let code = result.status_code;
        if let Some(err) = result.error {
            self.status_error = Some(format!("Нет сети (кэш): {err}"));
        } else if code == 404 || code == 403 || code == 401 {
            self.status_error = Some(format!("Ошибка HTTP {code}: проверьте ссылку .ics"));
        } else if code != 200 {
            self.status_error = Some(format!("Ошибка HTTP {code}"));
        } else if !contains_vcalendar(&result.body) {
            self.status_error = Some("Ответ не является .ics файлом".to_owned());
        } else {
            let _ = save_cached_ics(&self.cache_dir, &source_id, &result.body);
            self.cached_by_source.insert(source_id, result.body);
            let now = host.local_time_now();
            self.last_updated = Some(format!("{:02}:{:02}", now.hour, now.minute));
            self.status_error = None;
            self.reparse_all_sources(now);
        }
        if self.window_open {
            self.redraw(host);
        }
    }

    fn reload_config_and_refresh(&mut self, host: &mut dyn WidgetPlatform) {
        if let Ok(reloaded) = load_widget_config(&self.config_path) {
            self.config = reloaded;
        }
        self.load_disk_caches();
        let now = host.local_time_now();
        self.reparse_all_sources(now);
        self.sync_timers(host);
        self.trigger_network_refresh(host);
        self.redraw(host);
    }

    fn trigger_network_refresh(&mut self, host: &mut dyn WidgetPlatform) {
        if self.config.sources.is_empty() {
            return;
        }
        for src in &self.config.sources {
            let req_id = self.next_request_id;
            self.next_request_id = self.next_request_id.saturating_add(1);
            if host.start_http_get(req_id, &src.url).is_ok() {
                self.pending_requests.insert(req_id, src.id.clone());
            }
        }
        self.redraw(host);
    }

    fn set_layer(&mut self, host: &mut dyn WidgetPlatform, layer: WidgetLayer) {
        self.config.layer = layer;
        let _ = host.set_widget_layer(CALENDAR_WIDGET_ID, layer, self.config.acrylic);
        self.persist();
    }

    fn scroll_by(&mut self, host: &mut dyn WidgetPlatform, delta: isize) {
        let now = host.local_time_now();
        let total = filter_events(&self.all_events, &self.config, now).len();
        let max_rows = usize::from(self.config.visible_rows);
        let max_off = total.saturating_sub(max_rows);
        let next = if delta < 0 {
            self.scroll_offset.saturating_sub(delta.unsigned_abs())
        } else {
            self.scroll_offset
                .saturating_add(delta as usize)
                .min(max_off)
        };
        if next != self.scroll_offset {
            self.scroll_offset = next;
            self.redraw(host);
        }
    }

    fn load_disk_caches(&mut self) {
        for src in &self.config.sources {
            if let Some(bytes) = load_cached_ics(&self.cache_dir, &src.id) {
                self.cached_by_source.insert(src.id.clone(), bytes);
            }
        }
    }

    fn reparse_all_sources(&mut self, now: LocalDateTime) {
        let (merged, any_busy) = parse_all_sources(&self.config, &self.cached_by_source, now);
        self.all_events = merged;
        self.busy_warning = any_busy;
    }

    fn redraw(&mut self, host: &mut dyn WidgetPlatform) {
        if !self.window_open {
            return;
        }
        let now = host.local_time_now();
        let total = filter_events(&self.all_events, &self.config, now).len();
        let max_rows = usize::from(self.config.visible_rows);
        self.scroll_offset = self.scroll_offset.min(total.saturating_sub(max_rows));
        let rows = build_display_rows(&self.all_events, &self.config, now, self.scroll_offset);
        let (footer_text, footer_error) = build_footer_status(
            self.status_error.as_deref(),
            self.busy_warning,
            !self.config.sources.is_empty(),
            self.last_updated.as_deref(),
            self.scroll_offset,
            total,
            rows.len(),
        );
        let view = RenderView {
            width: self.config.width,
            visible_rows: self.config.visible_rows,
            acrylic: self.config.acrylic,
            title: &self.config.title,
            period: self.config.period,
            date_format: self.config.date_format,
            rows: &rows,
            hovered: self.hovered,
            footer_text: &footer_text,
            footer_error,
            can_scroll_up: self.scroll_offset > 0,
            can_scroll_down: self.scroll_offset + max_rows < total,
            is_loading: !self.pending_requests.is_empty(),
        };
        let frame = render_calendar_widget(&view);
        let _ = host.present_widget_frame(CALENDAR_WIDGET_ID, self.window_rect(), &frame);
    }

    fn sync_timers(&self, host: &mut dyn WidgetPlatform) {
        let minute_tick = self.config.date_format == DateFormatMode::Countdown;
        let _ = host.set_widget_timers(
            CALENDAR_WIDGET_ID,
            self.config.refresh_interval_sec,
            minute_tick,
        );
    }

    fn window_rect(&self) -> PixelRect {
        PixelRect {
            x: self.config.x,
            y: self.config.y,
            width: self.config.width,
            height: widget_height(self.config.visible_rows),
        }
    }

    fn persist(&self) {
        let _ = save_widget_config(&self.config_path, &self.config);
    }
}
