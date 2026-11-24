use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use asrbox_platform_api::WidgetLayer;

use crate::labels::{default_label_for_path, DockLabelMode};

pub type DockId = u64;
pub const SETTINGS_SLOT_ID: u64 = u64::MAX;
pub const DRAG_HANDLE_SLOT_ID: u64 = u64::MAX - 1;
pub const DEFAULT_DOCK_OPACITY: u8 = 85;
pub const MIN_DOCK_OPACITY: u8 = 20;
pub const MAX_DOCK_OPACITY: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockEdge {
    Left,
    Right,
    Top,
    #[default]
    Bottom,
}

impl DockEdge {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockAlignment {
    #[default]
    Center,
    Offset(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum IconSource {
    #[default]
    AutoShell,
    CustomFile(PathBuf),
    ExeResource {
        path: PathBuf,
        index: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverZoomMode {
    #[default]
    SingleIcon,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoverZoomConfig {
    pub enabled: bool,
    pub mode: HoverZoomMode,
    pub scale: f32,
    pub duration_ms: u16,
}

impl Default for HoverZoomConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: HoverZoomMode::SingleIcon,
            scale: 1.35,
            duration_ms: 120,
        }
    }
}

impl HoverZoomConfig {
    pub fn validate(&self) -> Result<(), DockConfigError> {
        if !self.scale.is_finite() || !(1.0..=2.0).contains(&self.scale) {
            return Err(DockConfigError::InvalidZoomScale);
        }
        if self.duration_ms > 600 {
            return Err(DockConfigError::InvalidZoomDuration(self.duration_ms));
        }
        Ok(())
    }

    pub fn effective_scale(&self) -> f32 {
        if self.enabled {
            self.scale.clamp(1.0, 2.0)
        } else {
            1.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockItemKind {
    #[default]
    Shortcut,
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockItem {
    pub id: u64,
    pub kind: DockItemKind,
    pub label: String,
    pub target: PathBuf,
    pub icon: IconSource,
}

impl DockItem {
    pub fn from_dropped_path(id: u64, target: PathBuf) -> Result<Self, DockConfigError> {
        if target.as_os_str().is_empty() {
            return Err(DockConfigError::EmptyTargetPath);
        }
        let label = default_label_for_path(&target);
        Ok(Self {
            id,
            kind: DockItemKind::Shortcut,
            label,
            target,
            icon: IconSource::AutoShell,
        })
    }

    pub fn separator(id: u64) -> Self {
        Self {
            id,
            kind: DockItemKind::Separator,
            label: "Разделитель".to_owned(),
            target: PathBuf::new(),
            icon: IconSource::AutoShell,
        }
    }

    pub fn is_separator(&self) -> bool {
        self.kind == DockItemKind::Separator
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockConfig {
    pub id: DockId,
    pub name: String,
    pub enabled: bool,
    pub locked: bool,
    pub layer: WidgetLayer,
    pub label_mode: DockLabelMode,
    pub free_pos: Option<(i32, i32)>,
    pub monitor_index: usize,
    pub edge: DockEdge,
    pub alignment: DockAlignment,
    pub edge_margin_px: u16,
    pub icon_size_px: u16,
    pub opacity_percent: u8,
    pub item_spacing_px: u16,
    pub padding_px: u16,
    pub corner_radius_px: u16,
    pub show_settings_button: bool,
    pub show_running_indicator: bool,
    pub focus_running_window: bool,
    pub hover_zoom: HoverZoomConfig,
    pub items: Vec<DockItem>,
}

impl DockConfig {
    pub fn new(id: DockId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            enabled: true,
            locked: true,
            layer: WidgetLayer::Wallpaper,
            label_mode: DockLabelMode::IconsOnly,
            free_pos: None,
            monitor_index: 0,
            edge: DockEdge::Bottom,
            alignment: DockAlignment::Center,
            edge_margin_px: 8,
            icon_size_px: 48,
            opacity_percent: DEFAULT_DOCK_OPACITY,
            item_spacing_px: 10,
            padding_px: 12,
            corner_radius_px: 16,
            show_settings_button: true,
            show_running_indicator: true,
            focus_running_window: true,
            hover_zoom: HoverZoomConfig::default(),
            items: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), DockConfigError> {
        if self.name.trim().is_empty() {
            return Err(DockConfigError::EmptyName);
        }
        if !(24..=128).contains(&self.icon_size_px) {
            return Err(DockConfigError::InvalidIconSize(self.icon_size_px));
        }
        if self.item_spacing_px > 48 {
            return Err(DockConfigError::InvalidSpacing(self.item_spacing_px));
        }
        if !(4..=48).contains(&self.padding_px) {
            return Err(DockConfigError::InvalidPadding(self.padding_px));
        }
        self.hover_zoom.validate()?;
        for item in &self.items {
            if !item.is_separator() && item.target.as_os_str().is_empty() {
                return Err(DockConfigError::EmptyTargetPath);
            }
        }
        Ok(())
    }

    pub fn next_item_id(&self) -> u64 {
        self.items
            .iter()
            .map(|item| item.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockConfigError {
    EmptyName,
    EmptyTargetPath,
    InvalidIconSize(u16),
    InvalidSpacing(u16),
    InvalidPadding(u16),
    InvalidZoomScale,
    InvalidZoomDuration(u16),
    DockNotFound(DockId),
    ItemNotFound(u64),
}

impl fmt::Display for DockConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "dock name must not be empty"),
            Self::EmptyTargetPath => write!(f, "dock item target path must not be empty"),
            Self::InvalidIconSize(px) => write!(f, "icon_size_px {px} is out of 24..=128"),
            Self::InvalidSpacing(px) => write!(f, "item_spacing_px {px} exceeds 48"),
            Self::InvalidPadding(px) => write!(f, "padding_px {px} is out of 4..=48"),
            Self::InvalidZoomScale => write!(f, "hover zoom scale must be in 1.0..=2.0"),
            Self::InvalidZoomDuration(ms) => write!(f, "hover zoom duration {ms} ms exceeds 600"),
            Self::DockNotFound(id) => write!(f, "dock {id} not found"),
            Self::ItemNotFound(id) => write!(f, "dock item {id} not found"),
        }
    }
}

impl Error for DockConfigError {}

#[derive(Debug, Clone)]
pub struct DockRegistry {
    next_dock_id: DockId,
    next_item_id: u64,
    docks: Vec<DockConfig>,
}

impl Default for DockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DockRegistry {
    pub fn new() -> Self {
        Self {
            next_dock_id: 1,
            next_item_id: 1,
            docks: Vec::new(),
        }
    }

    pub fn create_dock(&mut self, name: impl Into<String>) -> Result<DockId, DockConfigError> {
        let id = self.next_dock_id;
        let config = DockConfig::new(id, name);
        config.validate()?;
        self.next_dock_id = self.next_dock_id.saturating_add(1);
        self.docks.push(config);
        Ok(id)
    }

    pub fn remove_dock(&mut self, id: DockId) -> Result<DockConfig, DockConfigError> {
        let pos = self
            .docks
            .iter()
            .position(|dock| dock.id == id)
            .ok_or(DockConfigError::DockNotFound(id))?;
        Ok(self.docks.remove(pos))
    }

    pub fn get(&self, id: DockId) -> Option<&DockConfig> {
        self.docks.iter().find(|dock| dock.id == id)
    }

    pub fn get_mut(&mut self, id: DockId) -> Option<&mut DockConfig> {
        self.docks.iter_mut().find(|dock| dock.id == id)
    }

    pub fn all(&self) -> &[DockConfig] {
        &self.docks
    }

    pub fn len(&self) -> usize {
        self.docks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docks.is_empty()
    }

    pub fn add_dropped_item(
        &mut self,
        dock_id: DockId,
        target: PathBuf,
        insert_at: Option<usize>,
    ) -> Result<u64, DockConfigError> {
        let item_id = self.next_item_id;
        let item = DockItem::from_dropped_path(item_id, target)?;
        let dock = self
            .get_mut(dock_id)
            .ok_or(DockConfigError::DockNotFound(dock_id))?;
        let index = insert_at.unwrap_or(dock.items.len()).min(dock.items.len());
        dock.items.insert(index, item);
        self.next_item_id = self.next_item_id.saturating_add(1);
        Ok(item_id)
    }

    pub fn set_item_icon(
        &mut self,
        dock_id: DockId,
        item_id: u64,
        source: IconSource,
    ) -> Result<(), DockConfigError> {
        let dock = self
            .get_mut(dock_id)
            .ok_or(DockConfigError::DockNotFound(dock_id))?;
        let item = dock
            .items
            .iter_mut()
            .find(|entry| entry.id == item_id)
            .ok_or(DockConfigError::ItemNotFound(item_id))?;
        item.icon = source;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_and_validation_work() -> Result<(), Box<dyn Error>> {
        let mut registry = DockRegistry::new();
        for idx in 1..=10 {
            let id = registry.create_dock(format!("Dock {idx}"))?;
            let dock = registry
                .get_mut(id)
                .ok_or(DockConfigError::DockNotFound(id))?;
            dock.edge = [
                DockEdge::Bottom,
                DockEdge::Left,
                DockEdge::Top,
                DockEdge::Right,
            ][idx % 4];
        }
        assert_eq!(registry.len(), 10);
        let item_id = registry.add_dropped_item(1, PathBuf::from("C:/Apps/Code.exe"), None)?;
        let custom = IconSource::CustomFile(PathBuf::from("data/icons/code.png"));
        registry.set_item_icon(1, item_id, custom.clone())?;
        let dock = registry
            .get_mut(1)
            .ok_or(DockConfigError::DockNotFound(1))?;
        dock.items.push(DockItem::separator(99));
        dock.validate()?;
        assert!(dock.items[1].is_separator());
        assert_eq!(dock.items[0].icon, custom);
        dock.icon_size_px = 12;
        assert_eq!(dock.validate(), Err(DockConfigError::InvalidIconSize(12)));
        dock.icon_size_px = 48;
        dock.hover_zoom.scale = 3.0;
        assert_eq!(dock.validate(), Err(DockConfigError::InvalidZoomScale));
        Ok(())
    }
}
