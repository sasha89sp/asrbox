use asrbox_platform_api::{
    draw_asr_glass_logo, AppLanguage, BgraFrame, FluentPalette, PixelRect, SettingsInputRects,
    TextAlign, WidgetFrame, WidgetTextCmd,
};

use crate::draw::{fill_rect, fill_rounded_rect, push_text, stroke_rounded_rect};
use crate::i18n::tr;
use crate::layout::{SettingsLayout, HEADER_H, SIDEBAR_WIDTH, TAB_STRIP_H};
use crate::render_controls::{draw_control, is_chrome_control};
use crate::types::SettingsTab;

fn pr(x: i32, y: i32, width: u32, height: u32) -> PixelRect {
    PixelRect {
        x,
        y,
        width,
        height,
    }
}

pub fn render_settings_frame(
    layout: &SettingsLayout,
    active_tab: SettingsTab,
    selected_dock: usize,
    hovered_id: Option<usize>,
    pal: &FluentPalette,
    lang: AppLanguage,
) -> (WidgetFrame, SettingsInputRects) {
    let mut bgra = BgraFrame::transparent(layout.width, layout.height);
    let mut texts = Vec::new();
    fill_rect(
        &mut bgra,
        pr(0, 0, layout.width, layout.height),
        pal.window_bg,
    );
    draw_sidebar(&mut bgra, &mut texts, layout.height, pal, lang);
    draw_content_cards(&mut bgra, &mut texts, layout, pal);
    for ctrl in &layout.controls {
        if is_chrome_control(ctrl) {
            continue;
        }
        draw_control(
            &mut bgra,
            &mut texts,
            ctrl,
            active_tab,
            selected_dock,
            hovered_id == Some(ctrl.id),
            layout.content_top_y,
            pal,
            lang,
        );
    }
    draw_content_header(&mut bgra, &mut texts, layout, pal, lang);
    for ctrl in &layout.controls {
        if !is_chrome_control(ctrl) {
            continue;
        }
        draw_control(
            &mut bgra,
            &mut texts,
            ctrl,
            active_tab,
            selected_dock,
            hovered_id == Some(ctrl.id),
            layout.content_top_y,
            pal,
            lang,
        );
    }
    let top_y = layout.content_top_y;
    let clip = |opt: Option<PixelRect>| {
        opt.filter(|r| r.y >= top_y && r.y + r.height as i32 <= layout.height as i32 - 8)
    };
    let inputs = SettingsInputRects {
        primary: clip(layout.input_rects.primary),
        secret: clip(layout.input_rects.secret),
        extra: clip(layout.input_rects.extra),
        search: layout.input_rects.search,
        aux: clip(layout.input_rects.aux),
        zip_secret: clip(layout.input_rects.zip_secret),
        gh_repo: clip(layout.input_rects.gh_repo),
        gh_branch: clip(layout.input_rects.gh_branch),
        gh_token: clip(layout.input_rects.gh_token),
    };
    (WidgetFrame { bgra, texts }, inputs)
}

fn draw_sidebar(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    height: u32,
    pal: &FluentPalette,
    lang: AppLanguage,
) {
    fill_rect(bgra, pr(0, 0, SIDEBAR_WIDTH, height), pal.sidebar_bg);
    fill_rect(
        bgra,
        pr(SIDEBAR_WIDTH as i32 - 1, 0, 1, height),
        pal.divider,
    );
    if pal.design_theme.is_glass() {
        draw_asr_glass_logo(bgra, 16, 14, 52, pal.is_dark, false);
        push_text(
            texts,
            pr(76, 18, 124, 20),
            "asrbox",
            15,
            true,
            pal.text_primary,
            TextAlign::Left,
        );
        push_text(
            texts,
            pr(76, 38, 124, 16),
            concat!("v", env!("CARGO_PKG_VERSION")),
            12,
            false,
            pal.text_secondary,
            TextAlign::Left,
        );
        push_text(
            texts,
            pr(76, 54, 124, 14),
            tr(lang, "Настройки", "Settings"),
            11,
            false,
            pal.text_muted,
            TextAlign::Left,
        );
    } else {
        fill_rounded_rect(bgra, pr(16, 20, 34, 34), 6, pal.accent);
        push_text(
            texts,
            pr(16, 20, 34, 34),
            "A",
            16,
            true,
            pal.accent_on,
            TextAlign::Center,
        );
        push_text(
            texts,
            pr(58, 19, 140, 19),
            "asrbox",
            14,
            true,
            pal.text_primary,
            TextAlign::Left,
        );
        push_text(
            texts,
            pr(58, 39, 140, 16),
            tr(lang, "Настройки", "Settings"),
            12,
            false,
            pal.text_secondary,
            TextAlign::Left,
        );
    }
}

fn draw_content_header(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    l: &SettingsLayout,
    pal: &FluentPalette,
    lang: AppLanguage,
) {
    let x = SIDEBAR_WIDTH as i32;
    let w = l.width.saturating_sub(SIDEBAR_WIDTH);
    if l.has_tab_strip {
        fill_rect(bgra, pr(x, 0, w, TAB_STRIP_H as u32), pal.sidebar_bg);
        fill_rect(bgra, pr(x, TAB_STRIP_H - 1, w, 1), pal.divider);
    }
    let hy = if l.has_tab_strip { TAB_STRIP_H } else { 0 };
    fill_rect(bgra, pr(x, hy, w, HEADER_H as u32), pal.window_bg);
    let has_inline = l.controls.iter().any(|c| c.in_header);
    let title_w = if has_inline {
        142u32
    } else if l.search_rect.is_some() {
        w.saturating_sub(274).max(160)
    } else {
        w.saturating_sub(48)
    };
    let sub_w = if l.search_rect.is_some() && !has_inline {
        w.saturating_sub(274).max(160)
    } else {
        w.saturating_sub(48)
    };
    push_text(
        texts,
        pr(x + 24, hy + 13, title_w, 28),
        &l.header_title,
        20,
        true,
        pal.text_primary,
        TextAlign::Left,
    );
    push_text(
        texts,
        pr(x + 24, hy + 46, sub_w, 20),
        &l.header_subtitle,
        12,
        false,
        pal.text_secondary,
        TextAlign::Left,
    );
    if let Some(sr) = l.search_rect {
        let rad = if pal.design_theme.is_glass() { 6 } else { 4 };
        fill_rounded_rect(bgra, sr, rad, pal.input_bg);
        stroke_rounded_rect(bgra, sr, rad, pal.control_border);
        push_text(
            texts,
            pr(sr.x + 6, sr.y, 18, sr.height),
            "⌕",
            14,
            false,
            pal.text_secondary,
            TextAlign::Center,
        );
        let _ = lang;
    }
}

