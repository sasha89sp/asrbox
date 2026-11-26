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

pub fn encode_ui_cmd(action: &str, arg: &str) -> String {
    format!("UI_CMD|{}|{}", escape_field(action), escape_field(arg))
}

pub fn decode_ui_cmd(line: &str) -> Option<(String, String)> {
    let rest = line.trim().strip_prefix("UI_CMD|")?;
    let (act, arg) = rest.split_once('|')?;
    Some((unescape_field(act), unescape_field(arg)))
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
        if let (Some(id), Some(en), Some(run), Some(txt)) =
            (p.next(), p.next(), p.next(), p.next())
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
    fn ui_cmd_and_status_roundtrip() {
        let encoded = encode_ui_cmd("save_gh", "owner/repo|main|tok\\1");
        assert_eq!(
            decode_ui_cmd(&encoded),
            Some(("save_gh".to_owned(), "owner/repo|main|tok\\1".to_owned()))
        );
        let st = UiRuntimeStatus {
            autostart: true,
            has_zip_pw: true,
            has_gh_tok: false,
            has_ya_key: true,
            overlay_setup: false,
            diag_verbose: true,
            diag_file_log: false,
            backup_busy: false,
            diag_summary: "RAM 18.2 МБ | CPU 0.1%".to_owned(),
            status_banner: "OK".to_owned(),
            modules: vec![UiModuleStatus {
                id: "dock".to_owned(),
                enabled: true,
                running: true,
                status_text: "PID 1234 · работает".to_owned(),
            }],
        };
        assert_eq!(decode_runtime_status(&encode_runtime_status(&st)), Some(st));
    }
}
