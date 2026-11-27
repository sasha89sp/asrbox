#![cfg(windows)]

use std::env;
use std::path::PathBuf;

use asrbox_core::SupervisorCmd;
use asrbox_mod_about::{build_paths_clipboard, AppPaths};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_dock::load_docks_from_file;
use asrbox_mod_settings::{BrowserTab, SettingsTab};
use asrbox_mod_widgets::{load_sysmon_config, load_weather_config, load_widget_config};
use asrbox_platform_api::{
    AboutUiCommand, DebugUiCommand, DockPlatform, IconProvider, PixelPoint, ToolsPlatform,
    TrayAction, TrayMenuEntry, TrayModuleFlags, WidgetPlatform,
};

use crate::{AppController, APP_VERSION};

pub(crate) fn resolve_app_paths() -> AppPaths {
    let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("asrbox.exe"));
    let base = exe
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    AppPaths::from_exe_and_data(exe, base.join("data"))
}

impl<I: IconProvider> AppController<I> {
    pub(crate) fn on_tray<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        screen: PixelPoint,
    ) {
        self.debug.record_event("app", "tray_menu");
        let entries = self.current_tray_entries();
        let cal_vis = self
            .calendar
            .as_ref()
            .map(|c| c.is_visible())
            .unwrap_or_else(|| {
                load_widget_config(&self.paths.widgets_cfg)
                    .map(|c| c.visible)
                    .unwrap_or(true)
            });
        let sys_vis = self
            .sysmon
            .as_ref()
            .map(|s| s.is_visible())
            .unwrap_or_else(|| {
                load_sysmon_config(&self.paths.sysmon_cfg)
                    .map(|c| c.visible)
                    .unwrap_or(true)
            });
        let weather_vis = self
            .weather
            .as_ref()
            .map(|w| w.is_visible())
            .unwrap_or_else(|| {
                load_weather_config(&self.paths.weather_cfg)
                    .map(|c| c.visible)
                    .unwrap_or(true)
            });
        let notes_vis = self.slint_ui.is_notes_open();
        let flags = TrayModuleFlags {
            dock_module_enabled: self.modules_cfg.dock_enabled,
            widgets_module_enabled: self.modules_cfg.widgets_enabled,
            overlay_module_enabled: self.modules_cfg.overlay_enabled,
            overlay_setup_mode: self.overlay_setup_mode,
            calendar_visible: cal_vis,
            sys_monitor_visible: sys_vis,
            weather_visible: weather_vis,
            notes_visible: notes_vis,
            autostart_enabled: host.is_autostart_enabled(),
            menu_vis: self.menus_cfg.tray,
        };
        match host.show_tray_menu(screen, &entries, flags) {
            Ok(Some(action)) => self.apply_tray_action(host, action),
            Ok(None) => {}
            Err(_) => {
                let _ = self.slint_ui.show_tray_menu(screen, &entries, flags);
            }
        }
    }

    fn current_tray_entries(&self) -> Vec<TrayMenuEntry> {
        if let Some(docks) = &self.docks {
            return docks.tray_entries();
        }
        load_docks_from_file(&self.paths.docks_cfg)
            .unwrap_or_default()
            .into_iter()
            .map(|d| TrayMenuEntry {
                dock_id: d.id,
                title: d.name,
                enabled: d.enabled,
            })
            .collect()
    }

    pub(crate) fn apply_tray_action<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: TrayAction,
    ) {
        use asrbox_mod_backup::BackupMode;
        let now = host.local_time_now();
        match action {
            TrayAction::OpenSettings => self.open_settings_tab(host, SettingsTab::General),
            TrayAction::ToggleAutostart => {
                let next = !host.is_autostart_enabled();
                let res = host.set_autostart_enabled(next);
                let msg = if next {
                    "Трей: автозагрузка включена"
                } else {
                    "Трей: автозагрузка отключена"
                };
                self.log_io_result(now, "app", msg, res);
                self.render_settings_ui(host);
            }
            TrayAction::ToggleDockModule => {
                self.modules_cfg.dock_enabled = !self.modules_cfg.dock_enabled;
                self.persist_modules_cfg();
                self.sync_runtime_modules(host);
                self.render_settings_ui(host);
            }
            TrayAction::ToggleWidgetsModule => {
                self.modules_cfg.widgets_enabled = !self.modules_cfg.widgets_enabled;
                self.persist_modules_cfg();
                self.sync_runtime_modules(host);
                self.render_settings_ui(host);
            }
            TrayAction::AddDock => {
                self.send_or_run_dock_cmd(host, SupervisorCmd::AddDock);
                self.debug
                    .log(now, LogLevel::Info, "dock", "Трей: добавлена док-панель");
            }
            TrayAction::ToggleDock(id) => {
                self.send_or_run_dock_cmd(host, SupervisorCmd::ToggleDock(id));
                self.debug
                    .log(now, LogLevel::Info, "dock", format!("Трей: док {id}"));
            }
            TrayAction::RemoveDock(id) => {
                self.send_or_run_dock_cmd(host, SupervisorCmd::RemoveDock(id));
                self.debug.log(
                    now,
                    LogLevel::Info,
                    "dock",
                    format!("Трей: удалён док {id}"),
                );
            }
            TrayAction::OpenDockConfig => {
                let res = ToolsPlatform::open_text_file(host, &self.paths.docks_cfg);
                self.log_io_result(now, "dock", "Трей: открыт docks.cfg", res);
            }
            TrayAction::ToggleCalendarWidget => {
                self.safe_mode = false;
                self.send_or_run_widgets_cmd(host, SupervisorCmd::ToggleCalendar);
            }
            TrayAction::RefreshCalendarWidget => {
                self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshCalendar);
            }
            TrayAction::OpenWidgetConfig => {
                let res = ToolsPlatform::open_text_file(host, &self.paths.widgets_cfg);
                self.log_io_result(now, "widgets", "Трей: открыт widgets.cfg", res);
            }
            TrayAction::ToggleSysMonitorWidget => {
                self.safe_mode = false;
                self.send_or_run_widgets_cmd(host, SupervisorCmd::ToggleSysMonitor);
            }
            TrayAction::RefreshSysMonitorWidget => {
                self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshSysMonitor);
            }
            TrayAction::OpenSysMonitorConfig => {
                let res = ToolsPlatform::open_text_file(host, &self.paths.sysmon_cfg);
                self.log_io_result(now, "widgets", "Трей: открыт sysmon.cfg", res);
            }
            TrayAction::ToggleWeatherWidget => {
                self.safe_mode = false;
                self.send_or_run_widgets_cmd(host, SupervisorCmd::ToggleWeather);
            }
            TrayAction::RefreshWeatherWidget => {
                self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshWeather);
            }
            TrayAction::ToggleNotesWidget => {
                self.safe_mode = false;
                self.toggle_slint_notes(host);
            }
            TrayAction::ToggleOverlayModule => {
                self.modules_cfg.overlay_enabled = !self.modules_cfg.overlay_enabled;
                if !self.modules_cfg.overlay_enabled {
                    self.overlay_setup_mode = false;
                }
                self.persist_modules_cfg();
                self.sync_runtime_modules(host);
                self.render_settings_ui(host);
            }
            TrayAction::ToggleOverlaySetupMode => {
                let next = !self.overlay_setup_mode;
                self.set_overlay_setup_mode(host, next);
                self.render_settings_ui(host);
            }
            TrayAction::OpenOverlayTab => self.open_settings_tab(host, SettingsTab::Overlay),
            TrayAction::OpenOverlayConfig => {
                let res = ToolsPlatform::open_text_file(host, &self.paths.overlay_cfg);
                self.log_io_result(now, "overlay", "Трей: открыт overlay.cfg", res);
            }
            TrayAction::BackupSettingsNow => {
                match self.backup.run_backup(host, BackupMode::SettingsOnly, now) {
                    Ok(m) => self.debug.log(now, LogLevel::Info, "backup", m),
                    Err(e) => self.debug.log(now, LogLevel::Error, "backup", e),
                }
                self.render_settings_ui(host);
            }
            TrayAction::BackupFullNow => {
                match self
                    .backup
                    .run_backup(host, BackupMode::FullAppAndSettings, now)
                {
                    Ok(m) => self.debug.log(now, LogLevel::Info, "backup", m),
                    Err(e) => self.debug.log(now, LogLevel::Error, "backup", e),
                }
                self.render_settings_ui(host);
            }
            TrayAction::OpenBackupTab => self.open_settings_tab(host, SettingsTab::General),
            TrayAction::OpenDebugWindow => {
                self.open_browser_tab(host, BrowserTab::Diagnostics);
            }
            TrayAction::OpenAboutWindow => {
                self.open_browser_tab(host, BrowserTab::VersionHistory);
            }
            TrayAction::Exit => {
                self.slint_ui.stop_all();
                self.supervisor.stop_all();
                if let Some(docks) = &self.docks {
                    docks.persist();
                }
                host.request_quit();
            }
        }
    }

    pub(crate) fn on_debug_cmd<H: WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        cmd: DebugUiCommand,
    ) {
        let now = host.local_time_now();
        match cmd {
            DebugUiCommand::ToggleVerbose => self.debug.toggle_verbose(now),
            DebugUiCommand::ToggleFileLog => self.debug.toggle_file_logging(now),
            DebugUiCommand::CycleFilter => self.debug.cycle_filter(),
            DebugUiCommand::ClearLogs => self.debug.clear(now),
            DebugUiCommand::CopyReport => {
                let proc = host.query_process_metrics();
                let rep = self
                    .debug
                    .build_report(APP_VERSION, &self.paths.data_dir, proc);
                let res = host.copy_to_clipboard(&rep);
                self.log_io_result(now, "debug", "Отчёт скопирован в буфер обмена", res);
            }
            DebugUiCommand::SaveReport => {
                let proc = host.query_process_metrics();
                match self
                    .debug
                    .save_report(APP_VERSION, &self.paths.data_dir, proc)
                {
                    Ok(path) => {
                        self.debug.log(
                            now,
                            LogLevel::Info,
                            "debug",
                            format!("Отчёт сохранён: {}", path.display()),
                        );
                        let _ = ToolsPlatform::open_text_file(host, &path);
                    }
                    Err(e) => self.debug.log(now, LogLevel::Error, "debug", e),
                }
            }
            DebugUiCommand::WindowClosed => {
                self.debug
                    .log(now, LogLevel::Trace, "debug", "Окно диагностики закрыто");
            }
        }
        if host.is_debug_window_open() {
            self.refresh_debug_ui(host);
        }
    }

    pub(crate) fn on_about_cmd<H: WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        cmd: AboutUiCommand,
    ) {
        let now = host.local_time_now();
        match cmd {
            AboutUiCommand::OpenDataFolder => {
                let _ = std::fs::create_dir_all(&self.paths.data_dir);
                let res = host.open_folder(&self.paths.data_dir);
                self.log_io_result(now, "about", "Открыта папка data", res);
            }
            AboutUiCommand::CopyPaths => {
                let txt = build_paths_clipboard(&self.paths);
                let res = host.copy_to_clipboard(&txt);
                self.log_io_result(now, "about", "Пути скопированы в буфер", res);
            }
            AboutUiCommand::WindowClosed => {}
        }
    }

    pub(crate) fn refresh_debug_ui<H: ToolsPlatform>(&mut self, host: &mut H) {
        let proc = host.query_process_metrics();
        let (summary, logs) = self.debug.render_live(proc);
        host.update_debug_window(
            &summary,
            &logs,
            self.debug.verbose_trace(),
            self.debug.file_logging(),
            self.debug.filter_label(),
        );
    }

    pub(crate) fn log_io_result(
        &mut self,
        now: asrbox_platform_api::LocalDateTime,
        module: &'static str,
        ok_msg: &str,
        res: Result<(), String>,
    ) {
        match res {
            Ok(()) => self.debug.log(now, LogLevel::Info, module, ok_msg),
            Err(e) => self.debug.log(now, LogLevel::Error, module, e),
        }
    }
}

pub(crate) fn install_crash_log_hook(logs_dir: &std::path::Path) {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    let dir = logs_dir.to_path_buf();
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = fs::create_dir_all(&dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("crash.log"))
        {
            let loc = info.location().map_or_else(
                || "unknown".to_owned(),
                |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
            );
            let msg = info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("<non-string panic>");
            let _ = writeln!(
                file,
                "[v{APP_VERSION}] pid={} panic at {loc}: {msg}",
                std::process::id()
            );
        }
        prev(info);
    }));
}
