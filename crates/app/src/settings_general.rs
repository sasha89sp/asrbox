#![cfg(windows)]

use asrbox_core::{IsolationMode, ModuleId};
use asrbox_mod_about::{build_about_text, build_paths_clipboard};
use asrbox_mod_backup::BackupMode;
use asrbox_mod_debug::LogLevel;
use asrbox_mod_dock::{save_docks_to_file, DockConfig};
use asrbox_mod_overlay::{save_overlay_config, OverlayConfig};
use asrbox_mod_settings::{BrowserTab, DiagnosticsLiveView, SettingsAction};
use asrbox_mod_widgets::{
    save_notes_config, save_sysmon_config, save_weather_config, save_widget_config,
    CalendarWidgetConfig, NotesWidgetConfig, SysMonitorConfig, WeatherWidgetConfig,
};
use asrbox_platform_api::{
    save_appearance_config, save_menu_config, AppearanceConfig, ContextMenuConfig, DockPlatform,
    IconProvider, ToolsPlatform, WidgetPlatform,
};

use crate::dock_shortcuts::{cleanup_unused_shortcuts, shortcuts_dir_for_config};
use crate::{AppController, APP_VERSION};

impl<I: IconProvider> AppController<I> {
    pub(crate) fn open_browser_tab<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        tab: BrowserTab,
    ) {
        self.settings_ui.open_browser_tab(tab);
        if !host.is_settings_window_open() {
            let title = format!("Настройки — asrbox v{APP_VERSION}");
            let w = self.settings_ui.width;
            let h = self.settings_ui.height;
            if let Err(err) = host.open_settings_window(&title, w, h) {
                self.debug
                    .log(host.local_time_now(), LogLevel::Error, "settings", err);
                return;
            }
            let pal = self.appearance.palette();
            host.apply_settings_theme(pal);
        }
        self.render_settings_ui(host);
    }

    pub(crate) fn apply_appearance_change<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        animate: bool,
    ) {
        let _ = save_appearance_config(&self.paths.appearance_cfg, &self.appearance);
        let pal = self.appearance.palette();
        if animate {
            if self.settings_ui.set_target_palette(pal) {
                host.set_settings_anim_timer(true);
            }
        } else {
            self.settings_ui.set_palette_immediate(pal);
            host.set_settings_anim_timer(false);
        }
        host.apply_settings_theme(pal);
        DockPlatform::set_theme(host, pal);
        WidgetPlatform::set_theme(host, pal);
        if let Some(docks) = self.docks.as_mut() {
            docks.set_palette(host, pal);
        }
        if let Some(cal) = self.calendar.as_mut() {
            cal.set_palette(host, pal);
        }
        if let Some(sys) = self.sysmon.as_mut() {
            sys.set_palette(host, pal);
        }
        if let Some(weather) = self.weather.as_mut() {
            weather.set_palette(host, pal);
        }
        if let Some(notes) = self.notes.as_mut() {
            notes.set_palette(host, pal);
        }
        if let Some(ov) = self.overlay.as_mut() {
            ov.set_palette(host, pal);
        }
        for id in ModuleId::ALL {
            self.reload_module_config(host, id);
        }
    }

    pub(crate) fn apply_general_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        let now = host.local_time_now();
        match action {
            SettingsAction::SetDesignTheme(dt) => {
                if self.appearance.design_theme != *dt {
                    self.appearance.design_theme = *dt;
                    self.apply_appearance_change(host, true);
                }
            }
            SettingsAction::SetThemeMode(mode) => {
                if self.appearance.theme != *mode {
                    self.appearance.theme = *mode;
                    self.apply_appearance_change(host, true);
                }
            }
            SettingsAction::SetAccentPreset(preset) => {
                if self.appearance.accent != *preset {
                    self.appearance.accent = *preset;
                    self.apply_appearance_change(host, true);
                }
            }
            SettingsAction::SetFontRenderer(renderer) => {
                if self.appearance.font_renderer != *renderer {
                    self.appearance.font_renderer = *renderer;
                    self.apply_appearance_change(host, false);
                }
            }
            SettingsAction::SetLanguage(lang) => {
                if self.appearance.language != *lang {
                    self.appearance.language = *lang;
                    self.apply_appearance_change(host, false);
                }
            }
            SettingsAction::SetTrayMenuVis(vis) => {
                self.menus_cfg.tray = *vis;
                let _ = save_menu_config(&self.paths.menus_cfg, &self.menus_cfg);
            }
            SettingsAction::SetDockMenuVis(vis) => {
                self.menus_cfg.dock = *vis;
                let _ = save_menu_config(&self.paths.menus_cfg, &self.menus_cfg);
                if let Some(docks) = self.docks.as_mut() {
                    docks.menu_vis = *vis;
                }
                self.reload_module_config(host, ModuleId::Dock);
            }
            SettingsAction::SetWidgetMenuVis(vis) => {
                self.menus_cfg.widget = *vis;
                let _ = save_menu_config(&self.paths.menus_cfg, &self.menus_cfg);
                self.reload_module_config(host, ModuleId::Widgets);
            }
            SettingsAction::ConfirmResetAll => self.reset_all_settings(host),
            SettingsAction::OpenDiagnosticsTab => {
                self.open_browser_tab(host, BrowserTab::Diagnostics);
            }
            SettingsAction::SetDiagnosticsMonitorEnabled(on) => {
                self.diagnostics_monitor_enabled = *on;
            }
            SettingsAction::CycleDebugFilter => self.debug.cycle_filter(),
            SettingsAction::ClearDebugLogs => self.debug.clear(now),
            SettingsAction::CopyDebugReport => {
                let proc = host.query_process_metrics();
                let rep = self
                    .debug
                    .build_report(APP_VERSION, &self.paths.data_dir, proc);
                let res = host.copy_to_clipboard(&rep);
                self.log_io_result(now, "debug", "Отчёт скопирован в буфер обмена", res);
            }
            SettingsAction::SaveDebugReport => {
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
            SettingsAction::OpenVersionHistoryTab => {
                self.open_browser_tab(host, BrowserTab::VersionHistory);
            }
            SettingsAction::CopyVersionInfo => {
                let txt = build_about_text(APP_VERSION, &self.paths);
                let res = host.copy_to_clipboard(&txt);
                self.log_io_result(now, "about", "Информация о версии скопирована", res);
            }
            SettingsAction::CopyPathsToClipboard => {
                let txt = build_paths_clipboard(&self.paths);
                let res = host.copy_to_clipboard(&txt);
                self.log_io_result(now, "about", "Пути программы скопированы в буфер", res);
            }
            other => return self.apply_backup_and_sync_action(host, other),
        }
        true
    }

    fn apply_backup_and_sync_action<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        let now = host.local_time_now();
        let outcome = match action {
            SettingsAction::SetBackupEncryptZip(on) => {
                self.backup.set_encrypt_zip(*on);
                return true;
            }
            SettingsAction::SaveBackupZipPassword => {
                Some(self.backup.save_zip_password_from_input(host))
            }
            SettingsAction::ClearBackupZipPassword => Some(self.backup.clear_zip_password(host)),
            SettingsAction::SetGitHubIncludeInBackup(on) => {
                self.backup.set_github_include_in_backup(*on);
                return true;
            }
            SettingsAction::SaveGitHubSettings => {
                Some(self.backup.save_github_settings_from_inputs(host))
            }
            SettingsAction::ClearGitHubToken => Some(self.backup.clear_github_token(host)),
            SettingsAction::DownloadGitHubSourceNow => {
                Some(self.backup.download_github_source_now(host, now))
            }
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
            SettingsAction::ToggleBackupTargetServer(id) => {
                self.backup.toggle_backup_target_server(id);
                return true;
            }
            SettingsAction::OpenAddWebDavMenu => {
                let pal = self.appearance.palette();
                let lang = self.appearance.language;
                if let Err(e) = self.backup.open_add_webdav_menu(host, pal, lang) {
                    self.debug.log(now, LogLevel::Error, "webdav", e);
                }
                return true;
            }
            SettingsAction::TestWebDavDraft => Some(self.backup.test_draft(host)),
            SettingsAction::SaveWebDavDraft => Some(self.backup.save_draft(host)),
            SettingsAction::CancelWebDavDraft => {
                self.backup.cancel_draft(host);
                return true;
            }
            SettingsAction::TestWebDavServer(id) => Some(self.backup.test_server(host, id)),
            SettingsAction::DeleteWebDavServer(id) => Some(self.backup.delete_server(host, id)),
            SettingsAction::SaveWebDavServerRemoteDir(id) => {
                Some(self.backup.save_server_remote_dir(host, id))
            }
            SettingsAction::OpenWebDavBrowser(id) => match self.backup.open_browser(host, id) {
                Ok((server_id, server_name)) => {
                    self.settings_ui
                        .open_browser_tab(BrowserTab::WebDavBrowser {
                            server_id,
                            server_name,
                        });
                    return true;
                }
                Err(e) => Some(Err(e)),
            },
            SettingsAction::WebDavBrowseDir { server_id, path } => {
                Some(self.backup.browse_dir(host, server_id, path))
            }
            SettingsAction::WebDavBrowseUp(id) => Some(self.backup.browse_up(host, id)),
            SettingsAction::WebDavCreateFolderFromInput(id) => {
                Some(self.backup.create_folder_from_input(host, id))
            }
            SettingsAction::WebDavSelectCurrentFolder(id) => {
                Some(self.backup.select_current_folder(id))
            }
            SettingsAction::PickSyncFiles => Some(self.backup.pick_sync_files(host)),
            SettingsAction::PickSyncFolder => Some(self.backup.pick_sync_folder(host)),
            SettingsAction::RemoveSyncItem(id) => {
                self.backup.remove_sync_item(id);
                return true;
            }
            SettingsAction::ToggleSyncServer(id) => {
                self.backup.toggle_sync_server(id);
                return true;
            }
            SettingsAction::SetSyncIntervalMin(mins) => {
                self.backup.set_sync_interval_min(*mins);
                return true;
            }
            SettingsAction::SyncFilesNow => Some(self.backup.sync_files_now(host, now)),
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

    fn reset_all_settings<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
    ) {
        let now = host.local_time_now();
        self.settings_ui.confirming_reset = false;
        self.appearance = AppearanceConfig::default();
        self.menus_cfg = ContextMenuConfig::default();
        let _ = save_menu_config(&self.paths.menus_cfg, &self.menus_cfg);
        if let Some(docks) = self.docks.as_mut() {
            docks.menu_vis = self.menus_cfg.dock;
        }
        let def_docks = vec![DockConfig::new(1, "Док 1")];
        let _ = save_docks_to_file(&self.paths.docks_cfg, &def_docks, APP_VERSION);
        let sc_dir = shortcuts_dir_for_config(&self.paths.docks_cfg);
        cleanup_unused_shortcuts(&def_docks, &sc_dir);
        let _ = save_widget_config(&self.paths.widgets_cfg, &CalendarWidgetConfig::default());
        let _ = save_sysmon_config(&self.paths.sysmon_cfg, &SysMonitorConfig::default());
        let _ = save_weather_config(&self.paths.weather_cfg, &WeatherWidgetConfig::default());
        let _ = save_notes_config(&self.paths.notes_cfg, &NotesWidgetConfig::default());
        let _ = save_overlay_config(&self.paths.overlay_cfg, &OverlayConfig::default());
        if self.modules_cfg.isolation != IsolationMode::Process {
            self.modules_cfg.isolation = IsolationMode::Process;
            self.persist_modules_cfg();
            self.sync_runtime_modules(host);
        }
        self.apply_appearance_change(host, true);
        self.debug.log(
            now,
            LogLevel::Info,
            "settings",
            "Выполнен полный сброс настроек (бэкапы, заметки и пароли сохранены)",
        );
    }

    pub(crate) fn build_diagnostics_view<H: ToolsPlatform>(
        &self,
        host: &mut H,
    ) -> DiagnosticsLiveView {
        let proc = host.query_process_metrics();
        let ram_mb = proc.ram_kb as f64 / 1024.0;
        let cpu_whole = proc.cpu_tenths_pct / 10;
        let cpu_frac = proc.cpu_tenths_pct % 10;
        let metrics_lines = vec![
            format!(
                "RAM: {ram_mb:.1} МБ · CPU: {cpu_whole}.{cpu_frac}% · Uptime: {} с",
                proc.uptime_sec
            ),
            format!(
                "GDI: {} · USER: {} · Записей в буфере: {}",
                proc.gdi_handles,
                proc.user_handles,
                self.debug.entries().len()
            ),
        ];
        let module_lines = self
            .supervisor
            .status_views(&self.modules_cfg)
            .into_iter()
            .map(|m| {
                let pid_s = m.pid.map_or_else(|| "—".to_owned(), |p| p.to_string());
                let st = if m.running {
                    "работает"
                } else {
                    "остановлен"
                };
                format!(
                    "{}: {} · PID {} · перезапусков: {} · {}",
                    m.id.as_str(),
                    st,
                    pid_s,
                    m.restarts,
                    m.note
                )
            })
            .collect();
        let filt = self.debug.filter_module();
        let log_lines: Vec<String> = self
            .debug
            .entries()
            .iter()
            .rev()
            .filter(|e| filt.is_none_or(|f| e.module == f))
            .take(80)
            .map(|e| {
                format!(
                    "[{}] [{}] [{}] {}",
                    e.timestamp,
                    e.level.tag(),
                    e.module,
                    e.message
                )
            })
            .collect();
        DiagnosticsLiveView {
            monitor_enabled: self.diagnostics_monitor_enabled,
            verbose: self.debug.verbose_trace(),
            file_log: self.debug.file_logging(),
            filter_label: self.debug.filter_label().to_owned(),
            summary_line: format!(
                "RAM {ram_mb:.1} МБ · CPU {cpu_whole}.{cpu_frac}% · фильтр: {}",
                self.debug.filter_label()
            ),
            metrics_lines,
            module_lines,
            log_lines,
        }
    }
}
