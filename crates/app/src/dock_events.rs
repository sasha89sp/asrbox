use std::time::Instant;

use asrbox_mod_dock::{to_dock_point, DockEvent};
use asrbox_mod_settings::SettingsTab;
use asrbox_platform_api::{
    DockPlatform, IconProvider, PlatformEvent, ToolsPlatform, WidgetPlatform,
};

use crate::AppController;

impl<I: IconProvider> AppController<I> {
    pub(crate) fn handle_dock_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: PlatformEvent,
    ) {
        let t0 = Instant::now();
        let mut open_settings = false;
        let mut log_msg: Option<String> = None;
        {
            let Some(docks) = self.docks.as_mut() else {
                return;
            };
            match ev {
                PlatformEvent::PointerMove { dock_id, point } => {
                    self.debug.record_event("dock", "pointer_move");
                    if docks.step_dock(host, dock_id, DockEvent::PointerMove(to_dock_point(point)))
                    {
                        self.record_frame("dock", t0);
                    }
                }
                PlatformEvent::PointerLeave { dock_id } => {
                    self.debug.record_event("dock", "pointer_leave");
                    if docks.step_dock(host, dock_id, DockEvent::PointerLeave) {
                        self.record_frame("dock", t0);
                    }
                }
                PlatformEvent::AnimationTick { dock_id, dt_ms } => {
                    if docks.step_dock(host, dock_id, DockEvent::AnimationTick { dt_ms }) {
                        self.record_frame("dock", t0);
                    }
                }
                PlatformEvent::RunningPollTick => {
                    if docks.refresh_running_apps(host) {
                        self.record_frame("dock", t0);
                    }
                }
                PlatformEvent::LeftButtonDown {
                    dock_id,
                    point,
                    screen_point,
                } => {
                    docks.on_left_down(host, dock_id, to_dock_point(point), screen_point);
                }
                PlatformEvent::DockDragMove { screen_point, .. } => {
                    docks.on_drag_move(host, screen_point);
                }
                PlatformEvent::LeftButtonUp { screen_point, .. } => {
                    if let Some(m) = docks.on_left_up(host, screen_point) {
                        if m == "open_settings" {
                            open_settings = true;
                        } else {
                            log_msg = Some(m);
                        }
                    }
                }
                PlatformEvent::RightClick {
                    dock_id,
                    point,
                    screen_point,
                } => {
                    if let Some(m) = docks.on_right_click(
                        host,
                        dock_id,
                        to_dock_point(point),
                        screen_point,
                        &mut self.safe_mode,
                    ) {
                        if m == "open_settings" {
                            open_settings = true;
                        } else {
                            log_msg = Some(m);
                        }
                    }
                }
                PlatformEvent::DropFiles {
                    dock_id,
                    paths,
                    point,
                } => {
                    let added = docks.on_drop(host, dock_id, paths, to_dock_point(point));
                    log_msg = Some(format!("dock[{dock_id}] добавлено значков: {added}"));
                }
                _ => {}
            }
        }
        if open_settings {
            self.open_settings_tab(host, SettingsTab::Dock);
        }
        if let Some(m) = log_msg {
            self.log_tagged(host, "dock", m);
            self.render_settings_ui(host);
        }
    }
}
