use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::Path;

use asrbox_platform_api::ProcessMetrics;

use crate::{LogEntry, ModuleStats};

pub fn redact_secrets(input: &str) -> String {
    let mut out = mask_after_marker(input, "private-", |c| c == '/' || c.is_whitespace());
    for key in ["token=", "key=", "password=", "secret="] {
        out = mask_after_marker(&out, key, |c| c == '&' || c.is_whitespace() || c == '"');
    }
    out
}

fn mask_after_marker<F: Fn(char) -> bool>(input: &str, marker: &str, is_end: F) -> String {
    let mut result = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(idx) = rest.find(marker) {
        let after = idx + marker.len();
        result.push_str(&rest[..after]);
        let tail = &rest[after..];
        let end_len = tail.find(&is_end).unwrap_or(tail.len());
        if end_len > 0 {
            result.push_str("***");
        }
        rest = &tail[end_len..];
    }
    result.push_str(rest);
    result
}

pub fn format_live_summary(
    proc: ProcessMetrics,
    stats: &BTreeMap<String, ModuleStats>,
    log_count: usize,
) -> String {
    let ram_mb = (proc.ram_kb as f64) / 1024.0;
    let cpu_pct = (proc.cpu_tenths_pct as f64) / 10.0;
    let up_m = proc.uptime_sec / 60;
    let up_s = proc.uptime_sec % 60;
    let mut out = format!(
        "Процесс: RAM {ram_mb:.1} МБ | CPU {cpu_pct:.1}% | GDI {} | USER {} | Аптайм {up_m}м {up_s:02}с | Записей: {log_count}\r\n",
        proc.gdi_handles, proc.user_handles
    );
    out.push_str(
        "Модуль    | Событий | Кадров | Ошибок | Ср.кадр | Макс   | Последнее событие\r\n",
    );
    for (name, st) in stats {
        let avg_ms = if st.redraws > 0 {
            (st.total_frame_us as f64) / (st.redraws as f64) / 1000.0
        } else {
            0.0
        };
        let max_ms = (st.max_frame_us as f64) / 1000.0;
        let last = if st.last_event.is_empty() {
            "-"
        } else {
            &st.last_event
        };
        out.push_str(&format!(
            "{name:<9} | {:>7} | {:>6} | {:>6} | {:>5.2}мс | {:>4.1}мс | {last}\r\n",
            st.events, st.redraws, st.errors, avg_ms, max_ms
        ));
    }
    out
}

pub fn format_live_logs(entries: &VecDeque<LogEntry>, module_filter: Option<&str>) -> String {
    let mut out = String::new();
    let filtered: Vec<&LogEntry> = entries
        .iter()
        .filter(|e| module_filter.is_none_or(|m| e.module == m))
        .collect();
    let skip = filtered.len().saturating_sub(350);
    for e in filtered.into_iter().skip(skip) {
        out.push_str(&format!(
            "[{}] [{:<5}] [{:<8}] {}\r\n",
            e.timestamp,
            e.level.tag(),
            e.module,
            e.message
        ));
    }
    if out.is_empty() {
        out.push_str("Событий по выбранному фильтру пока нет.\r\n");
    }
    out
}

pub fn format_full_report(
    version: &str,
    data_dir: &Path,
    proc: ProcessMetrics,
    stats: &BTreeMap<String, ModuleStats>,
    entries: &VecDeque<LogEntry>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "=== ДИАГНОСТИЧЕСКИЙ ОТЧЁТ asrbox v{version} ===\r\n"
    ));
    out.push_str(&format!("Папка данных: {}\r\n\r\n", data_dir.display()));
    out.push_str(&format_live_summary(proc, stats, entries.len()));
    out.push_str("\r\n--- Конфиг виджетов (секреты замаскированы) ---\r\n");
    append_redacted_file(&mut out, &data_dir.join("config").join("widgets.cfg"));
    out.push_str("\r\n--- Конфиг док-панелей ---\r\n");
    append_redacted_file(&mut out, &data_dir.join("config").join("docks.cfg"));
    out.push_str("\r\n--- Последние логи ---\r\n");
    out.push_str(&format_live_logs(entries, None));
    out
}

fn append_redacted_file(out: &mut String, path: &Path) {
    match fs::read_to_string(path) {
        Ok(raw) => {
            for line in raw.lines() {
                out.push_str(&redact_secrets(line));
                out.push_str("\r\n");
            }
        }
        Err(_) => {
            out.push_str(&format!("(файл отсутствует: {})\r\n", path.display()));
        }
    }
}
