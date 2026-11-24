use asrbox_core::{IsolationMode, ModuleId};
use asrbox_platform_api::{AccentPreset, FluentPalette, ThemeMode, WebDavDirEntry, WidgetLayer};

use crate::*;

fn sample_snapshot() -> SettingsSnapshot {
    SettingsSnapshot {
        appearance: AppearanceSettingsView::default(),
        isolation: IsolationMode::Process,
        autostart_enabled: false,
        modules: vec![ModuleStatusView {
            id: ModuleId::Dock,
            enabled: true,
            running: true,
            pid: Some(1234),
            restarts: 0,
            note: String::new(),
        }],
        docks: vec![DockSettingsView {
            id: "dock-1".to_owned(),
            edge_index: 0,
            layer: WidgetLayer::Wallpaper,
            label_mode_index: 0,
            icon_size_px: 48,
            opacity_percent: 85,
            locked: true,
            show_running_indicator: true,
            focus_running_window: true,
            item_count: 1,
            items: vec![DockItemView {
                id: 1,
                label: "Browser".to_owned(),
            }],
        }],
        calendar: CalendarSettingsView {
            visible: true,
            opacity_percent: 88,
            layer: WidgetLayer::Wallpaper,
            period_index: 2,
            refresh_interval_sec: 1800,
            width: 336,
            visible_rows: 8,
            commented_calendar_hint: false,
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
                username: "user@mail.ru".to_owned(),
                remote_dir: "asrbox-backups".to_owned(),
                has_saved_password: true,
                status_text: "HTTP 207".to_owned(),
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
            summary_line: "RAM 18 MB".to_owned(),
            metrics_lines: vec!["RAM: 18 MB".to_owned()],
            module_lines: vec!["dock: running".to_owned()],
            log_lines: vec!["[12:00] ok".to_owned()],
            ..DiagnosticsLiveView::default()
        },
        about: AboutSettingsView {
            version: "0.0.5".to_owned(),
            exe_path: "asrbox.exe".to_owned(),
            data_dir: "data".to_owned(),
            config_dir: "data/config".to_owned(),
            backups_dir: "data/backups".to_owned(),
            logs_dir: "data/logs".to_owned(),
            changelog_lines: vec!["## [0.0.5]".to_owned(), "- Fluent Design".to_owned()],
        },
    }
}

#[test]
fn renders_all_tabs_and_browser_tabs_in_light_and_dark() {
    let snap = sample_snapshot();
    let mut ui = SettingsUiState::default();
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        ui.set_palette_immediate(FluentPalette::from_theme(theme, AccentPreset::Blue));
        for tab in SettingsTab::ALL {
            ui.active_tab = tab;
            ui.active_browser_tab = BrowserTab::Main;
            let (frame, inputs) = ui.render(&snap);
            assert_eq!(frame.bgra.width, 880);
            assert!(!frame.texts.is_empty());
            assert!(inputs.search.is_some());
        }
    }
    ui.open_browser_tab(BrowserTab::Diagnostics);
    assert_eq!(ui.browser_tabs.len(), 2);
    let (f1, _) = ui.render(&snap);
    assert!(!f1.texts.is_empty());
    ui.open_browser_tab(BrowserTab::VersionHistory);
    let (f2, _) = ui.render(&snap);
    assert!(!f2.texts.is_empty());
    ui.open_browser_tab(BrowserTab::WebDavBrowser {
        server_id: "srv-1".to_owned(),
        server_name: "Mail.ru".to_owned(),
    });
    let (f3, inputs3) = ui.render(&snap);
    assert!(!f3.texts.is_empty());
    assert!(inputs3.aux.is_some());
    ui.close_browser_tab(1);
    assert_eq!(ui.browser_tabs.len(), 3);
}
