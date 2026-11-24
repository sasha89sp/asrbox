use asrbox_core::{IsolationMode, ModuleId};
use asrbox_platform_api::{AccentPreset, AppLanguage, ThemeMode, WebDavDirEntry, WidgetLayer};

use crate::i18n::tr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Modules,
    Dock,
    Calendar,
    SysMonitor,
    Overlay,
}

impl SettingsTab {
    pub const ALL: [Self; 6] = [
        Self::General,
        Self::Modules,
        Self::Dock,
        Self::Calendar,
        Self::SysMonitor,
        Self::Overlay,
    ];

    pub fn label(self, lang: AppLanguage) -> &'static str {
        match self {
            Self::General => tr(lang, "Общие настройки", "General"),
            Self::Modules => tr(lang, "Модули", "Modules"),
            Self::Dock => tr(lang, "Док-панели", "Dock Panels"),
            Self::Calendar => tr(lang, "Google Календарь", "Google Calendar"),
            Self::SysMonitor => tr(lang, "Системный монитор", "System Monitor"),
            Self::Overlay => tr(lang, "Игровой оверлей", "Game Overlay"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::General => "⚙",
            Self::Modules => "◈",
            Self::Dock => "▭",
            Self::Calendar => "▦",
            Self::SysMonitor => "▤",
            Self::Overlay => "◎",
        }
    }
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
}
impl Default for AppearanceSettingsView {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            accent: AccentPreset::Blue,
            language: AppLanguage::Ru,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    // Appearance & general
    SetThemeMode(ThemeMode),
    SetAccentPreset(AccentPreset),
    SetLanguage(AppLanguage),
    ResetSection(SettingsTab),
    SetIsolation(IsolationMode),
    SetAutostart(bool),
    SetModuleEnabled { id: ModuleId, enabled: bool },
    RestartModule(ModuleId),
    // Dock
    AddDock,
    RemoveDock(String),
    AddDockSeparator(String),
    SetDockEdge { dock_id: String, edge_index: u8 },
    SetDockLayer { dock_id: String, layer: WidgetLayer },
    SetDockLabelMode { dock_id: String, mode_index: u8 },
    SetDockIconSize { dock_id: String, size_px: u32 },
    SetDockOpacity { dock_id: String, opacity: u8 },
    SetDockLocked { dock_id: String, locked: bool },
    SetDockShowRunning { dock_id: String, enabled: bool },
    SetDockFocusRunning { dock_id: String, enabled: bool },
    RenameDockItemFromInput { dock_id: String, item_id: u64 },
    ResetDockItemLabel { dock_id: String, item_id: u64 },
    OpenDocksConfig,
    // Calendar
    SetCalendarVisible(bool),
    SetCalendarOpacity(u8),
    SetCalendarLayer(WidgetLayer),
    SetCalendarPeriod(u8),
    SetCalendarSyncInterval(u32),
    SetCalendarWidth(u32),
    SetCalendarRows(u8),
    MatchCalendarToSysMonitor,
    AddCalendarUrlFromInput,
    RemoveCalendarSource(usize),
    UncommentCalendarSources,
    RefreshCalendarNow,
    OpenWidgetsConfig,
    // System Monitor
    SetSysMonitorVisible(bool),
    SetSysMonitorOpacity(u8),
    SetSysMonitorLayer(WidgetLayer),
    SetSysMonitorRefreshMs(u32),
    SetSysMonitorWidth(u32),
    SetSysMonitorHeight(u32),
    MatchSysMonitorToCalendar,
    RefreshSysMonitorNow,
    OpenSysMonitorConfig,
    // Game Overlay
    SetOverlayAutoGamesOnly(bool),
    SetOverlayForceTopmost(bool),
    SetOverlaySetupMode(bool),
    SetOverlayAnchor(u8),
    SetOverlayOpacity(u8),
    SetOverlayWidth(u32),
    SetOverlayHeight(u32),
    SetOverlaySizePreset(u32, u32),
    SetOverlayRefreshMs(u32),
    OpenOverlayConfig,
    // Backup, Multi-server WebDAV & Sync
    BackupSettingsNow,
    BackupFullNow,
    UploadLatestBackupNow,
    OpenBackupsFolder,
    SetAutoUploadBackup(bool),
    ToggleBackupTargetServer(String),
    OpenAddWebDavMenu,
    TestWebDavDraft,
    SaveWebDavDraft,
    CancelWebDavDraft,
    TestWebDavServer(String),
    DeleteWebDavServer(String),
    SaveWebDavServerRemoteDir(String),
    OpenWebDavBrowser(String),
    WebDavBrowseDir { server_id: String, path: String },
    WebDavBrowseUp(String),
    WebDavCreateFolderFromInput(String),
    WebDavSelectCurrentFolder(String),
    PickSyncFiles,
    PickSyncFolder,
    RemoveSyncItem(String),
    ToggleSyncServer(String),
    SetSyncIntervalMin(u32),
    SyncFilesNow,
    // Diagnostics & Version History (browser tabs) & Paths
    OpenDiagnosticsTab,
    SetDiagnosticsMonitorEnabled(bool),
    SetDebugVerbose(bool),
    SetDebugFileLog(bool),
    CycleDebugFilter,
    ClearDebugLogs,
    CopyDebugReport,
    SaveDebugReport,
    OpenVersionHistoryTab,
    CopyVersionInfo,
    OpenDataFolder,
    CopyPathsToClipboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderKind {
    DockIconSize,
    DockOpacity,
    CalendarOpacity,
    CalendarWidth,
    CalendarRows,
    SysMonitorOpacity,
    SysMonitorWidth,
    SysMonitorHeight,
    OverlayOpacity,
    OverlayWidth,
    OverlayHeight,
}

impl SliderKind {
    pub fn range(self) -> (u32, u32) {
        match self {
            Self::DockIconSize => (24, 128),
            Self::DockOpacity
            | Self::CalendarOpacity
            | Self::SysMonitorOpacity
            | Self::OverlayOpacity => (20, 100),
            Self::CalendarWidth => (280, 860),
            Self::CalendarRows => (3, 16),
            Self::SysMonitorWidth => (440, 900),
            Self::SysMonitorHeight => (380, 780),
            Self::OverlayWidth => (220, 640),
            Self::OverlayHeight => (132, 380),
        }
    }
}
