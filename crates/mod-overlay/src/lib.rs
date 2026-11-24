pub mod config;
pub mod draw;
pub mod render;

use std::path::PathBuf;

use asrbox_core::{Module, ModuleHealth, ModuleId};
use asrbox_platform_api::{
    FluentPalette, OverlayTelemetry, PixelRect, WidgetLayer, WidgetPlatform, WidgetSizeBounds,
};

pub use config::{
    load_overlay_config, parse_overlay_config, save_overlay_config, serialize_overlay_config,
    OverlayAnchor, OverlayConfig, MAX_OVERLAY_HEIGHT, MAX_OVERLAY_REFRESH_MS, MAX_OVERLAY_WIDTH,
    MIN_OVERLAY_HEIGHT, MIN_OVERLAY_REFRESH_MS, MIN_OVERLAY_WIDTH, OVERLAY_WIDGET_ID,
};
pub use render::render_game_overlay;

pub struct GameOverlay {
    config_path: PathBuf,
    cfg: OverlayConfig,
    snapshot: OverlayTelemetry,
    monitor_rect: PixelRect,
    window_created: bool,
    window_visible: bool,
    setup_mode: bool,
    running: bool,
    palette: FluentPalette,
}

impl GameOverlay {
    pub fn load(config_path: PathBuf, primary_monitor: PixelRect) -> Self {
        let cfg = load_overlay_config(&config_path).unwrap_or_default();
        Self {
            config_path,
            cfg,
            snapshot: OverlayTelemetry::default(),
            monitor_rect: primary_monitor,
            window_created: false,
            window_visible: false,
            setup_mode: false,
            running: true,
            palette: FluentPalette::default(),
        }
    }

    pub fn set_palette<P: WidgetPlatform>(&mut self, host: &mut P, palette: FluentPalette) {
        self.palette = palette;
        if self.window_visible {
            let _ = self.present(host);
        }
    }

    pub fn config(&self) -> &OverlayConfig {
        &self.cfg
    }

    pub fn is_setup_mode(&self) -> bool {
        self.setup_mode
    }

    pub fn should_display(&self) -> bool {
        self.setup_mode || !self.cfg.auto_games_only || self.snapshot.game_active
    }

    pub fn status_summary(&self) -> String {
        let mode = if self.setup_mode {
            "настройка"
        } else if self.cfg.auto_games_only {
            if self.snapshot.game_active {
                "в игре"
            } else {
                "ожидание игры"
            }
        } else {
            "всегда"
        };
        format!(
            "{mode}, {}x{}, anchor={}, opacity={}%",
            self.cfg.width,
            self.cfg.height,
            self.cfg.anchor.as_str(),
            self.cfg.opacity_percent
        )
    }

    pub fn sync_window<P: WidgetPlatform>(&mut self, host: &mut P) -> Result<(), String> {
        let rect = self.cfg.compute_rect(self.monitor_rect);
        if !self.window_created {
            host.create_widget_window(
                OVERLAY_WIDGET_ID,
                "asrbox_game_overlay",
                rect,
                WidgetLayer::Topmost,
                i32::MAX,
            )?;
            host.set_widget_resize_bounds(
                OVERLAY_WIDGET_ID,
                WidgetSizeBounds {
                    min_w: MIN_OVERLAY_WIDTH,
                    min_h: MIN_OVERLAY_HEIGHT,
                    max_w: MAX_OVERLAY_WIDTH,
                    max_h: MAX_OVERLAY_HEIGHT,
                },
            )?;
            self.window_created = true;
            self.window_visible = true;
        }
        host.set_widget_click_through(OVERLAY_WIDGET_ID, !self.setup_mode)?;
        host.set_widget_layer(OVERLAY_WIDGET_ID, WidgetLayer::Topmost)?;
        let poll_ms = if self.should_display() {
            self.cfg.refresh_interval_ms
        } else {
            1500
        };
        host.set_widget_refresh_ms(OVERLAY_WIDGET_ID, poll_ms)?;
        self.apply_visibility_and_present(host)?;
        self.request_sample(host);
        Ok(())
    }

    pub fn reload_config<P: WidgetPlatform>(&mut self, host: &mut P) -> Result<(), String> {
        self.cfg = load_overlay_config(&self.config_path).unwrap_or_default();
        self.sync_window(host)
    }

    pub fn set_setup_mode<P: WidgetPlatform>(
        &mut self,
        host: &mut P,
        enabled: bool,
    ) -> Result<(), String> {
        if self.setup_mode == enabled && self.window_created {
            return Ok(());
        }
        self.setup_mode = enabled;
        self.sync_window(host)
    }

    pub fn on_refresh_tick<P: WidgetPlatform>(&mut self, host: &mut P) {
        self.request_sample(host);
    }

