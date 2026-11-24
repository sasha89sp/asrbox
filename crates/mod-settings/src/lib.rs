pub mod draw;
pub mod layout;
pub mod layout_backup;
pub mod layout_cal;
pub mod layout_dock;
pub mod layout_overlay;
pub mod layout_sys;
pub mod layout_tabs;
pub mod render;
pub mod types;

use asrbox_platform_api::{PixelRect, SettingsInputRects, SettingsUiEvent, WidgetFrame};

pub use layout::{build_layout, MIN_SETTINGS_HEIGHT, MIN_SETTINGS_WIDTH};
pub use render::render_settings_frame;
pub use types::{
    AboutSettingsView, BackupSettingsView, CalendarSettingsView, CalendarSourceView,
    DebugSettingsView, DockItemView, DockSettingsView, ModuleStatusView, OverlaySettingsView,
    SettingsAction, SettingsSnapshot, SettingsTab, SliderKind, SysMonitorSettingsView,
    WebDavSettingsView,
};

use layout::{rect_contains, HitKind, SettingsLayout};

pub struct SettingsUiState {
    pub active_tab: SettingsTab,
    pub selected_dock: usize,
    pub selected_dock_item: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub scroll_y: i32,
    hovered_id: Option<usize>,
    dragging_slider: Option<(SliderKind, PixelRect)>,
    last_dock_input_key: Option<(String, u64)>,
    layout: SettingsLayout,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::Modules,
            selected_dock: 0,
            selected_dock_item: None,
            width: 840,
            height: 580,
            scroll_y: 0,
            hovered_id: None,
            dragging_slider: None,
            last_dock_input_key: None,
            layout: SettingsLayout::default(),
        }
    }
}

impl SettingsUiState {
    pub fn reset_dock_input_sync(&mut self) {
        self.last_dock_input_key = None;
    }

    pub fn take_dock_input_sync(&mut self, snap: &SettingsSnapshot) -> Option<String> {
        if self.active_tab != SettingsTab::Dock {
            self.last_dock_input_key = None;
            return None;
        }
        if !snap.docks.is_empty() && self.selected_dock >= snap.docks.len() {
            self.selected_dock = snap.docks.len() - 1;
        }
        let dock = snap.docks.get(self.selected_dock)?;
        let item = self
            .selected_dock_item
            .and_then(|id| dock.items.iter().find(|i| i.id == id))
            .or_else(|| dock.items.first());
        let Some(item) = item else {
            self.selected_dock_item = None;
            self.last_dock_input_key = None;
            return None;
        };
        self.selected_dock_item = Some(item.id);
        let key = (dock.id.clone(), item.id);
        if self.last_dock_input_key.as_ref() == Some(&key) {
            return None;
        }
        self.last_dock_input_key = Some(key);
        Some(item.label.clone())
    }

