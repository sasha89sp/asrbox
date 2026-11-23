use std::collections::HashMap;
use std::path::PathBuf;

use asrbox_mod_about::{build_about_text, AppPaths};
use asrbox_mod_debug::{DebugHub, LogLevel};
use asrbox_mod_dock::{
    parse_docks, render_dock_frame, serialize_docks, DockConfig, DockInstance, DockItem, Rect,
};
use asrbox_mod_widgets::{
    parse_ics_bytes, parse_widget_config, render_calendar_widget, serialize_widget_config,
    CalendarWidgetConfig, DateFormatMode, DisplayRow, IcsDateTime, ParseOptions, PeriodFilter,
    RenderView, WidgetHitTarget,
};
use asrbox_platform_api::{LocalDateTime, ProcessMetrics};

pub fn run_selftest(app_version: &str) -> Result<(), String> {
    selftest_dock(app_version)?;
    selftest_widgets()?;
    selftest_debug_and_about(app_version)?;
    Ok(())
}

fn selftest_dock(app_version: &str) -> Result<(), String> {
    let mut cfg = DockConfig::new(1, "Selftest Dock");
    cfg.items.push(DockItem::separator(1));
    let area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let dock = DockInstance::new(cfg.clone(), area).map_err(|e| e.to_string())?;
    let frame = render_dock_frame(dock.config(), dock.layout(), &HashMap::new());
    if frame.pixels.is_empty() {
        return Err("rendered dock frame is empty".to_owned());
    }
    let parsed = parse_docks(&serialize_docks(&[cfg], app_version))?;
    if parsed.len() != 1 || parsed[0].items.len() != 1 || !parsed[0].items[0].is_separator() {
        return Err("dock config roundtrip mismatch".to_owned());
    }
    Ok(())
}

fn selftest_widgets() -> Result<(), String> {
    let sample = concat!(
        "BEGIN:VCALENDAR\r\n",
        "BEGIN:VEVENT\r\nUID:st-1\r\nSUMMARY:Тест календаря\r\n",
        "DTSTART:20261123T070000Z\r\nDTEND:20261123T080000Z\r\n",
        "RRULE:FREQ=DAILY;COUNT=2\r\nEND:VEVENT\r\n",
        "END:VCALENDAR\r\n",
    );
    let opts = ParseOptions {
        window_start_min: IcsDateTime::date(2026, 11, 22).to_epoch_min(),
        window_end_min: IcsDateTime::date(2026, 11, 30).to_epoch_min(),
        local_utc_offset_min: 180,
    };
    let cal = parse_ics_bytes(sample.as_bytes(), "selftest", None, opts);
    if cal.events.len() != 2 {
        return Err("widget ICS selftest failed".to_owned());
    }
    let rows = vec![DisplayRow {
        title: cal.events[0].summary.clone(),
        subtitle: "Сегодня · 10:00".to_owned(),
        bar_rgb: [52, 211, 153],
        tooltip: "Тест".to_owned(),
    }];
    let view = RenderView {
        width: 336,
        visible_rows: 8,
        opacity_percent: 88,
        title: "Google Календарь",
        period: PeriodFilter::Days30,
        date_format: DateFormatMode::Normal,
        rows: &rows,
        hovered: WidgetHitTarget::None,
        footer_text: "Обн. 10:00 · 1–1 из 1",
        footer_error: false,
        can_scroll_up: false,
        can_scroll_down: false,
        is_loading: false,
    };
    let frame = render_calendar_widget(&view);
    if frame.bgra.pixels.is_empty() || frame.texts.is_empty() {
        return Err("widget frame render failed".to_owned());
    }
    let cfg = CalendarWidgetConfig::default();
    if parse_widget_config(&serialize_widget_config(&cfg)) != cfg {
        return Err("widget config roundtrip mismatch".to_owned());
    }
    Ok(())
}

fn selftest_debug_and_about(app_version: &str) -> Result<(), String> {
    let mut hub = DebugHub::new(PathBuf::from("data/logs"));
    let now = LocalDateTime {
        year: 2026,
        month: 11,
        day: 23,
        hour: 12,
        minute: 0,
        second: 0,
        utc_offset_min: 180,
    };
    hub.log(
        now,
        LogLevel::Info,
        "http",
        "GET https://example.com/private-secret999/basic.ics",
    );
    let report = hub.build_report(
        app_version,
        &PathBuf::from("data"),
        ProcessMetrics::default(),
    );
    if !report.contains("private-***") || report.contains("secret999") {
        return Err("debug secret redaction selftest failed".to_owned());
    }
    let paths = AppPaths::from_exe_and_data(PathBuf::from("asrbox.exe"), PathBuf::from("data"));
    let about = build_about_text(app_version, &paths);
    if !about.contains(app_version) || !about.contains("ПОРТАТИВНАЯ СБОРКА") {
        return Err("about module selftest failed".to_owned());
    }
    Ok(())
}
