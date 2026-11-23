#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod docks;
mod selftest;
#[cfg(windows)]
mod tools;

use std::env;
use std::process::ExitCode;

#[cfg(windows)]
use std::time::Instant;

#[cfg(windows)]
use asrbox_mod_about::AppPaths;
#[cfg(windows)]
use asrbox_mod_debug::{DebugHub, LogLevel};
#[cfg(windows)]
use asrbox_mod_dock::{to_dock_point, to_dock_rect, DockEvent, Rect};
#[cfg(windows)]
use asrbox_mod_widgets::CalendarWidget;
#[cfg(windows)]
use asrbox_platform_api::{
    DockPlatform, IconProvider, PlatformEvent, ToolsPlatform, WidgetPlatform,
};

#[cfg(windows)]
use crate::docks::DockManager;
#[cfg(windows)]
use crate::tools::resolve_app_paths;

pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    let paths = resolve_app_paths();
    let icons = WindowsIconProvider::new();
    let mut ctrl = AppController::load(paths, work_area, icons, safe_mode)?;
    ctrl.sync_all(&mut host)?;

    host.run_event_loop(|platform, event| ctrl.handle_event(platform, event));
    Ok(())
}

#[cfg(windows)]
pub(crate) struct AppController<I: IconProvider> {
    pub(crate) paths: AppPaths,
    pub(crate) docks: DockManager<I>,
    pub(crate) calendar: CalendarWidget,
    pub(crate) debug: DebugHub,
    pub(crate) safe_mode: bool,
}

#[cfg(windows)]
impl<I: IconProvider> AppController<I> {
    fn load(
        paths: AppPaths,
        work_area: Rect,
        icon_provider: I,
        safe_mode: bool,
    ) -> Result<Self, String> {
        let docks = DockManager::load(
            paths.docks_cfg.clone(),
            APP_VERSION,
            work_area,
            icon_provider,
        )?;
        let calendar = CalendarWidget::load(paths.widgets_cfg.clone(), paths.cache_dir.clone());
        let debug = DebugHub::new(paths.logs_dir.clone());
        Ok(Self {
            paths,
            docks,
            calendar,
            debug,
            safe_mode,
        })
    }

