use asrbox_platform_api::{PixelPoint, WidgetSizeBounds};
use windows::core::PCWSTR;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    IDC_ARROW, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE,
};

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
