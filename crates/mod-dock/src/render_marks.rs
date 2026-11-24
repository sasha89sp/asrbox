use asrbox_platform_api::{BgraFrame, FluentPalette};

use crate::config::DockEdge;
use crate::layout::{DockLayout, DockSlotLayout, Rect, SlotKind};
use crate::render_ops::blend_pixel;

pub(crate) fn draw_drag_handle(
    frame: &mut BgraFrame,
    rect: Rect,
    edge: DockEdge,
    pal: &FluentPalette,
) {
    let cx = rect.x + i32::try_from(rect.width / 2).unwrap_or(0);
    let cy = rect.y + i32::try_from(rect.height / 2).unwrap_or(0);
    let m = pal.text_muted;
    let dot = [m[0], m[1], m[2], 160];
    let offsets = if edge.is_horizontal() {
        [(-2, -6), (2, -6), (-2, 0), (2, 0), (-2, 6), (2, 6)]
    } else {
        [(-6, -2), (-6, 2), (0, -2), (0, 2), (6, -2), (6, 2)]
    };
    for (ox, oy) in offsets {
        for dy in 0..=1 {
            for dx in 0..=1 {
                blend_pixel(frame, cx + ox + dx, cy + oy + dy, dot);
            }
        }
    }
}

pub(crate) fn draw_running_dot(
    frame: &mut BgraFrame,
    base: Rect,
    edge: DockEdge,
    pal: &FluentPalette,
) {
    let (cx, cy) = match edge {
        DockEdge::Bottom => (
            base.x + i32::try_from(base.width / 2).unwrap_or(0),
            base.y + i32::try_from(base.height).unwrap_or(0) + 5,
        ),
        DockEdge::Top => (
            base.x + i32::try_from(base.width / 2).unwrap_or(0),
            base.y - 5,
        ),
        DockEdge::Left => (
            base.x - 5,
            base.y + i32::try_from(base.height / 2).unwrap_or(0),
        ),
        DockEdge::Right => (
            base.x + i32::try_from(base.width).unwrap_or(0) + 5,
            base.y + i32::try_from(base.height / 2).unwrap_or(0),
        ),
    };
    let acc = pal.accent;
    for dy in -3..=3 {
        for dx in -3..=3 {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d <= 2.4 {
                let cov = (2.4 - d).clamp(0.0, 1.0);
                let a = (245.0 * cov).round() as u8;
                blend_pixel(frame, cx + dx, cy + dy, [acc[0], acc[1], acc[2], a]);
            }
        }
    }
}

pub(crate) fn draw_insert_marker(
    frame: &mut BgraFrame,
    layout: &DockLayout,
    edge: DockEdge,
    idx: usize,
    pal: &FluentPalette,
) {
    let items: Vec<DockSlotLayout> = layout
        .slots
        .iter()
        .copied()
        .filter(|s| matches!(s.kind, SlotKind::Shortcut | SlotKind::Separator))
        .collect();
    let accent = [pal.accent[0], pal.accent[1], pal.accent[2], 245];
    if edge.is_horizontal() {
        let x = match items.get(idx) {
            Some(s) => s.base_rect.x - 4,
            None => items.last().map_or(layout.bar_rect.x + 12, |s| {
                s.base_rect.x + i32::try_from(s.base_rect.width).unwrap_or(0) + 4
            }),
        };
        let y0 = layout.bar_rect.y + 8;
        let y1 = layout.bar_rect.y + i32::try_from(layout.bar_rect.height).unwrap_or(48) - 8;
        for py in y0..y1 {
            blend_pixel(frame, x, py, accent);
            blend_pixel(frame, x + 1, py, accent);
        }
    } else {
        let y = match items.get(idx) {
            Some(s) => s.base_rect.y - 4,
            None => items.last().map_or(layout.bar_rect.y + 12, |s| {
                s.base_rect.y + i32::try_from(s.base_rect.height).unwrap_or(0) + 4
            }),
        };
        let x0 = layout.bar_rect.x + 8;
        let x1 = layout.bar_rect.x + i32::try_from(layout.bar_rect.width).unwrap_or(48) - 8;
        for px in x0..x1 {
            blend_pixel(frame, px, y, accent);
            blend_pixel(frame, px, y + 1, accent);
        }
    }
}

pub(crate) fn draw_separator(
    frame: &mut BgraFrame,
    rect: Rect,
    edge: DockEdge,
    dimmed: bool,
    pal: &FluentPalette,
) {
    let cx = rect.x + i32::try_from(rect.width / 2).unwrap_or(0);
    let cy = rect.y + i32::try_from(rect.height / 2).unwrap_or(0);
    let d = pal.divider;
    let line = [d[0], d[1], d[2], if dimmed { 48 } else { 180 }];
    if edge.is_horizontal() {
        let m = (i32::try_from(rect.height).unwrap_or(40) / 6).max(4);
        let y0 = rect.y.saturating_add(m);
        let y1 = rect
            .y
            .saturating_add(i32::try_from(rect.height).unwrap_or(40))
            .saturating_sub(m);
        for py in y0..y1 {
            blend_pixel(frame, cx, py, line);
        }
    } else {
        let m = (i32::try_from(rect.width).unwrap_or(40) / 6).max(4);
        let x0 = rect.x.saturating_add(m);
        let x1 = rect
            .x
            .saturating_add(i32::try_from(rect.width).unwrap_or(40))
            .saturating_sub(m);
        for px in x0..x1 {
            blend_pixel(frame, px, cy, line);
        }
    }
}
