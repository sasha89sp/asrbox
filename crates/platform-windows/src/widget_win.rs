use asrbox_platform_api::{
    LocalDateTime, PixelPoint, PixelRect, WidgetFrame, WidgetLayer, WidgetMenuAction,
    WidgetMenuState, WidgetPlatform,
};
use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GWLP_USERDATA, GetCursorPos, KillTimer, SW_HIDE,
    SW_SHOWNOACTIVATE, SetTimer, SetWindowLongPtrW, ShowWindow, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::http::spawn_http_get;
use crate::present_layered_bgra;
use crate::tray::open_url_str;
use crate::widget_proc::{
    WIDGET_MINUTE_TIMER_ID, WIDGET_REFRESH_TIMER_ID, apply_widget_layer, register_widget_meta,
    remove_widget_meta, show_widget_context_menu, update_widget_layer_meta,
};
use crate::widget_text::{composite_widget_frame, query_local_time, render_tooltip_frame};
use crate::window::WindowsDockHost;

impl WidgetPlatform for WindowsDockHost {
    fn local_time_now(&self) -> LocalDateTime {
        query_local_time()
    }

    fn create_widget_window(
        &mut self,
        widget_id: u64,
        title: &str,
        rect: PixelRect,
        layer: WidgetLayer,
        acrylic: bool,
        drag_header_h: i32,
    ) -> Result<(), String> {
        if self.widget_windows.contains_key(&widget_id) {
            return Ok(());
        }
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        // SAFETY: Creating a layered popup widget window on the UI thread.
        unsafe {
            let hwnd = CreateWindowExW(
                ex_style,
                w!("AsrboxWidgetWindow"),
                PCWSTR(title_w.as_ptr()),
                WS_POPUP,
                rect.x,
                rect.y,
                i32::try_from(rect.width).unwrap_or(336),
                i32::try_from(rect.height).unwrap_or(520),
                None,
                None,
                Some(self.hinstance),
                None,
            )
            .map_err(|err| format!("CreateWindowExW(widget) failed: {err}"))?;

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, widget_id as isize);
            register_widget_meta(widget_id, layer, drag_header_h);
            apply_widget_layer(hwnd, layer, acrylic);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            self.widget_windows.insert(widget_id, hwnd);
        }
        Ok(())
    }

    fn destroy_widget_window(&mut self, widget_id: u64) -> Result<(), String> {
        remove_widget_meta(widget_id);
        if let Some(hwnd) = self.widget_windows.remove(&widget_id) {
            // SAFETY: `hwnd` belongs to this thread and is removed from the map.
            unsafe {
                let _ = KillTimer(Some(hwnd), WIDGET_MINUTE_TIMER_ID);
                let _ = KillTimer(Some(hwnd), WIDGET_REFRESH_TIMER_ID);
                let _ = DestroyWindow(hwnd);
            }
        }
        if let Some(tip_hwnd) = self.tooltip_hwnd {
            // SAFETY: Hiding tooltip popup when widget closes.
            unsafe {
                let _ = ShowWindow(tip_hwnd, SW_HIDE);
            }
        }
        Ok(())
    }

    fn set_widget_layer(
        &mut self,
        widget_id: u64,
        layer: WidgetLayer,
        acrylic: bool,
    ) -> Result<(), String> {
        update_widget_layer_meta(widget_id, layer);
        if let Some(&hwnd) = self.widget_windows.get(&widget_id) {
            apply_widget_layer(hwnd, layer, acrylic);
        }
        Ok(())
    }

    fn present_widget_frame(
        &mut self,
        widget_id: u64,
        rect: PixelRect,
        frame: &WidgetFrame,
    ) -> Result<(), String> {
        let hwnd = self
            .widget_windows
            .get(&widget_id)
            .copied()
            .ok_or_else(|| format!("unknown widget window {widget_id}"))?;
        let composited = composite_widget_frame(frame)?;
        present_layered_bgra(hwnd, rect, &composited)
    }

    fn set_widget_tooltip(&mut self, _widget_id: u64, text: Option<&str>) -> Result<(), String> {
        let Some(tip) = text.filter(|s| !s.trim().is_empty()) else {
            if let Some(hwnd) = self.tooltip_hwnd {
                // SAFETY: Hiding the owned tooltip window.
                unsafe {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
            return Ok(());
        };
        let tip_hwnd = self.ensure_tooltip_window()?;
        let frame = render_tooltip_frame(tip)?;
        let mut cursor = POINT::default();
        // SAFETY: Querying cursor position for non-activating tooltip placement.
        unsafe {
            let _ = GetCursorPos(&mut cursor);
        }
        let rect = PixelRect {
            x: cursor.x + 14,
            y: cursor.y + 18,
            width: frame.width,
            height: frame.height,
        };
        present_layered_bgra(tip_hwnd, rect, &frame)?;
        // SAFETY: Showing non-activating topmost tooltip popup.
        unsafe {
            let _ = ShowWindow(tip_hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }

    fn set_widget_timers(
        &mut self,
        widget_id: u64,
        refresh_sec: u32,
        minute_tick: bool,
    ) -> Result<(), String> {
        let Some(&hwnd) = self.widget_windows.get(&widget_id) else {
            return Ok(());
        };
        let refresh_ms = refresh_sec.clamp(60, 86_400).saturating_mul(1000);
        // SAFETY: Configuring Win32 timers for an owned widget HWND on UI thread.
        unsafe {
            let _ = SetTimer(Some(hwnd), WIDGET_REFRESH_TIMER_ID, refresh_ms, None);
            if minute_tick {
                let _ = SetTimer(Some(hwnd), WIDGET_MINUTE_TIMER_ID, 60_000, None);
            } else {
                let _ = KillTimer(Some(hwnd), WIDGET_MINUTE_TIMER_ID);
            }
        }
        Ok(())
    }

    fn show_widget_menu(
        &mut self,
        widget_id: u64,
        screen_point: PixelPoint,
        state: &WidgetMenuState,
    ) -> Result<Option<WidgetMenuAction>, String> {
        let hwnd = self
            .widget_windows
            .get(&widget_id)
            .copied()
            .unwrap_or(self.host_hwnd);
        show_widget_context_menu(hwnd, screen_point, state)
    }

    fn start_http_get(&mut self, request_id: u64, url: &str) -> Result<(), String> {
        spawn_http_get(self.host_hwnd, request_id, url.to_owned());
        Ok(())
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        open_url_str(url)
    }
}

impl WindowsDockHost {
    fn ensure_tooltip_window(&mut self) -> Result<HWND, String> {
        if let Some(hwnd) = self.tooltip_hwnd {
            return Ok(hwnd);
        }
        let ex = WS_EX_LAYERED
            | WS_EX_TOPMOST
            | WS_EX_TOOLWINDOW
            | WS_EX_NOACTIVATE
            | WS_EX_TRANSPARENT;
        // SAFETY: Creating a click-through layered tooltip popup on UI thread.
        let hwnd = unsafe {
            CreateWindowExW(
                ex,
                w!("AsrboxHostWindow"),
                w!("asrbox_tip"),
                WS_POPUP,
                0,
                0,
                10,
                10,
                None,
                None,
                Some(self.hinstance),
                None,
            )
            .map_err(|err| format!("CreateWindowExW(tooltip) failed: {err}"))?
        };
        self.tooltip_hwnd = Some(hwnd);
        Ok(hwnd)
    }
}
