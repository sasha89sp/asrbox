#![cfg(windows)]

use std::env;
use std::path::PathBuf;

use asrbox_mod_about::{build_about_text, build_paths_clipboard, AppPaths};
use asrbox_mod_debug::LogLevel;
use asrbox_platform_api::{
    AboutUiCommand, DebugUiCommand, DockPlatform, IconProvider, PixelPoint, ToolsPlatform,
    TrayAction, WidgetPlatform,
};

use crate::{AppController, APP_VERSION};

pub(crate) fn resolve_app_paths() -> AppPaths {
    let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("asrbox.exe"));
    let base = exe
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    AppPaths::from_exe_and_data(exe, base.join("data"))
}

impl<I: IconProvider> AppController<I> {
    pub(crate) fn on_tray<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        screen: PixelPoint,
    ) {
        self.debug.record_event("app", "tray_menu");
        let entries = self.docks.tray_entries();
        let cal_vis = self.calendar.is_visible();
        let Ok(Some(action)) = host.show_tray_menu(screen, &entries, cal_vis) else {
            return;
        };
        self.apply_tray_action(host, action);
    }

    fn apply_tray_action<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: TrayAction,
    ) {
        let now = host.local_time_now();
        match action {
            TrayAction::AddDock => {
                self.docks.add_new_dock(host, &mut self.safe_mode);
                self.debug
                    .log(now, LogLevel::Info, "dock", "Трей: добавлена док-панель");
            }
            TrayAction::ToggleDock(id) => {
                self.docks.toggle_dock(host, id, self.safe_mode);
                self.debug
                    .log(now, LogLevel::Info, "dock", format!("Трей: док {id}"));
            }
            TrayAction::RemoveDock(id) => {
                self.docks.remove_dock(host, id);
                self.debug.log(
                    now,
                    LogLevel::Info,
                    "dock",
                    format!("Трей: удалён док {id}"),
                );
            }
            TrayAction::OpenDockConfig => {
                let res = self.docks.open_config_file(host);
                self.log_io_result(now, "dock", "Трей: открыт docks.cfg", res);
            }
            TrayAction::ToggleCalendarWidget => {
                self.safe_mode = false;
                self.calendar.toggle_visible(host, false);
                self.debug.log(
                    now,
                    LogLevel::Info,
                    "widgets",
                    format!("Трей: календарь -> {}", self.calendar.status_summary()),
                );
            }
            TrayAction::OpenWidgetConfig => {
                let res = self.calendar.open_config_file(host);
                self.log_io_result(now, "widgets", "Трей: открыт widgets.cfg", res);
            }
            TrayAction::OpenDebugWindow => self.open_debug_window(host),
            TrayAction::OpenAboutWindow => self.open_about_window(host),
            TrayAction::Exit => {
                self.docks.persist();
                host.request_quit();
            }
        }
    }

    fn open_debug_window<H: WidgetPlatform + ToolsPlatform>(&mut self, host: &mut H) {
        let now = host.local_time_now();
        let title = format!("asrbox v{APP_VERSION} — Диагностика и логи");
        match host.open_debug_window(&title) {
            Ok(()) => {
                self.debug
                    .log(now, LogLevel::Info, "debug", "Открыто окно диагностики");
                self.refresh_debug_ui(host);
            }
            Err(e) => self.debug.log(now, LogLevel::Error, "debug", e),
        }
    }

    fn open_about_window<H: WidgetPlatform + ToolsPlatform>(&mut self, host: &mut H) {
        let now = host.local_time_now();
        let title = format!("О программе — asrbox v{APP_VERSION}");
        let content = build_about_text(APP_VERSION, &self.paths);
        let res = host.open_about_window(&title, &content);
        self.log_io_result(now, "about", "Открыто окно «О программе»", res);
    }

    pub(crate) fn on_debug_cmd<H: WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        cmd: DebugUiCommand,
    ) {
        let now = host.local_time_now();
        match cmd {
            DebugUiCommand::ToggleVerbose => self.debug.toggle_verbose(now),
            DebugUiCommand::ToggleFileLog => self.debug.toggle_file_logging(now),
            DebugUiCommand::CycleFilter => self.debug.cycle_filter(),
            DebugUiCommand::ClearLogs => self.debug.clear(now),
            DebugUiCommand::CopyReport => {
                let proc = host.query_process_metrics();
                let rep = self
                    .debug
                    .build_report(APP_VERSION, &self.paths.data_dir, proc);
                let res = host.copy_to_clipboard(&rep);
                self.log_io_result(now, "debug", "Отчёт скопирован в буфер обмена", res);
            }
            DebugUiCommand::SaveReport => {
                let proc = host.query_process_metrics();
                match self
                    .debug
                    .save_report(APP_VERSION, &self.paths.data_dir, proc)
                {
                    Ok(path) => {
                        self.debug.log(
                            now,
                            LogLevel::Info,
                            "debug",
                            format!("Отчёт сохранён: {}", path.display()),
                        );
                        let _ = ToolsPlatform::open_text_file(host, &path);
                    }
                    Err(e) => self.debug.log(now, LogLevel::Error, "debug", e),
                }
            }
            DebugUiCommand::WindowClosed => {
                self.debug
                    .log(now, LogLevel::Trace, "debug", "Окно диагностики закрыто");
            }
        }
        if host.is_debug_window_open() {
            self.refresh_debug_ui(host);
        }
    }

    pub(crate) fn on_about_cmd<H: WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        cmd: AboutUiCommand,
    ) {
        let now = host.local_time_now();
        match cmd {
            AboutUiCommand::OpenDataFolder => {
                let _ = std::fs::create_dir_all(&self.paths.data_dir);
                let res = host.open_folder(&self.paths.data_dir);
                self.log_io_result(now, "about", "Открыта папка data", res);
            }
            AboutUiCommand::CopyPaths => {
                let txt = build_paths_clipboard(&self.paths);
                let res = host.copy_to_clipboard(&txt);
                self.log_io_result(now, "about", "Пути скопированы в буфер", res);
            }
            AboutUiCommand::WindowClosed => {}
        }
    }

    pub(crate) fn refresh_debug_ui<H: ToolsPlatform>(&mut self, host: &mut H) {
        let proc = host.query_process_metrics();
        let (summary, logs) = self.debug.render_live(proc);
        host.update_debug_window(
            &summary,
            &logs,
            self.debug.verbose_trace(),
            self.debug.file_logging(),
            self.debug.filter_label(),
        );
    }

    fn log_io_result(
        &mut self,
        now: asrbox_platform_api::LocalDateTime,
        module: &'static str,
        ok_msg: &str,
        res: Result<(), String>,
    ) {
        match res {
            Ok(()) => self.debug.log(now, LogLevel::Info, module, ok_msg),
            Err(e) => self.debug.log(now, LogLevel::Error, module, e),
        }
    }
}
