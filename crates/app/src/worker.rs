#![cfg(windows)]

use std::collections::VecDeque;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use asrbox_core::{decode_supervisor_cmd, encode_worker_msg, ModuleId, SupervisorCmd, WorkerMsg};
use asrbox_mod_dock::{to_dock_point, to_dock_rect, DockEvent};
use asrbox_platform_api::{DockPlatform, PlatformEvent, ToolsPlatform};
use asrbox_platform_windows::{
    is_process_alive, wake_host_window, WindowsDockHost, WindowsIconProvider,
};

use crate::docks::DockManager;
use crate::tools::resolve_app_paths;
use crate::worker_overlay::run_overlay_worker;
use crate::worker_widgets::run_widgets_worker;
use crate::APP_VERSION;

pub fn run_worker(module: ModuleId, parent_pid: u32, safe_mode: bool) -> Result<(), String> {
    let mut host = WindowsDockHost::new_worker()?;
    let hwnd_raw = host.host_hwnd_raw();
    let cmd_queue = spawn_stdin_reader(hwnd_raw);
    host.set_supervisor_timer(true);
    emit_msg(&WorkerMsg::Ready {
        module: module.as_str().to_owned(),
        pid: std::process::id(),
    });
    match module {
        ModuleId::Dock => run_dock_worker(&mut host, cmd_queue, parent_pid, safe_mode),
        ModuleId::Widgets => run_widgets_worker(&mut host, cmd_queue, parent_pid, safe_mode),
        ModuleId::Overlay => run_overlay_worker(&mut host, cmd_queue, parent_pid),
    }
}

fn spawn_stdin_reader(hwnd_raw: usize) -> Arc<Mutex<VecDeque<SupervisorCmd>>> {
    let q = Arc::new(Mutex::new(VecDeque::new()));
    let q_bg = Arc::clone(&q);
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            if let Some(cmd) = decode_supervisor_cmd(&line) {
                if let Ok(mut guard) = q_bg.lock() {
                    guard.push_back(cmd);
                }
                wake_host_window(hwnd_raw);
            }
        }
        if let Ok(mut guard) = q_bg.lock() {
            guard.push_back(SupervisorCmd::Shutdown);
        }
        wake_host_window(hwnd_raw);
    });
    q
}

fn run_dock_worker(
    host: &mut WindowsDockHost,
    cmds: Arc<Mutex<VecDeque<SupervisorCmd>>>,
    parent_pid: u32,
    mut safe_mode: bool,
) -> Result<(), String> {
    let paths = resolve_app_paths();
    let work_area = to_dock_rect(host.primary_work_area()?);
    let icons = WindowsIconProvider::new();
    let mut docks = DockManager::load(paths.docks_cfg, APP_VERSION, work_area, icons, host)?;
    docks.sync_windows(host, safe_mode)?;
    emit_log(
        "INFO",
        "dock",
        format!("Воркер док-панелей готов (доков: {})", docks.dock_count()),
    );
    let mut stats = WorkerStats::new("dock");

    host.run_event_loop(|platform, ev| {
        if let PlatformEvent::SupervisorTick = ev {
            if parent_pid > 0 && !is_process_alive(parent_pid) {
                platform.request_quit();
                return;
            }
            for cmd in drain_cmds(&cmds) {
                match cmd {
                    SupervisorCmd::ReloadConfig => {
                        if let Ok(n) = docks.reload_from_disk(platform, safe_mode) {
                            emit_log("INFO", "dock", format!("Перечитан docks.cfg ({n})"));
                        }
                    }
                    SupervisorCmd::AddDock => {
                        docks.add_new_dock(platform, &mut safe_mode);
                        emit_msg(&WorkerMsg::ConfigChanged {
                            module: "dock".to_owned(),
                        });
                    }
                    SupervisorCmd::ToggleDock(id) => {
                        docks.toggle_dock(platform, id, safe_mode);
                        emit_msg(&WorkerMsg::ConfigChanged {
                            module: "dock".to_owned(),
                        });
                    }
                    SupervisorCmd::RemoveDock(id) => {
                        docks.remove_dock(platform, id);
                        emit_msg(&WorkerMsg::ConfigChanged {
                            module: "dock".to_owned(),
                        });
                    }
                    SupervisorCmd::Shutdown => {
                        docks.persist();
                        platform.request_quit();
                    }
                    _ => {}
                }
            }
            stats.maybe_emit(platform);
            return;
        }
        handle_dock_worker_event(platform, &mut docks, &mut safe_mode, &mut stats, ev);
    });
    Ok(())
}

