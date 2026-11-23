pub mod calendar;
pub mod calendar_view;
pub mod date;
pub mod draw;
pub mod format;
pub mod ics;
pub mod ics_time;
pub mod render;
pub mod rrule;
pub mod rrule_parse;
pub mod storage;
pub mod sys_config;
pub mod sys_render;
mod sys_sections;
pub mod sys_widget;

pub use calendar::{CalendarWidget, CALENDAR_WIDGET_ID};
pub use calendar_view::uncomment_calendar_sources;
pub use date::IcsDateTime;
pub use ics::{parse_ics_bytes, CalendarEvent, ParseOptions, ParsedCalendar};
pub use render::{
    hit_test_widget, render_calendar_widget, rows_for_height, widget_height, DisplayRow,
    RenderView, WidgetHitTarget, HEADER_HEIGHT,
};
pub use rrule::{expand_rrule, parse_rrule, ByDay, Freq, Rrule};
pub use storage::{
    cache_file_path, load_cached_ics, load_widget_config, parse_widget_config, save_cached_ics,
    save_widget_config, serialize_widget_config, CalendarSource, CalendarWidgetConfig,
    DateFormatMode, PeriodFilter,
};
pub use sys_config::{
    load_sysmon_config, parse_sysmon_config, save_sysmon_config, serialize_sysmon_config,
    SysMonitorConfig, MAX_REFRESH_MS, MAX_SYSMON_HEIGHT, MAX_SYSMON_WIDTH, MIN_REFRESH_MS,
    MIN_SYSMON_HEIGHT, MIN_SYSMON_WIDTH,
};
pub use sys_render::{
    hit_test_sysmon, render_sysmon_widget, SysHitTarget, SysRenderInput, SYSMON_HEADER_H,
};
pub use sys_widget::{SysMonitorWidget, SYS_WIDGET_ID};
