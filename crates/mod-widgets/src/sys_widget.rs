use std::path::PathBuf;

use asrbox_platform_api::{
    FluentPalette, PixelPoint, PixelRect, SysTelemetry, WidgetLayer, WidgetMenuAction,
    WidgetMenuState, WidgetMenuVisibility, WidgetPlatform, WidgetSizeBounds,
};

use crate::storage::{MAX_OPACITY_PERCENT, MIN_OPACITY_PERCENT};
use crate::sys_config::{
    load_sysmon_config, save_sysmon_config, SysMonitorConfig, MAX_REFRESH_MS, MAX_SYSMON_HEIGHT,
    MAX_SYSMON_WIDTH, MIN_REFRESH_MS, MIN_SYSMON_HEIGHT, MIN_SYSMON_WIDTH,
};
use crate::sys_render::{
    hit_test_sysmon, render_sysmon_widget, SysHitTarget, SysRenderInput, SYSMON_HEADER_H,
};

pub const SYS_WIDGET_ID: u64 = 2;

pub struct SysMonitorWidget {
    config_path: PathBuf,
    config: SysMonitorConfig,
    window_open: bool,
    hovered: SysHitTarget,
    last_tooltip: Option<String>,
    telemetry: SysTelemetry,
    palette: FluentPalette,
    menu_vis: WidgetMenuVisibility,
}

impl SysMonitorWidget {
    pub fn load(config_path: PathBuf) -> Self {
        let config = load_sysmon_config(&config_path).unwrap_or_default();
        Self {
            config_path,
            config,
            window_open: false,
            hovered: SysHitTarget::None,
            last_tooltip: None,
            telemetry: SysTelemetry::default(),
            palette: FluentPalette::default(),
            menu_vis: WidgetMenuVisibility::default(),
        }
    }

    pub fn set_menu_visibility(&mut self, vis: WidgetMenuVisibility) {
        self.menu_vis = vis;
    }

    pub fn set_palette(&mut self, host: &mut dyn WidgetPlatform, palette: FluentPalette) {
        self.palette = palette;
        if self.window_open {
            self.redraw(host);
        }
    }

    pub fn is_visible(&self) -> bool {
        self.config.visible
    }

    pub fn config(&self) -> &SysMonitorConfig {
        &self.config
    }

    pub fn status_summary(&self) -> String {
        format!(
            "visible={}, {}x{}, refresh={}ms, opacity={}%",
            self.config.visible,
            self.config.width,
            self.config.height,
            self.config.refresh_interval_ms,
            self.config.opacity_percent
        )
    }

    pub fn sync_window(
        &mut self,
        host: &mut dyn WidgetPlatform,
        safe_mode: bool,
    ) -> Result<(), String> {
        let show = !safe_mode && self.config.visible;
        if show {
            if !self.window_open {
                let rect = self.window_rect();
                host.create_widget_window(
                    SYS_WIDGET_ID,
                    "Системный монитор",
                    rect,
                    self.config.layer,
                    SYSMON_HEADER_H,
                )?;
                let _ = host.set_widget_resize_bounds(
                    SYS_WIDGET_ID,
                    WidgetSizeBounds {
                        min_w: MIN_SYSMON_WIDTH,
                        min_h: MIN_SYSMON_HEIGHT,
                        max_w: MAX_SYSMON_WIDTH,
                        max_h: MAX_SYSMON_HEIGHT,
                    },
                );
                self.window_open = true;
            }
            let _ = host.set_widget_layer(SYS_WIDGET_ID, self.config.layer);
            let _ = host.set_widget_refresh_ms(SYS_WIDGET_ID, self.config.refresh_interval_ms);
            self.redraw(host);
            let _ = host.request_sys_telemetry();
        } else if self.window_open {
            let _ = host.destroy_widget_window(SYS_WIDGET_ID);
            self.window_open = false;
        }
        Ok(())
    }

    pub fn toggle_visible(&mut self, host: &mut dyn WidgetPlatform, safe_mode: bool) {
        self.config.visible = !self.config.visible;
        self.persist();
        let _ = self.sync_window(host, safe_mode);
    }

