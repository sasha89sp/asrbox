#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod docks;
mod selftest;

use std::env;
use std::process::ExitCode;

#[cfg(windows)]
use std::path::PathBuf;

#[cfg(windows)]
use asrbox_mod_dock::{DockEvent, Rect, to_dock_point, to_dock_rect};
#[cfg(windows)]
use asrbox_mod_widgets::CalendarWidget;
#[cfg(windows)]
use asrbox_platform_api::{
    DockPlatform, IconProvider, PixelPoint, PlatformEvent, TrayAction, WidgetPlatform,
};

#[cfg(windows)]
use crate::docks::DockManager;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--version") {
        println!("asrbox {APP_VERSION}");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|arg| arg == "--selftest") {
        return match selftest::run_selftest(APP_VERSION) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }

    let safe_mode = args.iter().any(|arg| arg == "--safe-mode");
    match run_app(safe_mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

#[cfg(not(windows))]
fn run_app(_safe_mode: bool) -> Result<(), String> {
    Err("asrbox UI runs on Windows only".to_owned())
}

#[cfg(windows)]
fn run_app(safe_mode: bool) -> Result<(), String> {
    use asrbox_platform_windows::{SingleInstanceGuard, WindowsDockHost, WindowsIconProvider};

    let Some(_guard) = SingleInstanceGuard::acquire("Local\\asrbox_single_instance_v1")? else {
        return Ok(());
    };
    let tooltip = format!("asrbox v{APP_VERSION}");
    let mut host = WindowsDockHost::new(&tooltip)?;
    let work_area = to_dock_rect(host.primary_work_area()?);
    let data_dir = resolve_data_dir();
    let icons = WindowsIconProvider::new();
    let mut ctrl = AppController::load(data_dir, work_area, icons, safe_mode)?;
    ctrl.sync_all(&mut host)?;

    host.run_event_loop(|platform, event| ctrl.handle_event(platform, event));
    Ok(())
}

#[cfg(windows)]
struct AppController<I: IconProvider> {
    docks: DockManager<I>,
    calendar: CalendarWidget,
    safe_mode: bool,
}

#[cfg(windows)]
impl<I: IconProvider> AppController<I> {
    fn load(
        data_dir: PathBuf,
        work_area: Rect,
        icon_provider: I,
        safe_mode: bool,
    ) -> Result<Self, String> {
        let dock_cfg = data_dir.join("config").join("docks.cfg");
        let widget_cfg = data_dir.join("config").join("widgets.cfg");
        let cache_dir = data_dir.join("cache");
        let docks = DockManager::load(dock_cfg, APP_VERSION, work_area, icon_provider)?;
        let calendar = CalendarWidget::load(widget_cfg, cache_dir);
        Ok(Self {
            docks,
            calendar,
            safe_mode,
        })
    }

    fn sync_all<H: DockPlatform + WidgetPlatform>(&mut self, host: &mut H) -> Result<(), String> {
        self.docks.sync_windows(host, self.safe_mode)?;
        self.calendar.sync_window(host, self.safe_mode)?;
        Ok(())
    }

    fn handle_event<H: DockPlatform + WidgetPlatform>(&mut self, host: &mut H, ev: PlatformEvent) {
        match ev {
            PlatformEvent::PointerMove { dock_id, point } => {
                let d_ev = DockEvent::PointerMove(to_dock_point(point));
                self.docks.step_dock(host, dock_id, d_ev);
            }
            PlatformEvent::PointerLeave { dock_id } => {
                self.docks.step_dock(host, dock_id, DockEvent::PointerLeave);
            }
            PlatformEvent::AnimationTick { dock_id, dt_ms } => {
                let d_ev = DockEvent::AnimationTick { dt_ms };
                self.docks.step_dock(host, dock_id, d_ev);
            }
            PlatformEvent::DropFiles {
                dock_id,
                paths,
                point,
            } => self.docks.on_drop(host, dock_id, paths, to_dock_point(point)),
            PlatformEvent::LeftClick { dock_id, point } => {
                self.docks.on_left_click(host, dock_id, to_dock_point(point));
            }
            PlatformEvent::RightClick {
                dock_id,
                point,
                screen_point,
            } => {
                let pt = to_dock_point(point);
                self.docks
                    .on_right_click(host, dock_id, pt, screen_point, &mut self.safe_mode);
            }
            PlatformEvent::DisplayChanged { work_area } => self.docks.on_display(host, work_area),
            PlatformEvent::TrayContextMenu { screen_point } => self.on_tray(host, screen_point),
            other => self.handle_widget_event(host, other),
        }
    }

    fn handle_widget_event<H: WidgetPlatform>(&mut self, host: &mut H, ev: PlatformEvent) {
        match ev {
            PlatformEvent::WidgetPointerMove { point, .. } => {
                self.calendar.on_pointer_move(host, point);
            }
            PlatformEvent::WidgetPointerLeave { .. } => self.calendar.on_pointer_leave(host),
            PlatformEvent::WidgetLeftClick { point, .. } => {
                self.calendar.on_left_click(host, point);
            }
            PlatformEvent::WidgetRightClick { screen_point, .. } => {
                self.calendar.on_right_click(host, screen_point);
            }
            PlatformEvent::WidgetWheel { delta, .. } => self.calendar.on_wheel(host, delta),
            PlatformEvent::WidgetMoved { x, y, .. } => self.calendar.on_moved(x, y),
            PlatformEvent::WidgetMinuteTick { .. } => self.calendar.on_minute_tick(host),
            PlatformEvent::WidgetRefreshTick { .. } => self.calendar.on_refresh_tick(host),
            PlatformEvent::HttpFetchDone { request_id, result } => {
                self.calendar.on_http_done(host, request_id, result);
            }
            _ => {}
        }
    }

    fn on_tray<H: DockPlatform + WidgetPlatform>(&mut self, host: &mut H, screen: PixelPoint) {
        let entries = self.docks.tray_entries();
        let cal_vis = self.calendar.is_visible();
        let Ok(Some(action)) = host.show_tray_menu(screen, &entries, cal_vis) else {
            return;
        };
        match action {
            TrayAction::AddDock => self.docks.add_new_dock(host, &mut self.safe_mode),
            TrayAction::ToggleDock(id) => self.docks.toggle_dock(host, id, self.safe_mode),
            TrayAction::RemoveDock(id) => self.docks.remove_dock(host, id),
            TrayAction::ToggleCalendarWidget => {
                self.safe_mode = false;
                self.calendar.toggle_visible(host, false);
            }
            TrayAction::OpenWidgetConfig => self.calendar.open_config_file(host),
            TrayAction::Exit => {
                self.docks.persist();
                host.request_quit();
            }
        }
    }
}

#[cfg(windows)]
fn resolve_data_dir() -> PathBuf {
    let base = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("data")
}
