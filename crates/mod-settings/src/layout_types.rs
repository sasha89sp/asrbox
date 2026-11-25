use std::collections::HashSet;

use asrbox_platform_api::{AccentPreset, PixelRect, SettingsInputRects};

use crate::types::{
    BrowserTab, ComboKind, SettingsAction, SettingsSnapshot, SettingsTab, SliderKind, WidgetSubTab,
};

pub const SIDEBAR_WIDTH: u32 = 210;
pub const MIN_SETTINGS_WIDTH: u32 = 720;
pub const MIN_SETTINGS_HEIGHT: u32 = 460;
pub const TAB_STRIP_H: i32 = 36;
pub const HEADER_H: i32 = 76;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonTone {
    Secondary,
    Primary,
    Danger,
    Amber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitKind {
    Tab(SettingsTab),
    BrowserTabSelect {
        index: usize,
        label: String,
        active: bool,
        closable: bool,
    },
    BrowserTabClose {
        index: usize,
    },
    AccordionHeader {
        id: String,
        title: String,
        summary: String,
        expanded: bool,
        matched: bool,
    },
    ComboBox {
        label: String,
        dot_color: Option<[u8; 4]>,
        kind: ComboKind,
    },
    StartResetConfirm {
        label: String,
    },
    CancelResetConfirm {
        label: String,
    },
    SelectDock(usize),
    SelectWidgetSubTab {
        sub: WidgetSubTab,
        label: String,
        active: bool,
    },
    SelectDockItem {
        item_id: u64,
        label: String,
        active: bool,
    },
    StartEditServerDir {
        server_id: String,
        label: String,
    },
    CancelEditServerDir {
        label: String,
    },
    AccentSwatch {
        preset: AccentPreset,
        active: bool,
    },
    Chip {
        label: String,
        active: bool,
        action: SettingsAction,
    },
    Toggle {
        on: bool,
        action: SettingsAction,
    },
    Slider {
        kind: SliderKind,
        value: u32,
        min: u32,
        max: u32,
    },
    Segment {
        label: String,
        active: bool,
        action: SettingsAction,
    },
    Button {
        label: String,
        tone: ButtonTone,
        action: SettingsAction,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlItem {
    pub id: usize,
    pub rect: PixelRect,
    pub kind: HitKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardBlock {
    pub rect: PixelRect,
    pub amber: bool,
    pub highlight: bool,
    pub title: Option<String>,
    pub rows: Vec<CardRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardRow {
    pub y: i32,
    pub height: u32,
    pub title: String,
    pub subtitle: Option<String>,
    pub value_label: Option<String>,
    pub divider: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsLayout {
    pub width: u32,
    pub height: u32,
    pub content_top_y: i32,
    pub has_tab_strip: bool,
    pub header_title: String,
    pub header_subtitle: String,
    pub search_rect: Option<PixelRect>,
    pub cards: Vec<CardBlock>,
    pub controls: Vec<ControlItem>,
    pub input_rects: SettingsInputRects,
    pub content_height: u32,
}

pub struct BuildLayoutArgs<'a> {
    pub snap: &'a SettingsSnapshot,
    pub tab: SettingsTab,
    pub browser_tabs: &'a [BrowserTab],
    pub active_browser: &'a BrowserTab,
    pub selected_dock: usize,
    pub selected_dock_item: Option<u64>,
    pub selected_widget: WidgetSubTab,
    pub editing_server_dir: Option<&'a str>,
    pub expanded_sections: &'a HashSet<String>,
    pub confirming_reset: bool,
    pub search_query: &'a str,
    pub width: u32,
    pub height: u32,
    pub scroll_y: i32,
}

pub fn rect_contains(rect: PixelRect, x: i32, y: i32) -> bool {
    x >= rect.x
        && y >= rect.y
        && x < rect.x.saturating_add(rect.width as i32)
        && y < rect.y.saturating_add(rect.height as i32)
}
