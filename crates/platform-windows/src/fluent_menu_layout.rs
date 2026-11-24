use asrbox_platform_api::{FluentMenuItem, FluentMenuSpec, PixelPoint, PixelRect};

pub(crate) const MENU_SHADOW: i32 = 8;
pub(crate) const MENU_PAD: i32 = 5;
pub(crate) const MENU_ROW_H: i32 = 32;
pub(crate) const MENU_SEP_H: i32 = 9;
pub(crate) const MENU_QUICK_H: i32 = 38;
pub(crate) const MENU_RADIUS: f32 = 8.0;
pub(crate) const HOVER_RADIUS: f32 = 5.0;

#[derive(Debug, Clone)]
pub(crate) struct MenuHitRow {
    pub y: i32,
    pub h: i32,
    pub item_index: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct QuickHitBtn {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub id: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MenuLayout {
    pub total_w: u32,
    pub total_h: u32,
    pub panel_x: i32,
    pub panel_y: i32,
    pub panel_w: i32,
    pub panel_h: i32,
    pub quick_btns: Vec<QuickHitBtn>,
    pub rows: Vec<MenuHitRow>,
}

pub(crate) fn measure_menu(spec: &FluentMenuSpec) -> MenuLayout {
    let mut max_text_w = spec.min_width.max(180);
    for item in &spec.items {
        let (lbl, sc) = match item {
            FluentMenuItem::Action {
                label, shortcut, ..
            } => (label.as_str(), shortcut.as_deref().unwrap_or("")),
            FluentMenuItem::Submenu { label, .. } => (label.as_str(), "›"),
            FluentMenuItem::Separator => continue,
        };
        let lw = (lbl.chars().count() as i32) * 7 + 58;
        let sw = if sc.is_empty() {
            0
        } else {
            (sc.chars().count() as i32) * 6 + 22
        };
        max_text_w = max_text_w.max((lw + sw).min(340));
    }
    if !spec.quick_actions.is_empty() {
        let qw = (spec.quick_actions.len() as i32) * 44 + MENU_PAD * 2;
        max_text_w = max_text_w.max(qw);
    }
    let panel_w = max_text_w.clamp(180, 340);
    let mut y = MENU_SHADOW + MENU_PAD;
    let mut quick_btns = Vec::new();
    if !spec.quick_actions.is_empty() {
        let n = spec.quick_actions.len() as i32;
        let inner_w = panel_w - MENU_PAD * 2;
        let gap = 4;
        let btn_w = ((inner_w - gap * (n - 1)) / n).max(32);
        for (i, qb) in spec.quick_actions.iter().enumerate() {
            let bx = MENU_SHADOW + MENU_PAD + (i as i32) * (btn_w + gap);
            quick_btns.push(QuickHitBtn {
                x: bx,
                y,
                w: btn_w,
                h: MENU_QUICK_H - 4,
                id: qb.id,
                enabled: qb.enabled,
            });
        }
        y += MENU_QUICK_H + MENU_SEP_H;
    }

    let mut rows = Vec::with_capacity(spec.items.len());
    for (idx, item) in spec.items.iter().enumerate() {
        let h = match item {
            FluentMenuItem::Separator => MENU_SEP_H,
            FluentMenuItem::Action { .. } | FluentMenuItem::Submenu { .. } => MENU_ROW_H,
        };
        rows.push(MenuHitRow {
            y,
            h,
            item_index: idx,
        });
        y += h;
    }
    let panel_h = (y + MENU_PAD) - MENU_SHADOW;
    let total_w = (panel_w + MENU_SHADOW * 2) as u32;
    let total_h = (panel_h + MENU_SHADOW * 2) as u32;
    MenuLayout {
        total_w,
        total_h,
        panel_x: MENU_SHADOW,
        panel_y: MENU_SHADOW,
        panel_w,
        panel_h,
        quick_btns,
        rows,
    }
}

pub(crate) fn clamp_menu_origin(
    pt: PixelPoint,
    layout: &MenuLayout,
    work_area: PixelRect,
) -> PixelPoint {
    let min_x = work_area.x + 4 - MENU_SHADOW;
    let min_y = work_area.y + 4 - MENU_SHADOW;
    let max_x = work_area.x + work_area.width as i32 - layout.panel_w - 4 - MENU_SHADOW;
    let max_y = work_area.y + work_area.height as i32 - layout.panel_h - 4 - MENU_SHADOW;
    let mut x = pt.x - MENU_SHADOW;
    let mut y = pt.y - MENU_SHADOW;
    if x > max_x {
        x = (pt.x - layout.panel_w - MENU_SHADOW).clamp(min_x, max_x.max(min_x));
    }
    if y > max_y {
        y = (pt.y - layout.panel_h - MENU_SHADOW).clamp(min_y, max_y.max(min_y));
    }
    PixelPoint {
        x: x.clamp(min_x, max_x.max(min_x)),
        y: y.clamp(min_y, max_y.max(min_y)),
    }
}

pub(crate) fn clamp_submenu_origin(
    parent_origin: PixelPoint,
    parent_layout: &MenuLayout,
    row_y: i32,
    sub_layout: &MenuLayout,
    work_area: PixelRect,
) -> PixelPoint {
    let right_x = parent_origin.x + parent_layout.panel_x + parent_layout.panel_w - 2 - MENU_SHADOW;
    let max_x = work_area.x + work_area.width as i32 - sub_layout.panel_w - 4 - MENU_SHADOW;
    let min_x = work_area.x + 4 - MENU_SHADOW;
    let x = if right_x <= max_x {
        right_x
    } else {
        (parent_origin.x + parent_layout.panel_x - sub_layout.panel_w + 2 - MENU_SHADOW)
            .clamp(min_x, max_x.max(min_x))
    };
    let min_y = work_area.y + 4 - MENU_SHADOW;
    let max_y = work_area.y + work_area.height as i32 - sub_layout.panel_h - 4 - MENU_SHADOW;
    let desired_y = parent_origin.y + row_y - MENU_PAD - MENU_SHADOW;
    PixelPoint {
        x,
        y: desired_y.clamp(min_y, max_y.max(min_y)),
    }
}
