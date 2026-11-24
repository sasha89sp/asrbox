use asrbox_platform_api::{
    AppLanguage, BgraFrame, FluentPalette, PixelRect, SettingsInputRects, TextAlign, WidgetFrame,
    WidgetTextCmd,
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
        if is_chrome_control(&ctrl.kind) {
            continue;
        }
        let hov = hovered_id == Some(ctrl.id);
        draw_control(
            &mut bgra,
            &mut texts,
            ctrl,
            active_tab,
            selected_dock,
            hov,
            layout.content_top_y,
            pal,
            lang,
        );
    }
    draw_content_header(&mut bgra, &mut texts, layout, pal, lang);
    for ctrl in &layout.controls {
        if !is_chrome_control(&ctrl.kind) {
            continue;
        }
        let hov = hovered_id == Some(ctrl.id);
        draw_control(
            &mut bgra,
            &mut texts,
            ctrl,
            active_tab,
            selected_dock,
            hov,
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
    fill_rounded_rect(bgra, pr(16, 18, 32, 32), 8, pal.accent);
    push_text(
        texts,
        pr(16, 18, 32, 32),
        "A",
        16,
        true,
        pal.accent_on,
        TextAlign::Center,
    );
    push_text(
        texts,
        pr(58, 16, 138, 20),
        "asrbox",
        15,
        true,
        pal.text_primary,
        TextAlign::Left,
    );
    push_text(
        texts,
        pr(58, 36, 138, 16),
        tr(lang, "Настройки", "Settings"),
        12,
        false,
        pal.text_secondary,
        TextAlign::Left,
    );
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
    fill_rect(bgra, pr(x, hy + HEADER_H - 1, w, 1), pal.divider);
    let title_w = if l.search_rect.is_some() {
        w.saturating_sub(334).max(160)
    } else {
        w.saturating_sub(48)
    };
    push_text(
        texts,
        pr(x + 24, hy + 14, title_w, 26),
        &l.header_title,
        20,
        true,
        pal.text_primary,
        TextAlign::Left,
    );
    push_text(
        texts,
        pr(x + 24, hy + 44, title_w, 20),
        &l.header_subtitle,
        12,
        false,
        pal.text_secondary,
        TextAlign::Left,
    );
    if let Some(sr) = l.search_rect {
        fill_rounded_rect(bgra, sr, 6, pal.input_bg);
        stroke_rounded_rect(bgra, sr, 6, pal.control_border);
        push_text(
            texts,
            pr(sr.x + 6, sr.y, 18, sr.height),
            "⌕",
            14,
            false,
            pal.text_muted,
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
                    11,
                    true,
                    pal.text_muted,
                    TextAlign::Left,
                );
            }
        }
        let bg = if card.amber { pal.warn_bg } else { pal.card_bg };
        let brd = if card.amber {
            pal.warn_border
        } else {
            pal.card_border
        };
        fill_rounded_rect(bgra, cr, 8, bg);
        stroke_rounded_rect(bgra, cr, 8, brd);
        draw_card_rows(bgra, texts, cr, card.amber, &card.rows, top_y, win_h, pal);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_card_rows(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    cr: PixelRect,
    amber: bool,
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
        if !row.title.is_empty() {
            push_text(
                texts,
                pr(cr.x + 16, ty, cr.width.saturating_sub(180), 20),
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
                cr.width.saturating_sub(180)
            };
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
        if let Some(val) = &row.value_label {
            push_text(
                texts,
                pr(
                    cr.x + cr.width as i32 - 64,
                    row.y + (row.height as i32 - 20) / 2,
                    48,
                    20,
                ),
                val,
                12,
                true,
                pal.text_primary,
                TextAlign::Right,
            );
        }
        if row.divider {
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
