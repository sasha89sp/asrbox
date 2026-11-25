use asrbox_platform_api::{BgraFrame, FluentPalette, PixelRect, TextAlign, WidgetTextCmd};

use crate::draw::{fill_rounded_rect, push_text, stroke_rounded_rect};

fn pr(x: i32, y: i32, width: u32, height: u32) -> PixelRect {
    PixelRect {
        x,
        y,
        width,
        height,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_dock_item_card(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    label: &str,
    path: &str,
    is_folder: bool,
    is_separator: bool,
    active: bool,
    hovered: bool,
    pal: &FluentPalette,
) {
    let rad = if pal.design_theme.is_glass() { 8 } else { 5 };
    let base_bg = if pal.is_dark {
        [22, 25, 30, 235]
    } else {
        [255, 255, 255, 255]
    };
    let bg = if active {
        asrbox_platform_api::lerp_rgba(base_bg, pal.accent, if pal.is_dark { 0.14 } else { 0.08 })
    } else if hovered {
        pal.control_hover
    } else {
        base_bg
    };
    let brd = if active {
        asrbox_platform_api::lerp_rgba(pal.control_border, pal.accent, 0.72)
    } else {
        pal.control_border
    };
    fill_rounded_rect(bgra, r, rad, bg);
    stroke_rounded_rect(bgra, r, rad, brd);

    push_text(
        texts,
        pr(r.x + 6, r.y + 11, 14, 24),
        "⋮⋮",
        11,
        false,
        pal.text_muted,
        TextAlign::Center,
    );

    let icon_r = pr(r.x + 24, r.y + 7, 32, 32);
    let (icon_bg, icon_glyph, icon_col) = if is_separator {
        (pal.control_bg, "―", pal.text_muted)
    } else if is_folder {
        (
            if pal.is_dark {
                [46, 40, 26, 255]
            } else {
                [254, 243, 199, 255]
            },
            "▣",
            if pal.is_dark {
                [251, 191, 36, 255]
            } else {
                [180, 83, 9, 255]
            },
        )
    } else {
        (
            asrbox_platform_api::lerp_rgba(pal.control_bg, pal.accent, 0.22),
            "◆",
            pal.accent,
        )
    };
    fill_rounded_rect(bgra, icon_r, 7, icon_bg);
    stroke_rounded_rect(bgra, icon_r, 7, pal.control_border);
    push_text(
        texts,
        icon_r,
        icon_glyph,
        14,
        true,
        icon_col,
        TextAlign::Center,
    );

    let badge_text = if is_separator {
        "Разделитель"
    } else if is_folder {
        "Папка"
    } else {
        "Приложение"
    };
    let badge_w = if is_separator {
        86u32
    } else if is_folder {
        56u32
    } else {
        84u32
    };
    let btns_reserve = if is_separator { 92u32 } else { 122u32 };
    let badge_x = r.x + r.width as i32 - btns_reserve as i32 - badge_w as i32 - 8;
    let badge_r = pr(badge_x, r.y + 12, badge_w, 22);
    fill_rounded_rect(bgra, badge_r, 6, pal.control_bg);
    stroke_rounded_rect(bgra, badge_r, 6, pal.divider);
    push_text(
        texts,
        badge_r,
        badge_text,
        11,
        false,
        pal.text_secondary,
        TextAlign::Center,
    );

    let text_x = r.x + 66;
    let text_w = (badge_x - text_x - 8).max(40) as u32;
    if is_separator || path.is_empty() {
        push_text(
            texts,
            pr(text_x, r.y + 13, text_w, 20),
            label,
            13,
            active,
            pal.text_primary,
            TextAlign::Left,
        );
    } else {
        push_text(
            texts,
            pr(text_x, r.y + 6, text_w, 18),
            label,
            13,
            active,
            pal.text_primary,
            TextAlign::Left,
        );
        push_text(
            texts,
            pr(text_x, r.y + 24, text_w, 16),
            path,
            11,
            false,
            pal.text_secondary,
            TextAlign::Left,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_icon_btn(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    glyph: &str,
    danger_hover: bool,
    active: bool,
    hovered: bool,
    pal: &FluentPalette,
) {
    let rad = 6;
    if hovered {
        let bg = if danger_hover {
            [232, 17, 35, 225]
        } else {
            pal.control_hover
        };
        fill_rounded_rect(bgra, r, rad, bg);
    } else if active {
        let bg = asrbox_platform_api::lerp_rgba(pal.control_bg, pal.accent, 0.22);
        fill_rounded_rect(bgra, r, rad, bg);
    }
    let fg = if hovered && danger_hover {
        [255, 255, 255, 255]
    } else if active {
        pal.accent
    } else if hovered {
        pal.text_primary
    } else {
        pal.text_secondary
    };
    push_text(texts, r, glyph, 12, false, fg, TextAlign::Center);
}
