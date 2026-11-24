use asrbox_platform_api::{
    BgraFrame, PixelRect, SettingsInputRects, TextAlign, WidgetFrame, WidgetTextCmd,
};

use crate::draw::{
    draw_slider, draw_toggle, fill_rect, fill_rounded_rect, push_text, stroke_rounded_rect,
};
use crate::layout::{ButtonTone, ControlItem, HitKind, SettingsLayout, SIDEBAR_WIDTH};
use crate::types::SettingsTab;

const BG_WINDOW: [u8; 4] = [28, 29, 34, 255];
const BG_SIDEBAR: [u8; 4] = [22, 23, 27, 255];
const BG_CARD: [u8; 4] = [35, 37, 44, 255];
const BG_AMBER_CARD: [u8; 4] = [56, 38, 26, 255];
const BORDER_CARD: [u8; 4] = [48, 51, 62, 255];
const BORDER_AMBER: [u8; 4] = [232, 122, 72, 180];
const DIVIDER: [u8; 4] = [43, 46, 56, 255];
const ACCENT: [u8; 4] = [232, 122, 72, 255];
const ACCENT_HOVER: [u8; 4] = [244, 140, 90, 255];
const TEXT_PRIMARY: [u8; 4] = [238, 240, 244, 255];
const TEXT_SECONDARY: [u8; 4] = [148, 155, 168, 255];
const TEXT_MUTED: [u8; 4] = [112, 118, 130, 255];

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
) -> (WidgetFrame, SettingsInputRects) {
    let mut bgra = BgraFrame::transparent(layout.width, layout.height);
    let mut texts = Vec::new();
    fill_rect(&mut bgra, pr(0, 0, layout.width, layout.height), BG_WINDOW);
    draw_sidebar(&mut bgra, &mut texts, layout.height);
    draw_content_cards(&mut bgra, &mut texts, layout);
    for ctrl in &layout.controls {
        let hovered = hovered_id == Some(ctrl.id);
        draw_control(
            &mut bgra,
            &mut texts,
            ctrl,
            active_tab,
            selected_dock,
            hovered,
        );
    }
    draw_content_header(&mut bgra, &mut texts, layout);
    let clip = |opt: Option<PixelRect>| {
        opt.filter(|r| r.y >= 74 && r.y + r.height as i32 <= layout.height as i32 - 8)
    };
    let inputs = SettingsInputRects {
        primary: clip(layout.input_rects.primary),
        secret: clip(layout.input_rects.secret),
        extra: clip(layout.input_rects.extra),
    };
    (WidgetFrame { bgra, texts }, inputs)
}

fn draw_sidebar(bgra: &mut BgraFrame, texts: &mut Vec<WidgetTextCmd>, height: u32) {
    fill_rect(bgra, pr(0, 0, SIDEBAR_WIDTH, height), BG_SIDEBAR);
    fill_rect(bgra, pr(SIDEBAR_WIDTH as i32 - 1, 0, 1, height), DIVIDER);
    fill_rounded_rect(bgra, pr(18, 20, 32, 32), 9, ACCENT);
    push_text(
        texts,
        pr(18, 20, 32, 32),
        "A",
        16,
        true,
        [255, 255, 255, 255],
        TextAlign::Center,
    );
    push_text(
        texts,
        pr(60, 18, 140, 20),
        "asrbox",
        15,
        true,
        TEXT_PRIMARY,
        TextAlign::Left,
    );
    push_text(
        texts,
        pr(60, 37, 140, 16),
        "Настройки",
        12,
        false,
        TEXT_SECONDARY,
        TextAlign::Left,
    );
}

fn draw_content_header(bgra: &mut BgraFrame, texts: &mut Vec<WidgetTextCmd>, l: &SettingsLayout) {
    let x = SIDEBAR_WIDTH as i32;
    let w = l.width.saturating_sub(SIDEBAR_WIDTH);
    fill_rect(bgra, pr(x, 0, w, 74), BG_WINDOW);
    fill_rect(bgra, pr(x, 73, w, 1), DIVIDER);
    push_text(
        texts,
        pr(x + 28, 16, w.saturating_sub(56), 26),
        &l.header_title,
        20,
        true,
        TEXT_PRIMARY,
        TextAlign::Left,
    );
    push_text(
        texts,
        pr(x + 28, 44, w.saturating_sub(56), 20),
        &l.header_subtitle,
        12,
        false,
        TEXT_SECONDARY,
        TextAlign::Left,
    );
}

