#![cfg(windows)]

use asrbox_core::{IsolationMode, ModuleId};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_settings::{SettingsAction, SettingsTab, WidgetSubTab};
use asrbox_mod_widgets::{load_notes_config, save_notes_config, YANDEX_WEATHER_CRED_TARGET};
use asrbox_platform_api::{
    AccentPreset, AppLanguage, DesignTheme, DockPlatform, FontRenderer, IconProvider, ThemeMode,
    ToolsPlatform, WidgetPlatform,
};
use asrbox_ui_slint::{decode_ui_cmd, unescape_field, UiModuleStatus, UiRuntimeStatus};

use crate::slint_ui::UiProcKind;
use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn open_slint_settings<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        tab: SettingsTab,
        sub: WidgetSubTab,
    ) {
        let tab_idx = match tab {
            SettingsTab::General => 0,
            SettingsTab::Dock => 1,
            SettingsTab::Widgets => 2,
            SettingsTab::Overlay => 3,
        };
        let sub_idx = match sub {
            WidgetSubTab::Calendar => 0,
            WidgetSubTab::SysMonitor => 1,
            WidgetSubTab::Weather => 2,
            WidgetSubTab::Notes => 3,
        };
        let status = self.build_ui_runtime_status(host);
        if let Err(err) = self.slint_ui.open_settings(tab_idx, sub_idx, &status) {
            self.debug
                .log(host.local_time_now(), LogLevel::Error, "settings", err);
        }
    }

    pub(crate) fn notify_slint_refresh<H: ToolsPlatform>(
        &mut self,
        host: &mut H,
        refresh_notes: bool,
    ) {
        if self.slint_ui.is_settings_open() {
            let status = self.build_ui_runtime_status(host);
            self.slint_ui.send_settings_status(&status);
        }
        if refresh_notes {
            self.slint_ui.notify_notes_refresh();
        }
    }

    pub(crate) fn toggle_slint_notes<H: ToolsPlatform + WidgetPlatform>(&mut self, host: &mut H) {
        let mut cfg = load_notes_config(&self.paths.notes_cfg).unwrap_or_default();
        let next = !self.slint_ui.is_notes_open();
        cfg.visible = next;
        let _ = save_notes_config(&self.paths.notes_cfg, &cfg);
        if let Err(err) = self.slint_ui.set_notes_visible(next) {
            self.debug
                .log(host.local_time_now(), LogLevel::Error, "widgets", err);
        }
        self.notify_slint_refresh(host, false);
    }

    pub(crate) fn build_ui_runtime_status<H: ToolsPlatform>(
        &self,
        host: &mut H,
    ) -> UiRuntimeStatus {
        let proc = host.query_process_metrics();
        let ram_mb = proc.ram_kb as f64 / 1024.0;
        let cpu_w = proc.cpu_tenths_pct / 10;
        let cpu_f = proc.cpu_tenths_pct % 10;
        let diag_summary = format!(
            "RAM {ram_mb:.1} МБ · CPU {cpu_w}.{cpu_f}% · GDI {} · USER {} · лог {}",
            proc.gdi_handles,
            proc.user_handles,
            self.debug.entries().len()
        );
        let has_ya_key = host
            .cred_load(YANDEX_WEATHER_CRED_TARGET)
            .ok()
            .flatten()
            .is_some_and(|(_, s)| !s.trim().is_empty());
        let modules = self
            .supervisor
            .status_views(&self.modules_cfg)
            .into_iter()
            .map(|m| {
                let status_text = if m.running {
                    m.pid
                        .map_or_else(|| m.note.clone(), |p| format!("PID {p} · работает"))
                } else {
                    m.note.clone()
                };
                UiModuleStatus {
                    id: m.id.as_str().to_owned(),
                    enabled: m.enabled,
                    running: m.running,
                    status_text,
                }
            })
            .collect();
        let bkp_view = self.backup.backup_view(self.appearance.language);
        UiRuntimeStatus {
            autostart: host.is_autostart_enabled(),
            has_zip_pw: self.backup.has_zip_password,
            has_gh_tok: self.backup.has_github_token,
            has_ya_key,
            overlay_setup: self.overlay_setup_mode,
            diag_verbose: self.debug.verbose_trace(),
            diag_file_log: self.debug.file_logging(),
            backup_busy: bkp_view.busy,
            diag_summary,
            status_banner: self.last_ui_status_msg.clone(),
            modules,
        }
    }

    pub(crate) fn poll_slint_ui<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
    ) {
        let events = self.slint_ui.poll_events();
        if events.is_empty() {
            return;
        }
        let mut need_settings_sync = false;
        let mut need_notes_sync = false;
        for (kind, line) in events {
            let Some((act, arg)) = decode_ui_cmd(&line) else {
                continue;
            };
            let (s_dirty, n_dirty) = self.dispatch_slint_cmd(host, kind, &act, &arg);
            need_settings_sync |= s_dirty;
            need_notes_sync |= n_dirty;
        }
        if need_settings_sync || need_notes_sync {
            self.notify_slint_refresh(host, need_notes_sync);
        }
    }

    fn dispatch_slint_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        _kind: UiProcKind,
        act: &str,
        arg: &str,
    ) -> (bool, bool) {
        match act {
            "settings_closed" => {
                if self.overlay_setup_mode {
                    self.set_overlay_setup_mode(host, false);
                }
                return (false, false);
            }
            "notes_closed" | "notes_changed" => return (true, false),
            "open_notes_settings" => {
                self.open_slint_settings(host, SettingsTab::Widgets, WidgetSubTab::Notes);
                return (false, false);
            }
            "tray_action" | "tray_toggle_theme" => {
                return self.dispatch_tray_cmd(host, act, arg);
            }
            _ => {}
        }
        if let Some(res) = self.dispatch_appearance_and_sys_cmd(host, act, arg) {
            return res;
        }
        if let Some(res) = self.dispatch_tools_cmd(host, act, arg) {
            return res;
        }
        if let Some(res) = self.dispatch_backup_and_sync_cmd(host, act, arg) {
            return res;
        }
        self.dispatch_tabs_cmd(host, act, arg)
    }

    fn dispatch_appearance_and_sys_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> Option<(bool, bool)> {
        let mut notes_dirty = false;
        let action = match act {
            "set_dark_mode" => {
                notes_dirty = true;
                SettingsAction::SetThemeMode(if arg == "1" {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                })
            }
            "set_accent" => {
                notes_dirty = true;
                let idx = arg.parse::<usize>().unwrap_or(0);
                let preset = AccentPreset::ALL.get(idx).copied().unwrap_or_default();
                SettingsAction::SetAccentPreset(preset)
            }
            "set_design_theme" => {
                notes_dirty = true;
                SettingsAction::SetDesignTheme(if arg == "1" {
                    DesignTheme::Win11Classic
                } else {
                    DesignTheme::AsrGlass
                })
            }
            "set_language" => {
                notes_dirty = true;
                SettingsAction::SetLanguage(if arg == "1" {
                    AppLanguage::En
                } else {
                    AppLanguage::Ru
                })
            }
            "set_font_renderer" => SettingsAction::SetFontRenderer(if arg == "1" {
                FontRenderer::DirectWrite
            } else {
                FontRenderer::GdiClearType
            }),
            "set_autostart" => SettingsAction::SetAutostart(arg == "1"),
            "set_isolation" => SettingsAction::SetIsolation(if arg == "1" {
                IsolationMode::InProc
            } else {
                IsolationMode::Process
            }),
            "set_module_enabled" => {
                let (id_s, on_s) = arg.split_once('|')?;
                SettingsAction::SetModuleEnabled {
                    id: ModuleId::from_str_id(id_s)?,
                    enabled: on_s == "1",
                }
            }
            "restart_module" => SettingsAction::RestartModule(ModuleId::from_str_id(arg)?),
            "restart_all" => SettingsAction::RestartAllModules,
            _ => return None,
        };
        self.apply_settings_action(host, action);
        Some((true, notes_dirty))
    }

    fn dispatch_tools_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> Option<(bool, bool)> {
        let mut notes_dirty = false;
        let action = match act {
            "reset_all" => {
                notes_dirty = true;
                self.last_ui_status_msg = "Настройки сброшены".to_owned();
                SettingsAction::ConfirmResetAll
            }
            "set_diag_verbose" => SettingsAction::SetDebugVerbose(arg == "1"),
            "set_diag_file_log" => SettingsAction::SetDebugFileLog(arg == "1"),
            "copy_diag_report" => {
                self.last_ui_status_msg = "Диагностический отчёт скопирован".to_owned();
                SettingsAction::CopyDebugReport
            }
            "save_diag_report" => {
                self.last_ui_status_msg = "Отчёт диагностики сохранён".to_owned();
                SettingsAction::SaveDebugReport
            }
            "open_data_folder" => SettingsAction::OpenDataFolder,
            "copy_paths" => {
                self.last_ui_status_msg = "Пути скопированы в буфер обмена".to_owned();
                SettingsAction::CopyPathsToClipboard
            }
            _ => return None,
        };
        self.apply_settings_action(host, action);
        Some((true, notes_dirty))
    }

    fn dispatch_backup_and_sync_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> Option<(bool, bool)> {
        if let Some(res) = self.dispatch_direct_secret_cmd(host, act, arg) {
            self.record_ui_outcome(host.local_time_now(), "backup", res);
            return Some((true, false));
        }
        let action = match act {
            "backup_full" => SettingsAction::BackupFullNow,
            "backup_settings" => SettingsAction::BackupSettingsNow,
            "upload_latest" => SettingsAction::UploadLatestBackupNow,
            "open_backups_folder" => SettingsAction::OpenBackupsFolder,
            "set_auto_upload" => SettingsAction::SetAutoUploadBackup(arg == "1"),
            "set_encrypt_zip" => SettingsAction::SetBackupEncryptZip(arg == "1"),
            "clear_zip_pw" => SettingsAction::ClearBackupZipPassword,
            "test_webdav" => SettingsAction::TestWebDavServer(arg.to_owned()),
            "delete_webdav" => SettingsAction::DeleteWebDavServer(arg.to_owned()),
            "toggle_backup_target" => SettingsAction::ToggleBackupTargetServer(arg.to_owned()),
            "toggle_sync_target" => SettingsAction::ToggleSyncServer(arg.to_owned()),
            "set_gh_include" => SettingsAction::SetGitHubIncludeInBackup(arg == "1"),
            "clear_gh_token" => SettingsAction::ClearGitHubToken,
            "download_gh_now" => SettingsAction::DownloadGitHubSourceNow,
            "set_sync_interval" => SettingsAction::SetSyncIntervalMin(decode_sync_mins(arg)),
            "pick_sync_files" => SettingsAction::PickSyncFiles,
            "pick_sync_folder" => SettingsAction::PickSyncFolder,
            "remove_sync_item" => SettingsAction::RemoveSyncItem(arg.to_owned()),
            "sync_files_now" => SettingsAction::SyncFilesNow,
            _ => return None,
        };
        self.apply_settings_action(host, action);
        Some((true, false))
    }

    fn dispatch_direct_secret_cmd<H: ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> Option<Result<String, String>> {
        match act {
            "save_zip_pw" => Some(self.backup.save_zip_password_value(host, arg)),
            "save_webdav_server" => {
                let p: Vec<&str> = arg.splitn(5, '|').collect();
                if p.len() == 5 {
                    let prov = p[0].parse::<i32>().unwrap_or(0);
                    Some(self.backup.add_server_direct(
                        host,
                        prov,
                        &unescape_field(p[1]),
                        &unescape_field(p[2]),
                        &unescape_field(p[3]),
                        &unescape_field(p[4]),
                    ))
                } else {
                    Some(Err("Некорректные данные WebDAV".to_owned()))
                }
            }
            "save_gh_settings" => {
                let p: Vec<&str> = arg.splitn(3, '|').collect();
                if p.len() == 3 {
                    Some(self.backup.save_github_settings_values(
                        host,
                        &unescape_field(p[0]),
                        &unescape_field(p[1]),
                        &unescape_field(p[2]),
                    ))
                } else {
                    Some(Err("Некорректные данные GitHub".to_owned()))
                }
            }
            _ => None,
        }
    }

    pub(crate) fn record_ui_outcome(
        &mut self,
        now: asrbox_platform_api::LocalDateTime,
        tag: &'static str,
        res: Result<String, String>,
    ) {
        match res {
            Ok(msg) => {
                self.last_ui_status_msg = msg.clone();
                self.debug.log(now, LogLevel::Info, tag, msg);
            }
            Err(err) => {
                self.last_ui_status_msg = format!("Ошибка: {err}");
                self.debug.log(now, LogLevel::Error, tag, err);
            }
        }
    }
}

fn decode_sync_mins(arg: &str) -> u32 {
    match arg.parse::<i32>().unwrap_or(0) {
        1 => 15,
        2 => 30,
        3 => 60,
        4 => 360,
        5 => 1440,
        _ => 0,
    }
}
