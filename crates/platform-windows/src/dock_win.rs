use std::collections::HashSet;
use std::path::{Path, PathBuf};

use asrbox_platform_api::{
    BgraFrame, DockMenuAction, DockMenuState, DockPlatform, PixelPoint, PixelRect, TrayAction,
    TrayMenuEntry, TrayModuleFlags, WidgetFrame, WidgetLayer,
};
use windows::core::{w, PCWSTR};
use windows::Win32::UI::Shell::DragAcceptFiles;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, KillTimer, PostQuitMessage, SetTimer, SetWindowLongPtrW,
    ShowWindow, GWLP_USERDATA, SW_SHOWNOACTIVATE, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

use crate::dock_menu::show_dock_context_menu;
use crate::dock_proc::{remove_dock_layer_meta, set_dock_layer_meta};
use crate::present_layered_bgra;
use crate::query_primary_work_area;
use crate::running::{activate_or_launch_target, query_running_exe_names, resolve_target_exe_name};
use crate::shell::{open_in_notepad, pick_icon_dialog};
use crate::tray::show_tray_context_menu;
use crate::widget_proc::apply_widget_layer;
use crate::widget_text::composite_widget_frame;
use crate::window::{
    WindowsDockHost, ANIMATION_TICK_MS, ANIMATION_TIMER_ID, RUNNING_POLL_MS, RUNNING_POLL_TIMER_ID,
};

impl DockPlatform for WindowsDockHost {
    fn primary_work_area(&self) -> Result<PixelRect, String> {
        query_primary_work_area()
    }

    fn create_dock_window(
        &mut self,
        dock_id: u64,
        title: &str,
        rect: PixelRect,
    ) -> Result<(), String> {
        if self.windows.contains_key(&dock_id) {
            return Ok(());
        }
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        // SAFETY: Creating a layered popup dock window with a registered class on UI thread.
        unsafe {
            let hwnd = CreateWindowExW(
                ex_style,
                w!("AsrboxDockWindow"),
                PCWSTR(title_wide.as_ptr()),
                WS_POPUP,
                rect.x,
                rect.y,
                i32::try_from(rect.width).unwrap_or(100),
                i32::try_from(rect.height).unwrap_or(64),
                None,
                None,
                Some(self.hinstance),
                None,
            )
            .map_err(|err| format!("CreateWindowExW(dock) failed: {err}"))?;

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, dock_id as isize);
            set_dock_layer_meta(dock_id, WidgetLayer::Wallpaper);
            apply_widget_layer(hwnd, WidgetLayer::Wallpaper);
            DragAcceptFiles(hwnd, true);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            self.windows.insert(dock_id, hwnd);
        }
        Ok(())
    }

    fn destroy_dock_window(&mut self, dock_id: u64) -> Result<(), String> {
        remove_dock_layer_meta(dock_id);
        if let Some(hwnd) = self.windows.remove(&dock_id) {
            // SAFETY: `hwnd` belongs to this thread and is removed from the map.
            unsafe {
                let _ = KillTimer(Some(hwnd), ANIMATION_TIMER_ID);
                let _ = DestroyWindow(hwnd);
            }
        }
        Ok(())
    }

    fn set_dock_layer(&mut self, dock_id: u64, layer: WidgetLayer) -> Result<(), String> {
        set_dock_layer_meta(dock_id, layer);
        if let Some(&hwnd) = self.windows.get(&dock_id) {
            apply_widget_layer(hwnd, layer);
        }
        Ok(())
    }

    fn present_dock_frame(
        &mut self,
        dock_id: u64,
        rect: PixelRect,
        frame: &BgraFrame,
    ) -> Result<(), String> {
        let hwnd = self
            .windows
            .get(&dock_id)
            .copied()
            .ok_or_else(|| format!("unknown dock window {dock_id}"))?;
        present_layered_bgra(hwnd, rect, frame)
    }

    fn present_dock_widget_frame(
        &mut self,
        dock_id: u64,
        rect: PixelRect,
        frame: &WidgetFrame,
    ) -> Result<(), String> {
        let hwnd = self
            .windows
            .get(&dock_id)
            .copied()
            .ok_or_else(|| format!("unknown dock window {dock_id}"))?;
        let composited = composite_widget_frame(frame)?;
        present_layered_bgra(hwnd, rect, &composited)
    }

    fn set_dock_tooltip(&mut self, _dock_id: u64, text: Option<&str>) -> Result<(), String> {
        self.show_tooltip_text(text)
    }

    fn set_animation_timer(&mut self, dock_id: u64, active: bool) -> Result<(), String> {
        let Some(&hwnd) = self.windows.get(&dock_id) else {
            return Ok(());
        };
        // SAFETY: Starting/stopping a Win32 timer for an owned HWND on UI thread.
        unsafe {
            if active {
                let _ = SetTimer(Some(hwnd), ANIMATION_TIMER_ID, ANIMATION_TICK_MS, None);
            } else {
                let _ = KillTimer(Some(hwnd), ANIMATION_TIMER_ID);
            }
        }
        Ok(())
    }

    fn set_running_poll_timer(&mut self, active: bool) -> Result<(), String> {
        // SAFETY: Configuring lightweight 2s running-app polling timer on owned host_hwnd.
        unsafe {
            if active {
                let _ = SetTimer(
                    Some(self.host_hwnd),
                    RUNNING_POLL_TIMER_ID,
                    RUNNING_POLL_MS,
                    None,
                );
            } else {
                let _ = KillTimer(Some(self.host_hwnd), RUNNING_POLL_TIMER_ID);
            }
        }
        Ok(())
    }

    fn query_running_exe_names(&self) -> HashSet<String> {
        query_running_exe_names()
    }

    fn resolve_target_exe_name(&self, path: &Path) -> Option<String> {
        resolve_target_exe_name(path)
    }

    fn show_dock_menu(
        &mut self,
        dock_id: u64,
        screen_point: PixelPoint,
        state: &DockMenuState,
    ) -> Result<Option<DockMenuAction>, String> {
        let hwnd = self
            .windows
            .get(&dock_id)
            .copied()
            .unwrap_or(self.host_hwnd);
        show_dock_context_menu(self.hinstance, hwnd, screen_point, state, self.theme)
    }

    fn show_tray_menu(
        &mut self,
        screen_point: PixelPoint,
        docks: &[TrayMenuEntry],
        flags: TrayModuleFlags,
    ) -> Result<Option<TrayAction>, String> {
        show_tray_context_menu(
            self.hinstance,
            self.host_hwnd,
            screen_point,
            docks,
            flags,
            self.theme,
        )
    }

    fn pick_icon_file(&mut self) -> Result<Option<PathBuf>, String> {
        pick_icon_dialog(self.host_hwnd)
    }

    fn activate_or_launch_target(&self, path: &Path, focus_running: bool) -> Result<bool, String> {
        activate_or_launch_target(path, focus_running)
    }

    fn open_text_file(&self, path: &Path) -> Result<(), String> {
        open_in_notepad(path)
    }

    fn set_theme(&mut self, palette: asrbox_platform_api::FluentPalette) {
        self.theme = palette;
    }

    fn request_quit(&mut self) {
        // SAFETY: Posting WM_QUIT terminates the UI message loop cleanly.
        unsafe {
            PostQuitMessage(0);
        }
    }
}
