use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{
    DockAlignment, DockConfig, DockEdge, DockItem, DockItemKind, HoverZoomConfig, HoverZoomMode,
    IconSource,
};

const SETTINGS_VERSION: u32 = 1;

pub fn serialize_docks(docks: &[DockConfig], app_version: &str) -> String {
    let mut out = String::new();
    out.push_str("# asrbox docks config\n");
    out.push_str("# После ручного редактирования выберите 'Перечитать настройки' в меню панели.\n");
    out.push_str(&format!("settings_version={SETTINGS_VERSION}\n"));
    out.push_str(&format!("written_by={app_version}\n"));

    for dock in docks {
        out.push_str("[dock]\n");
        out.push_str(&format!("id={}\n", dock.id));
        out.push_str(&format!("name={}\n", escape_field(&dock.name)));
        out.push_str(&format!("enabled={}\n", dock.enabled));
        out.push_str(&format!("monitor_index={}\n", dock.monitor_index));
        out.push_str(&format!("edge={}\n", edge_to_str(dock.edge)));
        out.push_str(&format!("alignment={}\n", alignment_to_str(dock.alignment)));
        out.push_str(&format!("edge_margin_px={}\n", dock.edge_margin_px));
        out.push_str(&format!("icon_size_px={}\n", dock.icon_size_px));
        out.push_str(&format!("item_spacing_px={}\n", dock.item_spacing_px));
        out.push_str(&format!("padding_px={}\n", dock.padding_px));
        out.push_str(&format!("corner_radius_px={}\n", dock.corner_radius_px));
        out.push_str(&format!(
            "show_settings_button={}\n",
            dock.show_settings_button
        ));
        out.push_str(&format!("hover_zoom_enabled={}\n", dock.hover_zoom.enabled));
        out.push_str(&format!("hover_zoom_scale={:.3}\n", dock.hover_zoom.scale));
        out.push_str(&format!(
            "hover_zoom_duration_ms={}\n",
            dock.hover_zoom.duration_ms
        ));
        for item in &dock.items {
            if item.is_separator() {
                out.push_str(&format!("separator={}\n", item.id));
            } else {
                out.push_str(&format!("item={}\n", serialize_item(item)));
            }
        }
    }
    out
}

pub fn parse_docks(input: &str) -> Result<Vec<DockConfig>, String> {
    let mut docks = Vec::new();
    let mut current: Option<DockConfig> = None;

    for raw_line in input.trim_start_matches('\u{FEFF}').lines() {
        let line = raw_line.trim().trim_start_matches('\u{FEFF}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[dock]" {
            if let Some(dock) = current.take() {
                dock.validate().map_err(|e| e.to_string())?;
                docks.push(dock);
            }
            let next_id = (docks.len() as u64).saturating_add(1);
            current = Some(DockConfig::new(next_id, format!("Dock {next_id}")));
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if let Some(dock) = current.as_mut() {
            apply_field(dock, key.trim(), value.trim())?;
        }
    }

    if let Some(dock) = current {
        dock.validate().map_err(|e| e.to_string())?;
        docks.push(dock);
    }
    Ok(docks)
}

pub fn save_docks_to_file(
    path: &Path,
    docks: &[DockConfig],
    app_version: &str,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("cannot create config dir {}: {err}", parent.display()))?;
    }
    let content = serialize_docks(docks, app_version);
    fs::write(path, content).map_err(|err| format!("cannot write config {}: {err}", path.display()))
}

pub fn load_docks_from_file(path: &Path) -> Result<Vec<DockConfig>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .map_err(|err| format!("cannot read config {}: {err}", path.display()))?;
    parse_docks(&content)
}

fn apply_field(dock: &mut DockConfig, key: &str, val: &str) -> Result<(), String> {
    match key {
        "id" => dock.id = val.parse::<u64>().map_err(|e| format!("bad id: {e}"))?,
        "name" => dock.name = unescape_field(val),
        "enabled" => dock.enabled = parse_bool(val),
        "monitor_index" => {
            dock.monitor_index = val.parse().map_err(|e| format!("bad monitor_index: {e}"))?;
        }
        "edge" => dock.edge = str_to_edge(val),
        "alignment" => dock.alignment = str_to_alignment(val),
        "edge_margin_px" => {
            dock.edge_margin_px = val.parse().map_err(|e| format!("bad edge_margin: {e}"))?;
        }
        "icon_size_px" => {
            dock.icon_size_px = val.parse().map_err(|e| format!("bad icon_size: {e}"))?;
        }
        "item_spacing_px" => {
            dock.item_spacing_px = val.parse().map_err(|e| format!("bad item_spacing: {e}"))?;
        }
        "padding_px" => {
            dock.padding_px = val.parse().map_err(|e| format!("bad padding: {e}"))?;
        }
        "corner_radius_px" => {
            dock.corner_radius_px = val.parse().map_err(|e| format!("bad radius: {e}"))?;
        }
        "show_settings_button" => dock.show_settings_button = parse_bool(val),
        "hover_zoom_enabled" => dock.hover_zoom.enabled = parse_bool(val),
        "hover_zoom_scale" => {
            let scale: f32 = val.parse().map_err(|e| format!("bad zoom_scale: {e}"))?;
            dock.hover_zoom = HoverZoomConfig {
                scale,
                mode: HoverZoomMode::SingleIcon,
                ..dock.hover_zoom
            };
        }
        "hover_zoom_duration_ms" => {
            dock.hover_zoom.duration_ms = val.parse().map_err(|e| format!("bad duration: {e}"))?;
        }
        "separator" => {
            let id = val.parse::<u64>().unwrap_or_else(|_| dock.next_item_id());
            dock.items.push(DockItem::separator(id));
        }
        "item" => dock.items.push(parse_item(val)?),
        _ => {}
    }
    Ok(())
}

