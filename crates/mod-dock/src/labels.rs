use std::ffi::OsStr;
use std::path::Path;

use asrbox_platform_api::{BgraFrame, TextAlign, WidgetTextCmd};

use crate::config::DockEdge;
use crate::layout::{DockSlotLayout, Rect};
use crate::render_ops::{blend_pixel, rounded_rect_sdf};

pub const LABEL_STRIP_H: u32 = 18;
pub const LABEL_SIDE_W: u32 = 96;
pub const TEXT_PILL_H_HORIZ: u32 = 30;
pub const TEXT_PILL_H_VERT: u32 = 28;
pub const TEXT_PILL_W_VERT: u32 = 136;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockLabelMode {
    #[default]
    IconsOnly,
    HoverTooltip,
    IconAndLabel,
    LabelsOnly,
}

impl DockLabelMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IconsOnly => "icons",
            Self::HoverTooltip => "hover",
            Self::IconAndLabel => "icon_label",
            Self::LabelsOnly => "labels",
        }
    }

    pub fn from_str_value(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "hover" => Self::HoverTooltip,
            "icon_label" => Self::IconAndLabel,
            "labels" => Self::LabelsOnly,
            _ => Self::IconsOnly,
        }
    }

    pub fn to_index(self) -> u8 {
        match self {
            Self::IconsOnly => 0,
            Self::HoverTooltip => 1,
            Self::IconAndLabel => 2,
            Self::LabelsOnly => 3,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        match idx {
            1 => Self::HoverTooltip,
            2 => Self::IconAndLabel,
            3 => Self::LabelsOnly,
            _ => Self::IconsOnly,
        }
    }
}

