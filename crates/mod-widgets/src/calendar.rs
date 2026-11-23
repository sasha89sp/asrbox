use std::path::PathBuf;

use asrbox_platform_api::{
    HttpFetchResult, PixelPoint, PixelRect, WidgetLayer, WidgetMenuAction, WidgetMenuState,
    WidgetPlatform,
};

use crate::calendar_view::{
    build_calendar_frame, build_display_rows, clamp_scroll_offset, evaluate_http_response,
    filter_events, format_status_summary, tooltip_for_target, CalendarState, HttpEval,
};
use crate::format::event_day_url;
use crate::render::{hit_test_widget, widget_height, WidgetHitTarget, HEADER_HEIGHT};
use crate::storage::{
    load_widget_config, save_cached_ics, save_widget_config, CalendarWidgetConfig, DateFormatMode,
    MAX_OPACITY_PERCENT, MIN_OPACITY_PERCENT,
};

pub const CALENDAR_WIDGET_ID: u64 = 1;
const GOOGLE_CAL_HOME: &str = "https://calendar.google.com/calendar/r";
const GOOGLE_CAL_NEW_EVENT: &str = "https://calendar.google.com/calendar/r/eventedit";

pub struct CalendarWidget {
    config_path: PathBuf,
    cache_dir: PathBuf,
    config: CalendarWidgetConfig,
    window_open: bool,
    st: CalendarState,
}

