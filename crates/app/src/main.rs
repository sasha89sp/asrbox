#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod backup_ctl;
#[cfg(windows)]
mod dock_drag;
#[cfg(windows)]
mod dock_events;
#[cfg(windows)]
mod dock_ops;
mod dock_shortcuts;
#[cfg(windows)]
mod docks;
mod selftest;
mod selftest_tools;
#[cfg(windows)]
mod settings_ctl;
#[cfg(windows)]
mod settings_general;
#[cfg(windows)]
mod settings_widgets;
#[cfg(windows)]
mod supervisor;
#[cfg(windows)]
mod sync_ctl;
#[cfg(windows)]
mod tools;
#[cfg(windows)]
mod webdav_ctl;
#[cfg(windows)]
mod worker;
#[cfg(windows)]
mod worker_bus;
#[cfg(windows)]
mod worker_overlay;
#[cfg(windows)]
mod worker_widgets;

use std::env;
use std::process::ExitCode;

#[cfg(windows)]
use asrbox_core::{load_modules_config, IsolationMode, ModuleId, ModulesConfig};
#[cfg(windows)]
use asrbox_mod_about::AppPaths;
#[cfg(windows)]
use asrbox_mod_debug::{DebugHub, LogLevel};
#[cfg(windows)]
use asrbox_mod_dock::{to_dock_rect, Rect};
#[cfg(windows)]
use asrbox_mod_overlay::GameOverlay;
#[cfg(windows)]
use asrbox_mod_settings::{SettingsTab, SettingsUiState};
#[cfg(windows)]
use asrbox_mod_widgets::{CalendarWidget, SysMonitorWidget, CALENDAR_WIDGET_ID, SYS_WIDGET_ID};
#[cfg(windows)]
use asrbox_platform_api::{
    load_appearance_config, AppearanceConfig, DockPlatform, IconProvider, PlatformEvent,
    ToolsPlatform, WidgetPlatform,
};

#[cfg(windows)]
use crate::backup_ctl::BackupController;
#[cfg(windows)]
use crate::docks::DockManager;
#[cfg(windows)]
use crate::supervisor::ModuleSupervisor;
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
    let force_inproc = args.iter().any(|arg| arg == "--inproc");
    if let Some(worker_mod) = parse_arg_value(&args, "--worker") {
        let parent_pid = parse_arg_value(&args, "--parent-pid")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        return match run_worker_mode(&worker_mod, parent_pid, safe_mode) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }

    match run_app(safe_mode, force_inproc) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn parse_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

#[cfg(not(windows))]
fn run_worker_mode(_module: &str, _parent_pid: u32, _safe_mode: bool) -> Result<(), String> {
    Err("asrbox workers run on Windows only".to_owned())
}

#[cfg(windows)]
fn run_worker_mode(module: &str, parent_pid: u32, safe_mode: bool) -> Result<(), String> {
    let paths = resolve_app_paths();
    install_crash_log_hook(&paths.logs_dir);
    let id = ModuleId::from_str_id(module).ok_or_else(|| format!("unknown worker: {module}"))?;
    worker::run_worker(id, parent_pid, safe_mode)
}

#[cfg(not(windows))]
fn run_app(_safe_mode: bool, _force_inproc: bool) -> Result<(), String> {
    Err("asrbox UI runs on Windows only".to_owned())
}

