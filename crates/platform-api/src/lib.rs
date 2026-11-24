pub mod menu;
pub mod telemetry;
pub mod tools;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub use menu::{
    DockMenuAction, DockMenuState, TrayAction, TrayMenuEntry, TrayModuleFlags, WidgetLayer,
    WidgetMenuAction, WidgetMenuState,
};
pub use telemetry::{
    CpuTelemetry, DriveTelemetry, GpuTelemetry, NetTelemetry, OverlayTelemetry, ProcUsage,
    RamTelemetry, SysTelemetry, WidgetSizeBounds,
};
pub use tools::{
    AboutUiCommand, DebugUiCommand, ProcessMetrics, SettingsInputRects, SettingsUiEvent,
    ToolsPlatform, WebDavResult,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformEvent {
    PointerMove {
        dock_id: u64,
        point: PixelPoint,
    },
    PointerLeave {
        dock_id: u64,
    },
    LeftButtonDown {
        dock_id: u64,
        point: PixelPoint,
        screen_point: PixelPoint,
    },
    DockDragMove {
        dock_id: u64,
        point: PixelPoint,
        screen_point: PixelPoint,
    },
    LeftButtonUp {
        dock_id: u64,
        point: PixelPoint,
        screen_point: PixelPoint,
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
    RunningPollTick,
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
    WidgetResized {
        widget_id: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        finished: bool,
    },
    WidgetMinuteTick {
        widget_id: u64,
    },
    WidgetRefreshTick {
        widget_id: u64,
    },
    SysTelemetryReady {
        snapshot: Box<SysTelemetry>,
    },
    OverlayTelemetryReady {
        snapshot: Box<OverlayTelemetry>,
    },
    HttpFetchDone {
        request_id: u64,
        result: HttpFetchResult,
    },
    DebugWindowTick,
    DebugCommand(DebugUiCommand),
    AboutCommand(AboutUiCommand),
    SettingsEvent(SettingsUiEvent),
    WebDavDone(WebDavResult),
    SupervisorTick,
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
    fn set_dock_layer(&mut self, dock_id: u64, layer: WidgetLayer) -> Result<(), String>;
    fn present_dock_frame(
        &mut self,
        dock_id: u64,
        rect: PixelRect,
        frame: &BgraFrame,
    ) -> Result<(), String>;
    fn present_dock_widget_frame(
        &mut self,
        dock_id: u64,
        rect: PixelRect,
        frame: &WidgetFrame,
    ) -> Result<(), String>;
    fn set_dock_tooltip(&mut self, dock_id: u64, text: Option<&str>) -> Result<(), String>;
    fn set_animation_timer(&mut self, dock_id: u64, active: bool) -> Result<(), String>;
    fn set_running_poll_timer(&mut self, active: bool) -> Result<(), String>;
    fn query_running_exe_names(&self) -> HashSet<String>;
    fn resolve_target_exe_name(&self, path: &Path) -> Option<String>;
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
        flags: TrayModuleFlags,
    ) -> Result<Option<TrayAction>, String>;
    fn pick_icon_file(&mut self) -> Result<Option<PathBuf>, String>;
    fn activate_or_launch_target(&self, path: &Path, focus_running: bool) -> Result<bool, String>;
    fn open_text_file(&self, path: &Path) -> Result<(), String>;
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
        drag_header_h: i32,
    ) -> Result<(), String>;
    fn destroy_widget_window(&mut self, widget_id: u64) -> Result<(), String>;
    fn set_widget_layer(&mut self, widget_id: u64, layer: WidgetLayer) -> Result<(), String>;
    fn set_widget_click_through(
        &mut self,
        widget_id: u64,
        click_through: bool,
    ) -> Result<(), String>;
    fn set_widget_visible(&mut self, widget_id: u64, visible: bool) -> Result<(), String>;
    fn set_widget_resize_bounds(
        &mut self,
        widget_id: u64,
        bounds: WidgetSizeBounds,
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
    fn set_widget_refresh_ms(&mut self, widget_id: u64, interval_ms: u32) -> Result<(), String>;
    fn request_sys_telemetry(&mut self) -> Result<(), String>;
    fn request_overlay_telemetry(
        &mut self,
        full_sample: bool,
        extra_game_procs: &[String],
    ) -> Result<(), String>;
    fn show_widget_menu(
        &mut self,
        widget_id: u64,
        screen_point: PixelPoint,
        state: &WidgetMenuState,
    ) -> Result<Option<WidgetMenuAction>, String>;
    fn start_http_get(&mut self, request_id: u64, url: &str) -> Result<(), String>;
    fn open_url(&self, url: &str) -> Result<(), String>;
    fn open_text_file(&self, path: &Path) -> Result<(), String>;
}
