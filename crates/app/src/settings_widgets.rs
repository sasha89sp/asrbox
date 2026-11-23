#![cfg(windows)]

use asrbox_core::{ModuleId, SupervisorCmd};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_settings::SettingsAction;
use asrbox_mod_widgets::{
    load_sysmon_config, load_widget_config, rows_for_height, save_sysmon_config,
    save_widget_config, uncomment_calendar_sources, widget_height, CalendarSource, PeriodFilter,
    MAX_REFRESH_MS, MAX_SYSMON_HEIGHT, MAX_SYSMON_WIDTH, MIN_REFRESH_MS, MIN_SYSMON_HEIGHT,
    MIN_SYSMON_WIDTH,
};
use asrbox_platform_api::{DockPlatform, IconProvider, ToolsPlatform, WidgetPlatform};

use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn apply_widget_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) {
        if self.apply_sysmon_setting(host, action) {
            return;
        }
        self.apply_calendar_setting(host, action);
    }

    fn apply_sysmon_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        if matches!(action, SettingsAction::RefreshSysMonitorNow) {
            self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshSysMonitor);
            return true;
        }
        if matches!(action, SettingsAction::OpenSysMonitorConfig) {
            let _ = ToolsPlatform::open_text_file(host, &self.paths.sysmon_cfg);
            return true;
        }
        let mut cfg = load_sysmon_config(&self.paths.sysmon_cfg).unwrap_or_default();
        let changed = match action {
            SettingsAction::SetSysMonitorVisible(v) => {
                cfg.visible = *v;
                true
            }
            SettingsAction::SetSysMonitorOpacity(o) => {
                cfg.opacity_percent = (*o).clamp(20, 100);
                true
            }
            SettingsAction::SetSysMonitorLayer(l) => {
                cfg.layer = *l;
                true
            }
            SettingsAction::SetSysMonitorRefreshMs(ms) => {
                cfg.refresh_interval_ms = (*ms).clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
                true
            }
            SettingsAction::SetSysMonitorWidth(w) => {
                cfg.width = (*w).clamp(MIN_SYSMON_WIDTH, MAX_SYSMON_WIDTH);
                true
            }
            SettingsAction::SetSysMonitorHeight(h) => {
                cfg.height = (*h).clamp(MIN_SYSMON_HEIGHT, MAX_SYSMON_HEIGHT);
                true
            }
            SettingsAction::MatchSysMonitorToCalendar => {
                let cal = load_widget_config(&self.paths.widgets_cfg).unwrap_or_default();
                cfg.width = cal.width.clamp(MIN_SYSMON_WIDTH, MAX_SYSMON_WIDTH);
                cfg.height =
                    widget_height(cal.visible_rows).clamp(MIN_SYSMON_HEIGHT, MAX_SYSMON_HEIGHT);
                true
            }
            _ => return false,
        };
        if changed && save_sysmon_config(&self.paths.sysmon_cfg, &cfg).is_ok() {
            self.reload_module_config(host, ModuleId::Widgets);
        }
        true
    }

    fn apply_calendar_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) {
        let now = host.local_time_now();
        if matches!(action, SettingsAction::UncommentCalendarSources) {
            match uncomment_calendar_sources(&self.paths.widgets_cfg) {
                Ok(n) => {
                    self.debug.log(
                        now,
                        LogLevel::Info,
                        "widgets",
                        format!("Автоматически включено календарей (убран #): {n}"),
                    );
                    self.reload_module_config(host, ModuleId::Widgets);
                }
                Err(e) => self.debug.log(now, LogLevel::Error, "widgets", e),
            }
            return;
        }
        if matches!(action, SettingsAction::RefreshCalendarNow) {
            self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshCalendar);
            return;
        }
        let mut cfg = load_widget_config(&self.paths.widgets_cfg).unwrap_or_default();
        let changed = match action {
            SettingsAction::SetCalendarVisible(v) => {
                cfg.visible = *v;
                true
            }
            SettingsAction::SetCalendarOpacity(o) => {
                cfg.opacity_percent = (*o).clamp(20, 100);
                true
            }
            SettingsAction::SetCalendarLayer(l) => {
                cfg.layer = *l;
                true
            }
            SettingsAction::SetCalendarPeriod(p) => {
                cfg.period = match p {
                    0 => PeriodFilter::Today,
                    1 => PeriodFilter::Days7,
                    3 => PeriodFilter::All,
                    _ => PeriodFilter::Days30,
                };
                true
            }
            SettingsAction::SetCalendarSyncInterval(sec) => {
                cfg.refresh_interval_sec = (*sec).clamp(300, 86_400);
                true
            }
            SettingsAction::SetCalendarWidth(w) => {
                cfg.width = (*w).clamp(280, 860);
                true
            }
            SettingsAction::SetCalendarRows(r) => {
                cfg.visible_rows = (*r).clamp(3, 16);
                true
            }
            SettingsAction::MatchCalendarToSysMonitor => {
                let sys = load_sysmon_config(&self.paths.sysmon_cfg).unwrap_or_default();
                cfg.width = sys.width.clamp(280, 860);
                cfg.visible_rows = rows_for_height(sys.height);
                true
            }
            SettingsAction::AddCalendarUrlFromInput => {
                let raw = host.settings_input_text(0);
                let url = raw.trim().trim_matches('"').trim_matches('\'');
                if url.starts_with("https://") || url.starts_with("http://") {
                    let id = format!("cal_{}", cfg.sources.len() + 1);
                    cfg.sources.push(CalendarSource {
                        id,
                        url: url.to_owned(),
                        color_rgb: None,
                    });
                    cfg.commented_calendar_hint = false;
                    host.set_settings_input_text(0, "");
                    true
                } else {
                    self.debug.log(
                        now,
                        LogLevel::Warn,
                        "widgets",
                        "Введите корректный URL (https://.../basic.ics)",
                    );
                    false
                }
            }
            SettingsAction::RemoveCalendarSource(idx) if *idx < cfg.sources.len() => {
                cfg.sources.remove(*idx);
                true
            }
            _ => false,
        };
        if changed && save_widget_config(&self.paths.widgets_cfg, &cfg).is_ok() {
            self.reload_module_config(host, ModuleId::Widgets);
        }
    }

    pub(crate) fn apply_backup_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        use asrbox_mod_backup::BackupMode;
        let now = host.local_time_now();
        let outcome = match action {
            SettingsAction::BackupSettingsNow => {
                Some(self.backup.run_backup(host, BackupMode::SettingsOnly, now))
            }
            SettingsAction::BackupFullNow => Some(self.backup.run_backup(
                host,
                BackupMode::FullAppAndSettings,
                now,
            )),
            SettingsAction::UploadLatestBackupNow => Some(self.backup.upload_latest(host)),
            SettingsAction::OpenBackupsFolder => {
                let _ = std::fs::create_dir_all(&self.paths.backups_dir);
                let res = host.open_folder(&self.paths.backups_dir);
                self.log_io_result(now, "backup", "Открыта папка бэкапов", res);
                return true;
            }
            SettingsAction::SetAutoUploadBackup(on) => {
                self.backup.set_auto_upload(*on);
                return true;
            }
            SettingsAction::SetWebDavProvider(idx) => {
                self.backup.set_provider(*idx, host);
                self.last_settings_tab = None;
                return true;
            }
            SettingsAction::SaveWebDavSettings => Some(self.backup.save_webdav(host)),
            SettingsAction::TestWebDavConnection => Some(self.backup.test_webdav(host)),
            SettingsAction::DeleteWebDavPassword => Some(self.backup.delete_password(host)),
            _ => return false,
        };
        if let Some(res) = outcome {
            match res {
                Ok(msg) => self.debug.log(now, LogLevel::Info, "backup", msg),
                Err(err) => self.debug.log(now, LogLevel::Error, "backup", err),
            }
        }
        true
    }
}