fn handle_dock_worker_event(
    host: &mut WindowsDockHost,
    docks: &mut DockManager<WindowsIconProvider>,
    safe_mode: &mut bool,
    stats: &mut WorkerStats,
    ev: PlatformEvent,
) {
    let t0 = Instant::now();
    stats.events += 1;
    match ev {
        PlatformEvent::PointerMove { dock_id, point } => {
            if docks.step_dock(host, dock_id, DockEvent::PointerMove(to_dock_point(point))) {
                stats.record_frame(t0);
            }
        }
        PlatformEvent::PointerLeave { dock_id } => {
            if docks.step_dock(host, dock_id, DockEvent::PointerLeave) {
                stats.record_frame(t0);
            }
        }
        PlatformEvent::AnimationTick { dock_id, dt_ms } => {
            if docks.step_dock(host, dock_id, DockEvent::AnimationTick { dt_ms }) {
                stats.record_frame(t0);
            }
        }
        PlatformEvent::RunningPollTick => {
            if docks.refresh_running_apps(host) {
                stats.record_frame(t0);
            }
        }
        PlatformEvent::LeftButtonDown {
            dock_id,
            point,
            screen_point,
        } => {
            docks.on_left_down(dock_id, to_dock_point(point), screen_point);
        }
        PlatformEvent::DockDragMove { screen_point, .. } => {
            docks.on_drag_move(host, screen_point);
            stats.record_frame(t0);
        }
        PlatformEvent::LeftButtonUp { screen_point, .. } => {
            if let Some(msg) = docks.on_left_up(host, screen_point) {
                handle_dock_message(msg);
            }
        }
        PlatformEvent::RightClick {
            dock_id,
            point,
            screen_point,
        } => {
            if let Some(msg) =
                docks.on_right_click(host, dock_id, to_dock_point(point), screen_point, safe_mode)
            {
                handle_dock_message(msg);
            }
        }
        PlatformEvent::DropFiles {
            dock_id,
            paths,
            point,
        } => {
            let added = docks.on_drop(host, dock_id, paths, to_dock_point(point));
            emit_log(
                "INFO",
                "dock",
                format!("dock[{dock_id}] добавлено значков: {added}"),
            );
            emit_msg(&WorkerMsg::ConfigChanged {
                module: "dock".to_owned(),
            });
        }
        PlatformEvent::DisplayChanged { work_area } => docks.on_display(host, work_area),
        _ => {}
    }
}

fn handle_dock_message(msg: String) {
    if msg == "open_settings" {
        emit_msg(&WorkerMsg::OpenSettings {
            tab: "dock".to_owned(),
        });
        return;
    }
    let lvl = if msg.starts_with("error:") {
        "ERROR"
    } else {
        "INFO"
    };
    emit_log(lvl, "dock", msg);
    emit_msg(&WorkerMsg::ConfigChanged {
        module: "dock".to_owned(),
    });
}

pub(crate) struct WorkerStats {
    module: &'static str,
    pub(crate) events: u64,
    frames: u64,
    max_us: u64,
    last_emit: Instant,
}

impl WorkerStats {
    pub(crate) fn new(module: &'static str) -> Self {
        Self {
            module,
            events: 0,
            frames: 0,
            max_us: 0,
            last_emit: Instant::now(),
        }
    }

    pub(crate) fn record_frame(&mut self, t0: Instant) {
        self.frames += 1;
        let us = u64::try_from(t0.elapsed().as_micros()).unwrap_or(0);
        if us > self.max_us {
            self.max_us = us;
        }
    }

    pub(crate) fn maybe_emit(&mut self, host: &mut WindowsDockHost) {
        if self.last_emit.elapsed().as_secs() < 2 {
            return;
        }
        self.last_emit = Instant::now();
        let m = host.query_process_metrics();
        emit_msg(&WorkerMsg::Metrics {
            module: self.module.to_owned(),
            ram_kb: m.ram_kb,
            cpu_tenths: m.cpu_tenths_pct,
            gdi: m.gdi_handles,
            user: m.user_handles,
            events: self.events,
            frames: self.frames,
            max_us: self.max_us,
        });
    }
}

pub(crate) fn drain_cmds(q: &Arc<Mutex<VecDeque<SupervisorCmd>>>) -> Vec<SupervisorCmd> {
    q.lock()
        .map(|mut g| g.drain(..).collect())
        .unwrap_or_default()
}

pub(crate) fn emit_log(level: &str, module: &str, message: String) {
    emit_msg(&WorkerMsg::Log {
        level: level.to_owned(),
        module: module.to_owned(),
        message,
    });
}

pub(crate) fn emit_msg(msg: &WorkerMsg) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{}", encode_worker_msg(msg));
    let _ = out.flush();
}
