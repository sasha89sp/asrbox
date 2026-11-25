use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use asrbox_core::{
    decode_supervisor_cmd, decode_worker_msg, encode_supervisor_cmd, encode_worker_msg,
    parse_modules_config, serialize_modules_config, IsolationMode, ModulesConfig, SupervisorCmd,
    WorkerMsg,
};
use asrbox_mod_dock::{
    parse_docks, render_dock_widget_frame, serialize_docks, DockConfig, DockInstance, DockItem,
    DockLabelMode, DockRenderOverlay, Rect,
};
use asrbox_mod_overlay::{
    parse_overlay_config, render_game_overlay, serialize_overlay_config, OverlayAnchor,
    OverlayConfig,
};
use asrbox_mod_widgets::{
    parse_ics_bytes, parse_notes_config, parse_notes_db, parse_open_meteo_forecast,
    parse_sysmon_config, parse_weather_config, parse_widget_config, parse_yandex_forecast,
    render_calendar_widget, render_notes_widget, render_sysmon_widget, render_weather_widget,
    serialize_notes_config, serialize_notes_db, serialize_sysmon_config, serialize_weather_config,
    serialize_widget_config, CalendarWidgetConfig, DateFormatMode, DisplayRow, IcsDateTime,
    InlineEditMode, NotesDb, NotesRenderInput, NotesWidgetConfig, ParseOptions, PeriodFilter,
    RenderView, SysHitTarget, SysMonitorConfig, SysRenderInput, WeatherData, WeatherHitTarget,
    WeatherProvider, WeatherRenderInput, WeatherWidgetConfig, WidgetHitTarget,
};
use asrbox_platform_api::{
    draw_asr_glass_logo, parse_appearance_config, parse_menu_config, serialize_appearance_config,
    serialize_menu_config, AccentPreset, AppLanguage, AppearanceConfig, BgraFrame,
    ContextMenuConfig, DesignTheme, FluentPalette, FontRenderer, LocalDateTime, OverlayTelemetry,
    PixelRect, SysTelemetry, ThemeMode, WidgetLayer,
};

use crate::selftest_tools::{
    selftest_backup_and_webdav, selftest_debug_and_about, selftest_settings,
};

pub fn run_selftest(app_version: &str) -> Result<(), String> {
    selftest_core_and_theme()?;
    selftest_dock(app_version)?;
    selftest_widgets()?;
    selftest_overlay()?;
    selftest_settings(app_version)?;
    selftest_backup_and_webdav()?;
    selftest_debug_and_about(app_version)?;
    Ok(())
}

fn selftest_core_and_theme() -> Result<(), String> {
    let cfg = ModulesConfig {
        isolation: IsolationMode::Process,
        dock_enabled: true,
        widgets_enabled: false,
        overlay_enabled: true,
    };
    if parse_modules_config(&serialize_modules_config(&cfg)) != cfg {
        return Err("modules.cfg roundtrip mismatch".to_owned());
    }
    let cmd = SupervisorCmd::SetOverlaySetupMode(true);
    if decode_supervisor_cmd(&encode_supervisor_cmd(&cmd)) != Some(cmd) {
        return Err("supervisor IPC roundtrip mismatch".to_owned());
    }
    let msg = WorkerMsg::OpenSettings {
        tab: "overlay".to_owned(),
    };
    if decode_worker_msg(&encode_worker_msg(&msg)) != Some(msg) {
        return Err("worker IPC roundtrip mismatch".to_owned());
    }
    let app_cfg = AppearanceConfig {
        design_theme: DesignTheme::AsrGlass,
        theme: ThemeMode::Light,
        accent: AccentPreset::Emerald,
        language: AppLanguage::En,
        font_renderer: FontRenderer::DirectWrite,
    };
    if parse_appearance_config(&serialize_appearance_config(&app_cfg)) != app_cfg {
        return Err("appearance.cfg roundtrip mismatch".to_owned());
    }
    let mcfg = ContextMenuConfig::default();
    if parse_menu_config(&serialize_menu_config(&mcfg)) != mcfg {
        return Err("menus.cfg roundtrip mismatch".to_owned());
    }
    let mut logo_frame = BgraFrame::transparent(48, 48);
    draw_asr_glass_logo(&mut logo_frame, 0, 0, 48, true, true);
    if !logo_frame.pixels.iter().any(|&b| b > 0) {
        return Err("ASR Glass logo render selftest failed".to_owned());
    }
    let dark = FluentPalette::from_theme(ThemeMode::Dark, AccentPreset::Blue);
    let light = FluentPalette::from_theme(ThemeMode::Light, AccentPreset::Blue);
    let mid = FluentPalette::lerp(dark, light, 0.5);
    if mid.window_bg[0] <= dark.window_bg[0] || mid.window_bg[0] >= light.window_bg[0] {
        return Err("fluent palette lerp selftest failed".to_owned());
    }
    Ok(())
}

