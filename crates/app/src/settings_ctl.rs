#![cfg(windows)]

use asrbox_core::{save_modules_config, IsolationMode, ModuleId};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_dock::{load_docks_from_file, save_docks_to_file, DockConfig, DockEdge, DockItem};
use asrbox_mod_settings::{
    AboutSettingsView, CalendarSettingsView, CalendarSourceView, DebugSettingsView,
    DockSettingsView, SettingsAction, SettingsSnapshot, SettingsTab, SysMonitorSettingsView,
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
        self.render_settings_ui(host);
    }

    pub(crate) fn render_settings_ui<H: ToolsPlatform>(&mut self, host: &mut H) {
        if !host.is_settings_window_open() {
            return;
        }
        self.sync_settings_tab_inputs(host);
        let snap = self.build_settings_snapshot(host);
        let (frame, inputs) = self.settings_ui.render(&snap);
        host.present_settings_frame(&frame, inputs);
    }

    fn sync_settings_tab_inputs<H: ToolsPlatform>(&mut self, host: &H) {
        let cur = self.settings_ui.active_tab;
        if self.last_settings_tab == Some(cur) {
            return;
        }
        self.last_settings_tab = Some(cur);
        match cur {
            SettingsTab::Backup => {
                host.set_settings_input_text(0, self.backup.webdav_username());
                host.set_settings_input_text(1, "");
                host.set_settings_input_text(2, self.backup.webdav_custom_url());
            }
            SettingsTab::Calendar => {
                host.set_settings_input_text(0, "");
            }
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

    pub(crate) fn build_settings_snapshot(&self, host: &dyn ToolsPlatform) -> SettingsSnapshot {
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
                item_count: d.items.len(),
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

        SettingsSnapshot {
            isolation: self.modules_cfg.isolation,
            autostart_enabled: host.is_autostart_enabled(),
            modules: self.supervisor.status_views(&self.modules_cfg),
            docks,
            calendar,
            sysmon,
            backup: self.backup.backup_view(),
            webdav: self.backup.webdav_view(),
            debug: DebugSettingsView {
                verbose: self.debug.verbose_trace(),
                file_log: self.debug.file_logging(),
                summary_line: format!(
                    "Режим: {} · фильтр логов: {}",
                    self.modules_cfg.isolation.as_str(),
                    self.debug.filter_label()
                ),
            },
            about: AboutSettingsView {
                version: APP_VERSION.to_owned(),
                exe_path: self.paths.exe_path.display().to_string(),
                data_dir: self.paths.data_dir.display().to_string(),
                config_dir: self.paths.data_dir.join("config").display().to_string(),
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
            SettingsAction::OpenDebugWindow => self.open_debug_window(host),
            SettingsAction::OpenAboutWindow => self.open_about_window(host),
            SettingsAction::OpenDataFolder => {
                let _ = std::fs::create_dir_all(&self.paths.data_dir);
                let _ = host.open_folder(&self.paths.data_dir);
            }
            other => {
                if !self.apply_dock_setting(host, &other)
                    && !self.apply_backup_setting(host, &other)
                {
                    self.apply_widget_setting(host, &other);
                }
            }
        }
    }

    fn apply_dock_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        let mut cfgs = load_docks_from_file(&self.paths.docks_cfg).unwrap_or_default();
        if cfgs.is_empty() {
            cfgs.push(DockConfig::new(1, "Док 1"));
        }
        let changed = match action {
            SettingsAction::AddDock => {
                let next_id = cfgs.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                let mut c = DockConfig::new(next_id, format!("Док {next_id}"));
                c.edge = match cfgs.len() % 4 {
                    0 => DockEdge::Bottom,
                    1 => DockEdge::Left,
                    2 => DockEdge::Right,
                    _ => DockEdge::Top,
                };
                cfgs.push(c);
                true
            }
            SettingsAction::RemoveDock(id_s) if cfgs.len() > 1 => {
                let id = id_s.parse::<u64>().unwrap_or(0);
                let before = cfgs.len();
                cfgs.retain(|c| c.id != id);
                cfgs.len() != before
            }
            SettingsAction::AddDockSeparator(id_s) => edit_dock(&mut cfgs, id_s, |c| {
                let nid = c.next_item_id();
                c.items.push(DockItem::separator(nid));
            }),
            SettingsAction::SetDockEdge {
                dock_id,
                edge_index,
            } => edit_dock(&mut cfgs, dock_id, |c| {
                c.edge = match edge_index {
                    1 => DockEdge::Top,
                    2 => DockEdge::Left,
                    3 => DockEdge::Right,
                    _ => DockEdge::Bottom,
                };
                c.free_pos = None;
            }),
            SettingsAction::SetDockLayer { dock_id, layer } => {
                edit_dock(&mut cfgs, dock_id, |c| c.layer = *layer)
            }
            SettingsAction::SetDockIconSize { dock_id, size_px } => {
                edit_dock(&mut cfgs, dock_id, |c| {
                    c.icon_size_px = (*size_px).clamp(24, 128) as u16;
                })
            }
            SettingsAction::SetDockOpacity { dock_id, opacity } => {
                edit_dock(&mut cfgs, dock_id, |c| {
                    c.opacity_percent = (*opacity).clamp(20, 100);
                })
            }
            SettingsAction::SetDockLocked { dock_id, locked } => {
                edit_dock(&mut cfgs, dock_id, |c| c.locked = *locked)
            }
            SettingsAction::SetDockShowRunning { dock_id, enabled } => {
                edit_dock(&mut cfgs, dock_id, |c| c.show_running_indicator = *enabled)
            }
            SettingsAction::SetDockFocusRunning { dock_id, enabled } => {
                edit_dock(&mut cfgs, dock_id, |c| c.focus_running_window = *enabled)
            }
            _ => return false,
        };
        if changed && save_docks_to_file(&self.paths.docks_cfg, &cfgs, APP_VERSION).is_ok() {
            self.reload_module_config(host, ModuleId::Dock);
        }
        true
    }

    pub(crate) fn persist_modules_cfg(&self) {
        let path = self.paths.data_dir.join("config").join("modules.cfg");
        let _ = save_modules_config(&path, &self.modules_cfg);
    }
}

fn edit_dock<F: FnOnce(&mut DockConfig)>(cfgs: &mut [DockConfig], id_s: &str, f: F) -> bool {
    let Ok(id) = id_s.parse::<u64>() else {
        return false;
    };
    let Some(cfg) = cfgs.iter_mut().find(|c| c.id == id) else {
        return false;
    };
    f(cfg);
    true
}