impl CalendarWidget {
    pub fn load(config_path: PathBuf, cache_dir: PathBuf) -> Self {
        let config = load_widget_config(&config_path).unwrap_or_default();
        let mut st = CalendarState::default();
        st.load_caches(&config, &cache_dir);
        Self {
            config_path,
            cache_dir,
            config,
            window_open: false,
            st,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    pub fn source_count(&self) -> usize {
        self.config.sources.len()
    }

    pub fn status_summary(&self) -> String {
        format_status_summary(
            &self.config,
            self.st.all_events.len(),
            self.st.pending_requests.len(),
            self.st.status_error.as_deref(),
        )
    }

    pub fn sync_window(
        &mut self,
        host: &mut dyn WidgetPlatform,
        safe_mode: bool,
    ) -> Result<(), String> {
        let show = !safe_mode && self.config.visible;
        if show {
            let now = host.local_time_now();
            self.st.reparse(&self.config, now);
            if !self.window_open {
                let rect = self.window_rect();
                host.create_widget_window(
                    CALENDAR_WIDGET_ID,
                    &self.config.title,
                    rect,
                    self.config.layer,
                    HEADER_HEIGHT,
                )?;
                self.window_open = true;
            }
            self.sync_timers(host);
            self.redraw(host);
            if self.st.pending_requests.is_empty() && !self.config.sources.is_empty() {
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

    pub fn open_config_file(&self, host: &dyn WidgetPlatform) -> Result<(), String> {
        if !self.config_path.exists() {
            let _ = save_widget_config(&self.config_path, &self.config);
        }
        host.open_text_file(&self.config_path)
    }

    pub fn on_pointer_move(&mut self, host: &mut dyn WidgetPlatform, point: PixelPoint) {
        let target = hit_test_widget(
            point,
            self.config.width,
            self.config.visible_rows,
            self.st.shown_row_count,
        );
        if target == self.st.hovered {
            return;
        }
        self.st.hovered = target;
        let now = host.local_time_now();
        let rows = build_display_rows(
            &self.st.all_events,
            &self.config,
            now,
            self.st.scroll_offset,
        );
        let tip = tooltip_for_target(target, &rows);
        if tip != self.st.last_tooltip {
            let _ = host.set_widget_tooltip(CALENDAR_WIDGET_ID, tip.as_deref());
            self.st.last_tooltip = tip;
        }
        self.redraw(host);
    }

    pub fn on_pointer_leave(&mut self, host: &mut dyn WidgetPlatform) {
        if self.st.last_tooltip.is_some() {
            let _ = host.set_widget_tooltip(CALENDAR_WIDGET_ID, None);
            self.st.last_tooltip = None;
        }
        if self.st.hovered != WidgetHitTarget::None {
            self.st.hovered = WidgetHitTarget::None;
            self.redraw(host);
        }
    }

    pub fn on_left_click(&mut self, host: &mut dyn WidgetPlatform, point: PixelPoint) -> String {
        let now = host.local_time_now();
        let filtered = filter_events(&self.st.all_events, &self.config, now);
        let max_rows = usize::from(self.config.visible_rows);
        let shown = filtered
            .len()
            .saturating_sub(self.st.scroll_offset)
            .min(max_rows);
        let hit = hit_test_widget(point, self.config.width, self.config.visible_rows, shown);
        match hit {
            WidgetHitTarget::TitleLink => self.open_url_log(host, GOOGLE_CAL_HOME, "title"),
            WidgetHitTarget::AddEventBtn => {
                self.open_url_log(host, GOOGLE_CAL_NEW_EVENT, "add event")
            }
            WidgetHitTarget::SettingsBtn => "open_settings".to_owned(),
            WidgetHitTarget::RefreshBtn => self.reload_config_and_refresh(host),
            WidgetHitTarget::FilterTab(period) => {
                self.config.period = period;
                self.st.scroll_offset = 0;
                self.persist();
                self.redraw(host);
                format!("filter={}", period.as_str())
            }
            WidgetHitTarget::FormatCycleBtn => {
                self.config.date_format = self.config.date_format.next();
                self.persist();
                self.sync_timers(host);
                self.redraw(host);
                format!("format={}", self.config.date_format.as_str())
            }
            WidgetHitTarget::ScrollUpBtn => {
                self.scroll_by(host, -1);
                "scroll up".to_owned()
            }
            WidgetHitTarget::ScrollDownBtn => {
                self.scroll_by(host, 1);
                "scroll down".to_owned()
            }
            WidgetHitTarget::EventRow(idx) => {
                if let Some(ev) = filtered.get(self.st.scroll_offset + idx) {
                    let _ = host.open_url(&event_day_url(ev));
                }
                format!("click: event row {idx}")
            }
            WidgetHitTarget::HeaderDrag | WidgetHitTarget::None => String::new(),
        }
    }

    pub fn on_right_click(&mut self, host: &mut dyn WidgetPlatform, screen: PixelPoint) -> String {
        let state = WidgetMenuState {
            layer: self.config.layer,
            opacity_percent: self.config.opacity_percent,
        };
        let Ok(Some(action)) = host.show_widget_menu(CALENDAR_WIDGET_ID, screen, &state) else {
            return String::new();
        };
        match action {
            WidgetMenuAction::Refresh => self.reload_config_and_refresh(host),
            WidgetMenuAction::OpenSettings => "open_settings".to_owned(),
            WidgetMenuAction::OpenConfig => self.open_cfg_log(host, "menu"),
            WidgetMenuAction::SetLayerWallpaper => self.set_layer(host, WidgetLayer::Wallpaper),
            WidgetMenuAction::SetLayerNormal => self.set_layer(host, WidgetLayer::Normal),
            WidgetMenuAction::SetLayerTopmost => self.set_layer(host, WidgetLayer::Topmost),
            WidgetMenuAction::SetOpacity(pct) => {
                self.config.opacity_percent = pct.clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT);
                self.persist();
                self.redraw(host);
                format!("opacity={}%", self.config.opacity_percent)
            }
            WidgetMenuAction::HideWidget => {
                self.config.visible = false;
                self.persist();
                let _ = self.sync_window(host, false);
                "widget hidden".to_owned()
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
    ) -> Option<(bool, String)> {
        let source_id = self.st.pending_requests.remove(&request_id)?;
        let out = match evaluate_http_response(&source_id, result) {
            HttpEval::Err {
                status_err,
                log_msg,
            } => {
                self.st.status_error = Some(status_err);
                (true, log_msg)
            }
            HttpEval::Ok(body) => {
                let bytes_len = body.len();
                let _ = save_cached_ics(&self.cache_dir, &source_id, &body);
                self.st.cached_by_source.insert(source_id.clone(), body);
                let now = host.local_time_now();
                self.st.last_updated = Some(format!("{:02}:{:02}", now.hour, now.minute));
                self.st.status_error = None;
                self.st.reparse(&self.config, now);
                let msg = format!(
                    "http[{source_id}] 200 OK ({bytes_len} B, events={})",
                    self.st.all_events.len()
                );
                (false, msg)
            }
        };
        if self.window_open {
            self.redraw(host);
        }
        Some(out)
    }

    fn open_url_log(&self, host: &dyn WidgetPlatform, url: &str, tag: &str) -> String {
        let _ = host.open_url(url);
        format!("click: {tag}")
    }

    fn open_cfg_log(&self, host: &dyn WidgetPlatform, tag: &str) -> String {
        match self.open_config_file(host) {
            Ok(()) => format!("{tag}: open widgets.cfg"),
            Err(err) => format!("error: open widgets.cfg failed: {err}"),
        }
    }

    pub fn reload_config_and_refresh(&mut self, host: &mut dyn WidgetPlatform) -> String {
        match load_widget_config(&self.config_path) {
            Ok(reloaded) => {
                self.config = reloaded;
                self.st.status_error = None;
            }
            Err(err) => {
                self.st.status_error = Some(format!("Ошибка конфига: {err}"));
                self.redraw(host);
                return format!("error: reload widgets.cfg failed: {err}");
            }
        }
        let valid_ids: Vec<String> = self.config.sources.iter().map(|s| s.id.clone()).collect();
        self.st
            .cached_by_source
            .retain(|k, _| valid_ids.contains(k));
        self.st.load_caches(&self.config, &self.cache_dir);
        let now = host.local_time_now();
        self.st.reparse(&self.config, now);
        if self.window_open {
            let _ = host.set_widget_layer(CALENDAR_WIDGET_ID, self.config.layer);
        }
        self.sync_timers(host);
        self.trigger_network_refresh(host);
        self.redraw(host);
        format!(
            "reload widgets.cfg: sources={}, opacity={}%",
            self.config.sources.len(),
            self.config.opacity_percent
        )
    }

    fn trigger_network_refresh(&mut self, host: &mut dyn WidgetPlatform) {
        if self.config.sources.is_empty() {
            return;
        }
        self.st.status_error = None;
        for src in &self.config.sources {
            let req_id = self.st.next_request_id;
            self.st.next_request_id = self.st.next_request_id.saturating_add(1);
            match host.start_http_get(req_id, &src.url) {
                Ok(()) => {
                    self.st.pending_requests.insert(req_id, src.id.clone());
                }
                Err(err) => self.st.status_error = Some(format!("HTTP запуск: {err}")),
            }
        }
        self.redraw(host);
    }

    fn set_layer(&mut self, host: &mut dyn WidgetPlatform, layer: WidgetLayer) -> String {
        self.config.layer = layer;
        let _ = host.set_widget_layer(CALENDAR_WIDGET_ID, layer);
        self.persist();
        format!("layer={layer:?}")
    }

    fn scroll_by(&mut self, host: &mut dyn WidgetPlatform, delta: isize) {
        let now = host.local_time_now();
        let total = filter_events(&self.st.all_events, &self.config, now).len();
        let next = clamp_scroll_offset(
            self.st.scroll_offset,
            delta,
            total,
            self.config.visible_rows,
        );
        if next != self.st.scroll_offset {
            self.st.scroll_offset = next;
            self.redraw(host);
        }
    }

    fn redraw(&mut self, host: &mut dyn WidgetPlatform) {
        if !self.window_open {
            return;
        }
        let now = host.local_time_now();
        let (frame, scroll_offset, shown) = build_calendar_frame(&self.config, now, &self.st);
        self.st.scroll_offset = scroll_offset;
        self.st.shown_row_count = shown;
        let _ = host.present_widget_frame(CALENDAR_WIDGET_ID, self.window_rect(), &frame);
    }

    fn sync_timers(&self, host: &mut dyn WidgetPlatform) {
        let minute = self.config.date_format == DateFormatMode::Countdown;
        let _ =
            host.set_widget_timers(CALENDAR_WIDGET_ID, self.config.refresh_interval_sec, minute);
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