#[cfg(windows)]
fn run_app(safe_mode: bool, force_inproc: bool) -> Result<(), String> {
    use asrbox_platform_windows::{SingleInstanceGuard, WindowsDockHost, WindowsIconProvider};

    let paths = resolve_app_paths();
    install_crash_log_hook(&paths.logs_dir);
    let Some(_guard) = SingleInstanceGuard::acquire("Local\\asrbox_single_instance_v1")? else {
        return Ok(());
    };
    let tooltip = format!("asrbox v{APP_VERSION}");
    let mut host = WindowsDockHost::new(&tooltip)?;
    let work_area = to_dock_rect(host.primary_work_area()?);
    let mut modules_cfg =
        load_modules_config(&paths.data_dir.join("config").join("modules.cfg")).unwrap_or_default();
    if force_inproc {
        modules_cfg.isolation = IsolationMode::InProc;
    }
    let hwnd_raw = host.host_hwnd_raw();
    host.set_supervisor_timer(true);
    let backup = BackupController::load(&paths, &host);
    let appearance = load_appearance_config(&paths.appearance_cfg);
    let pal = appearance.palette();
    DockPlatform::set_theme(&mut host, pal);
    WidgetPlatform::set_theme(&mut host, pal);
    let mut ctrl = AppController::new(
        paths,
        work_area,
        WindowsIconProvider::new,
        modules_cfg,
        appearance,
        backup,
        hwnd_raw,
        safe_mode,
    );
    ctrl.sync_runtime_modules(&mut host);

    host.run_event_loop(|platform, event| ctrl.handle_event(platform, event));
    ctrl.supervisor.stop_all();
    Ok(())
}

#[cfg(windows)]
pub(crate) struct AppController<I: IconProvider> {
    pub(crate) paths: AppPaths,
    pub(crate) work_area: Rect,
    pub(crate) make_icons: fn() -> I,
    pub(crate) modules_cfg: ModulesConfig,
    pub(crate) appearance: AppearanceConfig,
    pub(crate) supervisor: ModuleSupervisor,
    pub(crate) docks: Option<DockManager<I>>,
    pub(crate) calendar: Option<CalendarWidget>,
    pub(crate) sysmon: Option<SysMonitorWidget>,
    pub(crate) overlay: Option<GameOverlay>,
    pub(crate) overlay_setup_mode: bool,
    pub(crate) backup: BackupController,
    pub(crate) settings_ui: SettingsUiState,
    pub(crate) last_settings_tab: Option<SettingsTab>,
    pub(crate) diagnostics_monitor_enabled: bool,
    pub(crate) debug: DebugHub,
    pub(crate) safe_mode: bool,
}

