use asrbox_core::{IsolationMode, ModuleId};
use asrbox_platform_api::WidgetLayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Modules,
    Dock,
    Calendar,
    SysMonitor,
    Debug,
    About,
}

impl SettingsTab {
    pub const ALL: [Self; 6] = [
        Self::Modules,
        Self::Dock,
        Self::Calendar,
        Self::SysMonitor,
        Self::Debug,
        Self::About,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Modules => "Модули",
            Self::Dock => "Док-панели",
            Self::Calendar => "Google Календарь",
            Self::SysMonitor => "Системный монитор",
            Self::Debug => "Диагностика",
            Self::About => "О программе",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Modules => "◈",
            Self::Dock => "▭",
            Self::Calendar => "▦",
            Self::SysMonitor => "▤",
            Self::Debug => "◉",
            Self::About => "ⓘ",
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
pub struct DockSettingsView {
    pub id: String,
    pub edge_index: u8,
    pub icon_size_px: u32,
    pub opacity_percent: u8,
    pub locked: bool,
    pub show_running_indicator: bool,
    pub focus_running_window: bool,
    pub item_count: usize,
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
pub struct DebugSettingsView {
    pub verbose: bool,
    pub file_log: bool,
    pub summary_line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutSettingsView {
    pub version: String,
    pub exe_path: String,
    pub data_dir: String,
    pub config_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub isolation: IsolationMode,
    pub modules: Vec<ModuleStatusView>,
    pub docks: Vec<DockSettingsView>,
    pub calendar: CalendarSettingsView,
    pub sysmon: SysMonitorSettingsView,
    pub debug: DebugSettingsView,
    pub about: AboutSettingsView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    SetIsolation(IsolationMode),
    SetModuleEnabled { id: ModuleId, enabled: bool },
    RestartModule(ModuleId),
    AddDock,
    RemoveDock(String),
    AddDockSeparator(String),
    SetDockEdge { dock_id: String, edge_index: u8 },
    SetDockIconSize { dock_id: String, size_px: u32 },
    SetDockOpacity { dock_id: String, opacity: u8 },
    SetDockLocked { dock_id: String, locked: bool },
    SetDockShowRunning { dock_id: String, enabled: bool },
    SetDockFocusRunning { dock_id: String, enabled: bool },
    OpenDocksConfig,
    SetCalendarVisible(bool),
    SetCalendarOpacity(u8),
    SetCalendarLayer(WidgetLayer),
    SetCalendarPeriod(u8),
    SetCalendarWidth(u32),
    SetCalendarRows(u8),
    MatchCalendarToSysMonitor,
    AddCalendarUrlFromInput,
    RemoveCalendarSource(usize),
    UncommentCalendarSources,
    RefreshCalendarNow,
    OpenWidgetsConfig,
    SetSysMonitorVisible(bool),
    SetSysMonitorOpacity(u8),
    SetSysMonitorLayer(WidgetLayer),
    SetSysMonitorRefreshMs(u32),
    SetSysMonitorWidth(u32),
    SetSysMonitorHeight(u32),
    MatchSysMonitorToCalendar,
    RefreshSysMonitorNow,
    OpenSysMonitorConfig,
    SetDebugVerbose(bool),
    SetDebugFileLog(bool),
    OpenDebugWindow,
    OpenAboutWindow,
    OpenDataFolder,
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
}

impl SliderKind {
    pub fn range(self) -> (u32, u32) {
        match self {
            Self::DockIconSize => (24, 128),
            Self::DockOpacity | Self::CalendarOpacity | Self::SysMonitorOpacity => (20, 100),
            Self::CalendarWidth => (280, 860),
            Self::CalendarRows => (3, 16),
            Self::SysMonitorWidth => (440, 900),
            Self::SysMonitorHeight => (380, 780),
        }
    }
}