fn draw_content_cards(bgra: &mut BgraFrame, texts: &mut Vec<WidgetTextCmd>, l: &SettingsLayout) {
    let win_h = l.height as i32;
    for card in &l.cards {
        let cr = card.rect;
        if cr.y + cr.height as i32 <= 74 || cr.y >= win_h {
            continue;
        }
        if let Some(title) = &card.title {
            if cr.y - 22 >= 74 {
                push_text(
                    texts,
                    pr(cr.x + 6, cr.y - 22, cr.width, 18),
                    title,
                    11,
                    true,
                    TEXT_MUTED,
                    TextAlign::Left,
                );
            }
        }
        fill_rounded_rect(
            bgra,
            cr,
            12,
            if card.amber { BG_AMBER_CARD } else { BG_CARD },
        );
        stroke_rounded_rect(
            bgra,
            cr,
            12,
            if card.amber {
                BORDER_AMBER
            } else {
                BORDER_CARD
            },
        );
        for row in &card.rows {
            if row.y + row.height as i32 <= 74 || row.y >= win_h {
                continue;
            }
            let ty = if row.subtitle.is_some() {
                row.y + 9
            } else {
                row.y + (row.height as i32 - 20) / 2
            };
            if !row.title.is_empty() {
                push_text(
                    texts,
                    pr(cr.x + 16, ty, cr.width.saturating_sub(210), 20),
                    &row.title,
                    13,
                    true,
                    TEXT_PRIMARY,
                    TextAlign::Left,
                );
            }
            if let Some(sub) = &row.subtitle {
                let col = if card.amber {
                    [255, 214, 140, 255]
                } else {
                    TEXT_SECONDARY
                };
                push_text(
                    texts,
                    pr(cr.x + 16, row.y + 30, cr.width.saturating_sub(180), 18),
                    sub,
                    11,
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
                    TEXT_PRIMARY,
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
                    DIVIDER,
                );
            }
        }
    }
}

fn draw_control(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    ctrl: &ControlItem,
    active_tab: SettingsTab,
    selected_dock: usize,
    hovered: bool,
) {
    let is_sidebar = matches!(ctrl.kind, HitKind::Tab(_));
    let r = ctrl.rect;
    if !is_sidebar && (r.y + r.height as i32 <= 74 || r.y >= bgra.height as i32) {
        return;
    }
    match &ctrl.kind {
        HitKind::Tab(tab) => {
            let active = *tab == active_tab;
            if active {
                fill_rounded_rect(bgra, r, 8, [232, 122, 72, 46]);
                stroke_rounded_rect(bgra, r, 8, [232, 122, 72, 130]);
            } else if hovered {
                fill_rounded_rect(bgra, r, 8, [38, 40, 48, 255]);
            }
            let col = if active {
                [248, 168, 126, 255]
            } else {
                TEXT_PRIMARY
            };
            push_text(
                texts,
                pr(r.x + 12, r.y + 6, r.width - 20, 22),
                format!("{}  {}", tab.icon(), tab.label()),
                13,
                active,
                col,
                TextAlign::Left,
            );
        }
        HitKind::SelectDock(idx) => {
            let active = *idx == selected_dock;
            let bg = if active {
                ACCENT
            } else if hovered {
                [56, 60, 72, 255]
            } else {
                [42, 45, 54, 255]
            };
            fill_rounded_rect(bgra, r, 8, bg);
            push_text(
                texts,
                r,
                format!("Док #{}", idx + 1),
                12,
                active,
                TEXT_PRIMARY,
                TextAlign::Center,
            );
        }
        HitKind::SelectDockItem { label, active, .. } => {
            let bg = if *active {
                [232, 122, 72, 54]
            } else if hovered {
                [52, 56, 68, 255]
            } else {
                [42, 45, 54, 255]
            };
            fill_rounded_rect(bgra, r, 7, bg);
            if *active {
                stroke_rounded_rect(bgra, r, 7, [232, 122, 72, 190]);
            }
            push_text(
                texts,
                pr(r.x + 8, r.y, r.width.saturating_sub(16), r.height),
                label,
                12,
                *active,
                if *active {
                    [255, 224, 206, 255]
                } else {
                    TEXT_PRIMARY
                },
                TextAlign::Center,
            );
        }
        HitKind::Toggle { on, .. } => draw_toggle(bgra, r, *on, hovered),
        HitKind::Slider {
            value, min, max, ..
        } => {
            let ratio =
                ((*value).saturating_sub(*min) as f32) / (*max).saturating_sub(*min).max(1) as f32;
            draw_slider(bgra, r, ratio, hovered);
        }
        HitKind::Segment { label, active, .. } => {
            let bg = if *active {
                ACCENT
            } else if hovered {
                [56, 60, 72, 255]
            } else {
                [42, 45, 54, 255]
            };
            fill_rounded_rect(bgra, r, 7, bg);
            push_text(
                texts,
                r,
                label,
                12,
                *active,
                TEXT_PRIMARY,
                TextAlign::Center,
            );
        }
        HitKind::Button { label, tone, .. } => {
            let bg = match (tone, hovered) {
                (ButtonTone::Primary, false) => ACCENT,
                (ButtonTone::Primary, true) => ACCENT_HOVER,
                (ButtonTone::Danger, false) => [178, 54, 54, 255],
                (ButtonTone::Danger, true) => [212, 68, 66, 255],
                (ButtonTone::Amber, false) => [196, 128, 38, 255],
                (ButtonTone::Amber, true) => [222, 148, 48, 255],
                (ButtonTone::Secondary, false) => [48, 51, 62, 255],
                (ButtonTone::Secondary, true) => [62, 66, 78, 255],
            };
            fill_rounded_rect(bgra, r, 8, bg);
            push_text(
                texts,
                r,
                label,
                12,
                true,
                [255, 255, 255, 255],
                TextAlign::Center,
            );
        }
    }
}
