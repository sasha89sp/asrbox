use std::path::PathBuf;

use crate::config::{DockConfig, DockConfigError, DockItem};
use crate::layout::{DockLayout, HoverVisualState, Point, Rect};

use crate::layout::{compute_layout, drop_insert_index, hit_test_item};

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

#[derive(Debug, Clone)]
pub struct DockInstance {
    config: DockConfig,
    work_area: Rect,
    hovered_item: Option<u64>,
    active_zoom_item: Option<u64>,
    zoom_progress: f32,
    layout: DockLayout,
    metrics: DockMetrics,
    next_item_id: u64,
}

impl DockInstance {
    pub fn new(config: DockConfig, work_area: Rect) -> Result<Self, DockConfigError> {
        config.validate()?;
        let next_item_id = config
            .items
            .iter()
            .map(|item| item.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let layout = compute_layout(&config, work_area, HoverVisualState::default());
        Ok(Self {
            config,
            work_area,
            hovered_item: None,
            active_zoom_item: None,
            zoom_progress: 0.0,
            layout,
            metrics: DockMetrics::default(),
            next_item_id,
        })
    }

    pub fn config(&self) -> &DockConfig {
        &self.config
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
        let hit = hit_test_item(&self.layout, point);
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

        let target = if self.hovered_item.is_some() {
            1.0
        } else {
            0.0
        };
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

    fn is_animation_pending(&self) -> bool {
        if !self.config.hover_zoom.enabled || self.config.hover_zoom.duration_ms == 0 {
            return false;
        }
        match self.hovered_item {
            Some(_) => self.zoom_progress < 1.0,
            None => self.active_zoom_item.is_some() && self.zoom_progress > 0.0,
        }
    }

    fn recompute_layout(&mut self) {
        let hover = HoverVisualState {
            item_id: self.active_zoom_item,
            progress: self.zoom_progress,
        };
        self.layout = compute_layout(&self.config, self.work_area, hover);
    }

    fn record_redraw(&mut self) {
        self.metrics.redraws_requested = self.metrics.redraws_requested.saturating_add(1);
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
    fn idle_dock_produces_zero_redraws_and_stops_animation() -> Result<(), Box<dyn Error>> {
        let mut config = DockConfig::new(1, "Primary");
        let path = PathBuf::from("C:/Apps/Firefox.exe");
        config.items.push(DockItem::from_dropped_path(1, path)?);
        let mut dock = DockInstance::new(config, work_area())?;

        let idle_tick = dock.handle_event(DockEvent::AnimationTick { dt_ms: 16 });
        assert!(!idle_tick.needs_redraw);
        assert!(!idle_tick.animation_active);

        let slot = dock
            .layout()
            .slots
            .first()
            .copied()
            .ok_or("missing first slot")?;
        let center = Point {
            x: slot.base_rect.x + 10,
            y: slot.base_rect.y + 10,
        };

        let enter = dock.handle_event(DockEvent::PointerMove(center));
        assert!(enter.needs_redraw);
        assert!(enter.animation_active);

        let finish = dock.handle_event(DockEvent::AnimationTick { dt_ms: 120 });
        assert!(finish.needs_redraw);
        assert!(!finish.animation_active);

        let moved = Point {
            x: center.x + 2,
            y: center.y,
        };
        let same_icon_move = dock.handle_event(DockEvent::PointerMove(moved));
        assert!(!same_icon_move.needs_redraw);
        assert!(!same_icon_move.animation_active);

        let leave = dock.handle_event(DockEvent::PointerLeave);
        assert!(leave.needs_redraw);
        assert!(leave.animation_active);

        let settled = dock.handle_event(DockEvent::AnimationTick { dt_ms: 120 });
        assert!(settled.needs_redraw);
        assert!(!settled.animation_active);
        Ok(())
    }

    #[test]
    fn drop_files_adds_items_and_requests_redraw() -> Result<(), Box<dyn Error>> {
        let config = DockConfig::new(1, "Primary");
        let mut dock = DockInstance::new(config, work_area())?;

        let effect = dock.handle_event(DockEvent::DropFiles {
            paths: vec![
                PathBuf::from("C:/Apps/Note.exe"),
                PathBuf::from("C:/Apps/Terminal.lnk"),
            ],
            local_point: Point { x: 0, y: 0 },
        });
        assert!(effect.needs_redraw);
        assert_eq!(effect.added_item_ids.len(), 2);
        assert_eq!(dock.config().items.len(), 2);
        assert_eq!(dock.metrics().drops_accepted, 2);
        Ok(())
    }
}
