use asrbox_platform_api::{PixelRect, SettingsInputRects};

use crate::layout_backup::build_backup_tab;
use crate::layout_overlay::build_overlay_tab;
use crate::layout_sys::build_sysmon_tab;
use crate::layout_tabs::{
    build_about_tab, build_calendar_tab, build_debug_tab, build_dock_tab, build_modules_tab,
};
use crate::types::{SettingsAction, SettingsSnapshot, SettingsTab, SliderKind};

pub const SIDEBAR_WIDTH: u32 = 212;
pub const MIN_SETTINGS_WIDTH: u32 = 680;
pub const MIN_SETTINGS_HEIGHT: u32 = 480;

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
    SelectDock(usize),
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
    pub header_title: String,
    pub header_subtitle: String,
    pub cards: Vec<CardBlock>,
    pub controls: Vec<ControlItem>,
    pub input_rects: SettingsInputRects,
    pub content_height: u32,
}

pub fn build_layout(
    snap: &SettingsSnapshot,
    tab: SettingsTab,
    selected_dock: usize,
    width: u32,
    height: u32,
    scroll_y: i32,
) -> SettingsLayout {
    let w = width.max(MIN_SETTINGS_WIDTH);
    let h = height.max(MIN_SETTINGS_HEIGHT);
    let mut b = LayoutBuilder::new(w, h, scroll_y);
    b.add_sidebar_tabs();
    match tab {
        SettingsTab::Modules => build_modules_tab(&mut b, snap),
        SettingsTab::Dock => build_dock_tab(&mut b, snap, selected_dock),
        SettingsTab::Calendar => build_calendar_tab(&mut b, snap),
        SettingsTab::SysMonitor => build_sysmon_tab(&mut b, snap),
        SettingsTab::Overlay => build_overlay_tab(&mut b, snap),
        SettingsTab::Backup => build_backup_tab(&mut b, snap),
        SettingsTab::Debug => build_debug_tab(&mut b, snap),
        SettingsTab::About => build_about_tab(&mut b, snap),
    }
    b.finish()
}

pub fn rect_contains(rect: PixelRect, x: i32, y: i32) -> bool {
    x >= rect.x
        && y >= rect.y
        && x < rect.x.saturating_add(rect.width as i32)
        && y < rect.y.saturating_add(rect.height as i32)
}

pub(crate) struct LayoutBuilder {
    pub out: SettingsLayout,
    scroll_y: i32,
    pub content_x: i32,
    pub content_w: u32,
    pub cursor_y: i32,
    next_id: usize,
}

impl LayoutBuilder {
    fn new(width: u32, height: u32, scroll_y: i32) -> Self {
        let content_x = SIDEBAR_WIDTH as i32 + 28;
        let content_w = width.saturating_sub(SIDEBAR_WIDTH + 56).max(360);
        Self {
            out: SettingsLayout {
                width,
                height,
                ..Default::default()
            },
            scroll_y: scroll_y.max(0),
            content_x,
            content_w,
            cursor_y: 86,
            next_id: 1,
        }
    }

    fn add_sidebar_tabs(&mut self) {
        let mut y = 78;
        for tab in SettingsTab::ALL {
            self.push_control(
                PixelRect {
                    x: 12,
                    y,
                    width: SIDEBAR_WIDTH - 24,
                    height: 34,
                },
                HitKind::Tab(tab),
            );
            y += 38;
        }
    }

    pub(crate) fn set_header(&mut self, title: &str, subtitle: &str) {
        self.out.header_title = title.to_owned();
        self.out.header_subtitle = subtitle.to_owned();
    }

    pub(crate) fn push_control(&mut self, rect: PixelRect, kind: HitKind) {
        let id = self.next_id;
        self.next_id += 1;
        self.out.controls.push(ControlItem { id, rect, kind });
    }

    pub(crate) fn sy(&self, logical_y: i32) -> i32 {
        logical_y - self.scroll_y
    }

    pub(crate) fn add_card(&mut self, title: Option<&str>, amber: bool, rows: Vec<CardRow>) -> i32 {
        let top = self.cursor_y;
        let body_h: u32 = rows.iter().map(|r| r.height).sum();
        let header_h = if title.is_some() { 26 } else { 0 };
        let rect = PixelRect {
            x: self.content_x,
            y: self.sy(top + header_h),
            width: self.content_w,
            height: body_h.max(44),
        };
        let shifted_rows = rows
            .into_iter()
            .map(|mut r| {
                r.y = self.sy(top + header_h + r.y);
                r
            })
            .collect();
        self.out.cards.push(CardBlock {
            rect,
            amber,
            title: title.map(str::to_owned),
            rows: shifted_rows,
        });
        self.cursor_y = top + header_h + body_h as i32 + 18;
        top + header_h
    }

    pub(crate) fn add_segmented_row(
        &mut self,
        row_y_logical: i32,
        row_h: u32,
        items: Vec<(&str, bool, SettingsAction)>,
    ) {
        let count = items.len().max(1) as u32;
        let total_w = (count * 90)
            .min(self.content_w.saturating_sub(200))
            .max(160);
        let seg_w = total_w / count;
        let start_x = self.content_x + self.content_w as i32 - total_w as i32 - 16;
        let y = self.sy(row_y_logical + (row_h as i32 - 28) / 2);
        for (i, (label, active, action)) in items.into_iter().enumerate() {
            self.push_control(
                PixelRect {
                    x: start_x + (i as i32) * (seg_w as i32),
                    y,
                    width: seg_w.saturating_sub(2),
                    height: 28,
                },
                HitKind::Segment {
                    label: label.to_owned(),
                    active,
                    action,
                },
            );
        }
    }

    pub(crate) fn push_toggle(&mut self, x: i32, y_log: i32, on: bool, action: SettingsAction) {
        let r = PixelRect {
            x,
            y: self.sy(y_log),
            width: 44,
            height: 24,
        };
        self.push_control(r, HitKind::Toggle { on, action });
    }

    pub(crate) fn push_slider(&mut self, x: i32, y_log: i32, kind: SliderKind, value: u32) {
        let (min, max) = kind.range();
        let r = PixelRect {
            x,
            y: self.sy(y_log),
            width: 150,
            height: 24,
        };
        self.push_control(
            r,
            HitKind::Slider {
                kind,
                value,
                min,
                max,
            },
        );
    }

    pub(crate) fn push_btn(
        &mut self,
        bounds: (i32, i32, u32, u32),
        label: &str,
        tone: ButtonTone,
        action: SettingsAction,
    ) {
        let (x, y_log, width, height) = bounds;
        let r = PixelRect {
            x,
            y: self.sy(y_log),
            width,
            height,
        };
        self.push_control(
            r,
            HitKind::Button {
                label: label.to_owned(),
                tone,
                action,
            },
        );
    }

    fn finish(mut self) -> SettingsLayout {
        self.out.content_height = (self.cursor_y + 24).max(0) as u32;
        self.out
    }
}
