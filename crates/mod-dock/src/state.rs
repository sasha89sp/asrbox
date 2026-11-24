use std::path::PathBuf;

use crate::config::{DockConfig, DockConfigError, DockItem};
use crate::layout::{
    compute_layout, drop_insert_index, hit_test_slot, DockLayout, HoverVisualState, Point, Rect,
    SlotKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockEvent {
    PointerMove(Point),
    PointerLeave,
    AnimationTick {
        dt_ms: u16,
    },
    DropFiles {
        paths: Vec<PathBuf>,
        local_point: Point,
    },
    WorkAreaChanged(Rect),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DockMetrics {
    pub events_processed: u64,
    pub redraws_requested: u64,
    pub drops_accepted: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DockStepEffect {
    pub needs_redraw: bool,
    pub animation_active: bool,
    pub added_item_ids: Vec<u64>,
}

const LAUNCH_BOUNCE_TOTAL_MS: u16 = 460;

#[derive(Debug, Clone)]
pub struct DockInstance {
    config: DockConfig,
    work_area: Rect,
    hovered_item: Option<u64>,
    active_zoom_item: Option<u64>,
    zoom_progress: f32,
    bounce_item: Option<u64>,
    bounce_elapsed_ms: u16,
    layout: DockLayout,
    metrics: DockMetrics,
    next_item_id: u64,
}

impl DockInstance {
    pub fn new(config: DockConfig, work_area: Rect) -> Result<Self, DockConfigError> {
        config.validate()?;
        let next_item_id = config.next_item_id();
        let layout = compute_layout(&config, work_area, HoverVisualState::default());
        Ok(Self {
            config,
            work_area,
            hovered_item: None,
            active_zoom_item: None,
            zoom_progress: 0.0,
            bounce_item: None,
            bounce_elapsed_ms: 0,
            layout,
            metrics: DockMetrics::default(),
            next_item_id,
        })
    }

    pub fn config(&self) -> &DockConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut DockConfig {
        &mut self.config
    }

    pub fn layout(&self) -> &DockLayout {
        &self.layout
    }

    pub fn metrics(&self) -> DockMetrics {
        self.metrics
    }

    pub fn hovered_item(&self) -> Option<u64> {
        self.hovered_item
    }

    pub fn hovered_tooltip(&self) -> Option<String> {
        if self.config.label_mode != crate::labels::DockLabelMode::HoverTooltip {
            return None;
        }
        let id = self.hovered_item?;
        if id == crate::config::SETTINGS_SLOT_ID {
            return Some("Настройки asrbox".to_owned());
        }
        self.config
            .items
            .iter()
            .find(|i| i.id == id && !i.is_separator())
            .map(|i| i.label.clone())
    }

    pub fn refresh_layout(&mut self) {
        self.recompute_layout();
        self.record_redraw();
    }

    pub fn trigger_launch_bounce(&mut self, item_id: u64) -> DockStepEffect {
        if !self.config.launch_animation_enabled {
            return DockStepEffect::default();
        }
        self.bounce_item = Some(item_id);
        self.bounce_elapsed_ms = 0;
        self.recompute_layout();
        self.record_redraw();
        DockStepEffect {
            needs_redraw: true,
            animation_active: true,
            added_item_ids: Vec::new(),
        }
    }

    pub fn move_item_within(&mut self, item_id: u64, mut target_idx: usize) -> bool {
        let Some(old_idx) = self.config.items.iter().position(|i| i.id == item_id) else {
            return false;
        };
        if target_idx > old_idx {
            target_idx = target_idx.saturating_sub(1);
        }
        let target_idx = target_idx.min(self.config.items.len().saturating_sub(1));
        if old_idx == target_idx {
            return false;
        }
        let item = self.config.items.remove(old_idx);
        self.config.items.insert(target_idx, item);
        self.recompute_layout();
        self.record_redraw();
        true
    }

    pub fn take_item(&mut self, item_id: u64) -> Option<DockItem> {
        let pos = self.config.items.iter().position(|i| i.id == item_id)?;
        let item = self.config.items.remove(pos);
        if self.hovered_item == Some(item_id) {
            self.hovered_item = None;
        }
        if self.active_zoom_item == Some(item_id) {
            self.active_zoom_item = None;
            self.zoom_progress = 0.0;
        }
        if self.bounce_item == Some(item_id) {
            self.bounce_item = None;
            self.bounce_elapsed_ms = 0;
        }
        self.recompute_layout();
        self.record_redraw();
        Some(item)
    }

    pub fn insert_existing_item(&mut self, mut item: DockItem, target_idx: usize) -> u64 {
        let new_id = self.next_item_id.max(self.config.next_item_id());
        self.next_item_id = new_id.saturating_add(1);
        item.id = new_id;
        let idx = target_idx.min(self.config.items.len());
        self.config.items.insert(idx, item);
        self.recompute_layout();
        self.record_redraw();
        new_id
    }

    pub fn handle_event(&mut self, event: DockEvent) -> DockStepEffect {
        self.metrics.events_processed = self.metrics.events_processed.saturating_add(1);
        match event {
            DockEvent::PointerMove(point) => self.on_pointer_move(point),
            DockEvent::PointerLeave => self.on_pointer_leave(),
            DockEvent::AnimationTick { dt_ms } => self.on_animation_tick(dt_ms),
            DockEvent::DropFiles { paths, local_point } => self.on_drop(paths, local_point),
            DockEvent::WorkAreaChanged(rect) => self.on_work_area_changed(rect),
        }
    }

    fn on_pointer_move(&mut self, point: Point) -> DockStepEffect {
        let hit = hit_test_slot(&self.layout, point)
            .filter(|slot| !matches!(slot.kind, SlotKind::Separator | SlotKind::DragHandle))
            .map(|slot| slot.item_id);
        if hit == self.hovered_item {
            return DockStepEffect {
                needs_redraw: false,
                animation_active: self.is_animation_pending(),
                added_item_ids: Vec::new(),
            };
        }

        self.hovered_item = hit;
        if let Some(item_id) = hit {
            if self.active_zoom_item != Some(item_id) {
                self.active_zoom_item = Some(item_id);
                self.zoom_progress = 0.0;
            }
            if !self.config.hover_zoom.enabled {
                self.zoom_progress = 0.0;
            } else if self.config.hover_zoom.duration_ms == 0 {
                self.zoom_progress = 1.0;
            }
        } else if self.config.hover_zoom.duration_ms == 0 || !self.config.hover_zoom.enabled {
            self.zoom_progress = 0.0;
            self.active_zoom_item = None;
        }

        self.recompute_layout();
        self.record_redraw();
        DockStepEffect {
            needs_redraw: true,
            animation_active: self.is_animation_pending(),
            added_item_ids: Vec::new(),
        }
    }

    fn on_pointer_leave(&mut self) -> DockStepEffect {
        if self.hovered_item.is_none() && self.active_zoom_item.is_none() {
            return DockStepEffect::default();
        }
        self.hovered_item = None;
        if !self.config.hover_zoom.enabled || self.config.hover_zoom.duration_ms == 0 {
            self.zoom_progress = 0.0;
            self.active_zoom_item = None;
        }
        self.recompute_layout();
        self.record_redraw();
        DockStepEffect {
            needs_redraw: true,
            animation_active: self.is_animation_pending(),
            added_item_ids: Vec::new(),
        }
    }

    fn on_animation_tick(&mut self, dt_ms: u16) -> DockStepEffect {
        if !self.is_animation_pending() || dt_ms == 0 {
            return DockStepEffect::default();
        }
        if self.is_zoom_pending() {
            let target = if self.hovered_item.is_some() { 1.0 } else { 0.0 };
            let duration = f32::from(self.config.hover_zoom.duration_ms).max(1.0);
            let delta = f32::from(dt_ms) / duration;
            if target > self.zoom_progress {
                self.zoom_progress = (self.zoom_progress + delta).min(1.0);
            } else {
                self.zoom_progress = (self.zoom_progress - delta).max(0.0);
                if self.zoom_progress <= 0.0 {
                    self.active_zoom_item = None;
                }
            }
        }
        if self.bounce_item.is_some() {
            self.bounce_elapsed_ms = self.bounce_elapsed_ms.saturating_add(dt_ms);
            if self.bounce_elapsed_ms >= LAUNCH_BOUNCE_TOTAL_MS {
                self.bounce_item = None;
                self.bounce_elapsed_ms = 0;
            }
        }
        self.recompute_layout();
        self.record_redraw();
        DockStepEffect {
            needs_redraw: true,
            animation_active: self.is_animation_pending(),
            added_item_ids: Vec::new(),
        }
    }

    fn on_drop(&mut self, paths: Vec<PathBuf>, local_point: Point) -> DockStepEffect {
        let mut insert_at = drop_insert_index(&self.layout, self.config.edge, local_point);
        let mut added_item_ids = Vec::new();
        for path in paths {
            let item_id = self.next_item_id;
            if let Ok(item) = DockItem::from_dropped_path(item_id, path) {
                let idx = insert_at.min(self.config.items.len());
                self.config.items.insert(idx, item);
                insert_at = idx.saturating_add(1);
                self.next_item_id = self.next_item_id.saturating_add(1);
                self.metrics.drops_accepted = self.metrics.drops_accepted.saturating_add(1);
                added_item_ids.push(item_id);
            }
        }
        if added_item_ids.is_empty() {
            return DockStepEffect {
                needs_redraw: false,
                animation_active: self.is_animation_pending(),
                added_item_ids,
            };
        }
        self.recompute_layout();
        self.record_redraw();
        DockStepEffect {
            needs_redraw: true,
            animation_active: self.is_animation_pending(),
            added_item_ids,
        }
    }

    fn on_work_area_changed(&mut self, rect: Rect) -> DockStepEffect {
        if self.work_area == rect {
            return DockStepEffect {
                needs_redraw: false,
                animation_active: self.is_animation_pending(),
                added_item_ids: Vec::new(),
            };
        }
        self.work_area = rect;
        self.recompute_layout();
        self.record_redraw();
        DockStepEffect {
            needs_redraw: true,
            animation_active: self.is_animation_pending(),
            added_item_ids: Vec::new(),
        }
    }

    fn is_zoom_pending(&self) -> bool {
        if !self.config.hover_zoom.enabled || self.config.hover_zoom.duration_ms == 0 {
            return false;
        }
        match self.hovered_item {
            Some(_) => self.zoom_progress < 1.0,
            None => self.active_zoom_item.is_some() && self.zoom_progress > 0.0,
        }
    }

    fn is_animation_pending(&self) -> bool {
        self.is_zoom_pending() || self.bounce_item.is_some()
    }

    fn recompute_layout(&mut self) {
        let hover = HoverVisualState {
            item_id: self.active_zoom_item,
            progress: self.zoom_progress,
            bounce_item_id: self.bounce_item,
            bounce_offset_px: bounce_offset_px(self.bounce_elapsed_ms),
        };
        self.layout = compute_layout(&self.config, self.work_area, hover);
    }

    fn record_redraw(&mut self) {
        self.metrics.redraws_requested = self.metrics.redraws_requested.saturating_add(1);
    }
}

fn bounce_offset_px(elapsed_ms: u16) -> i32 {
    let pi = std::f32::consts::PI;
    if elapsed_ms < 240 {
        let t = f32::from(elapsed_ms) / 240.0;
        ((pi * t).sin() * 14.0).round() as i32
    } else if elapsed_ms < LAUNCH_BOUNCE_TOTAL_MS {
        let t = f32::from(elapsed_ms - 240) / f32::from(LAUNCH_BOUNCE_TOTAL_MS - 240);
        ((pi * t).sin() * 7.0).round() as i32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    fn work_area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn move_take_and_insert_works_for_items_and_separators() -> Result<(), Box<dyn Error>> {
        let mut cfg = DockConfig::new(1, "Primary");
        cfg.items
            .push(DockItem::from_dropped_path(1, PathBuf::from("a.exe"))?);
        cfg.items.push(DockItem::separator(2));
        cfg.items
            .push(DockItem::from_dropped_path(3, PathBuf::from("b.exe"))?);
        let mut dock = DockInstance::new(cfg, work_area())?;
        assert!(dock.move_item_within(2, 0));
        assert!(dock.config().items[0].is_separator());
        let taken = dock.take_item(2).ok_or("separator missing")?;
        assert_eq!(dock.config().items.len(), 2);
        let new_id = dock.insert_existing_item(taken, 2);
        assert!(new_id > 3);
        assert!(dock.config().items[2].is_separator());
        Ok(())
    }

    #[test]
    fn idle_dock_produces_zero_redraws_and_stops_animation() -> Result<(), Box<dyn Error>> {
        let mut config = DockConfig::new(1, "Primary");
        config
            .items
            .push(DockItem::from_dropped_path(1, PathBuf::from("a.exe"))?);
        let mut dock = DockInstance::new(config, work_area())?;
        let idle = dock.handle_event(DockEvent::AnimationTick { dt_ms: 16 });
        assert!(!idle.needs_redraw && !idle.animation_active);
        let slot = dock.layout().slots[0];
        let pt = Point {
            x: slot.base_rect.x + 10,
            y: slot.base_rect.y + 10,
        };
        assert!(dock.handle_event(DockEvent::PointerMove(pt)).needs_redraw);
        assert!(
            !dock
                .handle_event(DockEvent::AnimationTick { dt_ms: 120 })
                .animation_active
        );
        let b0 = dock.trigger_launch_bounce(1);
        assert!(b0.needs_redraw && b0.animation_active);
        let b_mid = dock.handle_event(DockEvent::AnimationTick { dt_ms: 120 });
        assert!(b_mid.animation_active);
        assert!(dock.layout().slots[0].visual_rect.y < dock.layout().slots[0].base_rect.y);
        let b_end = dock.handle_event(DockEvent::AnimationTick { dt_ms: 400 });
        assert!(!b_end.animation_active);
        Ok(())
    }
}