fn draw_content_cards(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    l: &SettingsLayout,
    pal: &FluentPalette,
) {
    let win_h = l.height as i32;
    let top_y = l.content_top_y;
    let rad = if pal.design_theme.is_glass() { 10 } else { 5 };
    for card in &l.cards {
        let cr = card.rect;
        if cr.y + cr.height as i32 <= top_y || cr.y >= win_h {
            continue;
        }
        if let Some(title) = &card.title {
            if cr.y - 22 >= top_y {
                push_text(
                    texts,
                    pr(cr.x + 2, cr.y - 22, cr.width, 18),
                    title,
                    12,
                    true,
                    pal.text_secondary,
                    TextAlign::Left,
                );
            }
        }
        let bg = if card.amber { pal.warn_bg } else { pal.card_bg };
        let brd = if card.amber {
            pal.warn_border
        } else if card.highlight {
            asrbox_platform_api::lerp_rgba(pal.card_border, pal.accent, 0.55)
        } else {
            pal.card_border
        };
        let split_cards =
            !pal.design_theme.is_glass() && !card.amber && card.rows.iter().any(|r| r.divider);
        if split_cards {
            draw_split_win11_cards(bgra, cr, &card.rows, bg, brd, top_y, win_h);
        } else {
            fill_rounded_rect(bgra, cr, rad, bg);
            stroke_rounded_rect(bgra, cr, rad, brd);
        }
        draw_card_rows(
            bgra,
            texts,
            cr,
            card.amber,
            split_cards,
            &card.rows,
            top_y,
            win_h,
            pal,
        );
    }
}

fn draw_split_win11_cards(
    bgra: &mut BgraFrame,
    cr: PixelRect,
    rows: &[crate::layout::CardRow],
    bg: [u8; 4],
    brd: [u8; 4],
    top_y: i32,
    win_h: i32,
) {
    let mut group_start = 0usize;
    for (idx, row) in rows.iter().enumerate() {
        let is_last = idx + 1 == rows.len();
        if row.divider || is_last {
            let first_y = rows[group_start].y;
            let last_end = row.y + row.height as i32;
            let gap_bottom = if is_last { 0 } else { 3 };
            let sub_h = (last_end - first_y - gap_bottom).max(24) as u32;
            if first_y + sub_h as i32 > top_y && first_y < win_h {
                let sub = pr(cr.x, first_y, cr.width, sub_h);
                fill_rounded_rect(bgra, sub, 5, bg);
                stroke_rounded_rect(bgra, sub, 5, brd);
            }
            group_start = idx + 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_card_rows(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    cr: PixelRect,
    amber: bool,
    split_cards: bool,
    rows: &[crate::layout::CardRow],
    top_y: i32,
    win_h: i32,
    pal: &FluentPalette,
) {
    for row in rows {
        if row.y + row.height as i32 <= top_y || row.y >= win_h {
            continue;
        }
        let ty = if row.subtitle.is_some() {
            row.y + 8
        } else {
            row.y + (row.height as i32 - 20) / 2
        };
        if !row.title.is_empty() && ty >= top_y {
            push_text(
                texts,
                pr(cr.x + 16, ty, cr.width.saturating_sub(210), 20),
                &row.title,
                14,
                false,
                pal.text_primary,
                TextAlign::Left,
            );
        }
        if let Some(sub) = &row.subtitle {
            let col = if amber {
                pal.warn_text
            } else {
                pal.text_secondary
            };
            let sy = if row.title.is_empty() {
                row.y + (row.height as i32 - 18) / 2
            } else {
                row.y + 28
            };
            let sw = if row.title.is_empty() {
                cr.width.saturating_sub(32)
            } else {
                cr.width.saturating_sub(210)
            };
            if sy >= top_y {
                push_text(
                    texts,
                    pr(cr.x + 16, sy, sw, 18),
                    sub,
                    12,
                    false,
                    col,
                    TextAlign::Left,
                );
            }
        }
        if let Some(val) = &row.value_label {
            let vy = row.y + (row.height as i32 - 26) / 2;
            if vy >= top_y {
                let badge = pr(cr.x + cr.width as i32 - 62, vy, 48, 26);
                let brad = if pal.design_theme.is_glass() { 6 } else { 4 };
                fill_rounded_rect(bgra, badge, brad, pal.control_bg);
                stroke_rounded_rect(bgra, badge, brad, pal.control_border);
                push_text(
                    texts,
                    badge,
                    val,
                    12,
                    false,
                    pal.text_primary,
                    TextAlign::Center,
                );
            }
        }
        if row.divider && !split_cards {
            fill_rect(
                bgra,
                pr(
                    cr.x + 16,
                    row.y + row.height as i32 - 1,
                    cr.width.saturating_sub(32),
                    1,
                ),
                pal.divider,
            );
        }
    }
}
