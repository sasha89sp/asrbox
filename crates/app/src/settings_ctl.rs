#![cfg(windows)]

use asrbox_core::{save_modules_config, IsolationMode, ModuleId};
use asrbox_mod_about::changelog_text;
use asrbox_mod_debug::LogLevel;
use asrbox_mod_dock::{load_docks_from_file, DockEdge};
use asrbox_mod_settings::{
    AboutSettingsView, AppearanceSettingsView, BrowserTab, CalendarSettingsView,
    CalendarSourceView, DiagnosticsLiveView, DockItemView, DockSettingsView, NotesSettingsView,
    SettingsAction, SettingsSnapshot, SettingsTab, SysMonitorSettingsView, WeatherSettingsView,
    WidgetSubTab,
};
use asrbox_mod_widgets::{
    load_notes_config, load_sysmon_config, load_weather_config, load_widget_config, PeriodFilter,
    WeatherProvider, YANDEX_WEATHER_CRED_TARGET,
};
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
        self.last_widget_subtab = None;
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

    pub(crate) fn open_widgets_subtab<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        sub: WidgetSubTab,
    ) {
        self.settings_ui.selected_widget = sub;
        self.open_settings_tab(host, SettingsTab::Widgets);
    }

    pub(crate) fn render_settings_ui<H: ToolsPlatform>(&mut self, host: &mut H) {
        if !host.is_settings_window_open() {
            return;
        }
        let snap = self.build_settings_snapshot(host);
        self.cached_settings_snap = Some(snap);
        self.present_cached_settings_ui(host);
    }

    fn present_cached_settings_ui<H: ToolsPlatform>(&mut self, host: &mut H) {
        if !host.is_settings_window_open() {
            return;
        }
        self.sync_settings_tab_inputs(host);
        let Some(snap) = self.cached_settings_snap.as_ref() else {
            return;
        };
        if let Some(txt) = self.settings_ui.take_dock_input_sync(snap) {
            host.set_settings_input_text(0, &txt);
        }
        if let Some(txt) = self.settings_ui.take_aux_prefill() {
            host.set_settings_input_text(4, &txt);
        }
        let (frame, inputs) = self.settings_ui.render(snap);
        host.present_settings_frame(&frame, inputs);
        if !self.settings_ui.is_animating() {
            host.set_settings_anim_timer(false);
        }
    }

    pub(crate) fn on_settings_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: SettingsUiEvent,
    ) {
        if matches!(ev, SettingsUiEvent::Closed) {
            self.last_settings_tab = None;
            self.last_widget_subtab = None;
            self.cached_settings_snap = None;
            host.set_settings_anim_timer(false);
            if self.overlay_setup_mode {
                self.set_overlay_setup_mode(host, false);
            }
            self.settings_ui.on_closed();
            return;
        }
        if self.cached_settings_snap.is_none() {
            self.cached_settings_snap = Some(self.build_settings_snapshot(host));
        }
        let prev_browser = self.settings_ui.active_browser_tab.clone();
        let (redraw, action) = match self.cached_settings_snap.as_ref() {
            Some(snap) => self.settings_ui.handle_event(ev, snap),
            None => (false, None),
        };
        if let Some(act) = action {
            self.apply_settings_action(host, act);
            self.render_settings_ui(host);
        } else if self.settings_ui.active_browser_tab != prev_browser {
            self.render_settings_ui(host);
        } else if redraw {
            self.present_cached_settings_ui(host);
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
                edge_margin_px: d.edge_margin_px,
                corner_radius_px: d.corner_radius_px,
                blur_enabled: d.blur_enabled,
                locked: d.locked,
                lock_items: d.lock_items,
                show_running_indicator: d.show_running_indicator,
                focus_running_window: d.focus_running_window,
                launch_animation_enabled: d.launch_animation_enabled,
                label_mode_index: d.label_mode.to_index(),
                item_count: d.items.len(),
                items: d
                    .items
                    .iter()
                    .map(|i| DockItemView {
                        id: i.id,
                        label: i.label.clone(),
                        target_path: i.target.display().to_string(),
                        is_folder: !i.is_separator() && i.target.is_dir(),
                        is_separator: i.is_separator(),
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

        let w_cfg = load_weather_config(&self.paths.weather_cfg).unwrap_or_default();
        let has_yandex_key = host
            .cred_load(YANDEX_WEATHER_CRED_TARGET)
            .ok()
            .flatten()
            .is_some_and(|(_, secret)| !secret.trim().is_empty());
        let weather = WeatherSettingsView {
            visible: w_cfg.visible,
            city: w_cfg.city,
            provider_is_yandex: w_cfg.provider == WeatherProvider::Yandex,
            forecast_days: w_cfg.forecast_days,
            has_yandex_key,
            opacity_percent: w_cfg.opacity_percent,
            layer: w_cfg.layer,
            refresh_interval_min: w_cfg.refresh_interval_min,
            width: w_cfg.width,
            height: w_cfg.height,
        };
        let n_cfg = load_notes_config(&self.paths.notes_cfg).unwrap_or_default();
        let notes = NotesSettingsView {
            visible: n_cfg.visible,
            opacity_percent: n_cfg.opacity_percent,
            layer: n_cfg.layer,
            show_folders_pane: n_cfg.show_folders_pane,
            show_list_pane: n_cfg.show_list_pane,
            show_editor_pane: n_cfg.show_editor_pane,
        };

        let diagnostics = if self.settings_ui.active_browser_tab == BrowserTab::Diagnostics {
            self.build_diagnostics_view(host)
        } else {
            DiagnosticsLiveView {
                monitor_enabled: self.diagnostics_monitor_enabled,
                verbose: self.debug.verbose_trace(),
                file_log: self.debug.file_logging(),
                filter_label: self.debug.filter_label().to_owned(),
                ..DiagnosticsLiveView::default()
            }
        };
        let changelog_lines = changelog_text()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(ToOwned::to_owned)
            .collect();

        SettingsSnapshot {
            appearance: AppearanceSettingsView {
                design_theme: self.appearance.design_theme,
                theme: self.appearance.theme,
                accent: self.appearance.accent,
                language: self.appearance.language,
                font_renderer: self.appearance.font_renderer,
            },
            isolation: self.modules_cfg.isolation,
            autostart_enabled: host.is_autostart_enabled(),
            modules: self.supervisor.status_views(&self.modules_cfg),
            docks,
            calendar,
            sysmon,
            weather,
            notes,
            overlay: self.build_overlay_settings_view(),
            backup: self.backup.backup_view(self.appearance.language),
            github: self.backup.github_view(),
            menus: self.menus_cfg,
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

    pub(crate) fn apply_settings_action<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: SettingsAction,
    ) {
        let now = host.local_time_now();
        match action {
            SettingsAction::OpenCombo {
                kind,
                anchor_client,
            } => self.handle_open_combo(host, &kind, anchor_client),
            SettingsAction::SelectDock(idx) => {
                self.settings_ui.selected_dock = idx;
                self.settings_ui.selected_dock_item = None;
                self.settings_ui.reset_dock_input_sync();
            }
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
            SettingsAction::RestartAllModules => {
                for id in ModuleId::ALL {
                    if !self.modules_cfg.is_enabled(id) {
                        continue;
                    }
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
