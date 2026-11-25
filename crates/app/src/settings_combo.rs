#![cfg(windows)]

use asrbox_core::IsolationMode;
use asrbox_mod_dock::{load_docks_from_file, DockEdge};
use asrbox_mod_settings::{ComboKind, SettingsAction, SettingsTab, WidgetSubTab};
use asrbox_mod_widgets::{load_notes_config, load_weather_config};
use asrbox_platform_api::{
    AccentPreset, AppLanguage, DesignTheme, DockPlatform, FluentMenuItem, FluentMenuSpec,
    FontRenderer, IconProvider, PixelPoint, ThemeMode, ToolsPlatform, WidgetLayer, WidgetPlatform,
};

use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn handle_open_combo<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        kind: &ComboKind,
        anchor_client: PixelPoint,
    ) {
        let screen_pt = host.settings_client_to_screen(anchor_client);
        let ru = self.appearance.language == AppLanguage::Ru;
        let spec = FluentMenuSpec {
            quick_actions: Vec::new(),
            items: self.build_combo_items(kind, ru),
            min_width: 224,
        };
        let pal = self.appearance.palette();
        let Ok(Some(cmd)) = host.show_fluent_popup_menu(screen_pt, &spec, pal) else {
            return;
        };
        if let Some(action) = decode_combo_selection(kind, cmd) {
            self.apply_settings_action(host, action);
        }
    }

    fn build_combo_items(&self, kind: &ComboKind, ru: bool) -> Vec<FluentMenuItem> {
        let lang = self.appearance.language;
        match kind {
            ComboKind::DesignTheme => vec![
                combo_item(
                    1,
                    DesignTheme::AsrGlass.label(lang),
                    self.appearance.design_theme == DesignTheme::AsrGlass,
                ),
                combo_item(
                    2,
                    DesignTheme::Win11Classic.label(lang),
                    self.appearance.design_theme == DesignTheme::Win11Classic,
                ),
            ],
            ComboKind::ThemeMode => vec![
                combo_item(
                    1,
                    if ru { "Тёмная" } else { "Dark" },
                    self.appearance.theme == ThemeMode::Dark,
                ),
                combo_item(
                    2,
                    if ru { "Светлая" } else { "Light" },
                    self.appearance.theme == ThemeMode::Light,
                ),
            ],
            ComboKind::AccentPreset => AccentPreset::ALL
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    combo_item((i + 1) as u32, p.label(lang), self.appearance.accent == *p)
                })
                .collect(),
            ComboKind::FontRenderer => vec![
                combo_item(
                    1,
                    FontRenderer::GdiClearType.label(),
                    self.appearance.font_renderer == FontRenderer::GdiClearType,
                ),
                combo_item(
                    2,
                    FontRenderer::DirectWrite.label(),
                    self.appearance.font_renderer == FontRenderer::DirectWrite,
                ),
            ],
            ComboKind::Language => vec![
                combo_item(1, "Русский", lang == AppLanguage::Ru),
                combo_item(2, "English", lang == AppLanguage::En),
            ],
            ComboKind::Isolation => vec![
                combo_item(
                    1,
                    if ru {
                        "Процессы (изолированно)"
                    } else {
                        "Processes (isolated)"
                    },
                    self.modules_cfg.isolation == IsolationMode::Process,
                ),
                combo_item(
                    2,
                    if ru {
                        "Один процесс (InProc)"
                    } else {
                        "Single process (InProc)"
                    },
                    self.modules_cfg.isolation == IsolationMode::InProc,
                ),
            ],
            ComboKind::DockSelect => {
                let docks = load_docks_from_file(&self.paths.docks_cfg).unwrap_or_default();
                docks
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let edge = edge_name(d.edge, ru);
                        let lbl = format!("Док {} ({edge})", d.id);
                        combo_item((i + 1) as u32, &lbl, i == self.settings_ui.selected_dock)
                    })
                    .collect()
            }
            ComboKind::DockEdge(dock_id) => {
                let cur = self
                    .find_dock_cfg(dock_id)
                    .map(|d| d.edge)
                    .unwrap_or(DockEdge::Bottom);
                vec![
                    combo_item(1, edge_name(DockEdge::Bottom, ru), cur == DockEdge::Bottom),
                    combo_item(2, edge_name(DockEdge::Top, ru), cur == DockEdge::Top),
                    combo_item(3, edge_name(DockEdge::Left, ru), cur == DockEdge::Left),
                    combo_item(4, edge_name(DockEdge::Right, ru), cur == DockEdge::Right),
                ]
            }
            ComboKind::DockLayer(dock_id) => {
                let cur = self
                    .find_dock_cfg(dock_id)
                    .map(|d| d.layer)
                    .unwrap_or(WidgetLayer::Wallpaper);
                layer_items(cur, ru)
            }
            ComboKind::DockLabelMode(dock_id) => {
                let cur = self
                    .find_dock_cfg(dock_id)
                    .map(|d| d.label_mode.to_index())
                    .unwrap_or(0);
                vec![
                    combo_item(
                        1,
                        if ru {
                            "Только иконки"
                        } else {
                            "Icons only"
                        },
                        cur == 0,
                    ),
                    combo_item(
                        2,
                        if ru {
                            "При наведении"
                        } else {
                            "On hover"
                        },
                        cur == 1,
                    ),
                    combo_item(
                        3,
                        if ru {
                            "Иконка + имя"
                        } else {
                            "Icon + label"
                        },
                        cur == 2,
                    ),
                    combo_item(
                        4,
                        if ru {
                            "Только имена"
                        } else {
                            "Labels only"
                        },
                        cur == 3,
                    ),
                ]
            }
            ComboKind::WeatherLayer => {
                let cur = load_weather_config(&self.paths.weather_cfg)
                    .map(|c| c.layer)
                    .unwrap_or(WidgetLayer::Wallpaper);
                layer_items(cur, ru)
            }
            ComboKind::NotesLayer => {
                let cur = load_notes_config(&self.paths.notes_cfg)
                    .map(|c| c.layer)
                    .unwrap_or(WidgetLayer::Normal);
                layer_items(cur, ru)
            }
        }
    }

    fn find_dock_cfg(&self, dock_id: &str) -> Option<asrbox_mod_dock::DockConfig> {
        let id = dock_id.parse::<u64>().ok()?;
        load_docks_from_file(&self.paths.docks_cfg)
            .ok()?
            .into_iter()
            .find(|d| d.id == id)
    }

    pub(crate) fn sync_settings_tab_inputs<H: ToolsPlatform>(&mut self, host: &H) {
        let cur = self.settings_ui.active_tab;
        let sub = self.settings_ui.selected_widget;
        let tab_changed = self.last_settings_tab != Some(cur);
        let sub_changed = cur == SettingsTab::Widgets && self.last_widget_subtab != Some(sub);
        if !tab_changed && !sub_changed {
            return;
        }
        let prev = self.last_settings_tab;
        self.last_settings_tab = Some(cur);
        self.last_widget_subtab = if cur == SettingsTab::Widgets {
            Some(sub)
        } else {
            None
        };
        match cur {
            SettingsTab::Dock => self.settings_ui.reset_dock_input_sync(),
            SettingsTab::Widgets => match sub {
                WidgetSubTab::Weather => {
                    let city = load_weather_config(&self.paths.weather_cfg)
                        .map(|c| c.city)
                        .unwrap_or_else(|_| "Москва".to_owned());
                    host.set_settings_input_text(0, &city);
                    host.set_settings_input_text(1, "");
                }
                _ => {
                    host.set_settings_input_text(0, "");
                    host.set_settings_input_text(1, "");
                }
            },
            SettingsTab::General => {
                if prev == Some(SettingsTab::Dock) || prev == Some(SettingsTab::Widgets) {
                    host.set_settings_input_text(0, "");
                    host.set_settings_input_text(1, "");
                }
                host.set_settings_input_text(5, "");
                host.set_settings_input_text(6, &self.backup.github_cfg.repo);
                host.set_settings_input_text(7, &self.backup.github_cfg.branch);
                host.set_settings_input_text(8, "");
            }
            SettingsTab::Overlay => {}
        }
    }
}

