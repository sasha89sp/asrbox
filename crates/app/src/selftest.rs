use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use asrbox_core::{
    decode_supervisor_cmd, decode_worker_msg, encode_supervisor_cmd, encode_worker_msg,
    parse_modules_config, serialize_modules_config, IsolationMode, ModuleId, ModulesConfig,
    SupervisorCmd, WorkerMsg,
};
use asrbox_mod_about::{build_about_text, AppPaths};
use asrbox_mod_backup::{parse_backup_config, serialize_backup_config, BackupConfig};
use asrbox_mod_debug::{DebugHub, LogLevel};
use asrbox_mod_dock::{
    parse_docks, render_dock_widget_frame, serialize_docks, DockConfig, DockInstance, DockItem,
    DockLabelMode, DockRenderOverlay, Rect,
};
use asrbox_mod_overlay::{
    parse_overlay_config, render_game_overlay, serialize_overlay_config, OverlayAnchor,
    OverlayConfig,
};
use asrbox_mod_settings::{
    AboutSettingsView, BackupSettingsView, CalendarSettingsView, DebugSettingsView, DockItemView,
    DockSettingsView, ModuleStatusView, OverlaySettingsView, SettingsSnapshot, SettingsTab,
    SettingsUiState, SysMonitorSettingsView, WebDavSettingsView,
};
use asrbox_mod_webdav::{
    base64_encode, parse_webdav_config, serialize_webdav_config, validate_https_url, WebDavConfig,
    WebDavProvider,
};
use asrbox_mod_widgets::{
    parse_ics_bytes, parse_sysmon_config, parse_widget_config, render_calendar_widget,
    render_sysmon_widget, serialize_sysmon_config, serialize_widget_config, CalendarWidgetConfig,
    DateFormatMode, DisplayRow, IcsDateTime, ParseOptions, PeriodFilter, RenderView, SysHitTarget,
    SysMonitorConfig, SysRenderInput, WidgetHitTarget,
};
use asrbox_platform_api::{
    LocalDateTime, OverlayTelemetry, PixelRect, ProcessMetrics, SysTelemetry, WidgetLayer,
};

pub fn run_selftest(app_version: &str) -> Result<(), String> {
    selftest_core()?;
    selftest_dock(app_version)?;
    selftest_widgets()?;
    selftest_overlay()?;
    selftest_settings(app_version)?;
    selftest_backup_and_webdav()?;
    selftest_debug_and_about(app_version)?;
    Ok(())
}

fn selftest_core() -> Result<(), String> {
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
    Ok(())
}

