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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconRequest {
    AutoShell(PathBuf),
    CustomFile(PathBuf),
    ExeResource {
        path: PathBuf,
        index: u32,
    },
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
    ) -> Result<Option<TrayAction>, String>;

    fn pick_icon_file(&mut self) -> Result<Option<PathBuf>, String>;

    fn launch_target(&self, path: &Path) -> Result<(), String>;

    fn request_quit(&mut self);
}
