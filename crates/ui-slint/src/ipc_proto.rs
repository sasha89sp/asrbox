use asrbox_platform_api::{TrayMenuEntry, TrayMenuVisibility, TrayModuleFlags};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiModuleStatus {
    pub id: String,
    pub enabled: bool,
    pub running: bool,
    pub status_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiRuntimeStatus {
    pub autostart: bool,
    pub has_zip_pw: bool,
    pub has_gh_tok: bool,
    pub has_ya_key: bool,
    pub overlay_setup: bool,
    pub diag_verbose: bool,
    pub diag_file_log: bool,
    pub backup_busy: bool,
    pub diag_summary: String,
    pub status_banner: String,
    pub modules: Vec<UiModuleStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayShowPayload {
    pub x: i32,
    pub y: i32,
    pub flags: TrayModuleFlags,
    pub docks: Vec<TrayMenuEntry>,
}

pub fn encode_ui_cmd(action: &str, arg: &str) -> String {
    format!("UI_CMD|{}|{}", escape_field(action), escape_field(arg))
}

pub fn decode_ui_cmd(line: &str) -> Option<(String, String)> {
    let rest = line.trim().strip_prefix("UI_CMD|")?;
    let (act, arg) = rest.split_once('|')?;
    Some((unescape_field(act), unescape_field(arg)))
}

pub fn encode_tray_show(p: &TrayShowPayload) -> String {
    let b = |v: bool| if v { '1' } else { '0' };
    let f = &p.flags;
    let mv = &f.menu_vis;
    let bits = format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        b(f.dock_module_enabled),
        b(f.widgets_module_enabled),
        b(f.overlay_module_enabled),
        b(f.overlay_setup_mode),
        b(f.calendar_visible),
        b(f.sys_monitor_visible),
        b(f.weather_visible),
        b(f.notes_visible),
        b(f.autostart_enabled),
        b(mv.show_autostart),
        b(mv.show_docks),
        b(mv.show_widgets),
        b(mv.show_overlay),
        b(mv.show_backup),
        b(mv.show_debug),
        b(mv.show_about),
    );
    let docks: Vec<String> = p
        .docks
        .iter()
        .map(|d| format!("{}~{}~{}", d.dock_id, b(d.enabled), escape_sub(&d.title)))
        .collect();
    format!(
        "SHOW|{}|{}|{}|{}",
        p.x,
        p.y,
        bits,
        escape_field(&docks.join(";"))
    )
}

pub fn decode_tray_show(line: &str) -> Option<TrayShowPayload> {
    let rest = line.trim().strip_prefix("SHOW|")?;
    let cols: Vec<&str> = rest.splitn(4, '|').collect();
    if cols.len() != 4 {
        return None;
    }
    let x = cols[0].parse::<i32>().ok()?;
    let y = cols[1].parse::<i32>().ok()?;
    let bits: Vec<u8> = cols[2].bytes().collect();
    if bits.len() != 16 {
        return None;
    }
    let at = |i: usize| bits.get(i).copied() == Some(b'1');
    let flags = TrayModuleFlags {
        dock_module_enabled: at(0),
        widgets_module_enabled: at(1),
        overlay_module_enabled: at(2),
        overlay_setup_mode: at(3),
        calendar_visible: at(4),
        sys_monitor_visible: at(5),
        weather_visible: at(6),
        notes_visible: at(7),
        autostart_enabled: at(8),
        menu_vis: TrayMenuVisibility {
            show_autostart: at(9),
            show_docks: at(10),
            show_widgets: at(11),
            show_overlay: at(12),
            show_backup: at(13),
            show_debug: at(14),
            show_about: at(15),
        },
    };
    let mut docks = Vec::new();
    for entry in unescape_field(cols[3]).split(';').filter(|s| !s.is_empty()) {
        let mut p = entry.splitn(3, '~');
        if let (Some(id_s), Some(en_s), Some(t_s)) = (p.next(), p.next(), p.next()) {
            if let Ok(dock_id) = id_s.parse::<u64>() {
                docks.push(TrayMenuEntry {
                    dock_id,
                    title: unescape_sub(t_s),
                    enabled: en_s == "1",
                });
            }
        }
    }
    Some(TrayShowPayload { x, y, flags, docks })
}

pub fn encode_runtime_status(st: &UiRuntimeStatus) -> String {
    let b = |v: bool| if v { '1' } else { '0' };
    let mods: Vec<String> = st
        .modules
        .iter()
        .map(|m| {
            format!(
                "{}~{}~{}~{}",
                escape_sub(&m.id),
                b(m.enabled),
                b(m.running),
                escape_sub(&m.status_text)
            )
        })
        .collect();
    format!(
        "STATUS|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        b(st.autostart),
        b(st.has_zip_pw),
        b(st.has_gh_tok),
        b(st.has_ya_key),
        b(st.overlay_setup),
        b(st.diag_verbose),
        b(st.diag_file_log),
        b(st.backup_busy),
        escape_field(&st.diag_summary),
        escape_field(&st.status_banner),
        escape_field(&mods.join(";")),
    )
}

pub fn decode_runtime_status(line: &str) -> Option<UiRuntimeStatus> {
    let rest = line.trim().strip_prefix("STATUS|")?;
    let cols: Vec<&str> = rest.split('|').collect();
    if cols.len() != 11 {
        return None;
    }
    let flag = |s: &str| s == "1";
    let mods_raw = unescape_field(cols[10]);
    let mut modules = Vec::new();
    for entry in mods_raw.split(';').filter(|s| !s.is_empty()) {
        let mut p = entry.splitn(4, '~');
        if let (Some(id), Some(en), Some(run), Some(txt)) = (p.next(), p.next(), p.next(), p.next())
        {
            modules.push(UiModuleStatus {
                id: unescape_sub(id),
                enabled: en == "1",
                running: run == "1",
                status_text: unescape_sub(txt),
            });
        }
    }
    Some(UiRuntimeStatus {
        autostart: flag(cols[0]),
        has_zip_pw: flag(cols[1]),
        has_gh_tok: flag(cols[2]),
        has_ya_key: flag(cols[3]),
        overlay_setup: flag(cols[4]),
        diag_verbose: flag(cols[5]),
        diag_file_log: flag(cols[6]),
        backup_busy: flag(cols[7]),
        diag_summary: unescape_field(cols[8]),
        status_banner: unescape_field(cols[9]),
        modules,
    })
}

pub fn escape_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\p"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

pub fn unescape_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('p') => out.push('|'),
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn escape_sub(s: &str) -> String {
    s.replace('%', "%25")
        .replace('~', "%7E")
        .replace(';', "%3B")
}

fn unescape_sub(s: &str) -> String {
    s.replace("%3B", ";")
        .replace("%7E", "~")
        .replace("%25", "%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_cmd_status_and_tray_roundtrip() {
        let encoded = encode_ui_cmd("save_gh", "owner/repo|main|tok\\1");
        assert_eq!(
            decode_ui_cmd(&encoded),
            Some(("save_gh".to_owned(), "owner/repo|main|tok\\1".to_owned()))
        );
        let st = UiRuntimeStatus {
            autostart: true,
            has_zip_pw: true,
            has_ya_key: true,
            diag_verbose: true,
            diag_summary: "RAM 18.2 МБ | CPU 0.1%".to_owned(),
            status_banner: "OK".to_owned(),
            modules: vec![UiModuleStatus {
                id: "dock".to_owned(),
                enabled: true,
                running: true,
                status_text: "PID 1234 · работает".to_owned(),
            }],
            ..UiRuntimeStatus::default()
        };
        assert_eq!(decode_runtime_status(&encode_runtime_status(&st)), Some(st));
        let tray = TrayShowPayload {
            x: 1620,
            y: 980,
            flags: TrayModuleFlags {
                dock_module_enabled: true,
                widgets_module_enabled: true,
                overlay_module_enabled: false,
                overlay_setup_mode: false,
                calendar_visible: true,
                sys_monitor_visible: false,
                weather_visible: true,
                notes_visible: true,
                autostart_enabled: true,
                menu_vis: TrayMenuVisibility::default(),
            },
            docks: vec![TrayMenuEntry {
                dock_id: 2,
                title: "Док | 2 ~ главный;тест".to_owned(),
                enabled: true,
            }],
        };
        assert_eq!(decode_tray_show(&encode_tray_show(&tray)), Some(tray));
    }
}
