use asrbox_core::{IsolationMode, ModuleId};
use asrbox_platform_api::{
    AccentPreset, AppLanguage, DesignTheme, DockMenuVisibility, FontRenderer, PixelPoint,
    ThemeMode, TrayMenuVisibility, WidgetLayer, WidgetMenuVisibility,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboKind {
    DesignTheme,
    ThemeMode,
    AccentPreset,
    FontRenderer,
    Language,
    Isolation,
    DockSelect,
    DockEdge(String),
    DockLayer(String),
    DockLabelMode(String),
    WeatherLayer,
    NotesLayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    // Appearance & general
    SetDesignTheme(DesignTheme),
    SetThemeMode(ThemeMode),
    SetAccentPreset(AccentPreset),
    SetFontRenderer(FontRenderer),
    SetLanguage(AppLanguage),
    OpenCombo {
        kind: ComboKind,
        anchor_client: PixelPoint,
    },
    ConfirmResetAll,
    SetIsolation(IsolationMode),
    SetAutostart(bool),
    SetModuleEnabled {
        id: ModuleId,
        enabled: bool,
    },
    RestartModule(ModuleId),
    RestartAllModules,
    // Context menus visibility
    SetTrayMenuVis(TrayMenuVisibility),
    SetDockMenuVis(DockMenuVisibility),
    SetWidgetMenuVis(WidgetMenuVisibility),
    // Dock
    SelectDock(usize),
    AddDock,
    RemoveDock(String),
    AddDockSeparator(String),
    SetDockEdge {
        dock_id: String,
        edge_index: u8,
    },
    SetDockLayer {
        dock_id: String,
        layer: WidgetLayer,
    },
    SetDockLabelMode {
        dock_id: String,
        mode_index: u8,
    },
    SetDockIconSize {
        dock_id: String,
        size_px: u32,
    },
    SetDockOpacity {
        dock_id: String,
        opacity: u8,
    },
    SetDockLocked {
        dock_id: String,
        locked: bool,
    },
    SetDockLockItems {
        dock_id: String,
        locked: bool,
    },
    SetDockShowRunning {
        dock_id: String,
        enabled: bool,
    },
    SetDockFocusRunning {
        dock_id: String,
        enabled: bool,
    },
    SetDockLaunchAnimation {
        dock_id: String,
        enabled: bool,
    },
    RenameDockItemFromInput {
        dock_id: String,
        item_id: u64,
    },
    ResetDockItemLabel {
        dock_id: String,
        item_id: u64,
    },
    OpenDockItemFolder {
        dock_id: String,
        item_id: u64,
    },
    RemoveDockItem {
        dock_id: String,
        item_id: u64,
    },
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
    // Weather
    SetWeatherVisible(bool),
    SetWeatherProvider(bool),
    SetWeatherForecastDays(u8),
    SaveYandexWeatherKey,
    ClearYandexWeatherKey,
    ApplyWeatherCityFromInput,
    SetWeatherLayer(WidgetLayer),
    SetWeatherOpacity(u8),
    SetWeatherRefreshMin(u32),
    SetWeatherWidth(u32),
    SetWeatherHeight(u32),
    RefreshWeatherNow,
    OpenWeatherConfig,
    // Notes
    SetNotesVisible(bool),
    SetNotesLayer(WidgetLayer),
    SetNotesOpacity(u8),
    ToggleNotesPane(u8),
    OpenNotesFolder,
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
    // Backup, AES-256, Multi-server WebDAV & Sync
    SetBackupEncryptZip(bool),
    SaveBackupZipPassword,
    ClearBackupZipPassword,
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
    WebDavBrowseDir {
        server_id: String,
        path: String,
    },
    WebDavBrowseUp(String),
    WebDavCreateFolderFromInput(String),
    WebDavSelectCurrentFolder(String),
    PickSyncFiles,
    PickSyncFolder,
    RemoveSyncItem(String),
    ToggleSyncServer(String),
    SetSyncIntervalMin(u32),
    SyncFilesNow,
    // Developer GitHub source backup
    SetGitHubIncludeInBackup(bool),
    SaveGitHubSettings,
    ClearGitHubToken,
    DownloadGitHubSourceNow,
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
    WeatherOpacity,
    WeatherWidth,
    WeatherHeight,
    NotesOpacity,
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
            | Self::WeatherOpacity
            | Self::NotesOpacity
            | Self::OverlayOpacity => (20, 100),
            Self::CalendarWidth => (280, 860),
            Self::CalendarRows => (3, 16),
            Self::SysMonitorWidth => (440, 900),
            Self::SysMonitorHeight => (380, 780),
            Self::WeatherWidth => (280, 620),
            Self::WeatherHeight => (310, 640),
            Self::OverlayWidth => (220, 640),
            Self::OverlayHeight => (132, 380),
        }
    }
}