    fn sync_all<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
    ) -> Result<(), String> {
        let now = host.local_time_now();
        self.debug.log(
            now,
            LogLevel::Info,
            "app",
            format!(
                "Запуск asrbox v{APP_VERSION} (portable: {}, доков: {}, календарей: {})",
                self.paths.data_dir.display(),
                self.docks.dock_count(),
                self.calendar.source_count()
            ),
        );
        self.docks.sync_windows(host, self.safe_mode)?;
        self.calendar.sync_window(host, self.safe_mode)?;
        self.debug.log(
            now,
            LogLevel::Info,
            "widgets",
            format!("Календарь: {}", self.calendar.status_summary()),
        );
        Ok(())
    }

    fn handle_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: PlatformEvent,
    ) {
        match ev {
            PlatformEvent::TrayContextMenu { screen_point } => self.on_tray(host, screen_point),
            PlatformEvent::DebugWindowTick => self.refresh_debug_ui(host),
            PlatformEvent::DebugCommand(cmd) => self.on_debug_cmd(host, cmd),
            PlatformEvent::AboutCommand(cmd) => self.on_about_cmd(host, cmd),
            PlatformEvent::DisplayChanged { work_area } => {
                self.debug.record_event("platform", "display_changed");
                self.docks.on_display(host, work_area);
                self.debug.log(
                    host.local_time_now(),
                    LogLevel::Info,
                    "platform",
                    format!("Изменение экрана: {}x{}", work_area.width, work_area.height),
                );
            }
            ev @ (PlatformEvent::PointerMove { .. }
            | PlatformEvent::PointerLeave { .. }
            | PlatformEvent::AnimationTick { .. }
            | PlatformEvent::DropFiles { .. }
            | PlatformEvent::LeftClick { .. }
            | PlatformEvent::RightClick { .. }) => self.handle_dock_event(host, ev),
            other => self.handle_widget_event(host, other),
        }
        if host.is_debug_window_open() && self.debug.take_dirty() {
            self.refresh_debug_ui(host);
        }
    }

    fn handle_dock_event<H: DockPlatform + WidgetPlatform>(
        &mut self,
        host: &mut H,
        ev: PlatformEvent,
    ) {
        match ev {
            PlatformEvent::PointerMove { dock_id, point } => {
                self.step_dock_timed(
                    host,
                    dock_id,
                    "pointer_move",
                    DockEvent::PointerMove(to_dock_point(point)),
                );
            }
            PlatformEvent::PointerLeave { dock_id } => {
                self.step_dock_timed(host, dock_id, "pointer_leave", DockEvent::PointerLeave);
            }
            PlatformEvent::AnimationTick { dock_id, dt_ms } => {
                self.step_dock_timed(
                    host,
                    dock_id,
                    "anim_tick",
                    DockEvent::AnimationTick { dt_ms },
                );
            }
            PlatformEvent::DropFiles {
                dock_id,
                paths,
                point,
            } => {
                self.debug.record_event("dock", "drop_files");
                let added = self
                    .docks
                    .on_drop(host, dock_id, paths, to_dock_point(point));
                self.debug.log(
                    host.local_time_now(),
                    LogLevel::Info,
                    "dock",
                    format!("dock[{dock_id}] добавлено значков: {added}"),
                );
            }
            PlatformEvent::LeftClick { dock_id, point } => {
                self.debug.record_event("dock", "left_click");
                if let Some(msg) = self
                    .docks
                    .on_left_click(host, dock_id, to_dock_point(point))
                {
                    self.log_tagged(host, "dock", msg);
                }
            }
            PlatformEvent::RightClick {
                dock_id,
                point,
                screen_point,
            } => {
                self.debug.record_event("dock", "right_click");
                let pt = to_dock_point(point);
                if let Some(msg) =
                    self.docks
                        .on_right_click(host, dock_id, pt, screen_point, &mut self.safe_mode)
                {
                    self.log_tagged(host, "dock", msg);
                }
            }
            _ => {}
        }
    }

    fn step_dock_timed<H: DockPlatform>(
        &mut self,
        host: &mut H,
        dock_id: u64,
        name: &'static str,
        ev: DockEvent,
    ) {
        let t0 = Instant::now();
        self.debug.record_event("dock", name);
        if self.docks.step_dock(host, dock_id, ev) {
            let us = u64::try_from(t0.elapsed().as_micros()).unwrap_or(0);
            self.debug.record_frame("dock", us);
        }
    }

    fn handle_widget_event<H: WidgetPlatform>(&mut self, host: &mut H, ev: PlatformEvent) {
        let t0 = Instant::now();
        match ev {
            PlatformEvent::WidgetPointerMove { point, .. } => {
                self.debug.record_event("widgets", "pointer_move");
                self.calendar.on_pointer_move(host, point);
            }
            PlatformEvent::WidgetPointerLeave { .. } => {
                self.debug.record_event("widgets", "pointer_leave");
                self.calendar.on_pointer_leave(host);
            }
            PlatformEvent::WidgetLeftClick { point, .. } => {
                self.debug.record_event("widgets", "left_click");
                let msg = self.calendar.on_left_click(host, point);
                if !msg.is_empty() {
                    self.log_tagged(host, "widgets", msg);
                }
                self.record_widget_frame(t0);
            }
            PlatformEvent::WidgetRightClick { screen_point, .. } => {
                self.debug.record_event("widgets", "right_click");
                let msg = self.calendar.on_right_click(host, screen_point);
                if !msg.is_empty() {
                    self.log_tagged(host, "widgets", msg);
                }
            }
            PlatformEvent::WidgetWheel { delta, .. } => {
                self.debug.record_event("widgets", "wheel");
                self.calendar.on_wheel(host, delta);
                self.record_widget_frame(t0);
            }
            PlatformEvent::WidgetMoved { x, y, .. } => {
                self.debug.record_event("widgets", "moved");
                self.calendar.on_moved(x, y);
                self.debug.log(
                    host.local_time_now(),
                    LogLevel::Trace,
                    "widgets",
                    format!("Перемещён виджет: ({x}, {y})"),
                );
            }
            PlatformEvent::WidgetMinuteTick { .. } => {
                self.debug.record_event("widgets", "minute_tick");
                self.calendar.on_minute_tick(host);
            }
            PlatformEvent::WidgetRefreshTick { .. } => {
                self.debug.record_event("widgets", "refresh_tick");
                self.calendar.on_refresh_tick(host);
                self.debug.log(
                    host.local_time_now(),
                    LogLevel::Info,
                    "http",
                    "Автообновление .ics",
                );
            }
            PlatformEvent::HttpFetchDone { request_id, result } => {
                self.debug.record_event("http", "fetch_done");
                if let Some((is_err, msg)) = self.calendar.on_http_done(host, request_id, result) {
                    let lvl = if is_err {
                        LogLevel::Error
                    } else {
                        LogLevel::Info
                    };
                    self.debug.log(host.local_time_now(), lvl, "http", msg);
                }
                self.record_widget_frame(t0);
            }
            _ => {}
        }
    }

    fn record_widget_frame(&mut self, t0: Instant) {
        let us = u64::try_from(t0.elapsed().as_micros()).unwrap_or(0);
        self.debug.record_frame("widgets", us);
    }

    fn log_tagged<H: WidgetPlatform>(&mut self, host: &H, module: &'static str, msg: String) {
        let lvl = if msg.starts_with("error:") {
            LogLevel::Error
        } else {
            LogLevel::Info
        };
        self.debug.log(host.local_time_now(), lvl, module, msg);
    }
}
