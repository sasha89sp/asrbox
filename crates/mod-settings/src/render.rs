use asrbox_platform_api::{BgraFrame, PixelRect, TextAlign, WidgetFrame, WidgetTextCmd};

use crate::draw::{
    draw_slider, draw_toggle, fill_rect, fill_rounded_rect, push_text, stroke_rounded_rect,
};
use crate::layout::{ButtonTone, ControlItem, HitKind, SettingsLayout, SIDEBAR_WIDTH};
use crate::types::SettingsTab;

const BG_WINDOW: [u8; 4] = [28, 28, 30, 255];
const BG_SIDEBAR: [u8; 4] = [37, 37, 40, 255];
const BG_CARD: [u8; 4] = [44, 44, 46, 255];
const BG_AMBER_CARD: [u8; 4] = [66, 48, 20, 255];
const BORDER_CARD: [u8; 4] = [60, 60, 66, 255];
const BORDER_AMBER: [u8; 4] = [255, 159, 10, 170];
const DIVIDER: [u8; 4] = [56, 56, 62, 255];
const ACCENT_BLUE: [u8; 4] = [10, 132, 255, 255];
const ACCENT_BLUE_HOVER: [u8; 4] = [36, 150, 255, 255];
const TEXT_PRIMARY: [u8; 4] = [245, 245, 247, 255];
const TEXT_SECONDARY: [u8; 4] = [152, 152, 160, 255];
const TEXT_MUTED: [u8; 4] = [120, 120, 128, 255];

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
) -> (WidgetFrame, Option<PixelRect>) {
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
    let url_rect = layout
        .url_input_rect
        .filter(|r| r.y >= 74 && r.y + r.height as i32 <= layout.height as i32 - 8);
    (WidgetFrame { bgra, texts }, url_rect)
}

fn draw_sidebar(bgra: &mut BgraFrame, texts: &mut Vec<WidgetTextCmd>, height: u32) {
    fill_rect(bgra, pr(0, 0, SIDEBAR_WIDTH, height), BG_SIDEBAR);
    fill_rect(bgra, pr(SIDEBAR_WIDTH as i32 - 1, 0, 1, height), DIVIDER);
    fill_rounded_rect(bgra, pr(18, 20, 32, 32), 8, ACCENT_BLUE);
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
                fill_rounded_rect(bgra, r, 8, ACCENT_BLUE);
            } else if hovered {
                fill_rounded_rect(bgra, r, 8, [54, 54, 58, 255]);
            }
            let col = if active {
                [255, 255, 255, 255]
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
                ACCENT_BLUE
            } else if hovered {
                [68, 68, 74, 255]
            } else {
                [52, 52, 56, 255]
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
                ACCENT_BLUE
            } else if hovered {
                [68, 68, 74, 255]
            } else {
                [54, 54, 58, 255]
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
                (ButtonTone::Primary, false) => ACCENT_BLUE,
                (ButtonTone::Primary, true) => ACCENT_BLUE_HOVER,
                (ButtonTone::Danger, false) => [190, 44, 44, 255],
                (ButtonTone::Danger, true) => [225, 58, 52, 255],
                (ButtonTone::Amber, false) => [214, 132, 12, 255],
                (ButtonTone::Amber, true) => [238, 152, 20, 255],
                (ButtonTone::Secondary, false) => [58, 58, 64, 255],
                (ButtonTone::Secondary, true) => [74, 74, 82, 255],
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
