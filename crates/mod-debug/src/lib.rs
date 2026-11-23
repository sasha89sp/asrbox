use std::collections::{BTreeMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use asrbox_platform_api::{LocalDateTime, ProcessMetrics};

pub mod report;

pub use report::{format_full_report, format_live_logs, format_live_summary, redact_secrets};

const MAX_RING_ENTRIES: usize = 1500;
const MAX_LOG_FILE_BYTES: u64 = 2 * 1024 * 1024;
const FILTER_SLOTS: &[Option<&str>] = &[
    None,
    Some("app"),
    Some("dock"),
    Some("widgets"),
    Some("overlay"),
    Some("http"),
    Some("platform"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Trace,
}

impl LogLevel {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Trace => "TRACE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub seq: u64,
    pub timestamp: String,
    pub level: LogLevel,
    pub module: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleStats {
    pub events: u64,
    pub redraws: u64,
    pub errors: u64,
    pub total_frame_us: u64,
    pub max_frame_us: u64,
    pub last_event: String,
}

pub struct DebugHub {
    entries: VecDeque<LogEntry>,
    next_seq: u64,
    verbose_trace: bool,
    file_logging: bool,
    filter_index: usize,
    logs_dir: PathBuf,
    stats: BTreeMap<String, ModuleStats>,
    dirty: bool,
}

impl DebugHub {
    pub fn new(logs_dir: PathBuf) -> Self {
        let mut hub = Self {
            entries: VecDeque::with_capacity(256),
            next_seq: 1,
            verbose_trace: false,
            file_logging: false,
            filter_index: 0,
            logs_dir,
            stats: BTreeMap::new(),
            dirty: true,
        };
        for m in ["app", "dock", "widgets", "overlay", "http", "platform"] {
            hub.stats.insert(m.to_owned(), ModuleStats::default());
        }
        hub
    }

    pub fn verbose_trace(&self) -> bool {
        self.verbose_trace
    }

    pub fn file_logging(&self) -> bool {
        self.file_logging
    }

    pub fn filter_module(&self) -> Option<&'static str> {
        FILTER_SLOTS.get(self.filter_index).copied().flatten()
    }

    pub fn filter_label(&self) -> &'static str {
        self.filter_module().unwrap_or("Все")
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.entries
    }

    pub fn stats(&self) -> &BTreeMap<String, ModuleStats> {
        &self.stats
    }

    pub fn take_dirty(&mut self) -> bool {
        let d = self.dirty;
        self.dirty = false;
        d
    }

    pub fn toggle_verbose(&mut self, now: LocalDateTime) {
        self.verbose_trace = !self.verbose_trace;
        let state = if self.verbose_trace {
            "ВКЛ"
        } else {
            "ВЫКЛ"
        };
        self.log(
            now,
            LogLevel::Info,
            "debug",
            format!("Подробные логи (TRACE): {state}"),
        );
    }

    pub fn toggle_file_logging(&mut self, now: LocalDateTime) {
        self.file_logging = !self.file_logging;
        let state = if self.file_logging {
            "ВКЛ"
        } else {
            "ВЫКЛ"
        };
        let path = self.log_file_path();
        self.log(
            now,
            LogLevel::Info,
            "debug",
            format!("Запись в файл {state} ({})", path.display()),
        );
    }

    pub fn cycle_filter(&mut self) {
        self.filter_index = (self.filter_index + 1) % FILTER_SLOTS.len();
        self.dirty = true;
    }

    pub fn clear(&mut self, now: LocalDateTime) {
        self.entries.clear();
        self.log(now, LogLevel::Info, "debug", "Буфер логов очищен");
    }

    pub fn log(
        &mut self,
        now: LocalDateTime,
        level: LogLevel,
        module: &str,
        message: impl AsRef<str>,
    ) {
        if level == LogLevel::Trace && !self.verbose_trace {
            return;
        }
        let clean_msg = redact_secrets(message.as_ref());
        let timestamp = format!("{:02}:{:02}:{:02}", now.hour, now.minute, now.second);
        let entry = LogEntry {
            seq: self.next_seq,
            timestamp,
            level,
            module: module.to_owned(),
            message: clean_msg,
        };
        self.next_seq = self.next_seq.saturating_add(1);
        if level == LogLevel::Error {
            self.stats.entry(module.to_owned()).or_default().errors += 1;
        }
        if self.file_logging {
            self.append_entry_to_file(&entry);
        }
        if self.entries.len() >= MAX_RING_ENTRIES {
            let _ = self.entries.pop_front();
        }
        self.entries.push_back(entry);
        self.dirty = true;
    }

    pub fn record_event(&mut self, module: &str, label: &str) {
        let s = self.stats.entry(module.to_owned()).or_default();
        s.events = s.events.saturating_add(1);
        s.last_event = label.to_owned();
        self.dirty = true;
    }

    pub fn record_frame(&mut self, module: &str, elapsed_us: u64) {
        let s = self.stats.entry(module.to_owned()).or_default();
        s.redraws = s.redraws.saturating_add(1);
        s.total_frame_us = s.total_frame_us.saturating_add(elapsed_us);
        if elapsed_us > s.max_frame_us {
            s.max_frame_us = elapsed_us;
        }
        self.dirty = true;
    }

    pub fn render_live(&self, proc: ProcessMetrics) -> (String, String) {
        let summary = format_live_summary(proc, &self.stats, self.entries.len());
        let logs = format_live_logs(&self.entries, self.filter_module());
        (summary, logs)
    }

    pub fn build_report(&self, version: &str, data_dir: &Path, proc: ProcessMetrics) -> String {
        format_full_report(version, data_dir, proc, &self.stats, &self.entries)
    }

    pub fn save_report(
        &self,
        version: &str,
        data_dir: &Path,
        proc: ProcessMetrics,
    ) -> Result<PathBuf, String> {
        fs::create_dir_all(&self.logs_dir)
            .map_err(|e| format!("не удалось создать папку логов: {e}"))?;
        let path = self.logs_dir.join("diag-report.txt");
        let text = self.build_report(version, data_dir, proc);
        fs::write(&path, text).map_err(|e| format!("не удалось записать отчёт: {e}"))?;
        Ok(path)
    }

    pub fn log_file_path(&self) -> PathBuf {
        self.logs_dir.join("asrbox-debug.log")
    }

    fn append_entry_to_file(&self, entry: &LogEntry) {
        if fs::create_dir_all(&self.logs_dir).is_err() {
            return;
        }
        let path = self.log_file_path();
        if let Ok(meta) = fs::metadata(&path) {
            if meta.len() >= MAX_LOG_FILE_BYTES {
                let old = self.logs_dir.join("asrbox-debug.log.old");
                let _ = fs::remove_file(&old);
                let _ = fs::rename(&path, &old);
            }
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(
                file,
                "#{} [{}] [{}] [{}] {}",
                entry.seq,
                entry.timestamp,
                entry.level.tag(),
                entry.module,
                entry.message
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_time() -> LocalDateTime {
        LocalDateTime {
            year: 2026,
            month: 11,
            day: 23,
            hour: 12,
            minute: 30,
            second: 45,
            utc_offset_min: 180,
        }
    }

    #[test]
    fn verbose_toggle_and_secret_redaction_work() {
        let mut hub = DebugHub::new(PathBuf::from("data/logs"));
        hub.log(sample_time(), LogLevel::Trace, "dock", "hidden trace");
        assert!(hub.entries().is_empty());

        hub.toggle_verbose(sample_time());
        hub.log(
            sample_time(),
            LogLevel::Trace,
            "http",
            "GET https://calendar.google.com/calendar/ical/u%40g.com/private-abcd123456/basic.ics",
        );
        assert_eq!(hub.entries().len(), 2);
        let last = hub.entries().back();
        assert!(last.is_some_and(
            |e| e.message.contains("private-***") && !e.message.contains("abcd123456")
        ));
    }

    #[test]
    fn ring_buffer_caps_and_records_metrics() {
        let mut hub = DebugHub::new(PathBuf::from("data/logs"));
        for i in 0..(MAX_RING_ENTRIES + 50) {
            hub.log(sample_time(), LogLevel::Info, "app", format!("msg {i}"));
        }
        assert_eq!(hub.entries().len(), MAX_RING_ENTRIES);
        hub.record_event("widgets", "refresh");
        hub.record_frame("widgets", 1800);
        let (summary, _) = hub.render_live(ProcessMetrics {
            ram_kb: 12_288,
            cpu_tenths_pct: 5,
            gdi_handles: 24,
            user_handles: 12,
            uptime_sec: 95,
        });
        assert!(summary.contains("12.0 МБ"));
        assert!(summary.contains("widgets"));
    }
}