fn edge_name(edge: DockEdge, ru: bool) -> &'static str {
    match (edge, ru) {
        (DockEdge::Bottom, true) => "Низ экрана",
        (DockEdge::Top, true) => "Верх экрана",
        (DockEdge::Left, true) => "Левый край",
        (DockEdge::Right, true) => "Правый край",
        (DockEdge::Bottom, false) => "Bottom",
        (DockEdge::Top, false) => "Top",
        (DockEdge::Left, false) => "Left",
        (DockEdge::Right, false) => "Right",
    }
}

fn combo_item(id: u32, label: &str, checked: bool) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        icon: None,
        label: label.to_owned(),
        shortcut: None,
        checked,
        enabled: true,
    }
}

fn layer_items(cur: WidgetLayer, ru: bool) -> Vec<FluentMenuItem> {
    vec![
        combo_item(
            1,
            if ru {
                "На рабочем столе"
            } else {
                "Desktop"
            },
            cur == WidgetLayer::Wallpaper,
        ),
        combo_item(
            2,
            if ru {
                "Обычное окно"
            } else {
                "Normal"
            },
            cur == WidgetLayer::Normal,
        ),
        combo_item(
            3,
            if ru {
                "Поверх всех окон"
            } else {
                "Always on top"
            },
            cur == WidgetLayer::Topmost,
        ),
    ]
}

