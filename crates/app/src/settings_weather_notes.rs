#![cfg(windows)]

use asrbox_core::{ModuleId, SupervisorCmd};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_settings::SettingsAction;
use asrbox_mod_widgets::{
    load_notes_config, load_weather_config, normalize_forecast_days, save_notes_config,
    save_weather_config, WeatherProvider, MAX_WEATHER_HEIGHT, MAX_WEATHER_WIDTH,
    MIN_WEATHER_HEIGHT, MIN_WEATHER_WIDTH, YANDEX_WEATHER_CRED_TARGET,
};
use asrbox_platform_api::{DockPlatform, IconProvider, ToolsPlatform, WidgetPlatform};

use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn apply_weather_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        let now = host.local_time_now();
        match action {
            SettingsAction::RefreshWeatherNow => {
                self.send_or_run_widgets_cmd(host, SupervisorCmd::RefreshWeather);
                return true;
            }
            SettingsAction::OpenWeatherConfig => {
                let _ = ToolsPlatform::open_text_file(host, &self.paths.weather_cfg);
                return true;
            }
            SettingsAction::SaveYandexWeatherKey => {
                let key = host.settings_input_text(1).trim().to_owned();
                if key.is_empty() {
                    self.debug.log(
                        now,
                        LogLevel::Warn,
                        "weather",
                        "Введите X-Yandex-Weather-Key перед сохранением",
                    );
                } else {
                    let res = host.cred_save(YANDEX_WEATHER_CRED_TARGET, "yandex", &key);
                    self.log_io_result(
                        now,
                        "weather",
                        "API-ключ Яндекс.Погоды сохранён в Credential Manager",
                        res,
                    );
                    host.set_settings_input_text(1, "");
                    self.reload_module_config(host, ModuleId::Widgets);
                }
                return true;
            }
            SettingsAction::ClearYandexWeatherKey => {
                let res = host.cred_delete(YANDEX_WEATHER_CRED_TARGET);
                self.log_io_result(
                    now,
                    "weather",
                    "API-ключ Яндекс.Погоды удалён из Credential Manager",
                    res,
                );
                host.set_settings_input_text(1, "");
                self.reload_module_config(host, ModuleId::Widgets);
                return true;
            }
            _ => {}
        }
        let mut cfg = load_weather_config(&self.paths.weather_cfg).unwrap_or_default();
        let changed = match action {
            SettingsAction::SetWeatherVisible(v) => {
                cfg.visible = *v;
                true
            }
            SettingsAction::SetWeatherProvider(is_yandex) => {
                cfg.provider = if *is_yandex {
                    WeatherProvider::Yandex
                } else {
                    WeatherProvider::OpenMeteo
                };
                true
            }
            SettingsAction::SetWeatherForecastDays(days) => {
                cfg.forecast_days = normalize_forecast_days(*days);
                if cfg.forecast_days == 7 && cfg.height < 460 {
                    cfg.height = 460;
                }
                true
            }
            SettingsAction::ApplyWeatherCityFromInput => {
                let city = host.settings_input_text(0).trim().to_owned();
                if city.is_empty() {
                    false
                } else {
                    cfg.city = city;
                    true
                }
            }
            SettingsAction::SetWeatherLayer(l) => {
                cfg.layer = *l;
                true
            }
            SettingsAction::SetWeatherOpacity(o) => {
                cfg.opacity_percent = (*o).clamp(20, 100);
                true
            }
            SettingsAction::SetWeatherRefreshMin(m) => {
                cfg.refresh_interval_min = (*m).clamp(5, 360);
                true
            }
            SettingsAction::SetWeatherWidth(w) => {
                cfg.width = (*w).clamp(MIN_WEATHER_WIDTH, MAX_WEATHER_WIDTH);
                true
            }
            SettingsAction::SetWeatherHeight(h) => {
                cfg.height = (*h).clamp(MIN_WEATHER_HEIGHT, MAX_WEATHER_HEIGHT);
                true
            }
            _ => return false,
        };
        if changed && save_weather_config(&self.paths.weather_cfg, &cfg).is_ok() {
            self.reload_module_config(host, ModuleId::Widgets);
        }
        true
    }

    pub(crate) fn apply_notes_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        if matches!(action, SettingsAction::OpenNotesFolder) {
            let _ = std::fs::create_dir_all(&self.paths.notes_dir);
            let _ = host.open_folder(&self.paths.notes_dir);
            return true;
        }
        let mut cfg = load_notes_config(&self.paths.notes_cfg).unwrap_or_default();
        let changed = match action {
            SettingsAction::SetNotesVisible(v) => {
                cfg.visible = *v;
                true
            }
            SettingsAction::SetNotesLayer(l) => {
                cfg.layer = *l;
                true
            }
            SettingsAction::SetNotesOpacity(o) => {
                cfg.opacity_percent = (*o).clamp(20, 100);
                true
            }
            SettingsAction::ToggleNotesPane(idx) => cfg.toggle_pane(*idx),
            _ => return false,
        };
        if changed && save_notes_config(&self.paths.notes_cfg, &cfg).is_ok() {
            if matches!(action, SettingsAction::SetNotesVisible(_)) {
                if let Err(err) = self.slint_ui.set_notes_visible(cfg.visible) {
                    self.debug
                        .log(host.local_time_now(), LogLevel::Error, "widgets", err);
                }
            } else {
                self.slint_ui.notify_notes_refresh();
            }
        }
        true
    }
}
