#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidgetLayer {
    #[default]
    Wallpaper,
    Normal,
    Topmost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetMenuState {
    pub layer: WidgetLayer,
    pub opacity_percent: u8,
    pub refresh_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetMenuAction {
    Refresh,
    OpenSettings,
    OpenConfig,
    SetLayerWallpaper,
    SetLayerNormal,
    SetLayerTopmost,
    SetOpacity(u8),
    SetRefreshMs(u32),
    HideWidget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockMenuState {
    pub edge_code: u8,
    pub is_centered: bool,
    pub icon_size_px: u16,
    pub opacity_percent: u8,
    pub locked: bool,
    pub hover_zoom_enabled: bool,
    pub show_running_indicator: bool,
    pub focus_running_window: bool,
    pub hovered_item_id: Option<u64>,
    pub hovered_is_separator: bool,
    pub show_settings_button: bool,
    pub can_remove_dock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockMenuAction {
    SetEdgeLeft,
    SetEdgeRight,
    SetEdgeTop,
    SetEdgeBottom,
    AlignCenter,
    ToggleLock,
    SetOpacity(u8),
    IconSizeSmall,
    IconSizeMedium,
    IconSizeLarge,
    ToggleHoverZoom,
    ToggleRunningIndicator,
    ToggleFocusRunning,
    ChooseItemIcon(u64),
    ResetItemIcon(u64),
    RemoveItem(u64),
    AddSeparator,
    ToggleSettingsButton,
    OpenSettings,
    OpenDockConfig,
    ReloadDockConfig,
    AddNewDock,
    RemoveDock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuEntry {
    pub dock_id: u64,
    pub title: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayModuleFlags {
    pub dock_module_enabled: bool,
    pub widgets_module_enabled: bool,
    pub calendar_visible: bool,
    pub sys_monitor_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    OpenSettings,
    ToggleDockModule,
    AddDock,
    ToggleDock(u64),
    RemoveDock(u64),
    OpenDockConfig,
    ToggleWidgetsModule,
    ToggleCalendarWidget,
    RefreshCalendarWidget,
    OpenWidgetConfig,
    ToggleSysMonitorWidget,
    RefreshSysMonitorWidget,
    OpenSysMonitorConfig,
    OpenDebugWindow,
    OpenAboutWindow,
    Exit,
}
