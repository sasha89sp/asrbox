use std::ffi::c_void;
use std::mem;

use asrbox_platform_api::{
    BgraFrame, FluentMenuItem, FluentMenuSpec, FluentPalette, PixelPoint, PixelRect,
};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, POINT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
    MonitorFromPoint, ReleaseDC, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION,
    DIB_RGB_COLORS, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, ShowWindow, UpdateLayeredWindow, SW_SHOWNOACTIVATE, ULW_ALPHA,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::fluent_menu::MenuWindow;
use crate::fluent_menu_render::{
    clamp_submenu_origin, measure_menu, render_fluent_menu, MenuLayout,
};
use crate::icons::make_top_down_bmi;

pub(crate) struct SubmenuPlan {
    pub row_idx: usize,
    pub hinstance: HINSTANCE,
    pub work_area: PixelRect,
    pub palette: FluentPalette,
    pub root_origin: PixelPoint,
    pub root_layout: MenuLayout,
    pub row_y: i32,
    pub items: Vec<FluentMenuItem>,
}

pub(crate) fn destroy_hwnd(hwnd: HWND) {
    // SAFETY: Destroying an owned submenu popup HWND after releasing RefCell borrows.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

pub(crate) fn create_submenu_window(plan: SubmenuPlan) -> Option<MenuWindow> {
    let sub_spec = FluentMenuSpec {
        quick_actions: Vec::new(),
        items: plan.items,
        min_width: 196,
    };
    let sub_layout = measure_menu(&sub_spec);
    let sub_origin = clamp_submenu_origin(
        plan.root_origin,
        &plan.root_layout,
        plan.row_y,
        &sub_layout,
        plan.work_area,
    );
    let sub_hwnd = create_menu_hwnd(plan.hinstance, sub_origin, &sub_layout, true).ok()?;
    let Ok(sub_frame) = render_fluent_menu(&sub_spec, &sub_layout, &plan.palette, None, None, None)
    else {
        destroy_hwnd(sub_hwnd);
        return None;
    };
    let _ = present_with_alpha(sub_hwnd, sub_origin, &sub_frame, 255);
    // SAFETY: Showing non-activating submenu popup alongside active root menu.
    unsafe {
        let _ = ShowWindow(sub_hwnd, SW_SHOWNOACTIVATE);
    }
    Some(MenuWindow {
        hwnd: sub_hwnd,
        spec: sub_spec,
        layout: sub_layout,
        origin: sub_origin,
        hovered_row: None,
        hovered_quick: None,
        alpha: 255,
        frame: Some(sub_frame),
    })
}

pub(crate) fn repaint_window(
    win: &mut MenuWindow,
    palette: &FluentPalette,
    open_sub: Option<usize>,
) {
    if let Ok(frame) = render_fluent_menu(
        &win.spec,
        &win.layout,
        palette,
        win.hovered_row,
        win.hovered_quick,
        open_sub,
    ) {
        let _ = present_with_alpha(win.hwnd, win.origin, &frame, win.alpha);
        win.frame = Some(frame);
    }
}

pub(crate) fn hit_quick(layout: &MenuLayout, pt: PixelPoint) -> Option<usize> {
    layout
        .quick_btns
        .iter()
        .position(|b| pt.x >= b.x && pt.x < b.x + b.w && pt.y >= b.y && pt.y < b.y + b.h)
}

pub(crate) fn hit_row(layout: &MenuLayout, pt: PixelPoint) -> Option<usize> {
    if pt.x < layout.panel_x || pt.x >= layout.panel_x + layout.panel_w {
        return None;
    }
    layout
        .rows
        .iter()
        .find(|r| pt.y >= r.y && pt.y < r.y + r.h)
        .map(|r| r.item_index)
}

pub(crate) fn create_menu_hwnd(
    hinstance: HINSTANCE,
    origin: PixelPoint,
    layout: &MenuLayout,
    no_activate: bool,
) -> Result<HWND, String> {
    let mut ex = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
    if no_activate {
        ex |= WS_EX_NOACTIVATE;
    }
    // SAFETY: Creating layered Fluent popup menu window from registered class on UI thread.
    unsafe {
        CreateWindowExW(
            ex,
            w!("AsrboxFluentMenuWindow"),
            w!(""),
            WS_POPUP,
            origin.x,
            origin.y,
            layout.total_w as i32,
            layout.total_h as i32,
            None,
            None,
            Some(hinstance),
            None,
        )
        .map_err(|e| format!("CreateWindowExW(fluent_menu) failed: {e}"))
    }
}

pub(crate) fn monitor_work_area_at(pt: PixelPoint) -> PixelRect {
    // SAFETY: Querying monitor work area at cursor coordinates using valid MONITORINFO struct.
    unsafe {
        let hmon = MonitorFromPoint(POINT { x: pt.x, y: pt.y }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: u32::try_from(mem::size_of::<MONITORINFO>()).unwrap_or(0),
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            let r = mi.rcWork;
            return PixelRect {
                x: r.left,
                y: r.top,
                width: u32::try_from((r.right - r.left).max(1)).unwrap_or(1920),
                height: u32::try_from((r.bottom - r.top).max(1)).unwrap_or(1080),
            };
        }
    }
    crate::query_primary_work_area().unwrap_or(PixelRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    })
}

pub(crate) fn present_with_alpha(
    hwnd: HWND,
    origin: PixelPoint,
    frame: &BgraFrame,
    const_alpha: u8,
) -> Result<(), String> {
    let w = i32::try_from(frame.width).map_err(|_| "menu width overflow".to_owned())?;
    let h = i32::try_from(frame.height).map_err(|_| "menu height overflow".to_owned())?;
    let bmi = make_top_down_bmi(w, h);
    let mut bits: *mut c_void = std::ptr::null_mut();
    // SAFETY: All GDI handles are freed before return; copy matches DIB dimensions.
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbmp = match CreateDIBSection(Some(hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(b) => b,
            Err(e) => {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(None, hdc_screen);
                return Err(format!("CreateDIBSection(menu) failed: {e}"));
            }
        };
        if !bits.is_null() {
            let n = ((frame.width as usize) * (frame.height as usize) * 4).min(frame.pixels.len());
            std::ptr::copy_nonoverlapping(frame.pixels.as_ptr(), bits as *mut u8, n);
        }
        let prev = SelectObject(hdc_mem, hbmp.into());
        let pt_dst = POINT {
            x: origin.x,
            y: origin.y,
        };
        let sz = SIZE { cx: w, cy: h };
        let pt_src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: const_alpha,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let res = UpdateLayeredWindow(
            hwnd,
            Some(hdc_screen),
            Some(&pt_dst),
            Some(&sz),
            Some(hdc_mem),
            Some(&pt_src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );
        let _ = SelectObject(hdc_mem, prev);
        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
        res.map_err(|e| format!("UpdateLayeredWindow(menu) failed: {e}"))
    }
}
