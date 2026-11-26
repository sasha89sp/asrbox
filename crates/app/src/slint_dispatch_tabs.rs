#![cfg(windows)]

use asrbox_core::{ModuleId, SupervisorCmd};
use asrbox_mod_settings::SettingsAction;
use asrbox_mod_widgets::{
    load_weather_config, load_widget_config, save_weather_config, save_widget_config,
    CalendarSource, YANDEX_WEATHER_CRED_TARGET,
};
use asrbox_platform_api::{
    DockPlatform, IconProvider, ThemeMode, ToolsPlatform, TrayAction, WidgetLayer, WidgetPlatform,
};

use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn dispatch_tabs_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> (bool, bool) {
        if let Some(res) = self.dispatch_dock_cmd(host, act, arg) {
            return res;
        }
        if let Some(res) = self.dispatch_cal_and_sys_cmd(host, act, arg) {
            return res;
        }
        if let Some(res) = self.dispatch_weather_and_notes_cmd(host, act, arg) {
            return res;
        }
        self.dispatch_overlay_cmd(host, act, arg)
    }

    pub(crate) fn dispatch_tray_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> (bool, bool) {
        if act == "tray_toggle_theme" {
            let next = if self.appearance.theme == ThemeMode::Light {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            };
            self.apply_settings_action(host, SettingsAction::SetThemeMode(next));
            return (true, true);
        }
        let (name, id_s) = arg.split_once('|').unwrap_or((arg, "0"));
        let id = id_s.parse::<u64>().unwrap_or(0);
        if name == "refresh_all" {
            self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshWeather);
            self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshSysMonitor);
            self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshCalendar);
            self.reload_module_config(host, ModuleId::Dock);
            return (true, true);
        }
        let Some(action) = decode_tray_action(name, id) else {
            return (false, false);
        };
        self.apply_tray_action(host, action);
        (true, false)
    }

    fn dispatch_cal_and_sys_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> Option<(bool, bool)> {
        if act == "cal_add_url" {
            self.add_calendar_url_direct(host, arg);
            return Some((true, false));
        }
        let action = match act {
            "cal_visible" => SettingsAction::SetCalendarVisible(arg == "1"),
            "cal_layer" => SettingsAction::SetCalendarLayer(idx_to_layer(arg)),
            "cal_period" => SettingsAction::SetCalendarPeriod(arg.parse::<u8>().unwrap_or(2)),
            "cal_sync" => SettingsAction::SetCalendarSyncInterval(decode_cal_sync_sec(arg)),
            "cal_opacity" => SettingsAction::SetCalendarOpacity(arg.parse::<u8>().unwrap_or(88)),
            "cal_width" => SettingsAction::SetCalendarWidth(arg.parse::<u32>().unwrap_or(336)),
            "cal_rows" => SettingsAction::SetCalendarRows(arg.parse::<u8>().unwrap_or(8)),
            "cal_refresh" => SettingsAction::RefreshCalendarNow,
            "cal_match_sys" => SettingsAction::MatchCalendarToSysMonitor,
            "cal_remove_src" => {
                SettingsAction::RemoveCalendarSource(arg.parse::<usize>().unwrap_or(0))
            }
            "cal_uncomment" => SettingsAction::UncommentCalendarSources,
            "open_widgets_cfg" => SettingsAction::OpenWidgetsConfig,
            "sys_visible" => SettingsAction::SetSysMonitorVisible(arg == "1"),
            "sys_layer" => SettingsAction::SetSysMonitorLayer(idx_to_layer(arg)),
            "sys_refresh" => SettingsAction::SetSysMonitorRefreshMs(decode_sys_refresh_ms(arg)),
            "sys_opacity" => SettingsAction::SetSysMonitorOpacity(arg.parse::<u8>().unwrap_or(88)),
            "sys_width" => SettingsAction::SetSysMonitorWidth(arg.parse::<u32>().unwrap_or(348)),
            "sys_height" => SettingsAction::SetSysMonitorHeight(arg.parse::<u32>().unwrap_or(410)),
            "sys_refresh_now" => SettingsAction::RefreshSysMonitorNow,
            "sys_match_cal" => SettingsAction::MatchSysMonitorToCalendar,
            "open_sysmon_cfg" => SettingsAction::OpenSysMonitorConfig,
            _ => return None,
        };
        self.apply_settings_action(host, action);
        Some((true, false))
    }

    fn dispatch_weather_and_notes_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> Option<(bool, bool)> {
        if act == "weather_city" || act == "weather_save_key" {
            self.apply_weather_direct(host, act, arg);
            return Some((true, false));
        }
        let mut notes_dirty = false;
        let action = match act {
            "weather_visible" => SettingsAction::SetWeatherVisible(arg == "1"),
            "weather_provider" => SettingsAction::SetWeatherProvider(arg == "1"),
            "weather_days" => {
                let d = match arg.parse::<i32>().unwrap_or(1) {
                    0 => 1,
                    2 => 7,
                    _ => 3,
                };
                SettingsAction::SetWeatherForecastDays(d)
            }
            "weather_clear_key" => SettingsAction::ClearYandexWeatherKey,
            "weather_layer" => SettingsAction::SetWeatherLayer(idx_to_layer(arg)),
            "weather_opacity" => SettingsAction::SetWeatherOpacity(arg.parse::<u8>().unwrap_or(90)),
            "weather_refresh_min" => {
                SettingsAction::SetWeatherRefreshMin(arg.parse::<u32>().unwrap_or(30))
            }
            "weather_width" => SettingsAction::SetWeatherWidth(arg.parse::<u32>().unwrap_or(360)),
            "weather_height" => SettingsAction::SetWeatherHeight(arg.parse::<u32>().unwrap_or(410)),
            "weather_refresh_now" => SettingsAction::RefreshWeatherNow,
            "open_weather_cfg" => SettingsAction::OpenWeatherConfig,
            "notes_visible" => {
                notes_dirty = true;
                SettingsAction::SetNotesVisible(arg == "1")
            }
            "notes_layer" => {
                notes_dirty = true;
                SettingsAction::SetNotesLayer(idx_to_layer(arg))
            }
            "notes_opacity" => {
                notes_dirty = true;
                SettingsAction::SetNotesOpacity(arg.parse::<u8>().unwrap_or(94))
            }
            "notes_toggle_pane" => {
                notes_dirty = true;
                SettingsAction::ToggleNotesPane(arg.parse::<u8>().unwrap_or(0))
            }
            "open_notes_folder" => SettingsAction::OpenNotesFolder,
            _ => return None,
        };
        self.apply_settings_action(host, action);
        Some((true, notes_dirty))
    }

    fn dispatch_overlay_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> (bool, bool) {
        let action = match act {
            "ov_auto_games" => SettingsAction::SetOverlayAutoGamesOnly(arg == "1"),
            "ov_force_topmost" => SettingsAction::SetOverlayForceTopmost(arg == "1"),
            "ov_setup_mode" => SettingsAction::SetOverlaySetupMode(arg == "1"),
            "ov_anchor" => SettingsAction::SetOverlayAnchor(arg.parse::<u8>().unwrap_or(1)),
            "ov_opacity" => SettingsAction::SetOverlayOpacity(arg.parse::<u8>().unwrap_or(82)),
            "ov_width" => SettingsAction::SetOverlayWidth(arg.parse::<u32>().unwrap_or(300)),
            "ov_height" => SettingsAction::SetOverlayHeight(arg.parse::<u32>().unwrap_or(176)),
            "ov_refresh_ms" => {
                SettingsAction::SetOverlayRefreshMs(arg.parse::<u32>().unwrap_or(1000))
            }
            "ov_preset" => {
                let Some((w_s, h_s)) = arg.split_once('|') else {
                    return (false, false);
                };
                SettingsAction::SetOverlaySizePreset(
                    w_s.parse::<u32>().unwrap_or(300),
                    h_s.parse::<u32>().unwrap_or(176),
                )
            }
            "open_overlay_cfg" => SettingsAction::OpenOverlayConfig,
            _ => return (false, false),
        };
        self.apply_settings_action(host, action);
        (true, false)
    }

    fn add_calendar_url_direct<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        arg: &str,
    ) {
        let url = arg.trim().trim_matches('"').trim_matches('\'');
        if !url.starts_with("https://") && !url.starts_with("http://") {
            self.last_ui_status_msg = "Введите корректный URL (.ics)".to_owned();
            return;
        }
        let mut cfg = load_widget_config(&self.paths.widgets_cfg).unwrap_or_default();
        let id = format!("cal_{}", cfg.sources.len() + 1);
        cfg.sources.push(CalendarSource {
            id,
            url: url.to_owned(),
            color_rgb: None,
        });
        cfg.commented_calendar_hint = false;
        if save_widget_config(&self.paths.widgets_cfg, &cfg).is_ok() {
            self.reload_module_config(host, ModuleId::Widgets);
        }
        self.last_ui_status_msg = "Календарь добавлен".to_owned();
    }

    fn apply_weather_direct<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) {
        let now = host.local_time_now();
        if act == "weather_city" {
            let city = arg.trim();
            if !city.is_empty() {
                let mut cfg = load_weather_config(&self.paths.weather_cfg).unwrap_or_default();
                cfg.city = city.to_owned();
                if save_weather_config(&self.paths.weather_cfg, &cfg).is_ok() {
                    self.reload_module_config(host, ModuleId::Widgets);
                    self.last_ui_status_msg = format!("Город погоды: {city}");
                }
            }
            return;
        }
        let key = arg.trim();
        if key.is_empty() {
            self.last_ui_status_msg = "Введите X-Yandex-Weather-Key".to_owned();
            return;
        }
        let res = host
            .cred_save(YANDEX_WEATHER_CRED_TARGET, "yandex", key)
            .map(|()| "API-ключ Яндекс.Погоды сохранён в Credential Manager".to_owned());
        self.record_ui_outcome(now, "weather", res);
        self.reload_module_config(host, ModuleId::Widgets);
    }
}

