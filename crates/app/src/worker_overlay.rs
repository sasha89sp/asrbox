#![cfg(windows)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use asrbox_core::{IsolationMode, ModuleId, SupervisorCmd, WorkerMsg};
use asrbox_mod_debug::LogLevel;
use asrbox_mod_overlay::{
    load_overlay_config, save_overlay_config, GameOverlay, OverlayAnchor, MAX_OVERLAY_HEIGHT,
    MAX_OVERLAY_REFRESH_MS, MAX_OVERLAY_WIDTH, MIN_OVERLAY_HEIGHT, MIN_OVERLAY_REFRESH_MS,
    MIN_OVERLAY_WIDTH, OVERLAY_WIDGET_ID,
};
use asrbox_mod_settings::{OverlaySettingsView, SettingsAction};
use asrbox_platform_api::{
    load_appearance_config, DockPlatform, IconProvider, PixelRect, PlatformEvent, ToolsPlatform,
    WidgetPlatform,
};
use asrbox_platform_windows::{is_process_alive, WindowsDockHost};

use crate::tools::resolve_app_paths;
use crate::worker::{drain_cmds, emit_log, emit_msg, WorkerStats};
use crate::AppController;

pub fn run_overlay_worker(
    host: &mut WindowsDockHost,
    cmds: Arc<Mutex<VecDeque<SupervisorCmd>>>,
    parent_pid: u32,
) -> Result<(), String> {
    let paths = resolve_app_paths();
    let appearance_cfg = paths.appearance_cfg.clone();
    let pal = load_appearance_config(&appearance_cfg).palette();
    WidgetPlatform::set_theme(host, pal);
    let monitor = host.primary_work_area().unwrap_or(PixelRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    });
    let mut overlay = GameOverlay::load(paths.overlay_cfg, monitor);
    overlay.set_palette(host, pal);
    overlay.sync_window(host)?;
    emit_log(
        "INFO",
        "overlay",
        format!(
            "Воркер игрового оверлея готов: {}",
            overlay.status_summary()
        ),
    );
    let mut stats = WorkerStats::new("overlay");

    host.run_event_loop(|platform, ev| {
        if let PlatformEvent::SupervisorTick = ev {
            if parent_pid > 0 && !is_process_alive(parent_pid) {
                platform.request_quit();
                return;
            }
            for cmd in drain_cmds(&cmds) {
                apply_overlay_worker_cmd(platform, &mut overlay, &appearance_cfg, cmd);
            }
            stats.maybe_emit(platform);
            return;
        }
        handle_overlay_worker_event(platform, &mut overlay, &mut stats, ev);
    });
    Ok(())
}

fn apply_overlay_worker_cmd(
    host: &mut WindowsDockHost,
    overlay: &mut GameOverlay,
    appearance_cfg: &std::path::Path,
    cmd: SupervisorCmd,
) {
    match cmd {
        SupervisorCmd::ReloadConfig => {
            let pal = load_appearance_config(appearance_cfg).palette();
            WidgetPlatform::set_theme(host, pal);
            overlay.set_palette(host, pal);
            if overlay.reload_config(host).is_ok() {
                emit_log(
                    "INFO",
                    "overlay",
                    format!("Перечитан overlay.cfg: {}", overlay.status_summary()),
                );
            }
        }
        SupervisorCmd::SetOverlaySetupMode(on) => {
            if overlay.set_setup_mode(host, on).is_ok() {
                let label = if on {
                    "Режим настройки оверлея ВКЛ (можно перемещать и менять размер)"
                } else {
                    "Режим настройки оверлея ВЫКЛ (клики проходят насквозь)"
                };
                emit_log("INFO", "overlay", label.to_owned());
            }
        }
        SupervisorCmd::Shutdown => host.request_quit(),
        _ => {}
    }
}

