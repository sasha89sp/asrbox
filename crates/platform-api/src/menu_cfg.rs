use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayMenuVisibility {
    pub show_autostart: bool,
    pub show_docks: bool,
    pub show_widgets: bool,
    pub show_overlay: bool,
    pub show_backup: bool,
    pub show_debug: bool,
    pub show_about: bool,
}

impl Default for TrayMenuVisibility {
    fn default() -> Self {
        Self {
            show_autostart: true,
            show_docks: true,
            show_widgets: true,
            show_overlay: true,
            show_backup: true,
            show_debug: true,
            show_about: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockMenuVisibility {
    pub show_item_actions: bool,
    pub show_add_separator: bool,
    pub show_edge_and_layer: bool,
    pub show_appearance: bool,
    pub show_behavior: bool,
    pub show_lock_controls: bool,
    pub show_dock_management: bool,
}

impl Default for DockMenuVisibility {
    fn default() -> Self {
        Self {
            show_item_actions: true,
            show_add_separator: true,
            show_edge_and_layer: true,
            show_appearance: true,
            show_behavior: true,
            show_lock_controls: true,
            show_dock_management: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetMenuVisibility {
    pub show_refresh: bool,
    pub show_layer: bool,
    pub show_opacity: bool,
    pub show_interval: bool,
    pub show_open_config: bool,
    pub show_hide: bool,
}

impl Default for WidgetMenuVisibility {
    fn default() -> Self {
        Self {
            show_refresh: true,
            show_layer: true,
            show_opacity: true,
            show_interval: true,
            show_open_config: true,
            show_hide: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContextMenuConfig {
    pub tray: TrayMenuVisibility,
    pub dock: DockMenuVisibility,
    pub widget: WidgetMenuVisibility,
}

pub fn load_menu_config(path: &Path) -> ContextMenuConfig {
    let Ok(raw) = fs::read_to_string(path) else {
        return ContextMenuConfig::default();
    };
    parse_menu_config(&raw)
}

pub fn save_menu_config(path: &Path, cfg: &ContextMenuConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all menus.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_menu_config(cfg)).map_err(|e| format!("write menus.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename menus.cfg: {e}"))?;
    Ok(())
}

pub fn parse_menu_config(raw: &str) -> ContextMenuConfig {
    let mut cfg = ContextMenuConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let b = parse_flag(v);
        match k.trim() {
            "tray.autostart" => cfg.tray.show_autostart = b,
            "tray.docks" => cfg.tray.show_docks = b,
            "tray.widgets" => cfg.tray.show_widgets = b,
            "tray.overlay" => cfg.tray.show_overlay = b,
            "tray.backup" => cfg.tray.show_backup = b,
            "tray.debug" => cfg.tray.show_debug = b,
            "tray.about" => cfg.tray.show_about = b,
            "dock.item_actions" => cfg.dock.show_item_actions = b,
            "dock.add_separator" => cfg.dock.show_add_separator = b,
            "dock.edge_and_layer" => cfg.dock.show_edge_and_layer = b,
            "dock.appearance" => cfg.dock.show_appearance = b,
            "dock.behavior" => cfg.dock.show_behavior = b,
            "dock.lock_controls" => cfg.dock.show_lock_controls = b,
            "dock.dock_management" => cfg.dock.show_dock_management = b,
            "widget.refresh" => cfg.widget.show_refresh = b,
            "widget.layer" => cfg.widget.show_layer = b,
            "widget.opacity" => cfg.widget.show_opacity = b,
            "widget.interval" => cfg.widget.show_interval = b,
            "widget.open_config" => cfg.widget.show_open_config = b,
            "widget.hide" => cfg.widget.show_hide = b,
            _ => {}
        }
    }
    cfg
}

pub fn serialize_menu_config(cfg: &ContextMenuConfig) -> String {
    let f = |b: bool| if b { 1 } else { 0 };
    format!(
        "# Настройка пунктов контекстных меню asrbox\n\
         version=1\n\
         tray.autostart={}\n\
         tray.docks={}\n\
         tray.widgets={}\n\
         tray.overlay={}\n\
         tray.backup={}\n\
         tray.debug={}\n\
         tray.about={}\n\
         dock.item_actions={}\n\
         dock.add_separator={}\n\
         dock.edge_and_layer={}\n\
         dock.appearance={}\n\
         dock.behavior={}\n\
         dock.lock_controls={}\n\
         dock.dock_management={}\n\
         widget.refresh={}\n\
         widget.layer={}\n\
         widget.opacity={}\n\
         widget.interval={}\n\
         widget.open_config={}\n\
         widget.hide={}\n",
        f(cfg.tray.show_autostart),
        f(cfg.tray.show_docks),
        f(cfg.tray.show_widgets),
        f(cfg.tray.show_overlay),
        f(cfg.tray.show_backup),
        f(cfg.tray.show_debug),
        f(cfg.tray.show_about),
        f(cfg.dock.show_item_actions),
        f(cfg.dock.show_add_separator),
        f(cfg.dock.show_edge_and_layer),
        f(cfg.dock.show_appearance),
        f(cfg.dock.show_behavior),
        f(cfg.dock.show_lock_controls),
        f(cfg.dock.show_dock_management),
        f(cfg.widget.show_refresh),
        f(cfg.widget.show_layer),
        f(cfg.widget.show_opacity),
        f(cfg.widget.show_interval),
        f(cfg.widget.show_open_config),
        f(cfg.widget.show_hide),
    )
}

fn parse_flag(v: &str) -> bool {
    !matches!(v.trim(), "0" | "false" | "off" | "no")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_config_roundtrip() {
        let mut cfg = ContextMenuConfig::default();
        cfg.tray.show_debug = false;
        cfg.dock.show_appearance = false;
        cfg.widget.show_open_config = false;
        assert_eq!(parse_menu_config(&serialize_menu_config(&cfg)), cfg);
    }
}