fn decode_cal_sync_sec(arg: &str) -> u32 {
    match arg.parse::<i32>().unwrap_or(2) {
        0 => 300,
        1 => 900,
        3 => 3600,
        4 => 10_800,
        _ => 1800,
    }
}

fn decode_sys_refresh_ms(arg: &str) -> u32 {
    match arg.parse::<i32>().unwrap_or(1) {
        0 => 1000,
        2 => 3000,
        3 => 5000,
        4 => 10_000,
        _ => 2000,
    }
}

pub(crate) fn idx_to_layer(raw: &str) -> WidgetLayer {
    match raw.parse::<i32>().unwrap_or(0) {
        1 => WidgetLayer::Normal,
        2 => WidgetLayer::Topmost,
        _ => WidgetLayer::Wallpaper,
    }
}

fn decode_tray_action(name: &str, id: u64) -> Option<TrayAction> {
    Some(match name {
        "open_settings" => TrayAction::OpenSettings,
        "toggle_dock_module" => TrayAction::ToggleDockModule,
        "toggle_dock" => TrayAction::ToggleDock(id),
        "add_dock" => TrayAction::AddDock,
        "open_dock_cfg" => TrayAction::OpenDockConfig,
        "toggle_widgets_module" => TrayAction::ToggleWidgetsModule,
        "toggle_weather" => TrayAction::ToggleWeatherWidget,
        "toggle_sysmon" => TrayAction::ToggleSysMonitorWidget,
        "toggle_calendar" => TrayAction::ToggleCalendarWidget,
        "toggle_notes" => TrayAction::ToggleNotesWidget,
        "toggle_overlay_module" => TrayAction::ToggleOverlayModule,
        "toggle_overlay_setup" => TrayAction::ToggleOverlaySetupMode,
        "open_overlay_tab" => TrayAction::OpenOverlayTab,
        "backup_settings" => TrayAction::BackupSettingsNow,
        "backup_full" => TrayAction::BackupFullNow,
        "open_backup_tab" => TrayAction::OpenBackupTab,
        "toggle_autostart" => TrayAction::ToggleAutostart,
        "open_debug" => TrayAction::OpenDebugWindow,
        "open_about" => TrayAction::OpenAboutWindow,
        "exit" => TrayAction::Exit,
        _ => return None,
    })
}
