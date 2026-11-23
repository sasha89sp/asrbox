#![cfg(windows)]

use std::time::Instant;

use asrbox_core::{IsolationMode, ModuleId, SupervisorCmd, WorkerMsg};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_settings::SettingsTab;
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
                    if self.handle_worker_msg(host, now, msg) {
                        settings_dirty = true;
                    }
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
                    self.debug.log(
                        now,
                        LogLevel::Error,
                        "app",
                        format!("Сбой процесса {} (код {:?}) -> {act}", id.as_str(), code),
                    );
                }
                SupervisorEvent::Error(e) => self.debug.log(now, LogLevel::Error, "app", e),
            }
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
                self.debug.log(
                    now,
                    LogLevel::Info,
                    "app",
                    format!("Модуль {module} готов (PID {pid})"),
                );
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
                let mod_tag: &'static str = match module.as_str() {
                    "dock" => "dock",
                    "widgets" => "widgets",
                    "http" => "http",
                    _ => "app",
                };
                self.debug.log(now, lvl, mod_tag, message);
                false
            }
            WorkerMsg::OpenSettings { tab } => {
                let t = match tab.as_str() {
                    "dock" => SettingsTab::Dock,
                    "calendar" => SettingsTab::Calendar,
                    _ => SettingsTab::Modules,
                };
                self.open_settings_tab(host, t);
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
                let tag: &'static str = match module.as_str() {
                    "dock" => "dock",
                    "widgets" => "widgets",
                    _ => "app",
                };
                self.debug.record_event(tag, "worker_metrics");
                self.debug.log(
                    now,
                    LogLevel::Trace,
                    tag,
                    format!(
                        "worker[{module}]: RAM {:.1} МБ, CPU {}.{}%, ev={events}, fr={frames}, max={:.1}мс",
                        ram_kb as f64 / 1024.0,
                        cpu_tenths / 10,
                        cpu_tenths % 10,
                        max_us as f64 / 1000.0
                    ),
                );
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
        let Some(cal) = self.calendar.as_mut() else {
            return;
        };
        match cmd {
            SupervisorCmd::ReloadConfig => {
                let _ = cal.reload_config_and_refresh(host);
                let _ = cal.sync_window(host, self.safe_mode);
            }
            SupervisorCmd::ToggleCalendar => cal.toggle_visible(host, false),
            SupervisorCmd::RefreshCalendar => cal.on_refresh_tick(host),
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
        }
    }

    pub(crate) fn handle_widget_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: PlatformEvent,
    ) {
        let t0 = Instant::now();
        let mut open_settings = false;
        let mut msg_out: Option<(LogLevel, &'static str, String)> = None;
        {
            let Some(cal) = self.calendar.as_mut() else {
                return;
            };
            match ev {
                PlatformEvent::WidgetPointerMove { point, .. } => cal.on_pointer_move(host, point),
                PlatformEvent::WidgetPointerLeave { .. } => cal.on_pointer_leave(host),
                PlatformEvent::WidgetLeftClick { point, .. } => {
                    let m = cal.on_left_click(host, point);
                    if m == "open_settings" {
                        open_settings = true;
                    } else if !m.is_empty() {
                        msg_out = Some((LogLevel::Info, "widgets", m));
                    }
                    self.record_frame("widgets", t0);
                }
                PlatformEvent::WidgetRightClick { screen_point, .. } => {
                    let m = cal.on_right_click(host, screen_point);
                    if m == "open_settings" {
                        open_settings = true;
                    } else if !m.is_empty() {
                        msg_out = Some((LogLevel::Info, "widgets", m));
                    }
                }
                PlatformEvent::WidgetWheel { delta, .. } => {
                    cal.on_wheel(host, delta);
                    self.record_frame("widgets", t0);
                }
                PlatformEvent::WidgetMoved { x, y, .. } => cal.on_moved(x, y),
                PlatformEvent::WidgetMinuteTick { .. } => cal.on_minute_tick(host),
                PlatformEvent::WidgetRefreshTick { .. } => cal.on_refresh_tick(host),
                PlatformEvent::HttpFetchDone { request_id, result } => {
                    if let Some((is_err, m)) = cal.on_http_done(host, request_id, result) {
                        let lvl = if is_err {
                            LogLevel::Error
                        } else {
                            LogLevel::Info
                        };
                        msg_out = Some((lvl, "http", m));
                    }
                    self.record_frame("widgets", t0);
                }
                _ => {}
            }
        }
        if open_settings {
            self.open_settings_tab(host, SettingsTab::Calendar);
        }
        if let Some((lvl, tag, m)) = msg_out {
            self.debug.log(host.local_time_now(), lvl, tag, m);
            self.render_settings_ui(host);
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
        let lvl = if msg.starts_with("error:") {
            LogLevel::Error
        } else {
            LogLevel::Info
        };
        self.debug.log(host.local_time_now(), lvl, module, msg);
    }
}
