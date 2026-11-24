use std::path::PathBuf;

use asrbox_core::{IsolationMode, ModuleId};
use asrbox_mod_about::{build_about_text, changelog_text, AppPaths};
use asrbox_mod_backup::{parse_backup_config, serialize_backup_config, BackupConfig};
use asrbox_mod_debug::{DebugHub, LogLevel};
use asrbox_mod_settings::{
    AboutSettingsView, AppearanceSettingsView, BackupSettingsView, BrowserTab,
    CalendarSettingsView, DiagnosticsLiveView, DockItemView, DockSettingsView, ModuleStatusView,
    OverlaySettingsView, SettingsSnapshot, SettingsTab, SettingsUiState, SysMonitorSettingsView,
    WebDavBrowserView, WebDavServerView,
};
use asrbox_mod_webdav::{
    base64_encode, parse_propfind_xml, parse_sync_config, parse_webdav_store,
    serialize_sync_config, serialize_webdav_store, validate_https_url, FileSyncConfig,
    SyncInterval, SyncItem, WebDavProvider,
};
use asrbox_platform_api::{
    AccentPreset, FluentPalette, LocalDateTime, ProcessMetrics, ThemeMode, WebDavDirEntry,
    WidgetLayer,
};

pub fn selftest_settings(app_version: &str) -> Result<(), String> {
    let snap = sample_settings_snapshot(app_version);
    let mut ui = SettingsUiState::default();
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        ui.set_palette_immediate(FluentPalette::from_theme(theme, AccentPreset::Blue));
        for tab in SettingsTab::ALL {
            ui.active_tab = tab;
            ui.active_browser_tab = BrowserTab::Main;
            let (frame, _) = ui.render(&snap);
            if frame.bgra.pixels.is_empty() || frame.texts.is_empty() {
                return Err(format!("settings render failed on {tab:?}"));
            }
        }
    }
    for btab in [
        BrowserTab::Diagnostics,
        BrowserTab::VersionHistory,
        BrowserTab::WebDavBrowser {
            server_id: "srv-1".to_owned(),
            server_name: "Mail.ru".to_owned(),
        },
    ] {
        ui.open_browser_tab(btab);
        let (frame, _) = ui.render(&snap);
        if frame.bgra.pixels.is_empty() || frame.texts.is_empty() {
            return Err("settings browser tab render failed".to_owned());
        }
    }
    Ok(())
}

fn sample_settings_snapshot(app_version: &str) -> SettingsSnapshot {
    SettingsSnapshot {
        appearance: AppearanceSettingsView::default(),
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
            servers: vec![WebDavServerView {
                id: "srv-1".to_owned(),
                name: "Mail.ru".to_owned(),
                provider_label: "Облако Mail.ru".to_owned(),
                effective_url: "https://webdav.cloud.mail.ru".to_owned(),
                username: "test@mail.ru".to_owned(),
                remote_dir: "/asrbox-backups".to_owned(),
                has_saved_password: true,
                status_text: "OK".to_owned(),
            }],
            browser: Some(WebDavBrowserView {
                server_id: "srv-1".to_owned(),
                server_name: "Mail.ru".to_owned(),
                current_dir: "/".to_owned(),
                entries: vec![WebDavDirEntry {
                    name: "Backups".to_owned(),
                    full_path: "/Backups".to_owned(),
                    is_dir: true,
                }],
                loading: false,
                status_text: String::new(),
            }),
            ..BackupSettingsView::default()
        },
        diagnostics: DiagnosticsLiveView {
            monitor_enabled: true,
            filter_label: "Все".to_owned(),
            summary_line: "ok".to_owned(),
            metrics_lines: vec!["RAM: 16 MB".to_owned()],
            module_lines: vec!["dock: running".to_owned()],
            log_lines: vec!["[12:00] ok".to_owned()],
            ..DiagnosticsLiveView::default()
        },
        about: AboutSettingsView {
            version: app_version.to_owned(),
            exe_path: "asrbox.exe".to_owned(),
            data_dir: "data".to_owned(),
            config_dir: "data/config".to_owned(),
            backups_dir: "data/backups".to_owned(),
            logs_dir: "data/logs".to_owned(),
            changelog_lines: changelog_text()
                .lines()
                .take(20)
                .map(ToOwned::to_owned)
                .collect(),
        },
    }
}

pub fn selftest_backup_and_webdav() -> Result<(), String> {
    let bcfg = BackupConfig {
        auto_upload_cloud: true,
        target_server_ids: vec!["srv-1".to_owned()],
        last_backup_file: "asrbox-settings-20261123-120000.zip".to_owned(),
        last_backup_time: "23.11.2026 12:00".to_owned(),
        last_cloud_status: "OK".to_owned(),
    };
    if parse_backup_config(&serialize_backup_config(&bcfg)) != bcfg {
        return Err("backup config roundtrip mismatch".to_owned());
    }
    let v1 =
        "settings_version=1\nprovider=mailru\nusername=test@mail.ru\nremote_dir=/asrbox-backups\n";
    let store = parse_webdav_store(v1);
    if store.servers.len() != 1 || store.servers[0].provider != WebDavProvider::MailRu {
        return Err("webdav v1 migration mismatch".to_owned());
    }
    if parse_webdav_store(&serialize_webdav_store(&store)) != store {
        return Err("webdav v2 store roundtrip mismatch".to_owned());
    }
    validate_https_url(&store.servers[0].effective_url())?;
    let xml = "<d:multistatus><d:response><d:href>/Backups/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response></d:multistatus>";
    if parse_propfind_xml(xml, "", "/").len() != 1 {
        return Err("webdav propfind selftest failed".to_owned());
    }
    let scfg = FileSyncConfig {
        items: vec![SyncItem {
            path: "C:\\Docs".to_owned(),
            is_dir: true,
        }],
        target_server_ids: vec!["srv-1".to_owned()],
        interval: SyncInterval::Min30,
        last_sync_time: String::new(),
        last_sync_status: String::new(),
    };
    if parse_sync_config(&serialize_sync_config(&scfg)) != scfg {
        return Err("sync.cfg roundtrip mismatch".to_owned());
    }
    if base64_encode(b"asrbox:ok") != "YXNyYm94Om9r" {
        return Err("webdav base64 selftest failed".to_owned());
    }
    Ok(())
}

pub fn selftest_debug_and_about(app_version: &str) -> Result<(), String> {
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