    pub fn on_telemetry_ready<P: WidgetPlatform>(
        &mut self,
        host: &mut P,
        snap: OverlayTelemetry,
    ) -> Option<String> {
        let prev_game = self.snapshot.game_active;
        let prev_proc = self.snapshot.game_process.clone();
        if let Some(mon) = snap.monitor_rect {
            self.monitor_rect = mon;
        }
        self.snapshot = snap;

        if self.setup_mode && self.snapshot.game_active {
            self.setup_mode = false;
            let _ = host.set_widget_click_through(OVERLAY_WIDGET_ID, true);
        }

        let want = self.should_display();
        let vis_changed = want != self.window_visible;
        if vis_changed {
            let poll_ms = if want {
                self.cfg.refresh_interval_ms
            } else {
                1500
            };
            let _ = host.set_widget_refresh_ms(OVERLAY_WIDGET_ID, poll_ms);
        }
        let _ = self.apply_visibility_and_present(host);
        if want && self.snapshot.game_active && self.cfg.force_topmost_fullscreen {
            let _ = host.set_widget_layer(OVERLAY_WIDGET_ID, WidgetLayer::Topmost);
        }

        if self.snapshot.game_active != prev_game {
            if self.snapshot.game_active {
                return Some(format!(
                    "Обнаружена игра: {} -> оверлей показан (click-through{})",
                    self.snapshot.game_process,
                    if self.cfg.force_topmost_fullscreen {
                        ", FSO/Topmost"
                    } else {
                        ""
                    }
                ));
            }
            if !prev_proc.is_empty() {
                return Some(format!("Выход из игры ({prev_proc}) -> оверлей скрыт"));
            }
        }
        None
    }

    pub fn on_moved<P: WidgetPlatform>(&mut self, host: &mut P, x: i32, y: i32) -> bool {
        if !self.setup_mode {
            return false;
        }
        self.cfg.anchor = OverlayAnchor::Custom;
        self.cfg.x = x;
        self.cfg.y = y;
        let _ = save_overlay_config(&self.config_path, &self.cfg);
        let _ = self.present(host);
        true
    }

    pub fn on_resized<P: WidgetPlatform>(
        &mut self,
        host: &mut P,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        finished: bool,
    ) -> bool {
        if !self.setup_mode {
            return false;
        }
        self.cfg.width = width.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
        self.cfg.height = height.clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT);
        if self.cfg.anchor == OverlayAnchor::Custom {
            self.cfg.x = x;
            self.cfg.y = y;
        }
        let _ = self.present(host);
        if finished {
            let _ = save_overlay_config(&self.config_path, &self.cfg);
            return true;
        }
        false
    }

    pub fn on_display_changed<P: WidgetPlatform>(&mut self, host: &mut P, work_area: PixelRect) {
        self.monitor_rect = work_area;
        if self.window_visible {
            let _ = self.present(host);
        }
    }

    fn request_sample<P: WidgetPlatform>(&self, host: &mut P) {
        let full = self.should_display();
        let _ = host.request_overlay_telemetry(full, &self.cfg.extra_games);
    }

    fn apply_visibility_and_present<P: WidgetPlatform>(
        &mut self,
        host: &mut P,
    ) -> Result<(), String> {
        let want = self.should_display();
        if want {
            self.present(host)?;
            if !self.window_visible {
                host.set_widget_visible(OVERLAY_WIDGET_ID, true)?;
                self.window_visible = true;
            }
        } else if self.window_visible {
            host.set_widget_visible(OVERLAY_WIDGET_ID, false)?;
            self.window_visible = false;
        }
        Ok(())
    }

    fn present<P: WidgetPlatform>(&self, host: &mut P) -> Result<(), String> {
        if !self.window_created {
            return Ok(());
        }
        let rect = self.cfg.compute_rect(self.monitor_rect);
        let frame = render_game_overlay(&self.cfg, &self.snapshot, self.setup_mode, &self.palette);
        host.present_widget_frame(OVERLAY_WIDGET_ID, rect, &frame)
    }
}

impl Module for GameOverlay {
    fn id(&self) -> ModuleId {
        ModuleId::Overlay
    }

    fn start(&mut self) -> Result<(), String> {
        self.running = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.running = false;
        Ok(())
    }

    fn health(&self) -> ModuleHealth {
        if self.running {
            ModuleHealth::Running {
                pid: None,
                restarts: 0,
            }
        } else {
            ModuleHealth::Stopped
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_game_overlay_across_sizes_and_modes() {
        let snap = OverlayTelemetry {
            fps: Some(144),
            fps_low_1pct: Some(112),
            display_hz: 144,
            gpu_temp_c: Some(65),
            gpu_usage_pct: Some(98),
            vram_used_mb: 15_155,
            vram_total_mb: 16_384,
            cpu_temp_c: Some(58),
            cpu_usage_pct: 42,
            ram_used_mb: 14_540,
            ram_total_mb: 32_768,
            ram_usage_pct: 44,
            net_ping_ms: Some(148),
            net_loss_tenths_pct: 24,
            game_active: true,
            game_process: "cyberpunk2077.exe".to_owned(),
            monitor_rect: None,
        };
        for (w, h, setup) in [(240, 144, false), (320, 188, false), (480, 280, true)] {
            let cfg = OverlayConfig {
                width: w,
                height: h,
                ..OverlayConfig::default()
            };
            let pal = FluentPalette::default();
            let frame = render_game_overlay(&cfg, &snap, setup, &pal);
            assert_eq!(frame.bgra.width, w);
            assert_eq!(frame.bgra.height, h);
            assert!(frame.texts.iter().any(|t| t.text == "144"));
            assert!(frame.texts.iter().any(|t| t.text == "65°C"));
            assert!(frame.texts.iter().any(|t| t.text == "2.4% PL"));
        }
    }
}