    pub fn render(&mut self, snap: &SettingsSnapshot) -> (WidgetFrame, SettingsInputRects) {
        if !snap.docks.is_empty() && self.selected_dock >= snap.docks.len() {
            self.selected_dock = snap.docks.len() - 1;
        }
        self.layout = build_layout(
            snap,
            self.active_tab,
            self.selected_dock,
            self.selected_dock_item,
            self.width,
            self.height,
            self.scroll_y,
        );
        let max_scroll = (self.layout.content_height as i32 - self.height as i32).max(0);
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
            self.layout = build_layout(
                snap,
                self.active_tab,
                self.selected_dock,
                self.selected_dock_item,
                self.width,
                self.height,
                self.scroll_y,
            );
        }
        render_settings_frame(
            &self.layout,
            self.active_tab,
            self.selected_dock,
            self.hovered_id,
        )
    }

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
                let step = if delta > 0 { -36 } else { 36 };
                let max_scroll = (self.layout.content_height as i32 - self.height as i32).max(0);
                let next = (self.scroll_y + step).clamp(0, max_scroll);
                let changed = next != self.scroll_y;
                self.scroll_y = next;
                (changed, None)
            }
            SettingsUiEvent::PointerMove { x, y } => self.on_pointer_move(x, y, snap),
            SettingsUiEvent::MouseDown { x, y } => self.on_mouse_down(x, y, snap),
            SettingsUiEvent::MouseUp { .. } => {
                let was = self.dragging_slider.take().is_some();
                (was, None)
            }
            SettingsUiEvent::Closed => {
                self.dragging_slider = None;
                self.hovered_id = None;
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
                self.scroll_y = 0;
                (true, None)
            }
            HitKind::SelectDock(idx) => {
                self.selected_dock = idx;
                self.selected_dock_item = None;
                self.last_dock_input_key = None;
                (true, None)
            }
            HitKind::SelectDockItem { item_id, .. } => {
                self.selected_dock_item = Some(item_id);
                self.last_dock_input_key = None;
                (true, None)
            }
            HitKind::Toggle { action, .. }
            | HitKind::Segment { action, .. }
            | HitKind::Button { action, .. } => (true, Some(action)),
            HitKind::Slider { kind, .. } => {
                self.dragging_slider = Some((kind, ctrl.rect));
                let val = slider_value_from_x(kind, ctrl.rect, x);
                (true, self.slider_action(kind, val, snap))
            }
        }
    }

    fn hit_control(&self, x: i32, y: i32) -> Option<&layout::ControlItem> {
        self.layout.controls.iter().find(|c| {
            let is_sidebar = matches!(c.kind, HitKind::Tab(_));
            if !is_sidebar && y < 74 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use asrbox_core::{IsolationMode, ModuleId};
    use asrbox_platform_api::WidgetLayer;

    fn sample_snapshot() -> SettingsSnapshot {
        SettingsSnapshot {
            isolation: IsolationMode::Process,
            autostart_enabled: false,
            modules: vec![ModuleStatusView {
                id: ModuleId::Dock,
                enabled: true,
                running: true,
                pid: Some(1234),
                restarts: 0,
                note: String::new(),
            }],
            docks: vec![DockSettingsView {
                id: "dock-1".to_owned(),
                edge_index: 0,
                layer: WidgetLayer::Wallpaper,
                label_mode_index: 0,
                icon_size_px: 48,
                opacity_percent: 85,
                locked: true,
                show_running_indicator: true,
                focus_running_window: true,
                item_count: 1,
                items: vec![DockItemView {
                    id: 1,
                    label: "Browser".to_owned(),
                }],
            }],
            calendar: CalendarSettingsView {
                visible: true,
                opacity_percent: 88,
                layer: WidgetLayer::Wallpaper,
                period_index: 2,
                refresh_interval_sec: 1800,
                width: 336,
                visible_rows: 8,
                commented_calendar_hint: true,
                sources: vec![],
            },
            sysmon: SysMonitorSettingsView {
                visible: true,
                opacity_percent: 88,
                layer: WidgetLayer::Wallpaper,
                refresh_interval_ms: 2000,
                width: 560,
                height: 492,
            },
            overlay: OverlaySettingsView {
                module_enabled: true,
                auto_games_only: true,
                force_topmost_fullscreen: true,
                setup_mode: false,
                anchor_index: 1,
                x: 24,
                y: 24,
                opacity_percent: 78,
                width: 320,
                height: 188,
                refresh_interval_ms: 1000,
            },
            backup: BackupSettingsView {
                auto_upload_cloud: false,
                last_backup_file: String::new(),
                last_backup_time: String::new(),
                cloud_status: String::new(),
                busy: false,
            },
            webdav: WebDavSettingsView {
                provider_index: 0,
                effective_url: "https://webdav.yandex.ru".to_owned(),
                username: String::new(),
                remote_dir: "asrbox-backups".to_owned(),
                has_saved_password: false,
                status_text: String::new(),
            },
            debug: DebugSettingsView {
                verbose: false,
                file_log: false,
                summary_line: "RAM 20 MB".to_owned(),
            },
            about: AboutSettingsView {
                version: "0.0.4".to_owned(),
                exe_path: "asrbox.exe".to_owned(),
                data_dir: "data".to_owned(),
                config_dir: "data/config".to_owned(),
            },
        }
    }

    #[test]
    fn renders_all_tabs_and_input_rects() {
        let snap = sample_snapshot();
        let mut ui = SettingsUiState::default();
        for tab in SettingsTab::ALL {
            ui.active_tab = tab;
            ui.scroll_y = if tab == SettingsTab::Dock { 260 } else { 0 };
            let (frame, inputs) = ui.render(&snap);
            assert_eq!(frame.bgra.width, 840);
            assert_eq!(frame.bgra.height, 580);
            assert!(!frame.texts.is_empty());
            match tab {
                SettingsTab::Calendar | SettingsTab::Dock => {
                    assert!(inputs.primary.is_some());
                    assert!(inputs.secret.is_none());
                }
                SettingsTab::Backup => {
                    assert!(inputs.primary.is_some());
                    assert!(inputs.secret.is_some());
                }
                _ => {
                    assert!(inputs.primary.is_none());
                    assert!(inputs.secret.is_none());
                }
            }
        }
    }
}
