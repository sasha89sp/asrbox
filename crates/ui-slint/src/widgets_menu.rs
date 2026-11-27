use asrbox_platform_api::{
    AppearanceConfig, FluentMenuItem, FluentMenuSpec, WidgetLayer, WidgetMenuVisibility,
};

use crate::win_os::show_native_fluent_menu;

pub struct WidgetMenuParams<'a> {
    pub hwnd_raw: isize,
    pub appearance: AppearanceConfig,
    pub vis: WidgetMenuVisibility,
    pub refresh_label: &'a str,
    pub settings_label: &'a str,
    pub config_label: &'a str,
    pub layer: WidgetLayer,
    pub opacity_percent: u8,
    pub interval_title: &'a str,
    pub interval_options: &'a [(u32, &'a str)],
    pub current_interval: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetMenuChoice {
    Refresh,
    OpenSettings,
    OpenConfig,
    SetLayer(WidgetLayer),
    SetOpacity(u8),
    SetInterval(u32),
    HideWidget,
}

const OPACITY_STEPS: [u8; 5] = [100, 90, 80, 65, 50];

pub fn show_widget_context_menu(p: &WidgetMenuParams<'_>) -> Option<WidgetMenuChoice> {
    let mut items = Vec::new();
    if p.vis.show_refresh {
        items.push(icon_act(1, p.refresh_label, "↻"));
    }
    items.push(icon_act(2, p.settings_label, "⚙"));
    if p.vis.show_open_config {
        items.push(icon_act(3, p.config_label, "📄"));
    }
    if p.vis.show_layer || p.vis.show_opacity || p.vis.show_interval {
        items.push(FluentMenuItem::Separator);
    }
    if p.vis.show_layer {
        items.push(FluentMenuItem::Submenu {
            icon: Some("◫".to_owned()),
            label: "Слой отображения".to_owned(),
            enabled: true,
            items: vec![
                chk_act(10, "Над обоями", p.layer == WidgetLayer::Wallpaper),
                chk_act(11, "Обычное окно", p.layer == WidgetLayer::Normal),
                chk_act(12, "Поверх окон", p.layer == WidgetLayer::Topmost),
            ],
        });
    }
    if p.vis.show_opacity {
        let ops = OPACITY_STEPS
            .iter()
            .enumerate()
            .map(|(i, &pct)| {
                let checked = match i {
                    0 => p.opacity_percent >= 96,
                    1 => (86..96).contains(&p.opacity_percent),
                    2 => (73..86).contains(&p.opacity_percent),
                    3 => (58..73).contains(&p.opacity_percent),
                    _ => p.opacity_percent < 58,
                };
                chk_act(20 + i as u32, &format!("{pct}%"), checked)
            })
            .collect();
        items.push(FluentMenuItem::Submenu {
            icon: Some("◐".to_owned()),
            label: "Прозрачность".to_owned(),
            enabled: true,
            items: ops,
        });
    }
    if p.vis.show_interval && !p.interval_options.is_empty() {
        let ivs = p
            .interval_options
            .iter()
            .enumerate()
            .map(|(i, &(val, lbl))| chk_act(30 + i as u32, lbl, p.current_interval == val))
            .collect();
        items.push(FluentMenuItem::Submenu {
            icon: Some("⏱".to_owned()),
            label: p.interval_title.to_owned(),
            enabled: true,
            items: ivs,
        });
    }
    if p.vis.show_hide {
        items.push(FluentMenuItem::Separator);
        items.push(icon_act(40, "Скрыть виджет", "✕"));
    }
    let spec = FluentMenuSpec {
        quick_actions: Vec::new(),
        min_width: 210,
        items,
    };
    let cmd = show_native_fluent_menu(p.hwnd_raw, &spec, p.appearance)
        .ok()
        .flatten()?;
    decode_choice(cmd, p.interval_options)
}

fn decode_choice(cmd: u32, intervals: &[(u32, &str)]) -> Option<WidgetMenuChoice> {
    match cmd {
        1 => Some(WidgetMenuChoice::Refresh),
        2 => Some(WidgetMenuChoice::OpenSettings),
        3 => Some(WidgetMenuChoice::OpenConfig),
        10 => Some(WidgetMenuChoice::SetLayer(WidgetLayer::Wallpaper)),
        11 => Some(WidgetMenuChoice::SetLayer(WidgetLayer::Normal)),
        12 => Some(WidgetMenuChoice::SetLayer(WidgetLayer::Topmost)),
        20..=24 => OPACITY_STEPS
            .get((cmd - 20) as usize)
            .copied()
            .map(WidgetMenuChoice::SetOpacity),
        30..=39 => intervals
            .get((cmd - 30) as usize)
            .map(|&(v, _)| WidgetMenuChoice::SetInterval(v)),
        40 => Some(WidgetMenuChoice::HideWidget),
        _ => None,
    }
}

fn chk_act(id: u32, label: &str, checked: bool) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        icon: None,
        label: label.to_owned(),
        shortcut: None,
        checked,
        enabled: true,
        destructive: false,
    }
}

fn icon_act(id: u32, label: &str, icon: &str) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        icon: Some(icon.to_owned()),
        label: label.to_owned(),
        shortcut: None,
        checked: false,
        enabled: true,
        destructive: id == 40,
    }
}