pub fn default_label_for_path(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

pub fn slot_cross_len(mode: DockLabelMode, edge: DockEdge, icon_px: u32) -> u32 {
    match mode {
        DockLabelMode::IconsOnly | DockLabelMode::HoverTooltip => icon_px,
        DockLabelMode::IconAndLabel => {
            if edge.is_horizontal() {
                icon_px.saturating_add(LABEL_STRIP_H)
            } else {
                icon_px.saturating_add(LABEL_SIDE_W)
            }
        }
        DockLabelMode::LabelsOnly => {
            if edge.is_horizontal() {
                TEXT_PILL_H_HORIZ
            } else {
                TEXT_PILL_W_VERT
            }
        }
    }
}

pub fn shortcut_main_len(mode: DockLabelMode, edge: DockEdge, icon_px: u32, label: &str) -> u32 {
    match mode {
        DockLabelMode::IconsOnly | DockLabelMode::HoverTooltip => icon_px,
        DockLabelMode::IconAndLabel => {
            if edge.is_horizontal() {
                icon_px.max(66)
            } else {
                icon_px
            }
        }
        DockLabelMode::LabelsOnly => {
            if edge.is_horizontal() {
                estimate_pill_width(label)
            } else {
                TEXT_PILL_H_VERT
            }
        }
    }
}

pub fn settings_btn_main_len(mode: DockLabelMode, edge: DockEdge, icon_px: u32) -> u32 {
    match mode {
        DockLabelMode::LabelsOnly => {
            if edge.is_horizontal() {
                TEXT_PILL_H_HORIZ
            } else {
                TEXT_PILL_H_VERT
            }
        }
        _ => icon_px,
    }
}

pub fn estimate_pill_width(label: &str) -> u32 {
    let chars = u32::try_from(label.chars().count()).unwrap_or(8);
    chars.saturating_mul(7).saturating_add(24).clamp(58, 132)
}

pub fn icon_and_label_rects(slot: DockSlotLayout, edge: DockEdge, icon_px: u32) -> (Rect, Rect) {
    let base = slot.base_rect;
    let scaled_icon = ((icon_px as f32) * slot.scale).round().max(16.0) as u32;
    if edge.is_horizontal() {
        let ix = base.x + (base.width as i32 - scaled_icon as i32) / 2;
        let iy = if edge == DockEdge::Bottom {
            base.y - scaled_icon.saturating_sub(icon_px) as i32
        } else {
            base.y
        };
        let icon_rect = Rect {
            x: ix,
            y: iy,
            width: scaled_icon,
            height: scaled_icon,
        };
        let text_rect = Rect {
            x: base.x - 2,
            y: base.y + icon_px as i32 + 1,
            width: base.width + 4,
            height: LABEL_STRIP_H.saturating_sub(1),
        };
        (icon_rect, text_rect)
    } else {
        let icon_rect = Rect {
            x: base.x,
            y: base.y + (base.height as i32 - scaled_icon as i32) / 2,
            width: scaled_icon,
            height: scaled_icon,
        };
        let text_rect = Rect {
            x: base.x + icon_px as i32 + 6,
            y: base.y,
            width: base.width.saturating_sub(icon_px + 8),
            height: base.height,
        };
        (icon_rect, text_rect)
    }
}

pub fn push_icon_label_text(
    texts: &mut Vec<WidgetTextCmd>,
    rect: Rect,
    label: &str,
    edge: DockEdge,
    dimmed: bool,
) {
    if label.trim().is_empty() || rect.width < 12 || rect.height < 10 {
        return;
    }
    texts.push(WidgetTextCmd {
        x: rect.x,
        y: rect.y,
        width: rect.width as i32,
        height: rect.height as i32,
        text: label.to_owned(),
        font_size_px: 11,
        bold: false,
        color_rgba: [236, 238, 244, if dimmed { 110 } else { 238 }],
        align: if edge.is_horizontal() {
            TextAlign::Center
        } else {
            TextAlign::Left
        },
    });
}

pub fn draw_labels_only_pill(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    rect: Rect,
    label: &str,
    hovered: bool,
    dimmed: bool,
    running: bool,
) {
    let alpha_mul = if dimmed { 0.38 } else { 1.0 };
    let radius = 8.0f32;
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = (rect.x + rect.width as i32).min(frame.width as i32);
    let y1 = (rect.y + rect.height as i32).min(frame.height as i32);
    let bg = if hovered { [54, 57, 68] } else { [40, 42, 50] };
    let border = if hovered || running {
        [232, 122, 72]
    } else {
        [72, 76, 90]
    };
    let border_a = if hovered {
        180.0
    } else if running {
        125.0
    } else {
        90.0
    };

    for py in y0..y1 {
        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, rect, radius);
            if dist <= 0.6 {
                let cov = (0.6 - dist).clamp(0.0, 1.0);
                let fill_a = (228.0 * cov * alpha_mul).round() as u8;
                blend_pixel(frame, px, py, [bg[0], bg[1], bg[2], fill_a]);
                if dist > -1.25 {
                    let ba = (border_a * cov * alpha_mul).round() as u8;
                    blend_pixel(frame, px, py, [border[0], border[1], border[2], ba]);
                }
            }
        }
    }

    let left_pad = if running {
        draw_pill_dot(
            frame,
            rect.x + 10,
            rect.y + (rect.height as i32) / 2,
            alpha_mul,
        );
        18
    } else {
        8
    };
    texts.push(WidgetTextCmd {
        x: rect.x + left_pad,
        y: rect.y,
        width: (rect.width as i32 - left_pad - 8).max(12),
        height: rect.height as i32,
        text: label.to_owned(),
        font_size_px: 12,
        bold: hovered,
        color_rgba: [238, 240, 244, if dimmed { 110 } else { 245 }],
        align: if running {
            TextAlign::Left
        } else {
            TextAlign::Center
        },
    });
}

fn draw_pill_dot(frame: &mut BgraFrame, cx: i32, cy: i32, alpha_mul: f32) {
    for dy in -3..=3 {
        for dx in -3..=3 {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d <= 2.4 {
                let cov = (2.4 - d).clamp(0.0, 1.0);
                let a = (242.0 * cov * alpha_mul).round() as u8;
                blend_pixel(frame, cx + dx, cy + dy, [236, 134, 86, a]);
            }
        }
    }
}