    pub fn reload_config(&mut self, host: &mut dyn WidgetPlatform, safe_mode: bool) -> String {
        match load_sysmon_config(&self.config_path) {
            Ok(cfg) => self.config = cfg,
            Err(err) => return format!("error: reload sysmon.cfg failed: {err}"),
        }
        let _ = self.sync_window(host, safe_mode);
        format!("reload sysmon.cfg: {}", self.status_summary())
    }

    pub fn on_pointer_move(&mut self, host: &mut dyn WidgetPlatform, pt: PixelPoint) {
        let hit = hit_test_sysmon(pt, self.config.width, self.config.height);
        if hit == self.hovered {
            return;
        }
        self.hovered = hit;
        let tip = match hit {
            SysHitTarget::SysInfoBtn => Some("Сведения о системе (msinfo32)".to_owned()),
            SysHitTarget::SettingsBtn => Some("Настройки системного монитора".to_owned()),
            SysHitTarget::RefreshBtn => Some("Обновить показатели сейчас".to_owned()),
            SysHitTarget::CpuSection => Some("Клик: открыть Монитор ресурсов (resmon)".to_owned()),
            SysHitTarget::GpuSection => Some("Клик: открыть Параметры экрана".to_owned()),
            SysHitTarget::RamDiskSection => Some("Клик: открыть Этот компьютер".to_owned()),
            SysHitTarget::NetSection => Some("Клик: открыть Сетевые подключения".to_owned()),
            SysHitTarget::TopProcsSection => Some("Клик: открыть Диспетчер задач".to_owned()),
            _ => None,
        };
        if tip != self.last_tooltip {
            let _ = host.set_widget_tooltip(SYS_WIDGET_ID, tip.as_deref());
            self.last_tooltip = tip;
        }
        self.redraw(host);
    }

    pub fn on_pointer_leave(&mut self, host: &mut dyn WidgetPlatform) {
        if self.last_tooltip.is_some() {
            let _ = host.set_widget_tooltip(SYS_WIDGET_ID, None);
            self.last_tooltip = None;
        }
        if self.hovered != SysHitTarget::None {
            self.hovered = SysHitTarget::None;
            self.redraw(host);
        }
    }

    pub fn on_left_click(&mut self, host: &mut dyn WidgetPlatform, pt: PixelPoint) -> String {
        match hit_test_sysmon(pt, self.config.width, self.config.height) {
            SysHitTarget::SettingsBtn => "open_settings_sysmon".to_owned(),
            SysHitTarget::RefreshBtn => {
                let _ = host.request_sys_telemetry();
                "sysmon manual refresh".to_owned()
            }
            SysHitTarget::SysInfoBtn => Self::launch_tool(host, "msinfo32.exe", ""),
            SysHitTarget::CpuSection => Self::launch_tool(host, "resmon.exe", ""),
            SysHitTarget::GpuSection => Self::launch_tool(host, "ms-settings:display", ""),
            SysHitTarget::RamDiskSection => {
                Self::launch_tool(host, "explorer.exe", "shell:MyComputerFolder")
            }
            SysHitTarget::NetSection => Self::launch_tool(host, "ncpa.cpl", ""),
            SysHitTarget::TopProcsSection => Self::launch_tool(host, "taskmgr.exe", ""),
            SysHitTarget::HeaderDrag | SysHitTarget::None => String::new(),
        }
    }

    fn launch_tool(host: &dyn WidgetPlatform, cmd: &str, args: &str) -> String {
        match host.launch_system_tool(cmd, args) {
            Ok(()) => format!("sysmon launch: {cmd}"),
            Err(e) => format!("error: sysmon launch {cmd}: {e}"),
        }
    }