fn selftest_dock(app_version: &str) -> Result<(), String> {
    let mut cfg = DockConfig::new(1, "Selftest Dock");
    cfg.locked = false;
    cfg.lock_items = true;
    cfg.opacity_percent = 65;
    cfg.icon_size_px = 54;
    cfg.layer = WidgetLayer::Wallpaper;
    cfg.label_mode = DockLabelMode::IconAndLabel;
    cfg.launch_animation_enabled = false;
    let item =
        DockItem::from_dropped_path(1, PathBuf::from("C:\\app.exe")).map_err(|e| e.to_string())?;
    cfg.items.push(item);
    cfg.items.push(DockItem::separator(2));
    let area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let dock = DockInstance::new(cfg.clone(), area).map_err(|e| e.to_string())?;
    let ov = DockRenderOverlay {
        dragged_item_id: None,
        insert_before_index: Some(0),
        hovered_item_id: Some(1),
        pressed_item_id: Some(1),
        palette: FluentPalette::default(),
    };
    let frame = render_dock_widget_frame(
        dock.config(),
        dock.layout(),
        &HashMap::new(),
        &HashSet::new(),
        ov,
    );
    if frame.bgra.pixels.is_empty() || frame.texts.is_empty() {
        return Err("rendered dock widget frame is empty".to_owned());
    }
    let parsed = parse_docks(&serialize_docks(&[cfg], app_version))?;
    if parsed.len() != 1
        || parsed[0].opacity_percent != 65
        || parsed[0].locked
        || !parsed[0].lock_items
        || parsed[0].layer != WidgetLayer::Wallpaper
        || parsed[0].label_mode != DockLabelMode::IconAndLabel
        || parsed[0].launch_animation_enabled
    {
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
        bar_rgb: [232, 122, 72],
        tooltip: "Тест".to_owned(),
        is_past: false,
    }];
    let palette = FluentPalette::default();
    let frame = render_calendar_widget(&RenderView {
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
        palette,
    });
    if frame.bgra.pixels.is_empty() || frame.texts.is_empty() {
        return Err("widget frame render failed".to_owned());
    }
    let cfg = CalendarWidgetConfig::default();
    if parse_widget_config(&serialize_widget_config(&cfg)) != cfg {
        return Err("widget config roundtrip mismatch".to_owned());
    }
    let sys_cfg = SysMonitorConfig::default();
    if parse_sysmon_config(&serialize_sysmon_config(&sys_cfg)) != sys_cfg {
        return Err("sysmon config roundtrip mismatch".to_owned());
    }
    let now = LocalDateTime {
        year: 2026,
        month: 11,
        day: 23,
        hour: 4,
        minute: 20,
        second: 0,
        utc_offset_min: 180,
    };
    let sframe = render_sysmon_widget(&SysRenderInput {
        width: sys_cfg.width,
        height: sys_cfg.height,
        opacity_percent: sys_cfg.opacity_percent,
        refresh_ms: sys_cfg.refresh_interval_ms,
        now,
        data: &SysTelemetry::default(),
        hovered: SysHitTarget::None,
        palette,
    });
    if sframe.bgra.pixels.is_empty() || sframe.texts.is_empty() {
        return Err("sysmon widget frame render failed".to_owned());
    }
    let w_cfg = WeatherWidgetConfig::default();
    if parse_weather_config(&serialize_weather_config(&w_cfg)) != w_cfg {
        return Err("weather config roundtrip mismatch".to_owned());
    }
    let sample_meteo = r#"{"current":{"temperature_2m":4.1,"apparent_temperature":1.2,"relative_humidity_2m":76,"surface_pressure":1002.6,"wind_speed_10m":3.4,"weather_code":2},"hourly":{"time":["2026-11-25T12:00"],"temperature_2m":[4.0],"weather_code":[2]},"daily":{"time":["2026-11-25"],"weather_code":[2],"temperature_2m_min":[0.0],"temperature_2m_max":[4.0]}}"#;
    let wdata = parse_open_meteo_forecast(sample_meteo.as_bytes(), 12, "12:00".to_owned())
        .unwrap_or_else(|_| WeatherData::default());
    let wframe = render_weather_widget(&WeatherRenderInput {
        width: w_cfg.width,
        height: w_cfg.height,
        opacity_percent: w_cfg.opacity_percent,
        city: &w_cfg.city,
        provider: WeatherProvider::OpenMeteo,
        forecast_days: 7,
        now,
        data: &wdata,
        loading: false,
        error: None,
        hovered: WeatherHitTarget::None,
        palette,
    });
    if wframe.bgra.pixels.is_empty() || wframe.texts.is_empty() {
        return Err("weather widget frame render failed".to_owned());
    }
    let sample_ya = r#"{"fact":{"temp":3,"feels_like":0,"condition":"partly-cloudy","wind_speed":3.2,"pressure_mm":750,"humidity":74},"forecasts":[{"date":"2026-11-25","parts":{"day_short":{"temp":4,"temp_min":-1,"condition":"cloudy"}},"hours":[{"hour":"12","temp":3,"condition":"cloudy"}]}]}"#;
    if parse_yandex_forecast(sample_ya.as_bytes(), 12, "12:00".to_owned()).is_err() {
        return Err("yandex weather parser selftest failed".to_owned());
    }
    let n_cfg = NotesWidgetConfig::default();
    if parse_notes_config(&serialize_notes_config(&n_cfg)) != n_cfg {
        return Err("notes config roundtrip mismatch".to_owned());
    }
    let db = NotesDb::default_seeded();
    if parse_notes_db(&serialize_notes_db(&db)) != db {
        return Err("notes db roundtrip mismatch".to_owned());
    }
    let nframe = render_notes_widget(&NotesRenderInput {
        cfg: &n_cfg,
        db: &db,
        search_query: "",
        hovered: None,
        list_scroll: 0,
        inline_mode: InlineEditMode::None,
        palette,
    });
    if nframe.frame.bgra.pixels.is_empty() || nframe.frame.texts.is_empty() {
        return Err("notes widget frame render failed".to_owned());
    }
    Ok(())
}

fn selftest_overlay() -> Result<(), String> {
    let cfg = OverlayConfig {
        force_topmost_fullscreen: false,
        anchor: OverlayAnchor::BottomRight,
        width: 340,
        height: 196,
        opacity_percent: 76,
        extra_games: vec!["cyberpunk2077.exe".to_owned()],
        ..OverlayConfig::default()
    };
    if parse_overlay_config(&serialize_overlay_config(&cfg)) != cfg {
        return Err("overlay config roundtrip mismatch".to_owned());
    }
    let mon = PixelRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let snap = OverlayTelemetry {
        fps: Some(144),
        fps_low_1pct: Some(112),
        display_hz: 144,
        gpu_temp_c: Some(65),
        gpu_usage_pct: Some(98),
        cpu_usage_pct: 42,
        net_ping_ms: Some(148),
        net_loss_tenths_pct: 24,
        game_active: true,
        monitor_rect: Some(mon),
        ..OverlayTelemetry::default()
    };
    let frame = render_game_overlay(&cfg, &snap, true, &FluentPalette::default());
    if frame.bgra.pixels.is_empty() || !frame.texts.iter().any(|t| t.text == "2.4% PL") {
        return Err("overlay render selftest failed".to_owned());
    }
    Ok(())
}