fn selftest_dock(app_version: &str) -> Result<(), String> {
    let mut cfg = DockConfig::new(1, "Selftest Dock");
    cfg.locked = false;
    cfg.opacity_percent = 65;
    cfg.icon_size_px = 54;
    cfg.layer = WidgetLayer::Wallpaper;
    cfg.label_mode = DockLabelMode::IconAndLabel;
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
        || parsed[0].layer != WidgetLayer::Wallpaper
        || parsed[0].label_mode != DockLabelMode::IconAndLabel
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
    let commented = "# calendar=main|auto|https://example.com/cal.ics\nvisible=1\n";
    let parsed_hint = parse_widget_config(commented);
    if !parsed_hint.commented_calendar_hint {
        return Err("commented_calendar_hint detection failed".to_owned());
    }
    let rows = vec![DisplayRow {
        title: cal.events[0].summary.clone(),
        subtitle: "Сегодня · 10:00".to_owned(),
        bar_rgb: [232, 122, 72],
        tooltip: "Тест".to_owned(),
        is_past: false,
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
    });
    if sframe.bgra.pixels.is_empty() || sframe.texts.is_empty() {
        return Err("sysmon widget frame render failed".to_owned());
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
    let rect = cfg.compute_rect(mon);
    if rect.x <= 0 || rect.y <= 0 || rect.width != 340 || rect.height != 196 {
        return Err("overlay corner layout mismatch".to_owned());
    }
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
    let frame = render_game_overlay(&cfg, &snap, true);
    if frame.bgra.pixels.is_empty() || !frame.texts.iter().any(|t| t.text == "2.4% PL") {
        return Err("overlay render selftest failed".to_owned());
    }
    Ok(())
}

fn selftest_settings(app_version: &str) -> Result<(), String> {
    let snap = SettingsSnapshot {
        isolation: IsolationMode::Process,
        autostart_enabled: false,
        modules: vec![ModuleStatusView {
            id: ModuleId::Dock,
            enabled: true,
            running: true,
            pid: Some(100),
            restarts: 0,
            note: String::new(),
        }],
        docks: vec![DockSettingsView {
            id: "1".to_owned(),
            edge_index: 0,
            layer: WidgetLayer::Wallpaper,
            icon_size_px: 48,
            opacity_percent: 85,
            locked: true,
            show_running_indicator: true,
            focus_running_window: true,
            label_mode_index: 2,
            item_count: 1,
            items: vec![DockItemView {
                id: 1,
                label: "App".to_owned(),
            }],
        }],
        calendar: CalendarSettingsView {
            visible: true,
            opacity_percent: 88,
            layer: WidgetLayer::Wallpaper,
            period_index: 2,
            refresh_interval_sec: 3600,
            width: 336,
            visible_rows: 8,
            commented_calendar_hint: true,
            sources: vec![],
        },
        sysmon: SysMonitorSettingsView {
            visible: true,
            opacity_percent: 88,
            layer: WidgetLayer::Wallpaper,
            refresh_interval_ms: 2000,
            width: 560,
            height: 492,
        },
        overlay: OverlaySettingsView {
            module_enabled: true,
            auto_games_only: true,
            force_topmost_fullscreen: true,
            setup_mode: false,
            anchor_index: 1,
            x: 24,
            y: 24,
            opacity_percent: 78,
            width: 320,
            height: 188,
            refresh_interval_ms: 1000,
        },
        backup: BackupSettingsView {
            auto_upload_cloud: false,
            last_backup_file: String::new(),
            last_backup_time: String::new(),
            cloud_status: String::new(),
            busy: false,
        },
        webdav: WebDavSettingsView {
            provider_index: 0,
            effective_url: "https://webdav.yandex.ru".to_owned(),
            username: String::new(),
            remote_dir: "/asrbox-backups".to_owned(),
            has_saved_password: false,
            status_text: String::new(),
        },
        debug: DebugSettingsView {
            verbose: false,
            file_log: false,
            summary_line: "ok".to_owned(),
        },
        about: AboutSettingsView {
            version: app_version.to_owned(),
            exe_path: "asrbox.exe".to_owned(),
            data_dir: "data".to_owned(),
            config_dir: "data/config".to_owned(),
        },
    };
    let mut ui = SettingsUiState::default();
    for tab in SettingsTab::ALL {
        ui.active_tab = tab;
        let (frame, _) = ui.render(&snap);
        if frame.bgra.pixels.is_empty() || frame.texts.is_empty() {
            return Err(format!("settings render failed on {tab:?}"));
        }
    }
    Ok(())
}

fn selftest_backup_and_webdav() -> Result<(), String> {
    let bcfg = BackupConfig {
        auto_upload_cloud: true,
        last_backup_file: "asrbox-settings-20261123-120000.zip".to_owned(),
        last_backup_time: "23.11.2026 12:00".to_owned(),
        last_cloud_status: "OK".to_owned(),
    };
    if parse_backup_config(&serialize_backup_config(&bcfg)) != bcfg {
        return Err("backup config roundtrip mismatch".to_owned());
    }
    let wcfg = WebDavConfig {
        provider: WebDavProvider::MailRu,
        custom_url: String::new(),
        username: "test@mail.ru".to_owned(),
        remote_dir: "/asrbox-backups".to_owned(),
    };
    if parse_webdav_config(&serialize_webdav_config(&wcfg)) != wcfg {
        return Err("webdav config roundtrip mismatch".to_owned());
    }
    validate_https_url(&wcfg.effective_url())?;
    if validate_https_url("http://insecure.local").is_ok() {
        return Err("webdav http rejection failed".to_owned());
    }
    if base64_encode(b"asrbox:ok") != "YXNyYm94Om9r" {
        return Err("webdav base64 selftest failed".to_owned());
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