fn parse_bool(val: &str) -> bool {
    val == "1" || val.eq_ignore_ascii_case("true")
}

fn serialize_item(item: &DockItem) -> String {
    let icon_part = match &item.icon {
        IconSource::AutoShell => "auto".to_owned(),
        IconSource::CustomFile(path) => {
            format!("custom:{}", escape_field(&path.to_string_lossy()))
        }
        IconSource::ExeResource { path, index } => {
            format!("res:{index}:{}", escape_field(&path.to_string_lossy()))
        }
    };
    format!(
        "{}|{}|{}|{}",
        item.id,
        escape_field(&item.label),
        escape_field(&item.target.to_string_lossy()),
        icon_part
    )
}

fn parse_item(val: &str) -> Result<DockItem, String> {
    let mut parts = val.splitn(4, '|');
    let id_str = parts.next().ok_or_else(|| "missing item id".to_owned())?;
    let label_str = parts
        .next()
        .ok_or_else(|| "missing item label".to_owned())?;
    let target_str = parts
        .next()
        .ok_or_else(|| "missing item target".to_owned())?;
    let icon_str = parts.next().unwrap_or("auto");

    let id = id_str
        .parse::<u64>()
        .map_err(|e| format!("bad item id: {e}"))?;
    let icon = parse_icon_source(icon_str);
    Ok(DockItem {
        id,
        kind: DockItemKind::Shortcut,
        label: unescape_field(label_str),
        target: PathBuf::from(unescape_field(target_str)),
        icon,
    })
}

fn parse_icon_source(raw: &str) -> IconSource {
    if let Some(custom) = raw.strip_prefix("custom:") {
        return IconSource::CustomFile(PathBuf::from(unescape_field(custom)));
    }
    if let Some(rest) = raw.strip_prefix("res:") {
        if let Some((idx_str, path_str)) = rest.split_once(':') {
            if let Ok(index) = idx_str.parse::<u32>() {
                return IconSource::ExeResource {
                    path: PathBuf::from(unescape_field(path_str)),
                    index,
                };
            }
        }
    }
    IconSource::AutoShell
}

fn edge_to_str(edge: DockEdge) -> &'static str {
    match edge {
        DockEdge::Left => "left",
        DockEdge::Right => "right",
        DockEdge::Top => "top",
        DockEdge::Bottom => "bottom",
    }
}

fn str_to_edge(val: &str) -> DockEdge {
    match val {
        "left" => DockEdge::Left,
        "right" => DockEdge::Right,
        "top" => DockEdge::Top,
        _ => DockEdge::Bottom,
    }
}

fn alignment_to_str(alignment: DockAlignment) -> String {
    match alignment {
        DockAlignment::Center => "center".to_owned(),
        DockAlignment::Offset(px) => format!("offset:{px}"),
    }
}

fn str_to_alignment(val: &str) -> DockAlignment {
    if let Some(raw) = val.strip_prefix("offset:") {
        if let Ok(px) = raw.parse::<i32>() {
            return DockAlignment::Offset(px);
        }
    }
    DockAlignment::Center
}

fn escape_field(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '|' => out.push_str("%7C"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            _ => out.push(ch),
        }
    }
    out
}

fn unescape_field(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex, 16) {
                    out.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn roundtrip_preserves_separators_settings_button_and_icons() -> Result<(), Box<dyn Error>> {
        let mut first = DockConfig::new(1, "Основная | Панель");
        first.show_settings_button = false;
        first.items.push(DockItem {
            id: 1,
            kind: DockItemKind::Shortcut,
            label: "Browser".to_owned(),
            target: PathBuf::from("C:\\Program Files\\App\\app.exe"),
            icon: IconSource::AutoShell,
        });
        first.items.push(DockItem::separator(2));
        first.items.push(DockItem {
            id: 3,
            kind: DockItemKind::Shortcut,
            label: "Notes".to_owned(),
            target: PathBuf::from("C:\\Apps\\Notes.lnk"),
            icon: IconSource::CustomFile(PathBuf::from("C:\\Icons\\notes 100%.png")),
        });

        let serialized = format!("\u{FEFF}{}", serialize_docks(&[first.clone()], "0.0.1"));
        let parsed = parse_docks(&serialized)?;
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], first);
        Ok(())
    }
}
