#![cfg(windows)]

use asrbox_core::{save_modules_config, IsolationMode, ModuleId};
use asrbox_mod_about::changelog_text;
use asrbox_mod_debug::LogLevel;
use asrbox_mod_dock::{load_docks_from_file, DockEdge};
use asrbox_mod_settings::{
    AboutSettingsView, AppearanceSettingsView, BrowserTab, CalendarSettingsView,
    CalendarSourceView, DockItemView, DockSettingsView, SettingsAction, SettingsSnapshot,
    SettingsTab, SysMonitorSettingsView,
};
use asrbox_mod_widgets::{load_sysmon_config, load_widget_config, PeriodFilter};
use asrbox_platform_api::{
    DockPlatform, IconProvider, SettingsUiEvent, ToolsPlatform, WidgetPlatform,
};

use crate::{AppController, APP_VERSION};

impl<I: IconProvider> AppController<I> {
    pub(crate) fn open_settings_tab<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        tab: SettingsTab,
    ) {
        self.settings_ui.active_tab = tab;
        self.settings_ui.active_browser_tab = BrowserTab::Main;
        self.settings_ui.scroll_y = 0;
        self.last_settings_tab = None;
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
        self.render_settings_ui(host);
    }

    pub(crate) fn render_settings_ui<H: ToolsPlatform>(&mut self, host: &mut H) {
        if !host.is_settings_window_open() {
            return;
        }
        self.sync_settings_tab_inputs(host);
        let snap = self.build_settings_snapshot(host);
        if let Some(txt) = self.settings_ui.take_dock_input_sync(&snap) {
            host.set_settings_input_text(0, &txt);
        }
        if let Some(txt) = self.settings_ui.take_aux_prefill() {
            host.set_settings_input_text(4, &txt);
        }
        let (frame, inputs) = self.settings_ui.render(&snap);
        host.present_settings_frame(&frame, inputs);
        if !self.settings_ui.is_animating() {
            host.set_settings_anim_timer(false);
        }
    }

    fn sync_settings_tab_inputs<H: ToolsPlatform>(&mut self, host: &H) {
        let cur = self.settings_ui.active_tab;
        if self.last_settings_tab == Some(cur) {
            return;
        }
        self.last_settings_tab = Some(cur);
        match cur {
            SettingsTab::Dock => self.settings_ui.reset_dock_input_sync(),
            SettingsTab::Calendar => host.set_settings_input_text(0, ""),
            _ => {}
        }
    }

    pub(crate) fn on_settings_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: SettingsUiEvent,
    ) {
        if matches!(ev, SettingsUiEvent::Closed) {
            self.last_settings_tab = None;
            host.set_settings_anim_timer(false);
            if self.overlay_setup_mode {
                self.set_overlay_setup_mode(host, false);
            }
        }
        let snap = self.build_settings_snapshot(host);
        let (redraw, action) = self.settings_ui.handle_event(ev, &snap);
        if let Some(act) = action {
            self.apply_settings_action(host, act);
            self.render_settings_ui(host);
        } else if redraw {
            self.render_settings_ui(host);
        }
    }

    pub(crate) fn build_settings_snapshot<H: ToolsPlatform>(
        &self,
        host: &mut H,
    ) -> SettingsSnapshot {
        let docks_cfg = load_docks_from_file(&self.paths.docks_cfg).unwrap_or_default();
        let docks = docks_cfg
            .iter()
            .map(|d| DockSettingsView {
                id: d.id.to_string(),
                edge_index: match d.edge {
                    DockEdge::Bottom => 0,
                    DockEdge::Top => 1,
                    DockEdge::Left => 2,
                    DockEdge::Right => 3,
                },
                layer: d.layer,
                icon_size_px: u32::from(d.icon_size_px),
                opacity_percent: d.opacity_percent,
                locked: d.locked,
                show_running_indicator: d.show_running_indicator,
                focus_running_window: d.focus_running_window,
                label_mode_index: d.label_mode.to_index(),
                item_count: d.items.len(),
                items: d
                    .items
                    .iter()
                    .filter(|i| !i.is_separator())
                    .map(|i| DockItemView {
                        id: i.id,
                        label: i.label.clone(),
                    })
                    .collect(),
            })
            .collect();

        let cal_cfg = load_widget_config(&self.paths.widgets_cfg).unwrap_or_default();
        let calendar = CalendarSettingsView {
            visible: cal_cfg.visible,
            opacity_percent: cal_cfg.opacity_percent,
            layer: cal_cfg.layer,
            period_index: match cal_cfg.period {
                PeriodFilter::Today => 0,
                PeriodFilter::Days7 => 1,
                PeriodFilter::Days30 => 2,
                PeriodFilter::All => 3,
            },
            refresh_interval_sec: cal_cfg.refresh_interval_sec,
            width: cal_cfg.width,
            visible_rows: cal_cfg.visible_rows,
            commented_calendar_hint: cal_cfg.commented_calendar_hint,
            sources: cal_cfg
                .sources
                .iter()
                .map(|s| CalendarSourceView {
                    id: s.id.clone(),
                    url: s.url.clone(),
                })
                .collect(),
        };

        let sys_cfg = load_sysmon_config(&self.paths.sysmon_cfg).unwrap_or_default();
        let sysmon = SysMonitorSettingsView {
            visible: sys_cfg.visible,
            opacity_percent: sys_cfg.opacity_percent,
            layer: sys_cfg.layer,
            refresh_interval_ms: sys_cfg.refresh_interval_ms,
            width: sys_cfg.width,
            height: sys_cfg.height,
        };

        let diagnostics = self.build_diagnostics_view(host);
        let changelog_lines = changelog_text()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(ToOwned::to_owned)
            .collect();

        SettingsSnapshot {
            appearance: AppearanceSettingsView {
                theme: self.appearance.theme,
                accent: self.appearance.accent,
                language: self.appearance.language,
            },
            isolation: self.modules_cfg.isolation,
            autostart_enabled: host.is_autostart_enabled(),
            modules: self.supervisor.status_views(&self.modules_cfg),
            docks,
            calendar,
            sysmon,
            overlay: self.build_overlay_settings_view(),
            backup: self.backup.backup_view(self.appearance.language),
            diagnostics,
            about: AboutSettingsView {
                version: APP_VERSION.to_owned(),
                exe_path: self.paths.exe_path.display().to_string(),
                data_dir: self.paths.data_dir.display().to_string(),
                config_dir: self.paths.data_dir.join("config").display().to_string(),
                backups_dir: self.paths.backups_dir.display().to_string(),
                logs_dir: self.paths.logs_dir.display().to_string(),
                changelog_lines,
            },
        }
    }

    fn apply_settings_action<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: SettingsAction,
    ) {
        let now = host.local_time_now();
        match action {
            SettingsAction::SetIsolation(mode) => {
                if self.modules_cfg.isolation != mode {
                    self.modules_cfg.isolation = mode;
                    self.persist_modules_cfg();
                    self.sync_runtime_modules(host);
                    self.debug.log(
                        now,
                        LogLevel::Info,
                        "app",
                        format!("Режим изоляции -> {}", mode.as_str()),
                    );
                }
            }
            SettingsAction::SetAutostart(enabled) => {
                let res = host.set_autostart_enabled(enabled);
                let msg = if enabled {
                    "Автозагрузка с Windows включена"
                } else {
                    "Автозагрузка с Windows отключена"
                };
                self.log_io_result(now, "app", msg, res);
            }
            SettingsAction::SetModuleEnabled { id, enabled } => {
                if id == ModuleId::Overlay && !enabled {
                    self.overlay_setup_mode = false;
                }
                self.modules_cfg.set_enabled(id, enabled);
                self.persist_modules_cfg();
                self.sync_runtime_modules(host);
                self.debug.log(
                    now,
                    LogLevel::Info,
                    "app",
                    format!(
                        "Модуль {} -> {}",
                        id.as_str(),
                        if enabled { "ВКЛ" } else { "ВЫКЛ" }
                    ),
                );
            }
            SettingsAction::RestartModule(id) => {
                if self.modules_cfg.isolation == IsolationMode::Process {
                    match self.supervisor.restart_module(id) {
                        Ok(pid) => self.debug.log(
                            now,
                            LogLevel::Info,
                            "app",
                            format!("Перезапущен {} (PID {pid})", id.as_str()),
                        ),
                        Err(e) => self.debug.log(now, LogLevel::Error, "app", e),
                    }
                } else {
                    self.reload_module_config(host, id);
                }
            }
            SettingsAction::OpenDocksConfig => {
                let _ = ToolsPlatform::open_text_file(host, &self.paths.docks_cfg);
            }
            SettingsAction::OpenWidgetsConfig => {
                let _ = ToolsPlatform::open_text_file(host, &self.paths.widgets_cfg);
            }
            SettingsAction::SetDebugVerbose(on) => {
                if self.debug.verbose_trace() != on {
                    self.debug.toggle_verbose(now);
                }
            }
            SettingsAction::SetDebugFileLog(on) => {
                if self.debug.file_logging() != on {
                    self.debug.toggle_file_logging(now);
                }
            }
            SettingsAction::OpenDataFolder => {
                let _ = std::fs::create_dir_all(&self.paths.data_dir);
                let _ = host.open_folder(&self.paths.data_dir);
            }
            other => {
                if !self.apply_general_setting(host, &other)
                    && !self.apply_dock_setting(host, &other)
                    && !self.apply_overlay_setting(host, &other)
                {
                    self.apply_widget_setting(host, &other);
                }
            }
        }
    }

    pub(crate) fn persist_modules_cfg(&self) {
        let path = self.paths.data_dir.join("config").join("modules.cfg");
        let _ = save_modules_config(&path, &self.modules_cfg);
    }
}
