use asrbox_platform_api::{PixelRect, SettingsUiEvent};

use crate::layout::{rect_contains, ControlItem, HitKind, MIN_SETTINGS_HEIGHT, MIN_SETTINGS_WIDTH};
use crate::types::{BrowserTab, SettingsAction, SettingsSnapshot, SliderKind};
use crate::SettingsUiState;

impl SettingsUiState {
    pub fn handle_event(
        &mut self,
        ev: SettingsUiEvent,
        snap: &SettingsSnapshot,
    ) -> (bool, Option<SettingsAction>) {
        match ev {
            SettingsUiEvent::Resized { width, height } => {
                self.width = width.max(MIN_SETTINGS_WIDTH);
                self.height = height.max(MIN_SETTINGS_HEIGHT);
                (true, None)
            }
            SettingsUiEvent::Wheel { delta } => {
                let step = if delta > 0 { -40 } else { 40 };
                let max_scroll = (self.layout.content_height as i32 - self.height as i32).max(0);
                let next = (self.scroll_y + step).clamp(0, max_scroll);
                let changed = next != self.scroll_y;
                self.scroll_y = next;
                (changed, None)
            }
            SettingsUiEvent::PointerMove { x, y } => self.on_pointer_move(x, y, snap),
            SettingsUiEvent::MouseDown { x, y } => self.on_mouse_down(x, y, snap),
            SettingsUiEvent::MouseUp { .. } => {
                self.last_slider_val = None;
                (self.dragging_slider.take().is_some(), None)
            }
            SettingsUiEvent::InputChanged { slot, text } => {
                if slot == 3 && self.search_query != text {
                    self.search_query = text;
                    self.scroll_y = 0;
                    return (true, None);
                }
                (false, None)
            }
            SettingsUiEvent::AnimTick { .. } => (self.anim_start.is_some(), None),
            SettingsUiEvent::Closed => {
                self.dragging_slider = None;
                self.last_slider_val = None;
                self.hovered_id = None;
                self.anim_start = None;
                (false, None)
            }
        }
    }

    fn on_pointer_move(
        &mut self,
        x: i32,
        y: i32,
        snap: &SettingsSnapshot,
    ) -> (bool, Option<SettingsAction>) {
        if let Some((kind, rect)) = self.dragging_slider {
            let val = slider_value_from_x(kind, rect, x);
            if self.last_slider_val == Some(val) {
                return (false, None);
            }
            self.last_slider_val = Some(val);
            return (true, self.slider_action(kind, val, snap));
        }
        let hit = self.hit_control(x, y).map(|c| c.id);
        let changed = hit != self.hovered_id;
        self.hovered_id = hit;
        (changed, None)
    }

    fn on_mouse_down(
        &mut self,
        x: i32,
        y: i32,
        snap: &SettingsSnapshot,
    ) -> (bool, Option<SettingsAction>) {
        let Some(ctrl) = self.hit_control(x, y).cloned() else {
            return (false, None);
        };
        match ctrl.kind {
            HitKind::Tab(tab) => {
                self.active_tab = tab;
                self.active_browser_tab = BrowserTab::Main;
                self.scroll_y = 0;
                (true, None)
            }
            HitKind::BrowserTabSelect { index, .. } => {
                if let Some(tab) = self.browser_tabs.get(index).cloned() {
                    self.active_browser_tab = tab;
                    self.scroll_y = 0;
                }
                (true, None)
            }
            HitKind::BrowserTabClose { index } => {
                self.close_browser_tab(index);
                (true, None)
            }
            HitKind::HeaderButton { action, .. } => (true, Some(action)),
            HitKind::SelectDock(idx) => {
                self.selected_dock = idx;
                self.selected_dock_item = None;
                self.last_dock_input_key = None;
                (true, None)
            }
            HitKind::SelectWidgetSubTab { sub, .. } => {
                self.selected_widget = sub;
                self.scroll_y = 0;
                (true, None)
            }
            HitKind::SelectDockItem { item_id, .. } => {
                self.selected_dock_item = Some(item_id);
                self.last_dock_input_key = None;
                (true, None)
            }
            HitKind::StartEditServerDir { server_id, .. } => {
                if let Some(srv) = snap.backup.servers.iter().find(|s| s.id == server_id) {
                    self.pending_aux_prefill = Some(srv.remote_dir.clone());
                }
                self.editing_server_dir = Some(server_id);
                (true, None)
            }
            HitKind::CancelEditServerDir { .. } => {
                self.editing_server_dir = None;
                (true, None)
            }
            HitKind::AccentSwatch { preset, .. } => {
                (true, Some(SettingsAction::SetAccentPreset(preset)))
            }
            HitKind::Toggle { action, .. }
            | HitKind::Segment { action, .. }
            | HitKind::Chip { action, .. }
            | HitKind::Button { action, .. } => {
                if matches!(action, SettingsAction::SaveWebDavServerRemoteDir(_)) {
                    self.editing_server_dir = None;
                }
                (true, Some(action))
            }
            HitKind::Slider { kind, .. } => {
                self.dragging_slider = Some((kind, ctrl.rect));
                let val = slider_value_from_x(kind, ctrl.rect, x);
                self.last_slider_val = Some(val);
                (true, self.slider_action(kind, val, snap))
            }
        }
    }