fn handle_overlay_worker_event(
    host: &mut WindowsDockHost,
    overlay: &mut GameOverlay,
    stats: &mut WorkerStats,
    ev: PlatformEvent,
) {
    let t0 = Instant::now();
    stats.events += 1;
    match ev {
        PlatformEvent::WidgetRefreshTick {
            widget_id: OVERLAY_WIDGET_ID,
        } => overlay.on_refresh_tick(host),
        PlatformEvent::OverlayTelemetryReady { snapshot } => {
            if let Some(msg) = overlay.on_telemetry_ready(host, *snapshot) {
                emit_log("INFO", "overlay", msg);
            }
            stats.record_frame(t0);
        }
        PlatformEvent::WidgetMoved {
            widget_id: OVERLAY_WIDGET_ID,
            x,
            y,
        } => {
            if overlay.on_moved(host, x, y) {
                emit_msg(&WorkerMsg::ConfigChanged {
                    module: "overlay".to_owned(),
                });
            }
        }
        PlatformEvent::WidgetResized {
            widget_id: OVERLAY_WIDGET_ID,
            x,
            y,
            width,
            height,
            finished,
        } => {
            if overlay.on_resized(host, x, y, width, height, finished) {
                emit_msg(&WorkerMsg::ConfigChanged {
                    module: "overlay".to_owned(),
                });
            }
            stats.record_frame(t0);
        }
        PlatformEvent::DisplayChanged { work_area } => {
            overlay.on_display_changed(host, work_area);
        }
        _ => {}
    }
}

