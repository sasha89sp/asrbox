use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PixelPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub utc_offset_min: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, String> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| "image dimensions overflow".to_owned())?;
        if pixels.len() != expected {
            return Err(format!(
                "invalid RGBA buffer size: expected {expected}, got {}",
                pixels.len()
            ));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgraFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl BgraFrame {
    pub fn transparent(width: u32, height: u32) -> Self {
        let byte_len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        Self {
            width,
            height,
            pixels: vec![0u8; byte_len],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetTextCmd {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: String,
    pub font_size_px: i32,
    pub bold: bool,
    pub color_rgba: [u8; 4],
    pub align: TextAlign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetFrame {
    pub bgra: BgraFrame,
    pub texts: Vec<WidgetTextCmd>,
}

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
    pub acrylic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetMenuAction {
    Refresh,
    OpenConfig,
    SetLayerWallpaper,
    SetLayerNormal,
    SetLayerTopmost,
    ToggleAcrylic,
    HideWidget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpFetchResult {
    pub status_code: u16,
    pub body: Vec<u8>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconRequest {
    AutoShell(PathBuf),
    CustomFile(PathBuf),
    ExeResource { path: PathBuf, index: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockMenuState {
    pub edge_code: u8,
    pub is_centered: bool,
    pub icon_size_px: u16,
    pub hover_zoom_enabled: bool,
    pub hovered_item_id: Option<u64>,
    pub can_remove_dock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockMenuAction {
    SetEdgeLeft,
    SetEdgeRight,
    SetEdgeTop,
    SetEdgeBottom,
    AlignCenter,
    OffsetMinus,
    OffsetPlus,
    IconSizeSmall,
    IconSizeMedium,
    IconSizeLarge,
    ToggleHoverZoom,
    ChooseItemIcon(u64),
    ResetItemIcon(u64),
    RemoveItem(u64),
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
pub enum TrayAction {
    AddDock,
    ToggleDock(u64),
    RemoveDock(u64),
    ToggleCalendarWidget,
    OpenWidgetConfig,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformEvent {
    PointerMove {
        dock_id: u64,
        point: PixelPoint,
    },
    PointerLeave {
        dock_id: u64,
    },
    LeftClick {
        dock_id: u64,
        point: PixelPoint,
    },
    RightClick {
        dock_id: u64,
        point: PixelPoint,
        screen_point: PixelPoint,
    },
    AnimationTick {
        dock_id: u64,
        dt_ms: u16,
    },
    DropFiles {
        dock_id: u64,
        paths: Vec<PathBuf>,
        point: PixelPoint,
    },
    DisplayChanged {
        work_area: PixelRect,
    },
    TrayContextMenu {
        screen_point: PixelPoint,
    },
    WidgetPointerMove {
        widget_id: u64,
        point: PixelPoint,
    },
    WidgetPointerLeave {
        widget_id: u64,
    },
    WidgetLeftClick {
        widget_id: u64,
        point: PixelPoint,
    },
    WidgetRightClick {
        widget_id: u64,
        screen_point: PixelPoint,
    },
    WidgetWheel {
        widget_id: u64,
        delta: i16,
    },
    WidgetMoved {
        widget_id: u64,
        x: i32,
        y: i32,
    },
    WidgetMinuteTick {
        widget_id: u64,
    },
    WidgetRefreshTick {
        widget_id: u64,
    },
    HttpFetchDone {
        request_id: u64,
        result: HttpFetchResult,
    },
}

pub trait IconProvider {
    fn load_icon(&self, request: &IconRequest, target_px: u32) -> Result<RgbaImage, String>;
}

pub trait DockPlatform {
    fn primary_work_area(&self) -> Result<PixelRect, String>;

    fn create_dock_window(
        &mut self,
        dock_id: u64,
        title: &str,
        rect: PixelRect,
    ) -> Result<(), String>;

    fn destroy_dock_window(&mut self, dock_id: u64) -> Result<(), String>;

    fn present_dock_frame(
        &mut self,
        dock_id: u64,
        rect: PixelRect,
        frame: &BgraFrame,
    ) -> Result<(), String>;

    fn set_animation_timer(&mut self, dock_id: u64, active: bool) -> Result<(), String>;

    fn show_dock_menu(
        &mut self,
        dock_id: u64,
        screen_point: PixelPoint,
        state: &DockMenuState,
    ) -> Result<Option<DockMenuAction>, String>;

    fn show_tray_menu(
        &mut self,
        screen_point: PixelPoint,
        docks: &[TrayMenuEntry],
        calendar_visible: bool,
    ) -> Result<Option<TrayAction>, String>;

    fn pick_icon_file(&mut self) -> Result<Option<PathBuf>, String>;

    fn launch_target(&self, path: &Path) -> Result<(), String>;

    fn request_quit(&mut self);
}

pub trait WidgetPlatform {
    fn local_time_now(&self) -> LocalDateTime;

    fn create_widget_window(
        &mut self,
        widget_id: u64,
        title: &str,
        rect: PixelRect,
        layer: WidgetLayer,
        acrylic: bool,
        drag_header_h: i32,
    ) -> Result<(), String>;

    fn destroy_widget_window(&mut self, widget_id: u64) -> Result<(), String>;

    fn set_widget_layer(
        &mut self,
        widget_id: u64,
        layer: WidgetLayer,
        acrylic: bool,
    ) -> Result<(), String>;

    fn present_widget_frame(
        &mut self,
        widget_id: u64,
        rect: PixelRect,
        frame: &WidgetFrame,
    ) -> Result<(), String>;

    fn set_widget_tooltip(&mut self, widget_id: u64, text: Option<&str>) -> Result<(), String>;

    fn set_widget_timers(
        &mut self,
        widget_id: u64,
        refresh_sec: u32,
        minute_tick: bool,
    ) -> Result<(), String>;

    fn show_widget_menu(
        &mut self,
        widget_id: u64,
        screen_point: PixelPoint,
        state: &WidgetMenuState,
    ) -> Result<Option<WidgetMenuAction>, String>;

    fn start_http_get(&mut self, request_id: u64, url: &str) -> Result<(), String>;

    fn open_url(&self, url: &str) -> Result<(), String>;
}