    fn hit_control(&self, x: i32, y: i32) -> Option<&ControlItem> {
        if let Some(close) = self.layout.controls.iter().find(|c| {
            matches!(c.kind, HitKind::BrowserTabClose { .. }) && rect_contains(c.rect, x, y)
        }) {
            return Some(close);
        }
        let top_y = self.layout.content_top_y;
        self.layout.controls.iter().find(|c| {
            let is_chrome = matches!(
                c.kind,
                HitKind::Tab(_)
                    | HitKind::BrowserTabSelect { .. }
                    | HitKind::BrowserTabClose { .. }
                    | HitKind::HeaderButton { .. }
            );
            if !is_chrome && y < top_y {
                return false;
            }
            rect_contains(c.rect, x, y)
        })
    }

    fn slider_action(
        &self,
        kind: SliderKind,
        val: u32,
        snap: &SettingsSnapshot,
    ) -> Option<SettingsAction> {
        match kind {
            SliderKind::DockIconSize => {
                snap.docks
                    .get(self.selected_dock)
                    .map(|d| SettingsAction::SetDockIconSize {
                        dock_id: d.id.clone(),
                        size_px: val,
                    })
            }
            SliderKind::DockOpacity => {
                snap.docks
                    .get(self.selected_dock)
                    .map(|d| SettingsAction::SetDockOpacity {
                        dock_id: d.id.clone(),
                        opacity: val as u8,
                    })
            }
            SliderKind::CalendarOpacity => Some(SettingsAction::SetCalendarOpacity(val as u8)),
            SliderKind::CalendarWidth => Some(SettingsAction::SetCalendarWidth(val)),
            SliderKind::CalendarRows => Some(SettingsAction::SetCalendarRows(val as u8)),
            SliderKind::SysMonitorOpacity => Some(SettingsAction::SetSysMonitorOpacity(val as u8)),
            SliderKind::SysMonitorWidth => Some(SettingsAction::SetSysMonitorWidth(val)),
            SliderKind::SysMonitorHeight => Some(SettingsAction::SetSysMonitorHeight(val)),
            SliderKind::OverlayOpacity => Some(SettingsAction::SetOverlayOpacity(val as u8)),
            SliderKind::OverlayWidth => Some(SettingsAction::SetOverlayWidth(val)),
            SliderKind::OverlayHeight => Some(SettingsAction::SetOverlayHeight(val)),
        }
    }
}

fn slider_value_from_x(kind: SliderKind, rect: PixelRect, x: i32) -> u32 {
    let (min, max) = kind.range();
    let ratio = ((x - rect.x) as f32 / rect.width.max(1) as f32).clamp(0.0, 1.0);
    min + ((max - min) as f32 * ratio).round() as u32
}