impl<I: IconProvider> AppController<I> {
    pub(crate) fn sync_inproc_overlay<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
    ) {
        let want =
            self.modules_cfg.isolation == IsolationMode::InProc && self.modules_cfg.overlay_enabled;
        if want && self.overlay.is_none() {
            let mon = PixelRect {
                x: self.work_area.x,
                y: self.work_area.y,
                width: self.work_area.width,
                height: self.work_area.height,
            };
            let mut ov = GameOverlay::load(self.paths.overlay_cfg.clone(), mon);
            ov.set_palette(host, self.appearance.palette());
            let _ = ov.set_setup_mode(host, self.overlay_setup_mode);
            self.debug.log(
                host.local_time_now(),
                LogLevel::Info,
                "overlay",
                format!("Игровой оверлей (inproc): {}", ov.status_summary()),
            );
            self.overlay = Some(ov);
        } else if !want && self.overlay.take().is_some() {
            let _ = host.destroy_widget_window(OVERLAY_WIDGET_ID);
        }
    }

    pub(crate) fn send_or_run_overlay_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        cmd: SupervisorCmd,
    ) {
        if self.modules_cfg.isolation == IsolationMode::Process {
            let _ = self.supervisor.send_cmd(ModuleId::Overlay, &cmd);
            return;
        }
        let Some(ov) = self.overlay.as_mut() else {
            return;
        };
        match cmd {
            SupervisorCmd::ReloadConfig => {
                let _ = ov.reload_config(host);
            }
            SupervisorCmd::SetOverlaySetupMode(on) => {
                let _ = ov.set_setup_mode(host, on);
            }
            _ => {}
        }
    }

    pub(crate) fn set_overlay_setup_mode<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        enabled: bool,
    ) {
        if enabled && !self.modules_cfg.overlay_enabled {
            self.modules_cfg.overlay_enabled = true;
            self.persist_modules_cfg();
            self.sync_runtime_modules(host);
        }
        if self.overlay_setup_mode == enabled {
            if enabled {
                self.send_or_run_overlay_cmd(host, SupervisorCmd::SetOverlaySetupMode(true));
            }
            return;
        }
        self.overlay_setup_mode = enabled;
        self.send_or_run_overlay_cmd(host, SupervisorCmd::SetOverlaySetupMode(enabled));
        let msg = if enabled {
            "Режим настройки игрового оверлея включён"
        } else {
            "Режим настройки игрового оверлея выключен (клик-сквозь активен)"
        };
        self.debug
            .log(host.local_time_now(), LogLevel::Info, "overlay", msg);
    }

    pub(crate) fn handle_overlay_inproc_event<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        ev: &PlatformEvent,
    ) -> bool {
        let t0 = Instant::now();
        let Some(ov) = self.overlay.as_mut() else {
            return false;
        };
        match ev {
            PlatformEvent::WidgetRefreshTick {
                widget_id: OVERLAY_WIDGET_ID,
            } => {
                ov.on_refresh_tick(host);
                true
            }
            PlatformEvent::OverlayTelemetryReady { snapshot } => {
                if let Some(msg) = ov.on_telemetry_ready(host, *snapshot.clone()) {
                    self.debug
                        .log(host.local_time_now(), LogLevel::Info, "overlay", msg);
                }
                let live_setup = ov.is_setup_mode();
                if live_setup != self.overlay_setup_mode {
                    self.overlay_setup_mode = live_setup;
                    self.render_settings_ui(host);
                }
                self.record_frame("overlay", t0);
                true
            }
            PlatformEvent::WidgetMoved {
                widget_id: OVERLAY_WIDGET_ID,
                x,
                y,
            } => {
                if ov.on_moved(host, *x, *y) {
                    self.render_settings_ui(host);
                }
                true
            }
            PlatformEvent::WidgetResized {
                widget_id: OVERLAY_WIDGET_ID,
                x,
                y,
                width,
                height,
                finished,
            } => {
                if ov.on_resized(host, *x, *y, *width, *height, *finished) {
                    self.render_settings_ui(host);
                }
                self.record_frame("overlay", t0);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn build_overlay_settings_view(&self) -> OverlaySettingsView {
        let cfg = load_overlay_config(&self.paths.overlay_cfg).unwrap_or_default();
        OverlaySettingsView {
            module_enabled: self.modules_cfg.overlay_enabled,
            auto_games_only: cfg.auto_games_only,
            force_topmost_fullscreen: cfg.force_topmost_fullscreen,
            setup_mode: self.overlay_setup_mode,
            anchor_index: cfg.anchor.to_index(),
            x: cfg.x,
            y: cfg.y,
            opacity_percent: cfg.opacity_percent,
            width: cfg.width,
            height: cfg.height,
            refresh_interval_ms: cfg.refresh_interval_ms,
        }
    }

    pub(crate) fn apply_overlay_setting<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        action: &SettingsAction,
    ) -> bool {
        if let SettingsAction::SetOverlaySetupMode(on) = action {
            self.set_overlay_setup_mode(host, *on);
            return true;
        }
        if let SettingsAction::OpenOverlayConfig = action {
            let _ = ToolsPlatform::open_text_file(host, &self.paths.overlay_cfg);
            return true;
        }
        let mut cfg = load_overlay_config(&self.paths.overlay_cfg).unwrap_or_default();
        match action {
            SettingsAction::SetOverlayAutoGamesOnly(v) => cfg.auto_games_only = *v,
            SettingsAction::SetOverlayForceTopmost(v) => cfg.force_topmost_fullscreen = *v,
            SettingsAction::SetOverlayAnchor(idx) => cfg.anchor = OverlayAnchor::from_index(*idx),
            SettingsAction::SetOverlayOpacity(v) => cfg.opacity_percent = (*v).clamp(20, 100),
            SettingsAction::SetOverlayWidth(w) => {
                cfg.width = (*w).clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
            }
            SettingsAction::SetOverlayHeight(h) => {
                cfg.height = (*h).clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT);
            }
            SettingsAction::SetOverlaySizePreset(w, h) => {
                cfg.width = (*w).clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
                cfg.height = (*h).clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT);
            }
            SettingsAction::SetOverlayRefreshMs(ms) => {
                cfg.refresh_interval_ms =
                    (*ms).clamp(MIN_OVERLAY_REFRESH_MS, MAX_OVERLAY_REFRESH_MS);
            }
            _ => return false,
        }
        if save_overlay_config(&self.paths.overlay_cfg, &cfg).is_ok() {
            self.reload_module_config(host, ModuleId::Overlay);
        }
        true
    }
}
