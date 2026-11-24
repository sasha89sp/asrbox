#![cfg(windows)]

use std::time::Instant;

use asrbox_core::{IsolationMode, ModuleId, SupervisorCmd, WorkerMsg};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_settings::{BrowserTab, SettingsTab, WidgetSubTab};
use asrbox_mod_widgets::{CALENDAR_WIDGET_ID, SYS_WIDGET_ID};
use asrbox_platform_api::{
    DockPlatform, IconProvider, LocalDateTime, PlatformEvent, ToolsPlatform, WidgetPlatform,
};

use crate::supervisor::SupervisorEvent;
use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn poll_supervisor<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
    ) {
        let events = self.supervisor.poll(&self.modules_cfg);
        let now = host.local_time_now();
        let mut settings_dirty = false;
        for ev in events {
            match ev {
                SupervisorEvent::WorkerMsg(msg) => {
                    settings_dirty |= self.handle_worker_msg(host, now, msg);
                }
                SupervisorEvent::WorkerCrashed {
                    id,
                    code,
                    restarted,
                } => {
                    settings_dirty = true;
                    let act = if restarted {
                        "автоперезапуск"
                    } else {
                        "остановлен"
                    };
                    let msg = format!("Сбой процесса {} (код {:?}) -> {act}", id.as_str(), code);
                    self.debug.log(now, LogLevel::Error, "app", msg);
                }
                SupervisorEvent::Error(e) => self.debug.log(now, LogLevel::Error, "app", e),
            }
        }
        if let Some(msg) = self.backup.maybe_auto_sync(host, now) {
            self.debug.log(now, LogLevel::Info, "webdav", msg);
            settings_dirty = true;
        }
        if host.is_settings_window_open()
            && self.diagnostics_monitor_enabled
            && self.settings_ui.active_browser_tab == BrowserTab::Diagnostics
        {
            settings_dirty = true;
        }
        if settings_dirty {
            self.render_settings_ui(host);
        }
    }

    fn handle_worker_msg<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        now: LocalDateTime,
        msg: WorkerMsg,
    ) -> bool {
        match msg {
            WorkerMsg::Ready { module, pid } => {
                let text = format!("Модуль {module} готов (PID {pid})");
                self.debug.log(now, LogLevel::Info, "app", text);
                if module == "overlay" && self.overlay_setup_mode {
                    let cmd = SupervisorCmd::SetOverlaySetupMode(true);
                    let _ = self.supervisor.send_cmd(ModuleId::Overlay, &cmd);
                }
                true
            }
            WorkerMsg::Log {
                level,
                module,
                message,
            } => {
                let lvl = match level.as_str() {
                    "ERROR" => LogLevel::Error,
                    "WARN" => LogLevel::Warn,
                    "TRACE" => LogLevel::Trace,
                    _ => LogLevel::Info,
                };
                self.debug.log(now, lvl, map_module_tag(&module), message);
                false
            }
            WorkerMsg::OpenSettings { tab } => {
                match tab.as_str() {
                    "dock" => self.open_settings_tab(host, SettingsTab::Dock),
                    "calendar" => self.open_widgets_subtab(host, WidgetSubTab::Calendar),
                    "sysmon" => self.open_widgets_subtab(host, WidgetSubTab::SysMonitor),
                    "overlay" => self.open_settings_tab(host, SettingsTab::Overlay),
                    _ => self.open_settings_tab(host, SettingsTab::General),
                }
                false
            }
            WorkerMsg::ConfigChanged { .. } => true,
            WorkerMsg::Metrics {
                module,
                ram_kb,
                cpu_tenths,
                events,
                frames,
                max_us,
                ..
            } => {
                let tag = map_module_tag(&module);
                self.debug.record_event(tag, "worker_metrics");
                let text = format!(
                    "worker[{module}]: RAM {:.1} МБ, CPU {}.{}%, ev={events}, fr={frames}, max={:.1}мс",
                    ram_kb as f64 / 1024.0,
                    cpu_tenths / 10,
                    cpu_tenths % 10,
                    max_us as f64 / 1000.0
                );
                self.debug.log(now, LogLevel::Trace, tag, text);
                false
            }
        }
    }

    pub(crate) fn send_or_run_dock_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        cmd: SupervisorCmd,
    ) {
        if self.modules_cfg.isolation == IsolationMode::Process {
            let _ = self.supervisor.send_cmd(ModuleId::Dock, &cmd);
            return;
        }
        let Some(docks) = self.docks.as_mut() else {
            return;
        };
        match cmd {
            SupervisorCmd::ReloadConfig => {
                let _ = docks.reload_from_disk(host, self.safe_mode);
            }
            SupervisorCmd::AddDock => docks.add_new_dock(host, &mut self.safe_mode),
            SupervisorCmd::ToggleDock(id) => docks.toggle_dock(host, id, self.safe_mode),
            SupervisorCmd::RemoveDock(id) => docks.remove_dock(host, id),
            _ => {}
        }
    }

    pub(crate) fn send_or_run_widgets_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        cmd: SupervisorCmd,
    ) {
        if self.modules_cfg.isolation == IsolationMode::Process {
            let _ = self.supervisor.send_cmd(ModuleId::Widgets, &cmd);
            return;
        }
        match (cmd, self.calendar.as_mut(), self.sysmon.as_mut()) {
            (SupervisorCmd::ReloadConfig, cal, sys) => {
                if let Some(c) = cal {
                    let _ = c.reload_config_and_refresh(host);
                    let _ = c.sync_window(host, self.safe_mode);
                }
                if let Some(s) = sys {
                    let _ = s.reload_config(host, self.safe_mode);
                }
            }
            (SupervisorCmd::ToggleCalendar, Some(c), _) => c.toggle_visible(host, false),
            (SupervisorCmd::RefreshCalendar, Some(c), _) => c.on_refresh_tick(host),
            (SupervisorCmd::ToggleSysMonitor, _, Some(s)) => s.toggle_visible(host, false),
            (SupervisorCmd::RefreshSysMonitor, _, Some(s)) => s.on_refresh_tick(host),
            _ => {}
        }
    }

    pub(crate) fn reload_module_config<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        id: ModuleId,
    ) {
        match id {
            ModuleId::Dock => self.send_or_run_dock_cmd(host, SupervisorCmd::ReloadConfig),
            ModuleId::Widgets => self.send_or_run_widgets_cmd(host, SupervisorCmd::ReloadConfig),
            ModuleId::Overlay => self.send_or_run_overlay_cmd(host, SupervisorCmd::ReloadConfig),
        }
    }

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
            PlatformEvent::WidgetPointerMove { widget_id, point } => {
                match (widget_id, self.calendar.as_mut(), self.sysmon.as_mut()) {
                    (CALENDAR_WIDGET_ID, Some(c), _) => c.on_pointer_move(host, point),
                    (SYS_WIDGET_ID, _, Some(s)) => s.on_pointer_move(host, point),
                    _ => {}
                }
            }
            PlatformEvent::WidgetPointerLeave { widget_id } => {
                match (widget_id, self.calendar.as_mut(), self.sysmon.as_mut()) {
                    (CALENDAR_WIDGET_ID, Some(c), _) => c.on_pointer_leave(host),
                    (SYS_WIDGET_ID, _, Some(s)) => s.on_pointer_leave(host),
                    _ => {}
                }
            }
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
        pt: asrbox_platform_api::PixelPoint,
        right: bool,
    ) -> String {
        match (widget_id, self.calendar.as_mut(), self.sysmon.as_mut()) {
            (CALENDAR_WIDGET_ID, Some(c), _) if right => c.on_right_click(host, pt),
            (CALENDAR_WIDGET_ID, Some(c), _) => c.on_left_click(host, pt),
            (SYS_WIDGET_ID, _, Some(s)) if right => s.on_right_click(host, pt),
            (SYS_WIDGET_ID, _, Some(s)) => s.on_left_click(host, pt),
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
            PlatformEvent::WidgetMoved { widget_id, x, y } => {
                match (widget_id, self.calendar.as_mut(), self.sysmon.as_mut()) {
                    (CALENDAR_WIDGET_ID, Some(c), _) => c.on_moved(x, y),
                    (SYS_WIDGET_ID, _, Some(s)) => s.on_moved(x, y),
                    _ => {}
                }
            }
            PlatformEvent::WidgetResized {
                widget_id,
                x,
                y,
                width,
                height,
                finished,
            } => {
                match (widget_id, self.calendar.as_mut(), self.sysmon.as_mut()) {
                    (CALENDAR_WIDGET_ID, Some(c), _) => {
                        c.on_resized(host, x, y, width, height, finished);
                    }
                    (SYS_WIDGET_ID, _, Some(s)) => {
                        s.on_resized(host, x, y, width, height, finished);
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
            PlatformEvent::WidgetRefreshTick { widget_id } => {
                match (widget_id, self.calendar.as_mut(), self.sysmon.as_mut()) {
                    (CALENDAR_WIDGET_ID, Some(c), _) => c.on_refresh_tick(host),
                    (SYS_WIDGET_ID, _, Some(s)) => s.on_refresh_tick(host),
                    _ => {}
                }
            }
            PlatformEvent::SysTelemetryReady { snapshot } => {
                if let Some(s) = self.sysmon.as_mut() {
                    s.on_telemetry_ready(host, *snapshot);
                    self.record_frame("widgets", t0);
                }
            }
            PlatformEvent::HttpFetchDone { request_id, result } => {
                if let Some(c) = self.calendar.as_mut() {
                    if let Some((is_err, m)) = c.on_http_done(host, request_id, result) {
                        let lvl = if is_err {
                            LogLevel::Error
                        } else {
                            LogLevel::Info
                        };
                        *msg_out = Some((lvl, "http", m));
                    }
                    self.record_frame("widgets", t0);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn record_frame(&mut self, tag: &'static str, t0: Instant) {
        let us = u64::try_from(t0.elapsed().as_micros()).unwrap_or(0);
        self.debug.record_frame(tag, us);
    }

    pub(crate) fn log_tagged<H: WidgetPlatform>(
        &mut self,
        host: &H,
        module: &'static str,
        msg: String,
    ) {
        let lvl = level_for_msg(&msg);
        self.debug.log(host.local_time_now(), lvl, module, msg);
    }
}

fn map_module_tag(module: &str) -> &'static str {
    match module {
        "dock" => "dock",
        "widgets" => "widgets",
        "overlay" => "overlay",
        "http" => "http",
        _ => "app",
    }
}

fn level_for_msg(msg: &str) -> LogLevel {
    if msg.starts_with("error:") {
        LogLevel::Error
    } else {
        LogLevel::Info
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
        _ => *msg_out = Some((level_for_msg(&msg), "widgets", msg)),
    }
}