#[cfg(windows)]
impl<I: IconProvider> AppController<I> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        paths: AppPaths,
        work_area: Rect,
        make_icons: fn() -> I,
        modules_cfg: ModulesConfig,
        appearance: AppearanceConfig,
        backup: BackupController,
        host_hwnd_raw: usize,
        safe_mode: bool,
    ) -> Self {
        let debug = DebugHub::new(paths.logs_dir.clone());
        let supervisor = ModuleSupervisor::new(host_hwnd_raw, safe_mode);
        let mut settings_ui = SettingsUiState::default();
        settings_ui.set_palette_immediate(appearance.palette());
        Self {
            paths,
            work_area,
            make_icons,
            modules_cfg,
            appearance,
            supervisor,
            docks: None,
            calendar: None,
            sysmon: None,
            overlay: None,
            overlay_setup_mode: false,
            backup,
            settings_ui,
            last_settings_tab: None,
            diagnostics_monitor_enabled: true,
            debug,
            safe_mode,
        }
    }

    pub(crate) fn sync_runtime_modules<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
    ) {
        let now = host.local_time_now();
        for msg in self.supervisor.sync_with_config(&self.modules_cfg) {
            self.log_tagged(host, "app", msg);
        }
        let pal = self.appearance.palette();
        let want_inproc_dock =
            self.modules_cfg.isolation == IsolationMode::InProc && self.modules_cfg.dock_enabled;
        if want_inproc_dock && self.docks.is_none() {
            if let Ok(mut dm) = DockManager::load(
                self.paths.docks_cfg.clone(),
                APP_VERSION,
                self.work_area,
                (self.make_icons)(),
                host,
            ) {
                dm.palette = pal;
                let _ = dm.sync_windows(host, self.safe_mode);
                self.docks = Some(dm);
            }
        } else if !want_inproc_dock {
            if let Some(mut dm) = self.docks.take() {
                dm.destroy_all_windows(host);
            }
        }

        let want_inproc_cal =
            self.modules_cfg.isolation == IsolationMode::InProc && self.modules_cfg.widgets_enabled;
        if want_inproc_cal && self.calendar.is_none() {
            let mut cal =
                CalendarWidget::load(self.paths.widgets_cfg.clone(), self.paths.cache_dir.clone());
            cal.set_palette(host, pal);
            let _ = cal.sync_window(host, self.safe_mode);
            self.debug.log(
                now,
                LogLevel::Info,
                "widgets",
                format!("Календарь (inproc): {}", cal.status_summary()),
            );
            self.calendar = Some(cal);
        } else if !want_inproc_cal && self.calendar.take().is_some() {
            let _ = host.destroy_widget_window(CALENDAR_WIDGET_ID);
        }
        if want_inproc_cal && self.sysmon.is_none() {
            let mut sys = SysMonitorWidget::load(self.paths.sysmon_cfg.clone());
            sys.set_palette(host, pal);
            let _ = sys.sync_window(host, self.safe_mode);
            self.debug.log(
                now,
                LogLevel::Info,
                "widgets",
                format!("Системный монитор (inproc): {}", sys.status_summary()),
            );
            self.sysmon = Some(sys);
        } else if !want_inproc_cal && self.sysmon.take().is_some() {
            let _ = host.destroy_widget_window(SYS_WIDGET_ID);
        }
        self.sync_inproc_overlay(host);
    }

    fn handle_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: PlatformEvent,
    ) {
        match ev {
            PlatformEvent::TrayContextMenu { screen_point } => self.on_tray(host, screen_point),
            PlatformEvent::SupervisorTick => self.poll_supervisor(host),
            PlatformEvent::SettingsEvent(sev) => self.on_settings_event(host, sev),
            PlatformEvent::DebugWindowTick => self.refresh_debug_ui(host),
            PlatformEvent::DebugCommand(cmd) => self.on_debug_cmd(host, cmd),
            PlatformEvent::AboutCommand(cmd) => self.on_about_cmd(host, cmd),
            PlatformEvent::WebDavDone(res) => {
                let ok = res.ok;
                let now = host.local_time_now();
                let msg = self.backup.on_webdav_done(host, res, now);
                let lvl = if ok { LogLevel::Info } else { LogLevel::Error };
                self.debug.log(now, lvl, "webdav", msg);
                self.render_settings_ui(host);
            }
            PlatformEvent::DisplayChanged { work_area } => {
                self.work_area = to_dock_rect(work_area);
                if let Some(docks) = self.docks.as_mut() {
                    docks.on_display(host, work_area);
                }
                if let Some(ov) = self.overlay.as_mut() {
                    ov.on_display_changed(host, work_area);
                }
            }
            ev @ (PlatformEvent::PointerMove { .. }
            | PlatformEvent::PointerLeave { .. }
            | PlatformEvent::LeftButtonDown { .. }
            | PlatformEvent::DockDragMove { .. }
            | PlatformEvent::LeftButtonUp { .. }
            | PlatformEvent::AnimationTick { .. }
            | PlatformEvent::RunningPollTick
            | PlatformEvent::DropFiles { .. }
            | PlatformEvent::RightClick { .. }) => self.handle_dock_event(host, ev),
            other => self.handle_widget_event(host, other),
        }
        if host.is_debug_window_open() && self.debug.take_dirty() {
            self.refresh_debug_ui(host);
        }
    }
}

#[cfg(windows)]
fn install_crash_log_hook(logs_dir: &std::path::Path) {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    let dir = logs_dir.to_path_buf();
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = fs::create_dir_all(&dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("crash.log"))
        {
            let loc = info.location().map_or_else(
                || "unknown".to_owned(),
                |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
            );
            let msg = info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("<non-string panic>");
            let _ = writeln!(
                file,
                "[v{APP_VERSION}] pid={} panic at {loc}: {msg}",
                std::process::id()
            );
        }
        prev(info);
    }));
}
