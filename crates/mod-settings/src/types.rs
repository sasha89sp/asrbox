use asrbox_core::{IsolationMode, ModuleId};
use asrbox_platform_api::{
    AccentPreset, AppLanguage, FontRenderer, ThemeMode, WebDavDirEntry, WidgetLayer,
};

pub use crate::actions::{SettingsAction, SliderKind};
use crate::i18n::tr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Dock,
    Widgets,
    Overlay,
}

impl SettingsTab {
    pub const ALL: [Self; 4] = [Self::General, Self::Dock, Self::Widgets, Self::Overlay];

    pub fn label(self, lang: AppLanguage) -> &'static str {
        match self {
            Self::General => tr(lang, "Общие настройки", "General"),
            Self::Dock => tr(lang, "Док-панели", "Dock Panels"),
            Self::Widgets => tr(lang, "Виджеты", "Widgets"),
            Self::Overlay => tr(lang, "Игровой оверлей", "Game Overlay"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::General => "⚙",
            Self::Dock => "▭",
            Self::Widgets => "▦",
            Self::Overlay => "◎",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidgetSubTab {
    #[default]
    Calendar,
    SysMonitor,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BrowserTab {
    #[default]
    Main,
    Diagnostics,
    VersionHistory,
    WebDavBrowser {
        server_id: String,
        server_name: String,
    },
}

impl BrowserTab {
    pub fn title(&self, lang: AppLanguage) -> String {
        match self {
            Self::Main => tr(lang, "⚙  Настройки", "⚙  Settings").to_owned(),
            Self::Diagnostics => tr(lang, "◉  Диагностика", "◉  Diagnostics").to_owned(),
            Self::VersionHistory => tr(lang, "ⓘ  История версий", "ⓘ  Version History").to_owned(),
            Self::WebDavBrowser { server_name, .. } => format!("☁  {server_name}"),
        }
    }
    pub fn closable(&self) -> bool {
        !matches!(self, Self::Main)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceSettingsView {
    pub theme: ThemeMode,
    pub accent: AccentPreset,
    pub language: AppLanguage,
    pub font_renderer: FontRenderer,
}

impl Default for AppearanceSettingsView {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            accent: AccentPreset::Blue,
            language: AppLanguage::Ru,
            font_renderer: FontRenderer::GdiClearType,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleStatusView {
    pub id: ModuleId,
    pub enabled: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub restarts: u32,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockItemView {
    pub id: u64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockSettingsView {
    pub id: String,
    pub edge_index: u8,
    pub layer: WidgetLayer,
    pub label_mode_index: u8,
    pub icon_size_px: u32,
    pub opacity_percent: u8,
    pub locked: bool,
    pub show_running_indicator: bool,
    pub focus_running_window: bool,
    pub launch_animation_enabled: bool,
    pub item_count: usize,
    pub items: Vec<DockItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarSourceView {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarSettingsView {
    pub visible: bool,
    pub opacity_percent: u8,
    pub layer: WidgetLayer,
    pub period_index: u8,
    pub refresh_interval_sec: u32,
    pub width: u32,
    pub visible_rows: u8,
    pub commented_calendar_hint: bool,
    pub sources: Vec<CalendarSourceView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysMonitorSettingsView {
    pub visible: bool,
    pub opacity_percent: u8,
    pub layer: WidgetLayer,
    pub refresh_interval_ms: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySettingsView {
    pub module_enabled: bool,
    pub auto_games_only: bool,
    pub force_topmost_fullscreen: bool,
    pub setup_mode: bool,
    pub anchor_index: u8,
    pub x: i32,
    pub y: i32,
    pub opacity_percent: u8,
    pub width: u32,
    pub height: u32,
    pub refresh_interval_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavServerView {
    pub id: String,
    pub name: String,
    pub provider_label: String,
    pub effective_url: String,
    pub username: String,
    pub remote_dir: String,
    pub has_saved_password: bool,
    pub status_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavDraftView {
    pub provider_index: u8,
    pub provider_label: String,
    pub effective_url: String,
    pub is_custom: bool,
    pub status_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavBrowserView {
    pub server_id: String,
    pub server_name: String,
    pub current_dir: String,
    pub entries: Vec<WebDavDirEntry>,
    pub loading: bool,
    pub status_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncItemView {
    pub id: String,
    pub local_path: String,
    pub is_dir: bool,
    pub remote_subdir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileSyncSettingsView {
    pub items: Vec<SyncItemView>,
    pub target_server_ids: Vec<String>,
    pub interval_minutes: u32,
    pub last_sync_time: String,
    pub status_text: String,
    pub busy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupSettingsView {
    pub auto_upload_cloud: bool,
    pub target_server_ids: Vec<String>,
    pub last_backup_file: String,
    pub last_backup_time: String,
    pub cloud_status: String,
    pub busy: bool,
    pub servers: Vec<WebDavServerView>,
    pub draft: Option<WebDavDraftView>,
    pub browser: Option<WebDavBrowserView>,
    pub sync: FileSyncSettingsView,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagnosticsLiveView {
    pub monitor_enabled: bool,
    pub verbose: bool,
    pub file_log: bool,
    pub filter_label: String,
    pub summary_line: String,
    pub metrics_lines: Vec<String>,
    pub module_lines: Vec<String>,
    pub log_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AboutSettingsView {
    pub version: String,
    pub exe_path: String,
    pub data_dir: String,
    pub config_dir: String,
    pub backups_dir: String,
    pub logs_dir: String,
    pub changelog_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub appearance: AppearanceSettingsView,
    pub isolation: IsolationMode,
    pub autostart_enabled: bool,
    pub modules: Vec<ModuleStatusView>,
    pub docks: Vec<DockSettingsView>,
    pub calendar: CalendarSettingsView,
    pub sysmon: SysMonitorSettingsView,
    pub overlay: OverlaySettingsView,
    pub backup: BackupSettingsView,
    pub diagnostics: DiagnosticsLiveView,
    pub about: AboutSettingsView,
}