    pub fn on_right_click(&mut self, host: &mut dyn WidgetPlatform, screen: PixelPoint) -> String {
        let state = WidgetMenuState {
            layer: self.config.layer,
            opacity_percent: self.config.opacity_percent,
            refresh_ms: Some(self.config.refresh_interval_ms),
            sync_interval_sec: None,
            menu_vis: self.menu_vis,
        };
        let Ok(Some(action)) = host.show_widget_menu(SYS_WIDGET_ID, screen, &state) else {
            return String::new();
        };
        match action {
            WidgetMenuAction::Refresh => {
                let _ = host.request_sys_telemetry();
                "sysmon refresh".to_owned()
            }
            WidgetMenuAction::OpenSettings => "open_settings_sysmon".to_owned(),
            WidgetMenuAction::OpenConfig => {
                if !self.config_path.exists() {
                    self.persist();
                }
                match host.open_text_file(&self.config_path) {
                    Ok(()) => "open sysmon.cfg".to_owned(),
                    Err(e) => format!("error: open sysmon.cfg: {e}"),
                }
            }
            WidgetMenuAction::SetLayerWallpaper => self.set_layer(host, WidgetLayer::Wallpaper),
            WidgetMenuAction::SetLayerNormal => self.set_layer(host, WidgetLayer::Normal),
            WidgetMenuAction::SetLayerTopmost => self.set_layer(host, WidgetLayer::Topmost),
            WidgetMenuAction::SetOpacity(pct) => {
                self.config.opacity_percent = pct.clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT);
                self.persist();
                self.redraw(host);
                format!("sysmon opacity={}%", self.config.opacity_percent)
            }
            WidgetMenuAction::SetRefreshMs(ms) => {
                self.config.refresh_interval_ms = ms.clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
                self.persist();
                let _ = host.set_widget_refresh_ms(SYS_WIDGET_ID, self.config.refresh_interval_ms);
                self.redraw(host);
                format!("sysmon refresh={}ms", self.config.refresh_interval_ms)
            }
            WidgetMenuAction::HideWidget => {
                self.config.visible = false;
                self.persist();
                let _ = self.sync_window(host, false);
                "sysmon hidden".to_owned()
            }
            WidgetMenuAction::SetSyncIntervalSec(_) => String::new(),
        }
    }

    pub fn on_moved(&mut self, x: i32, y: i32) {
        if self.config.x != x || self.config.y != y {
            self.config.x = x;
            self.config.y = y;
            self.persist();
        }
    }

    pub fn on_resized(
        &mut self,
        host: &mut dyn WidgetPlatform,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        finished: bool,
    ) {
        self.config.x = x;
        self.config.y = y;
        self.config.width = width.clamp(MIN_SYSMON_WIDTH, MAX_SYSMON_WIDTH);
        self.config.height = height.clamp(MIN_SYSMON_HEIGHT, MAX_SYSMON_HEIGHT);
        self.redraw(host);
        if finished {
            self.persist();
        }
    }

    pub fn on_refresh_tick(&mut self, host: &mut dyn WidgetPlatform) {
        if self.window_open {
            let _ = host.request_sys_telemetry();
        }
    }

    pub fn on_telemetry_ready(&mut self, host: &mut dyn WidgetPlatform, snap: SysTelemetry) {
        self.telemetry = snap;
        if self.window_open {
            self.redraw(host);
        }
    }

    fn set_layer(&mut self, host: &mut dyn WidgetPlatform, layer: WidgetLayer) -> String {
        self.config.layer = layer;
        let _ = host.set_widget_layer(SYS_WIDGET_ID, layer);
        self.persist();
        format!("sysmon layer={layer:?}")
    }

    fn redraw(&self, host: &mut dyn WidgetPlatform) {
        if !self.window_open {
            return;
        }
        let frame = render_sysmon_widget(&SysRenderInput {
            width: self.config.width,
            height: self.config.height,
            opacity_percent: self.config.opacity_percent,
            refresh_ms: self.config.refresh_interval_ms,
            now: host.local_time_now(),
            data: &self.telemetry,
            hovered: self.hovered,
            palette: self.palette,
        });
        let _ = host.present_widget_frame(SYS_WIDGET_ID, self.window_rect(), &frame);
    }

    fn window_rect(&self) -> PixelRect {
        PixelRect {
            x: self.config.x,
            y: self.config.y,
            width: self.config.width,
            height: self.config.height,
        }
    }

    fn persist(&self) {
        let _ = save_sysmon_config(&self.config_path, &self.config);
    }
}
