use asrbox_core::{IsolationMode, ModuleId};
use asrbox_platform_api::{
    AccentPreset, ContextMenuConfig, DesignTheme, FluentPalette, ThemeMode, WebDavDirEntry,
    WidgetLayer,
};

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
            lock_items: false,
            show_running_indicator: true,
            focus_running_window: true,
            launch_animation_enabled: true,
            item_count: 1,
            items: vec![DockItemView {
                id: 1,
                label: "Browser".to_owned(),
                target_path: "C:\\Apps\\browser.exe".to_owned(),
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
        weather: WeatherSettingsView::default(),
        notes: NotesSettingsView::default(),
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
            encrypt_zip: true,
            has_zip_password: true,
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
        github: GitHubBackupView {
            repo: "sasha89sp/asrbox".to_owned(),
            branch: "main".to_owned(),
            include_in_backup: true,
            has_token: true,
            busy: false,
            last_status: String::new(),
        },
        menus: ContextMenuConfig::default(),
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
            version: "0.0.8".to_owned(),
            exe_path: "asrbox.exe".to_owned(),
            data_dir: "data".to_owned(),
            config_dir: "data/config".to_owned(),
            backups_dir: "data/backups".to_owned(),
            logs_dir: "data/logs".to_owned(),
            changelog_lines: vec!["## [0.0.8]".to_owned(), "- ASR Glass".to_owned()],
        },
    }
}

#[test]
fn renders_all_tabs_themes_and_widgets_subtabs() {
    let snap = sample_snapshot();
    let mut ui = SettingsUiState::default();
    assert_eq!(SettingsTab::ALL.len(), 4);
    for style in [DesignTheme::AsrGlass, DesignTheme::Win11Classic] {
        for theme in [ThemeMode::Dark, ThemeMode::Light] {
            ui.set_palette_immediate(FluentPalette::from_appearance(
                style,
                theme,
                AccentPreset::Blue,
            ));
            for tab in SettingsTab::ALL {
                ui.active_tab = tab;
                ui.active_browser_tab = BrowserTab::Main;
                let (frame, inputs) = ui.render(&snap);
                assert_eq!(frame.bgra.width, 860);
                assert!(!frame.texts.is_empty());
                assert!(inputs.search.is_some());
            }
            ui.active_tab = SettingsTab::Widgets;
            for sub in [
                WidgetSubTab::Calendar,
                WidgetSubTab::SysMonitor,
                WidgetSubTab::Weather,
                WidgetSubTab::Notes,
            ] {
                ui.selected_widget = sub;
                let (frame, _) = ui.render(&snap);
                assert!(!frame.texts.is_empty());
            }
        }
    }
    ui.open_browser_tab(BrowserTab::Diagnostics);
    assert_eq!(ui.browser_tabs.len(), 2);
    let _ = ui.render(&snap);
    ui.open_browser_tab(BrowserTab::VersionHistory);
    let _ = ui.render(&snap);
    ui.open_browser_tab(BrowserTab::WebDavBrowser {
        server_id: "srv-1".to_owned(),
        server_name: "Mail.ru".to_owned(),
    });
    let (_, inputs3) = ui.render(&snap);
    assert!(inputs3.aux.is_some());
}

#[test]
fn global_search_expands_matching_sections_across_tabs() {
    let snap = sample_snapshot();
    let mut ui = SettingsUiState {
        search_query: "github".to_owned(),
        ..Default::default()
    };
    let (frame, _) = ui.render(&snap);
    assert!(frame.texts.iter().any(|t| t.text.contains("GitHub")));
    assert!(ui.layout.cards.iter().any(|c| c.highlight));
}

#[test]
fn backup_and_sync_server_chips_do_not_overlap_subtitles() {
    let snap = sample_snapshot();
    let mut ui = SettingsUiState {
        active_tab: SettingsTab::General,
        ..Default::default()
    };
    ui.expanded_sections.insert("backup_cloud".to_owned());
    let _ = ui.render(&snap);
    for card in &ui.layout.cards {
        for row in &card.rows {
            if row.subtitle.is_none() {
                continue;
            }
            let sub_bottom = row.y + 46;
            assert!(
                row.height >= 50,
                "row '{}' tight height {}",
                row.title,
                row.height
            );
            for ctrl in &ui.layout.controls {
                if let layout::HitKind::Chip { .. } = ctrl.kind {
                    let inside_row =
                        ctrl.rect.y >= row.y && ctrl.rect.y < row.y + row.height as i32;
                    if inside_row {
                        assert!(ctrl.rect.y >= sub_bottom);
                    }
                }
            }
        }
    }
}

#[test]
fn scrolled_content_texts_do_not_overlap_sticky_header() {
    let snap = sample_snapshot();
    let mut ui = SettingsUiState {
        active_tab: SettingsTab::General,
        scroll_y: 42,
        ..Default::default()
    };
    let (frame, _) = ui.render(&snap);
    let top_y = ui.layout.content_top_y;
    let sidebar_x = layout::SIDEBAR_WIDTH as i32;
    for cmd in &frame.texts {
        if cmd.x >= sidebar_x && cmd.y < top_y {
            let is_header = cmd.text == ui.layout.header_title
                || cmd.text == ui.layout.header_subtitle
                || cmd.text == "⌕";
            assert!(is_header, "content text '{}' bled into header", cmd.text);
        }
    }
}

#[test]
fn weather_city_uses_primary_input_and_yandex_key_uses_secret_input() {
    let mut snap = sample_snapshot();
    snap.weather.provider_is_yandex = true;
    let mut ui = SettingsUiState {
        active_tab: SettingsTab::Widgets,
        selected_widget: WidgetSubTab::Weather,
        ..Default::default()
    };
    ui.expanded_sections.insert("weather_main".to_owned());
    let (_, inputs) = ui.render(&snap);
    assert!(
        inputs.primary.is_some(),
        "weather city must bind to primary input slot 0"
    );
    assert!(
        inputs.secret.is_some(),
        "yandex weather API key must bind to secret input slot 1"
    );
}
