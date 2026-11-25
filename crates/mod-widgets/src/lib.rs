pub mod calendar;
pub mod calendar_view;
pub mod date;
pub mod draw;
pub mod format;
pub mod ics;
pub mod ics_time;
mod notes_menu;
pub mod notes_model;
mod notes_panes;
pub mod notes_render;
pub mod notes_storage;
pub mod notes_widget;
pub mod render;
pub mod rrule;
pub mod rrule_parse;
pub mod storage;
pub mod sys_config;
pub mod sys_render;
mod sys_sections;
pub mod sys_widget;
pub mod weather_api;
pub mod weather_config;
pub mod weather_render;
pub mod weather_widget;

pub use calendar::{CalendarWidget, CALENDAR_WIDGET_ID};
pub use calendar_view::uncomment_calendar_sources;
pub use date::IcsDateTime;
pub use ics::{parse_ics_bytes, CalendarEvent, ParseOptions, ParsedCalendar};
pub use notes_model::{
    FolderNode, NoteDocument, NotesDb, NotesWidgetConfig, TaskItem, MAX_NOTES_HEIGHT,
    MAX_NOTES_WIDTH, MIN_NOTES_HEIGHT, MIN_NOTES_WIDTH,
};
pub use notes_render::{
    hit_test_notes, render_notes_widget, InlineEditMode, NotesHitBox, NotesHitTarget,
    NotesRenderInput, NotesRenderOutput, NOTES_TOP_BAR_H,
};
pub use notes_storage::{
    load_notes_config, load_notes_db, parse_notes_config, parse_notes_db, save_notes_config,
    save_notes_db, serialize_notes_config, serialize_notes_db,
};
pub use notes_widget::{NotesWidget, NOTES_WIDGET_ID};
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
pub use weather_api::{
    build_forecast_url, build_geocode_url, parse_open_meteo_forecast, parse_open_meteo_geocode,
    wmo_description_ru, wmo_icon_kind, DailySlot, GeoMatch, HourlySlot, WeatherData,
    WeatherIconKind,
};
pub use weather_config::{
    load_weather_config, parse_weather_config, save_weather_config, serialize_weather_config,
    WeatherWidgetConfig, MAX_WEATHER_HEIGHT, MAX_WEATHER_WIDTH, MIN_WEATHER_HEIGHT,
    MIN_WEATHER_WIDTH,
};
pub use weather_render::{
    hit_test_weather, render_weather_widget, WeatherHitTarget, WeatherRenderInput, WEATHER_HEADER_H,
};
pub use weather_widget::{WeatherWidget, WEATHER_WIDGET_ID};
