use asrbox_core::{IsolationMode, ModuleId};
use asrbox_platform_api::{AccentPreset, AppLanguage, FontRenderer, ThemeMode, WidgetLayer};

use crate::types::SettingsTab;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    // Appearance & general
    SetThemeMode(ThemeMode),
    SetAccentPreset(AccentPreset),
    SetFontRenderer(FontRenderer),
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
    SetDockLaunchAnimation { dock_id: String, enabled: bool },
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
