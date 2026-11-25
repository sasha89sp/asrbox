use crate::config::{DockConfig, DockEdge};
use crate::layout::{DockLayout, Point, Rect, SlotKind};

pub fn drop_insert_index(layout: &DockLayout, edge: DockEdge, local_point: Point) -> usize {
    let horizontal = edge.is_horizontal();
    let mut item_count = 0usize;
    for slot in &layout.slots {
        if slot.kind == SlotKind::DragHandle {
            continue;
        }
        if slot.kind == SlotKind::SettingsButton {
            break;
        }
        let (origin, len, coord) = if horizontal {
            (slot.base_rect.x, slot.base_rect.width, local_point.x)
        } else {
            (slot.base_rect.y, slot.base_rect.height, local_point.y)
        };
        let midpoint = origin.saturating_add(i32::try_from(len / 2).unwrap_or(0));
        if coord < midpoint {
            return item_count;
        }
        item_count = item_count.saturating_add(1);
    }
    item_count
}

/// Applies free dragged dock coordinates clamped strictly inside `work_area`.
/// Does NOT auto-snap to screen edges or mutate `cfg.edge` — edge attachment
/// is controlled exclusively through Settings or the dock context menu.
pub fn apply_dragged_window_pos(
    cfg: &mut DockConfig,
    work_area: Rect,
    win_w: u32,
    win_h: u32,
    desired_x: i32,
    desired_y: i32,
) {
    let aw = i32::try_from(work_area.width).unwrap_or(1920);
    let ah = i32::try_from(work_area.height).unwrap_or(1080);
    let ww = i32::try_from(win_w).unwrap_or(100);
    let wh = i32::try_from(win_h).unwrap_or(64);
    let max_x = work_area.x.saturating_add(aw.saturating_sub(ww).max(0));
    let max_y = work_area.y.saturating_add(ah.saturating_sub(wh).max(0));
    let cx = desired_x.clamp(work_area.x, max_x);
    let cy = desired_y.clamp(work_area.y, max_y);
    cfg.free_pos = Some((cx, cy));
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;

    use crate::config::DockItem;
    use crate::layout::{compute_layout, HoverVisualState};

    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn unlocked_dock_free_drag_does_not_auto_snap_to_edges() -> Result<(), Box<dyn Error>> {
        let mut dock = DockConfig::new(1, "Free");
        dock.locked = false;
        dock.edge = DockEdge::Bottom;
        dock.items
            .push(DockItem::from_dropped_path(1, PathBuf::from("a.exe"))?);
        dock.items.push(DockItem::separator(2));
        apply_dragged_window_pos(&mut dock, area(), 200, 72, 650, 12);
        assert_eq!(dock.edge, DockEdge::Bottom);
        assert_eq!(dock.free_pos, Some((650, 12)));
        let layout = compute_layout(&dock, area(), HoverVisualState::default());
        assert_eq!(layout.slots[0].kind, SlotKind::DragHandle);
        let far_right = Point {
            x: 9999,
            y: layout.bar_rect.y + 10,
        };
        assert_eq!(drop_insert_index(&layout, DockEdge::Bottom, far_right), 2);
        Ok(())
    }
}