fn decode_layer(cmd: u32) -> Option<WidgetLayer> {
    match cmd {
        1 => Some(WidgetLayer::Wallpaper),
        2 => Some(WidgetLayer::Normal),
        3 => Some(WidgetLayer::Topmost),
        _ => None,
    }
}

fn decode_combo_selection(kind: &ComboKind, cmd: u32) -> Option<SettingsAction> {
    match kind {
        ComboKind::DesignTheme => match cmd {
            1 => Some(SettingsAction::SetDesignTheme(DesignTheme::AsrGlass)),
            2 => Some(SettingsAction::SetDesignTheme(DesignTheme::Win11Classic)),
            _ => None,
        },
        ComboKind::ThemeMode => match cmd {
            1 => Some(SettingsAction::SetThemeMode(ThemeMode::Dark)),
            2 => Some(SettingsAction::SetThemeMode(ThemeMode::Light)),
            _ => None,
        },
        ComboKind::AccentPreset => AccentPreset::ALL
            .get((cmd as usize).saturating_sub(1))
            .copied()
            .map(SettingsAction::SetAccentPreset),
        ComboKind::FontRenderer => match cmd {
            1 => Some(SettingsAction::SetFontRenderer(FontRenderer::GdiClearType)),
            2 => Some(SettingsAction::SetFontRenderer(FontRenderer::DirectWrite)),
            _ => None,
        },
        ComboKind::Language => match cmd {
            1 => Some(SettingsAction::SetLanguage(AppLanguage::Ru)),
            2 => Some(SettingsAction::SetLanguage(AppLanguage::En)),
            _ => None,
        },
        ComboKind::Isolation => match cmd {
            1 => Some(SettingsAction::SetIsolation(IsolationMode::Process)),
            2 => Some(SettingsAction::SetIsolation(IsolationMode::InProc)),
            _ => None,
        },
        ComboKind::DockSelect if cmd >= 1 => Some(SettingsAction::SelectDock((cmd - 1) as usize)),
        ComboKind::DockEdge(dock_id) if (1..=4).contains(&cmd) => {
            Some(SettingsAction::SetDockEdge {
                dock_id: dock_id.clone(),
                edge_index: (cmd - 1) as u8,
            })
        }
        ComboKind::DockLayer(dock_id) => {
            decode_layer(cmd).map(|layer| SettingsAction::SetDockLayer {
                dock_id: dock_id.clone(),
                layer,
            })
        }
        ComboKind::DockLabelMode(dock_id) if (1..=4).contains(&cmd) => {
            Some(SettingsAction::SetDockLabelMode {
                dock_id: dock_id.clone(),
                mode_index: (cmd - 1) as u8,
            })
        }
        ComboKind::WeatherLayer => decode_layer(cmd).map(SettingsAction::SetWeatherLayer),
        ComboKind::NotesLayer => decode_layer(cmd).map(SettingsAction::SetNotesLayer),
        _ => None,
    }
}
