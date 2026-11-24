#![cfg(windows)]

use asrbox_core::{ModuleId, SupervisorCmd};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_dock::{
    default_label_for_path, load_docks_from_file, save_docks_to_file, DockConfig, DockEdge,
    DockItem, DockLabelMode,
};
use asrbox_mod_settings::SettingsAction;
use asrbox_mod_widgets::{
    load_sysmon_config, load_widget_config, rows_for_height, save_sysmon_config,
    save_widget_config, uncomment_calendar_sources, widget_height, CalendarSource, PeriodFilter,
    MAX_REFRESH_MS, MAX_SYSMON_HEIGHT, MAX_SYSMON_WIDTH, MIN_REFRESH_MS, MIN_SYSMON_HEIGHT,
    MIN_SYSMON_WIDTH,
};
use asrbox_platform_api::{DockPlatform, IconProvider, ToolsPlatform, WidgetPlatform};

use crate::{AppController, APP_VERSION};

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

    pub(crate) fn apply_dock_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        let mut cfgs = load_docks_from_file(&self.paths.docks_cfg).unwrap_or_default();
        if cfgs.is_empty() {
            cfgs.push(DockConfig::new(1, "Док 1"));
        }
        let changed = match action {
            SettingsAction::RenameDockItemFromInput { dock_id, item_id } => {
                let text = host.settings_input_text(0).trim().to_owned();
                if text.is_empty() {
                    false
                } else {
                    self.settings_ui.reset_dock_input_sync();
                    edit_dock_item(&mut cfgs, dock_id, *item_id, |it| it.label = text)
                }
            }
            SettingsAction::ResetDockItemLabel { dock_id, item_id } => {
                self.settings_ui.reset_dock_input_sync();
                edit_dock_item(&mut cfgs, dock_id, *item_id, |it| {
                    it.label = default_label_for_path(&it.target);
                })
            }
            other => match apply_dock_prop(&mut cfgs, other) {
                Some(v) => v,
                None => return false,
            },
        };
        if changed && save_docks_to_file(&self.paths.docks_cfg, &cfgs, APP_VERSION).is_ok() {
            self.reload_module_config(host, ModuleId::Dock);
        }
        true
    }
}

fn apply_dock_prop(cfgs: &mut Vec<DockConfig>, action: &SettingsAction) -> Option<bool> {
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
        SettingsAction::AddDockSeparator(id_s) => edit_dock(cfgs, id_s, |c| {
            let nid = c.next_item_id();
            c.items.push(DockItem::separator(nid));
        }),
        SettingsAction::SetDockEdge {
            dock_id,
            edge_index,
        } => edit_dock(cfgs, dock_id, |c| {
            c.edge = match edge_index {
                1 => DockEdge::Top,
                2 => DockEdge::Left,
                3 => DockEdge::Right,
                _ => DockEdge::Bottom,
            };
            c.free_pos = None;
        }),
        SettingsAction::SetDockLayer { dock_id, layer } => {
            edit_dock(cfgs, dock_id, |c| c.layer = *layer)
        }
        SettingsAction::SetDockIconSize { dock_id, size_px } => edit_dock(cfgs, dock_id, |c| {
            c.icon_size_px = (*size_px).clamp(24, 128) as u16
        }),
        SettingsAction::SetDockOpacity { dock_id, opacity } => edit_dock(cfgs, dock_id, |c| {
            c.opacity_percent = (*opacity).clamp(20, 100)
        }),
        SettingsAction::SetDockLocked { dock_id, locked } => {
            edit_dock(cfgs, dock_id, |c| c.locked = *locked)
        }
        SettingsAction::SetDockShowRunning { dock_id, enabled } => {
            edit_dock(cfgs, dock_id, |c| c.show_running_indicator = *enabled)
        }
        SettingsAction::SetDockFocusRunning { dock_id, enabled } => {
            edit_dock(cfgs, dock_id, |c| c.focus_running_window = *enabled)
        }
        SettingsAction::SetDockLaunchAnimation { dock_id, enabled } => {
            edit_dock(cfgs, dock_id, |c| c.launch_animation_enabled = *enabled)
        }
        SettingsAction::SetDockLabelMode {
            dock_id,
            mode_index,
        } => edit_dock(cfgs, dock_id, |c| {
            c.label_mode = DockLabelMode::from_index(*mode_index)
        }),
        _ => return None,
    };
    Some(changed)
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

fn edit_dock_item<F: FnOnce(&mut DockItem)>(
    cfgs: &mut [DockConfig],
    id_s: &str,
    item_id: u64,
    f: F,
) -> bool {
    let Ok(id) = id_s.parse::<u64>() else {
        return false;
    };
    let Some(item) = cfgs.iter_mut().find(|c| c.id == id).and_then(|c| {
        c.items
            .iter_mut()
            .find(|i| i.id == item_id && !i.is_separator())
    }) else {
        return false;
    };
    f(item);
    true
}
