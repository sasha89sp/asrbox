#![cfg(windows)]

use std::time::Instant;

use asrbox_core::IsolationMode;
use asrbox_mod_debug::LogLevel;
use asrbox_mod_settings::{SettingsTab, WidgetSubTab};
use asrbox_mod_widgets::{
    CalendarWidget, SysMonitorWidget, WeatherWidget, CALENDAR_WIDGET_ID, SYS_WIDGET_ID,
    WEATHER_WIDGET_ID,
};
use asrbox_platform_api::{
    DockPlatform, FluentPalette, IconProvider, PixelPoint, PlatformEvent, ToolsPlatform,
    WidgetPlatform,
};

use crate::worker_bus::level_for_msg;
use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn handle_widget_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: PlatformEvent,
    ) {
        if self.handle_overlay_inproc_event(host, &ev) {
            return;
        }
        let t0 = Instant::now();
        let mut open_tab: Option<(SettingsTab, Option<WidgetSubTab>)> = None;
        let mut msg_out: Option<(LogLevel, &'static str, String)> = None;
        match ev {
            PlatformEvent::WidgetPointerMove { widget_id, point } => match widget_id {
                CALENDAR_WIDGET_ID => {
                    if let Some(c) = self.calendar.as_mut() {
                        c.on_pointer_move(host, point);
                    }
                }
                SYS_WIDGET_ID => {
                    if let Some(s) = self.sysmon.as_mut() {
                        s.on_pointer_move(host, point);
                    }
                }
                WEATHER_WIDGET_ID => {
                    if let Some(w) = self.weather.as_mut() {
                        w.on_pointer_move(host, point);
                    }
                }
                _ => {}
            },
            PlatformEvent::WidgetPointerLeave { widget_id } => match widget_id {
                CALENDAR_WIDGET_ID => {
                    if let Some(c) = self.calendar.as_mut() {
                        c.on_pointer_leave(host);
                    }
                }
                SYS_WIDGET_ID => {
                    if let Some(s) = self.sysmon.as_mut() {
                        s.on_pointer_leave(host);
                    }
                }
                WEATHER_WIDGET_ID => {
                    if let Some(w) = self.weather.as_mut() {
                        w.on_pointer_leave(host);
                    }
                }
                _ => {}
            },
            PlatformEvent::WidgetLeftClick { widget_id, point } => {
                let m = self.click_widget(host, widget_id, point, false);
                classify_widget_reply(m, &mut open_tab, &mut msg_out);
                self.record_frame("widgets", t0);
            }
            PlatformEvent::WidgetRightClick {
                widget_id,
                screen_point,
            } => {
                let m = self.click_widget(host, widget_id, screen_point, true);
                classify_widget_reply(m, &mut open_tab, &mut msg_out);
            }
            PlatformEvent::WidgetWheel { widget_id, delta } if widget_id == CALENDAR_WIDGET_ID => {
                if let Some(c) = self.calendar.as_mut() {
                    c.on_wheel(host, delta);
                    self.record_frame("widgets", t0);
                }
            }
            other => self.handle_widget_state_event(host, other, t0, &mut msg_out),
        }
        if let Some((tab, sub)) = open_tab {
            if let Some(wsub) = sub {
                self.open_widgets_subtab(host, wsub);
            } else {
                self.open_settings_tab(host, tab);
            }
        }
        if let Some((lvl, tag, m)) = msg_out {
            self.debug.log(host.local_time_now(), lvl, tag, m);
            self.render_settings_ui(host);
        }
    }

    fn click_widget<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        widget_id: u64,
        pt: PixelPoint,
        right: bool,
    ) -> String {
        match widget_id {
            CALENDAR_WIDGET_ID => self.calendar.as_mut().map_or_else(String::new, |c| {
                if right {
                    c.on_right_click(host, pt)
                } else {
                    c.on_left_click(host, pt)
                }
            }),
            SYS_WIDGET_ID => self.sysmon.as_mut().map_or_else(String::new, |s| {
                if right {
                    s.on_right_click(host, pt)
                } else {
                    s.on_left_click(host, pt)
                }
            }),
            WEATHER_WIDGET_ID => self.weather.as_mut().map_or_else(String::new, |w| {
                if right {
                    w.on_right_click(host, pt)
                } else {
                    w.on_left_click(host, pt)
                }
            }),
            _ => String::new(),
        }
    }

    fn handle_widget_state_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: PlatformEvent,
        t0: Instant,
        msg_out: &mut Option<(LogLevel, &'static str, String)>,
    ) {
        match ev {
            PlatformEvent::WidgetMoved { widget_id, x, y } => match widget_id {
                CALENDAR_WIDGET_ID => {
                    if let Some(c) = self.calendar.as_mut() {
                        c.on_moved(x, y);
                    }
                }
                SYS_WIDGET_ID => {
                    if let Some(s) = self.sysmon.as_mut() {
                        s.on_moved(x, y);
                    }
                }
                WEATHER_WIDGET_ID => {
                    if let Some(w) = self.weather.as_mut() {
                        w.on_moved(x, y);
                    }
                }
                _ => {}
            },
            PlatformEvent::WidgetResized {
                widget_id,
                x,
                y,
                width,
                height,
                finished,
            } => {
                match widget_id {
                    CALENDAR_WIDGET_ID => {
                        if let Some(c) = self.calendar.as_mut() {
                            c.on_resized(host, x, y, width, height, finished);
                        }
                    }
                    SYS_WIDGET_ID => {
                        if let Some(s) = self.sysmon.as_mut() {
                            s.on_resized(host, x, y, width, height, finished);
                        }
                    }
                    WEATHER_WIDGET_ID => {
                        if let Some(w) = self.weather.as_mut() {
                            w.on_resized(host, x, y, width, height, finished);
                        }
                    }
                    _ => {}
                }
                if finished {
                    self.render_settings_ui(host);
                }
                self.record_frame("widgets", t0);
            }
            PlatformEvent::WidgetMinuteTick {
                widget_id: CALENDAR_WIDGET_ID,
            } => {
                if let Some(c) = self.calendar.as_mut() {
                    c.on_minute_tick(host);
                }
            }
            PlatformEvent::WidgetRefreshTick { widget_id } => match widget_id {
                CALENDAR_WIDGET_ID => {
                    if let Some(c) = self.calendar.as_mut() {
                        c.on_refresh_tick(host);
                    }
                }
                SYS_WIDGET_ID => {
                    if let Some(s) = self.sysmon.as_mut() {
                        s.on_refresh_tick(host);
                    }
                }
                WEATHER_WIDGET_ID => {
                    if let Some(w) = self.weather.as_mut() {
                        w.on_refresh_tick(host);
                    }
                }
                _ => {}
            },
            PlatformEvent::SysTelemetryReady { snapshot } => {
                if let Some(s) = self.sysmon.as_mut() {
                    s.on_telemetry_ready(host, *snapshot);
                    self.record_frame("widgets", t0);
                }
            }
            PlatformEvent::HttpFetchDone { request_id, result } => {
                let outcome = if request_id >= 100_000 {
                    self.weather
                        .as_mut()
                        .and_then(|w| w.on_http_done(host, request_id, result))
                } else {
                    self.calendar
                        .as_mut()
                        .and_then(|c| c.on_http_done(host, request_id, result))
                };
                if let Some((is_err, m)) = outcome {
                    let lvl = if is_err {
                        LogLevel::Error
                    } else {
                        LogLevel::Info
                    };
                    *msg_out = Some((lvl, "http", m));
                }
                self.record_frame("widgets", t0);
            }
            _ => {}
        }
    }

    pub(crate) fn sync_inproc_widgets<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        pal: FluentPalette,
    ) {
        let now = host.local_time_now();
        let want_inproc_w =
            self.modules_cfg.isolation == IsolationMode::InProc && self.modules_cfg.widgets_enabled;
        let wvis = self.menus_cfg.widget;
        if want_inproc_w && self.calendar.is_none() {
            let mut cal =
                CalendarWidget::load(self.paths.widgets_cfg.clone(), self.paths.cache_dir.clone());
            cal.set_menu_visibility(wvis);
            cal.set_palette(host, pal);
            let _ = cal.sync_window(host, self.safe_mode);
            self.debug.log(
                now,
                LogLevel::Info,
                "widgets",
                format!("Календарь (inproc): {}", cal.status_summary()),
            );
            self.calendar = Some(cal);
        } else if !want_inproc_w && self.calendar.take().is_some() {
            let _ = host.destroy_widget_window(CALENDAR_WIDGET_ID);
        }
        if want_inproc_w && self.sysmon.is_none() {
            let mut sys = SysMonitorWidget::load(self.paths.sysmon_cfg.clone());
            sys.set_menu_visibility(wvis);
            sys.set_palette(host, pal);
            let _ = sys.sync_window(host, self.safe_mode);
            self.sysmon = Some(sys);
        } else if !want_inproc_w && self.sysmon.take().is_some() {
            let _ = host.destroy_widget_window(SYS_WIDGET_ID);
        }
        if want_inproc_w && self.weather.is_none() {
            let mut weather = WeatherWidget::load(self.paths.weather_cfg.clone());
            weather.set_menu_visibility(wvis);
            weather.set_palette(host, pal);
            let _ = weather.sync_window(host, self.safe_mode);
            self.weather = Some(weather);
        } else if !want_inproc_w && self.weather.take().is_some() {
            let _ = host.destroy_widget_window(WEATHER_WIDGET_ID);
        }
        if !self.modules_cfg.widgets_enabled {
            self.slint_ui.close_notes();
        }
    }
}

fn classify_widget_reply(
    msg: String,
    open_tab: &mut Option<(SettingsTab, Option<WidgetSubTab>)>,
    msg_out: &mut Option<(LogLevel, &'static str, String)>,
) {
    match msg.as_str() {
        "" => {}
        "open_settings" => *open_tab = Some((SettingsTab::Widgets, Some(WidgetSubTab::Calendar))),
        "open_settings_sysmon" => {
            *open_tab = Some((SettingsTab::Widgets, Some(WidgetSubTab::SysMonitor)))
        }
        "open_settings_weather" => {
            *open_tab = Some((SettingsTab::Widgets, Some(WidgetSubTab::Weather)))
        }
        "open_settings_notes" => {
            *open_tab = Some((SettingsTab::Widgets, Some(WidgetSubTab::Notes)))
        }
        _ => *msg_out = Some((level_for_msg(&msg), "widgets", msg)),
    }
}
