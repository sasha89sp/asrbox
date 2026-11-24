use std::path::Path;

use asrbox_platform_api::{
    LocalDateTime, PixelPoint, PixelRect, WidgetFrame, WidgetLayer, WidgetMenuAction,
    WidgetMenuState, WidgetPlatform, WidgetSizeBounds,
};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetCursorPos, GetWindowLongPtrW, KillTimer, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, GWLP_USERDATA, GWL_EXSTYLE, HWND_TOPMOST,
    IDC_ARROW, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNOACTIVATE,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::font_engine::set_active_font_renderer;
use crate::http::spawn_http_get;
use crate::overlay_telemetry::request_overlay_sample;
use crate::present_layered_bgra;
use crate::shell::{open_in_notepad, open_url_str};
use crate::sys_telemetry::request_telemetry_sample;
use crate::widget_menu::show_widget_context_menu;
use crate::widget_proc::{
    apply_widget_layer, register_widget_meta, remove_widget_meta, set_widget_bounds_meta,
    set_widget_click_through_meta, update_widget_layer_meta, WIDGET_MINUTE_TIMER_ID,
    WIDGET_REFRESH_TIMER_ID,
};
use crate::widget_text::{
    composite_widget_frame, compute_tooltip_rect, query_local_time, render_tooltip_frame,
};
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
        drag_header_h: i32,
    ) -> Result<(), String> {
        if self.widget_windows.contains_key(&widget_id) {
            return Ok(());
        }
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let mut ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        if layer == WidgetLayer::Topmost {
            ex_style |= WS_EX_TOPMOST;
        }
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
            apply_widget_layer(hwnd, layer);
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

    fn set_widget_layer(&mut self, widget_id: u64, layer: WidgetLayer) -> Result<(), String> {
        update_widget_layer_meta(widget_id, layer);
        if let Some(&hwnd) = self.widget_windows.get(&widget_id) {
            apply_widget_layer(hwnd, layer);
        }
        Ok(())
    }

    fn set_widget_click_through(
        &mut self,
        widget_id: u64,
        click_through: bool,
    ) -> Result<(), String> {
        set_widget_click_through_meta(widget_id, click_through);
        if let Some(&hwnd) = self.widget_windows.get(&widget_id) {
            // SAFETY: Toggling WS_EX_TRANSPARENT on an owned layered HWND on the UI thread.
            unsafe {
                let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let flag = WS_EX_TRANSPARENT.0 as isize;
                if click_through {
                    ex |= flag;
                } else {
                    ex &= !flag;
                }
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
        }
        Ok(())
    }

    fn set_widget_visible(&mut self, widget_id: u64, visible: bool) -> Result<(), String> {
        if let Some(&hwnd) = self.widget_windows.get(&widget_id) {
            // SAFETY: Showing/hiding an owned popup widget HWND without activating it.
            unsafe {
                if visible {
                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                } else {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
        }
        Ok(())
    }

    fn set_widget_resize_bounds(
        &mut self,
        widget_id: u64,
        bounds: asrbox_platform_api::WidgetSizeBounds,
    ) -> Result<(), String> {
        set_widget_bounds_meta(widget_id, bounds);
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
        self.show_tooltip_text(text)
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

    fn set_widget_refresh_ms(&mut self, widget_id: u64, interval_ms: u32) -> Result<(), String> {
        let Some(&hwnd) = self.widget_windows.get(&widget_id) else {
            return Ok(());
        };
        let ms = interval_ms.clamp(500, 60_000);
        // SAFETY: Configuring sub-minute Win32 refresh timer on owned widget HWND.
        unsafe {
            let _ = SetTimer(Some(hwnd), WIDGET_REFRESH_TIMER_ID, ms, None);
        }
        Ok(())
    }

    fn request_sys_telemetry(&mut self) -> Result<(), String> {
        request_telemetry_sample(self.host_hwnd);
        Ok(())
    }

    fn request_overlay_telemetry(
        &mut self,
        full_sample: bool,
        extra_game_procs: &[String],
    ) -> Result<(), String> {
        request_overlay_sample(self.host_hwnd, full_sample, extra_game_procs);
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
        show_widget_context_menu(self.hinstance, hwnd, screen_point, state, self.theme)
    }

    fn start_http_get(&mut self, request_id: u64, url: &str) -> Result<(), String> {
        spawn_http_get(self.host_hwnd, request_id, url.to_owned());
        Ok(())
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        open_url_str(url)
    }

    fn open_text_file(&self, path: &Path) -> Result<(), String> {
        open_in_notepad(path)
    }

    fn set_theme(&mut self, palette: asrbox_platform_api::FluentPalette) {
        self.theme = palette;
        set_active_font_renderer(palette.font_renderer);
    }
}

impl WindowsDockHost {
    pub(crate) fn show_tooltip_text(&mut self, text: Option<&str>) -> Result<(), String> {
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
        let frame = render_tooltip_frame(tip, &self.theme)?;
        let mut cursor = POINT::default();
        // SAFETY: Querying cursor position for non-activating tooltip placement.
        unsafe {
            let _ = GetCursorPos(&mut cursor);
        }
        let area = crate::query_primary_work_area().unwrap_or(PixelRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        });
        let rect = compute_tooltip_rect(cursor.x, cursor.y, frame.width, frame.height, area);
        present_layered_bgra(tip_hwnd, rect, &frame)?;
        // SAFETY: Showing non-activating topmost tooltip popup.
        unsafe {
            let _ = ShowWindow(tip_hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }

    fn ensure_tooltip_window(&mut self) -> Result<HWND, String> {
        if let Some(hwnd) = self.tooltip_hwnd {
            return Ok(hwnd);
        }
        let ex =
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT;
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

const EDGE_LEFT: u8 = 1;
const EDGE_RIGHT: u8 = 2;
const EDGE_TOP: u8 = 4;
const EDGE_BOTTOM: u8 = 8;
const RESIZE_BORDER_PX: i32 = 8;
const RESIZE_CORNER_PX: i32 = 14;

pub(crate) fn hit_resize_edges(pt: PixelPoint, w: i32, h: i32) -> u8 {
    if pt.x < 0 || pt.y < 0 || pt.x >= w || pt.y >= h {
        return 0;
    }
    let (l_c, r_c) = (pt.x < RESIZE_CORNER_PX, pt.x >= w - RESIZE_CORNER_PX);
    let (t_c, b_c) = (pt.y < RESIZE_CORNER_PX, pt.y >= h - RESIZE_CORNER_PX);
    if (l_c || r_c) && (t_c || b_c) {
        return if l_c { EDGE_LEFT } else { EDGE_RIGHT } | if t_c { EDGE_TOP } else { EDGE_BOTTOM };
    }
    let mut edges = 0u8;
    if pt.x < RESIZE_BORDER_PX {
        edges |= EDGE_LEFT;
    } else if pt.x >= w - RESIZE_BORDER_PX {
        edges |= EDGE_RIGHT;
    }
    if pt.y < RESIZE_BORDER_PX {
        edges |= EDGE_TOP;
    } else if pt.y >= h - RESIZE_BORDER_PX {
        edges |= EDGE_BOTTOM;
    }
    edges
}

pub(crate) fn cursor_for_edges(edges: u8) -> Option<PCWSTR> {
    match edges {
        0 => Some(IDC_ARROW),
        e if e == EDGE_LEFT || e == EDGE_RIGHT => Some(IDC_SIZEWE),
        e if e == EDGE_TOP || e == EDGE_BOTTOM => Some(IDC_SIZENS),
        e if e == (EDGE_LEFT | EDGE_TOP) || e == (EDGE_RIGHT | EDGE_BOTTOM) => Some(IDC_SIZENWSE),
        e if e == (EDGE_RIGHT | EDGE_TOP) || e == (EDGE_LEFT | EDGE_BOTTOM) => Some(IDC_SIZENESW),
        _ => None,
    }
}

pub(crate) fn compute_resized_rect(
    start: RECT,
    edges: u8,
    dx: i32,
    dy: i32,
    bounds: WidgetSizeBounds,
) -> RECT {
    let (mut left, mut right, mut top, mut bottom) =
        (start.left, start.right, start.top, start.bottom);
    let (min_w, max_w) = (bounds.min_w as i32, bounds.max_w as i32);
    let (min_h, max_h) = (bounds.min_h as i32, bounds.max_h as i32);
    if (edges & EDGE_RIGHT) != 0 {
        right = left + (right - left + dx).clamp(min_w, max_w);
    } else if (edges & EDGE_LEFT) != 0 {
        left = right - (right - left - dx).clamp(min_w, max_w);
    }
    if (edges & EDGE_BOTTOM) != 0 {
        bottom = top + (bottom - top + dy).clamp(min_h, max_h);
    } else if (edges & EDGE_TOP) != 0 {
        top = bottom - (bottom - top - dy).clamp(min_h, max_h);
    }
    RECT {
        left,
        top,
        right,
        bottom,
    }
}
