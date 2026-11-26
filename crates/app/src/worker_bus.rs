#![cfg(windows)]

use std::time::Instant;

use asrbox_core::{IsolationMode, ModuleId, SupervisorCmd, WorkerMsg};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_settings::{BrowserTab, SettingsTab, WidgetSubTab};
use asrbox_platform_api::{
    DockPlatform, IconProvider, LocalDateTime, ToolsPlatform, WidgetPlatform,
};

use crate::supervisor::SupervisorEvent;
use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn poll_supervisor<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
    ) {
        self.poll_slint_ui(host);
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
                    "weather" => self.open_widgets_subtab(host, WidgetSubTab::Weather),
                    "notes" => self.open_widgets_subtab(host, WidgetSubTab::Notes),
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
                docks.menu_vis = self.menus_cfg.dock;
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
        match cmd {
            SupervisorCmd::ReloadConfig => {
                let wvis = self.menus_cfg.widget;
                if let Some(c) = self.calendar.as_mut() {
                    c.set_menu_visibility(wvis);
                    let _ = c.reload_config_and_refresh(host);
                    let _ = c.sync_window(host, self.safe_mode);
                }
                if let Some(s) = self.sysmon.as_mut() {
                    s.set_menu_visibility(wvis);
                    let _ = s.reload_config(host, self.safe_mode);
                }
                if let Some(w) = self.weather.as_mut() {
                    w.set_menu_visibility(wvis);
                    let _ = w.reload_config(host, self.safe_mode);
                }
            }
            SupervisorCmd::ToggleCalendar => {
                if let Some(c) = self.calendar.as_mut() {
                    c.toggle_visible(host, false);
                }
            }
            SupervisorCmd::RefreshCalendar => {
                if let Some(c) = self.calendar.as_mut() {
                    c.on_refresh_tick(host);
                }
            }
            SupervisorCmd::ToggleSysMonitor => {
                if let Some(s) = self.sysmon.as_mut() {
                    s.toggle_visible(host, false);
                }
            }
            SupervisorCmd::RefreshSysMonitor => {
                if let Some(s) = self.sysmon.as_mut() {
                    s.on_refresh_tick(host);
                }
            }
            SupervisorCmd::ToggleWeather => {
                if let Some(w) = self.weather.as_mut() {
                    w.toggle_visible(host, false);
                }
            }
            SupervisorCmd::RefreshWeather => {
                if let Some(w) = self.weather.as_mut() {
                    w.on_refresh_tick(host);
                }
            }
            SupervisorCmd::ToggleNotes => self.toggle_slint_notes(host),
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

pub(crate) fn level_for_msg(msg: &str) -> LogLevel {
    if msg.starts_with("error:") {
        LogLevel::Error
    } else {
        LogLevel::Info
    }
}
