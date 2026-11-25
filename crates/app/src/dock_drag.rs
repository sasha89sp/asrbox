#![cfg(windows)]

use asrbox_mod_dock::{
    apply_dragged_window_pos, drop_insert_index, hit_test_slot, DockRenderOverlay, Point, Rect,
    SlotKind,
};
use asrbox_platform_api::{DockPlatform, PixelPoint};

use crate::docks::DockRuntime;

const DRAG_THRESHOLD_PX: i32 = 6;
const OUTSIDE_MARGIN_PX: i32 = 24;

#[derive(Debug, Clone)]
enum DragMode {
    ItemPending {
        source_dock_id: u64,
        item_id: u64,
        slot_kind: SlotKind,
        start_screen: PixelPoint,
    },
    ItemDragging {
        source_dock_id: u64,
        item_id: u64,
        target_dock_id: Option<u64>,
        insert_idx: Option<usize>,
    },
    WindowDragging {
        dock_id: u64,
        start_screen: PixelPoint,
        start_win_x: i32,
        start_win_y: i32,
        moved: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragReleaseOutcome {
    None,
    ClickSlot {
        dock_id: u64,
        slot_kind: SlotKind,
        item_id: u64,
    },
    Mutated(String),
}

#[derive(Default)]
pub struct DockDragState {
    mode: Option<DragMode>,
}

impl DockDragState {
    pub fn cancel(&mut self) -> bool {
        self.mode.take().is_some()
    }

    pub fn on_left_down(
        &mut self,
        docks: &[DockRuntime],
        dock_id: u64,
        local_pt: Point,
        screen_pt: PixelPoint,
    ) -> bool {
        self.mode = None;
        let Some(dock) = docks.iter().find(|d| d.instance.config().id == dock_id) else {
            return false;
        };
        let win = dock.instance.layout().window_rect;
        if let Some(slot) = hit_test_slot(dock.instance.layout(), local_pt) {
            match slot.kind {
                SlotKind::DragHandle => {
                    if !dock.instance.config().locked {
                        self.mode = Some(DragMode::WindowDragging {
                            dock_id,
                            start_screen: screen_pt,
                            start_win_x: win.x,
                            start_win_y: win.y,
                            moved: false,
                        });
                    }
                    false
                }
                SlotKind::Shortcut | SlotKind::Separator => {
                    self.mode = Some(DragMode::ItemPending {
                        source_dock_id: dock_id,
                        item_id: slot.item_id,
                        slot_kind: slot.kind,
                        start_screen: screen_pt,
                    });
                    slot.kind == SlotKind::Shortcut
                }
                SlotKind::SettingsButton => {
                    self.mode = Some(DragMode::ItemPending {
                        source_dock_id: dock_id,
                        item_id: slot.item_id,
                        slot_kind: SlotKind::SettingsButton,
                        start_screen: screen_pt,
                    });
                    true
                }
            }
        } else if !dock.instance.config().locked {
            self.mode = Some(DragMode::WindowDragging {
                dock_id,
                start_screen: screen_pt,
                start_win_x: win.x,
                start_win_y: win.y,
                moved: false,
            });
            false
        } else {
            false
        }
    }

    pub fn on_drag_move(
        &mut self,
        docks: &mut [DockRuntime],
        work_area: Rect,
        screen_pt: PixelPoint,
    ) -> bool {
        let Some(mode) = self.mode.clone() else {
            return false;
        };
        match mode {
            DragMode::ItemPending {
                source_dock_id,
                item_id,
                slot_kind,
                start_screen,
            } => {
                if slot_kind == SlotKind::SettingsButton {
                    return false;
                }
                if docks
                    .iter()
                    .find(|d| d.instance.config().id == source_dock_id)
                    .is_some_and(|d| d.instance.config().lock_items)
                {
                    return false;
                }
                let dist =
                    (screen_pt.x - start_screen.x).abs() + (screen_pt.y - start_screen.y).abs();
                if dist < DRAG_THRESHOLD_PX {
                    return false;
                }
                let (target_dock_id, insert_idx) = find_drop_target(docks, screen_pt);
                self.mode = Some(DragMode::ItemDragging {
                    source_dock_id,
                    item_id,
                    target_dock_id,
                    insert_idx,
                });
                true
            }
            DragMode::ItemDragging {
                source_dock_id,
                item_id,
                target_dock_id: prev_t,
                insert_idx: prev_i,
            } => {
                let (target_dock_id, insert_idx) = find_drop_target(docks, screen_pt);
                self.mode = Some(DragMode::ItemDragging {
                    source_dock_id,
                    item_id,
                    target_dock_id,
                    insert_idx,
                });
                prev_t != target_dock_id || prev_i != insert_idx
            }
            DragMode::WindowDragging {
                dock_id,
                start_screen,
                start_win_x,
                start_win_y,
                mut moved,
            } => {
                let dx = screen_pt.x - start_screen.x;
                let dy = screen_pt.y - start_screen.y;
                if !moved && dx.abs() + dy.abs() >= 3 {
                    moved = true;
                }
                if !moved {
                    return false;
                }
                if let Some(dock) = docks.iter_mut().find(|d| d.instance.config().id == dock_id) {
                    let wr = dock.instance.layout().window_rect;
                    apply_dragged_window_pos(
                        dock.instance.config_mut(),
                        work_area,
                        wr.width,
                        wr.height,
                        start_win_x + dx,
                        start_win_y + dy,
                    );
                    dock.instance.refresh_layout();
                }
                self.mode = Some(DragMode::WindowDragging {
                    dock_id,
                    start_screen,
                    start_win_x,
                    start_win_y,
                    moved,
                });
                true
            }
        }
    }

    pub fn on_left_up(
        &mut self,
        docks: &mut [DockRuntime],
        host: &dyn DockPlatform,
        screen_pt: PixelPoint,
    ) -> DragReleaseOutcome {
        let Some(mode) = self.mode.take() else {
            return DragReleaseOutcome::None;
        };
        match mode {
            DragMode::ItemPending {
                source_dock_id,
                item_id,
                slot_kind,
                ..
            } => DragReleaseOutcome::ClickSlot {
                dock_id: source_dock_id,
                slot_kind,
                item_id,
            },
            DragMode::WindowDragging { dock_id, moved, .. } => {
                if moved {
                    DragReleaseOutcome::Mutated(format!("dock[{dock_id}] перемещена"))
                } else {
                    DragReleaseOutcome::None
                }
            }
            DragMode::ItemDragging {
                source_dock_id,
                item_id,
                ..
            } => finish_item_drag(docks, host, source_dock_id, item_id, screen_pt),
        }
    }

    pub fn overlay_for(&self, dock: &DockRuntime) -> DockRenderOverlay {
        let mut ov = DockRenderOverlay::default();
        let id = dock.instance.config().id;
        match self.mode {
            Some(DragMode::ItemPending {
                source_dock_id,
                item_id,
                slot_kind,
                ..
            }) if id == source_dock_id && slot_kind != SlotKind::Separator => {
                ov.pressed_item_id = Some(item_id);
            }
            Some(DragMode::ItemDragging {
                source_dock_id,
                item_id,
                target_dock_id,
                insert_idx,
            }) => {
                if id == source_dock_id {
                    ov.dragged_item_id = Some(item_id);
                }
                if target_dock_id == Some(id) {
                    ov.insert_before_index = insert_idx;
                }
            }
            _ => {}
        }
        ov
    }
}

fn find_drop_target(docks: &[DockRuntime], screen_pt: PixelPoint) -> (Option<u64>, Option<usize>) {
    let pt = Point {
        x: screen_pt.x,
        y: screen_pt.y,
    };
    for dock in docks.iter().filter(|d| d.window_open) {
        let wr = dock.instance.layout().window_rect;
        if wr.expanded(OUTSIDE_MARGIN_PX).contains(pt) {
            let local = Point {
                x: pt.x - wr.x,
                y: pt.y - wr.y,
            };
            let idx = drop_insert_index(dock.instance.layout(), dock.instance.config().edge, local);
            return (Some(dock.instance.config().id), Some(idx));
        }
    }
    (None, None)
}

fn finish_item_drag(
    docks: &mut [DockRuntime],
    host: &dyn DockPlatform,
    source_dock_id: u64,
    item_id: u64,
    screen_pt: PixelPoint,
) -> DragReleaseOutcome {
    let (target_dock_id, insert_idx) = find_drop_target(docks, screen_pt);
    let Some(src_pos) = docks
        .iter()
        .position(|d| d.instance.config().id == source_dock_id)
    else {
        return DragReleaseOutcome::None;
    };
    match (target_dock_id, insert_idx) {
        (Some(tid), Some(idx)) if tid == source_dock_id => {
            if docks[src_pos].instance.move_item_within(item_id, idx) {
                DragReleaseOutcome::Mutated(format!(
                    "dock[{source_dock_id}] перемещён элемент #{item_id} -> {idx}"
                ))
            } else {
                DragReleaseOutcome::None
            }
        }
        (Some(tid), Some(idx)) => {
            let Some(dst_pos) = docks.iter().position(|d| d.instance.config().id == tid) else {
                return DragReleaseOutcome::None;
            };
            let Some(item) = docks[src_pos].instance.take_item(item_id) else {
                return DragReleaseOutcome::None;
            };
            let icon = docks[src_pos].icons.remove(&item_id);
            let exe = docks[src_pos].resolved_exes.remove(&item_id);
            docks[src_pos].running_items.remove(&item_id);
            let target_path = item.target.clone();
            let new_id = docks[dst_pos].instance.insert_existing_item(item, idx);
            if let Some(img) = icon {
                docks[dst_pos].icons.insert(new_id, img);
            }
            if let Some(name) = exe.or_else(|| host.resolve_target_exe_name(&target_path)) {
                docks[dst_pos].resolved_exes.insert(new_id, name);
            }
            DragReleaseOutcome::Mutated(format!(
                "перенос элемента из dock[{source_dock_id}] -> dock[{tid}] (поз. {idx})"
            ))
        }
        _ => {
            let Some(removed) = docks[src_pos].instance.take_item(item_id) else {
                return DragReleaseOutcome::None;
            };
            docks[src_pos].icons.remove(&item_id);
            docks[src_pos].resolved_exes.remove(&item_id);
            docks[src_pos].running_items.remove(&item_id);
            let kind = if removed.is_separator() {
                "разделитель".to_owned()
            } else {
                format!("значок «{}»", removed.label)
            };
            DragReleaseOutcome::Mutated(format!(
                "dock[{source_dock_id}] удалён {kind} (перетаскивание на рабочий стол)"
            ))
        }
    }
}
