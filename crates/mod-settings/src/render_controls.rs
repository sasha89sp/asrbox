use asrbox_platform_api::{
    AppLanguage, BgraFrame, FluentPalette, PixelRect, TextAlign, WidgetTextCmd,
};

use crate::draw::{
    draw_slider, draw_swatch, draw_toggle, fill_rounded_rect, push_text, stroke_rounded_rect,
};
use crate::i18n::tr;
use crate::layout::{ButtonTone, ControlItem, HitKind};
use crate::types::SettingsTab;

fn pr(x: i32, y: i32, width: u32, height: u32) -> PixelRect {
    PixelRect {
        x,
        y,
        width,
        height,
    }
}

pub(crate) fn is_chrome_control(kind: &HitKind) -> bool {
    matches!(
        kind,
        HitKind::Tab(_) | HitKind::BrowserTabSelect { .. } | HitKind::BrowserTabClose { .. }
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_control(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    ctrl: &ControlItem,
    active_tab: SettingsTab,
    selected_dock: usize,
    hovered: bool,
    top_y: i32,
    pal: &FluentPalette,
    lang: AppLanguage,
) {
    let r = ctrl.rect;
    if !is_chrome_control(&ctrl.kind) && (r.y < top_y || r.y >= bgra.height as i32) {
        return;
    }
    match &ctrl.kind {
        HitKind::Tab(tab) => {
            draw_sidebar_tab(bgra, texts, r, *tab, *tab == active_tab, hovered, pal, lang)
        }
        HitKind::BrowserTabSelect {
            label,
            active,
            closable,
            ..
        } => {
            draw_browser_tab(bgra, texts, r, label, *active, *closable, hovered, pal);
        }
        HitKind::BrowserTabClose { .. } => {
            if hovered {
                fill_rounded_rect(bgra, r, 4, pal.control_hover);
            }
            push_text(
                texts,
                r,
                "×",
                13,
                false,
                pal.text_secondary,
                TextAlign::Center,
            );
        }
        HitKind::AccordionHeader {
            title,
            summary,
            expanded,
            matched,
            ..
        } => {
            draw_accordion_header(
                bgra, texts, r, title, summary, *expanded, *matched, hovered, pal,
            );
        }
        HitKind::ComboBox {
            label, dot_color, ..
        } => {
            draw_combo_box(bgra, texts, r, label, *dot_color, hovered, pal);
        }
        HitKind::StartResetConfirm { label } => {
            draw_fluent_btn(bgra, texts, r, label, ButtonTone::Danger, hovered, pal);
        }
        HitKind::CancelResetConfirm { label }
        | HitKind::StartEditServerDir { label, .. }
        | HitKind::CancelEditServerDir { label } => {
            draw_fluent_btn(bgra, texts, r, label, ButtonTone::Secondary, hovered, pal);
        }
        HitKind::SelectDock(idx) => {
            let lbl = format!("{} #{}", tr(lang, "Док", "Dock"), idx + 1);
            draw_segment_or_chip(bgra, texts, r, &lbl, *idx == selected_dock, hovered, pal);
        }
        HitKind::SelectDockItem { label, active, .. }
        | HitKind::SelectWidgetSubTab { label, active, .. }
        | HitKind::Chip { label, active, .. }
        | HitKind::Segment { label, active, .. } => {
            draw_segment_or_chip(bgra, texts, r, label, *active, hovered, pal);
        }
        HitKind::AccentSwatch { preset, active } => {
            let col = preset.colors_for_style(pal.design_theme, pal.is_dark).0;
            draw_swatch(bgra, r, col, *active, hovered, pal);
        }
        HitKind::Toggle { on, .. } => draw_toggle(bgra, r, *on, hovered, pal),
        HitKind::Slider {
            value, min, max, ..
        } => {
            let ratio =
                ((*value).saturating_sub(*min) as f32) / (*max).saturating_sub(*min).max(1) as f32;
            draw_slider(bgra, r, ratio, hovered, pal);
        }
        HitKind::Button { label, tone, .. } => {
            draw_fluent_btn(bgra, texts, r, label, *tone, hovered, pal);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_accordion_header(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    title: &str,
    summary: &str,
    expanded: bool,
    matched: bool,
    hovered: bool,
    pal: &FluentPalette,
) {
    let rad = if pal.design_theme.is_glass() { 10 } else { 5 };
    let bg = if hovered {
        pal.control_hover
    } else {
        pal.card_bg
    };
    let brd = if matched {
        pal.accent
    } else if expanded {
        asrbox_platform_api::lerp_rgba(pal.card_border, pal.accent, 0.35)
    } else {
        pal.card_border
    };
    fill_rounded_rect(bgra, r, rad, bg);
    stroke_rounded_rect(bgra, r, rad, brd);
    if expanded {
        fill_rounded_rect(
            bgra,
            pr(r.x + 3, r.y + 10, 3, r.height.saturating_sub(20)),
            2,
            pal.accent,
        );
    }
    let summary_w = 190u32.min(r.width / 3);
    push_text(
        texts,
        pr(
            r.x + 14,
            r.y + 11,
            r.width.saturating_sub(summary_w + 54),
            20,
        ),
        title,
        13,
        true,
        pal.text_primary,
        TextAlign::Left,
    );
    if !summary.is_empty() {
        push_text(
            texts,
            pr(
                r.x + r.width as i32 - summary_w as i32 - 34,
                r.y + 12,
                summary_w,
                18,
            ),
            summary,
            12,
            false,
            pal.text_secondary,
            TextAlign::Right,
        );
    }
    let chev = if expanded { "⌃" } else { "⌄" };
    push_text(
        texts,
        pr(r.x + r.width as i32 - 30, r.y + 10, 22, 22),
        chev,
        14,
        false,
        if expanded {
            pal.accent
        } else {
            pal.text_secondary
        },
        TextAlign::Center,
    );
}

fn draw_combo_box(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    label: &str,
    dot_color: Option<[u8; 4]>,
    hovered: bool,
    pal: &FluentPalette,
) {
    let rad = if pal.design_theme.is_glass() { 7 } else { 4 };
    let bg = if hovered {
        pal.control_hover
    } else {
        pal.control_bg
    };
    let brd = if hovered {
        asrbox_platform_api::lerp_rgba(pal.control_border, pal.accent, 0.5)
    } else {
        pal.control_border
    };
    fill_rounded_rect(bgra, r, rad, bg);
    stroke_rounded_rect(bgra, r, rad, brd);
    let mut tx = r.x + 10;
    if let Some(dot) = dot_color {
        fill_rounded_rect(bgra, pr(tx, r.y + 9, 12, 12), 6, dot);
        tx += 18;
    }
    let tw = (r.x + r.width as i32 - 26 - tx).max(32) as u32;
    push_text(
        texts,
        pr(tx, r.y + 5, tw, 20),
        label,
        12,
        false,
        pal.text_primary,
        TextAlign::Left,
    );
    push_text(
        texts,
        pr(r.x + r.width as i32 - 24, r.y + 4, 18, 22),
        "⌄",
        13,
        false,
        pal.text_secondary,
        TextAlign::Center,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_sidebar_tab(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    tab: SettingsTab,
    active: bool,
    hovered: bool,
    pal: &FluentPalette,
    lang: AppLanguage,
) {
    let rad = if pal.design_theme.is_glass() { 8 } else { 4 };
    if active {
        fill_rounded_rect(bgra, r, rad, pal.nav_active_bg);
        fill_rounded_rect(
            bgra,
            pr(r.x + 2, r.y + 9, 3, r.height.saturating_sub(18)),
            2,
            pal.accent,
        );
    } else if hovered {
        fill_rounded_rect(bgra, r, rad, pal.nav_hover_bg);
    }
    push_text(
        texts,
        pr(r.x + 14, r.y + 7, r.width.saturating_sub(20), 22),
        format!("{}  {}", tab.icon(), tab.label(lang)),
        13,
        active,
        if active {
            pal.text_primary
        } else {
            pal.text_secondary
        },
        TextAlign::Left,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_browser_tab(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    label: &str,
    active: bool,
    closable: bool,
    hovered: bool,
    pal: &FluentPalette,
) {
    let rad = if pal.design_theme.is_glass() { 7 } else { 4 };
    if active {
        fill_rounded_rect(bgra, r, rad, pal.card_bg);
        stroke_rounded_rect(bgra, r, rad, pal.card_border);
        fill_rounded_rect(
            bgra,
            pr(r.x + 10, r.y + 1, r.width.saturating_sub(20), 2),
            1,
            pal.accent,
        );
    } else if hovered {
        fill_rounded_rect(bgra, r, rad, pal.nav_hover_bg);
    }
    let text_w = r.width.saturating_sub(if closable { 34 } else { 16 });
    push_text(
        texts,
        pr(r.x + 10, r.y + 4, text_w, 20),
        label,
        12,
        false,
        if active {
            pal.text_primary
        } else {
            pal.text_secondary
        },
        TextAlign::Left,
    );
}

fn draw_segment_or_chip(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    label: &str,
    active: bool,
    hovered: bool,
    pal: &FluentPalette,
) {
    let rad = if pal.design_theme.is_glass() { 7 } else { 4 };
    if active {
        let tint = if pal.is_dark { 0.18 } else { 0.12 };
        let base_bg = asrbox_platform_api::lerp_rgba(pal.control_bg, pal.accent, tint);
        let bg = if hovered {
            asrbox_platform_api::lerp_rgba(base_bg, pal.accent, 0.10)
        } else {
            base_bg
        };
        let border = asrbox_platform_api::lerp_rgba(pal.control_border, pal.accent, 0.55);
        fill_rounded_rect(bgra, r, rad, bg);
        stroke_rounded_rect(bgra, r, rad, border);
        let bar_w = r.width.saturating_sub(18).max(16);
        let bar_x = r.x + ((r.width - bar_w) / 2) as i32;
        fill_rounded_rect(
            bgra,
            pr(bar_x, r.y + r.height as i32 - 3, bar_w, 2),
            1,
            pal.accent,
        );
        push_text(
            texts,
            r,
            label,
            12,
            false,
            pal.text_primary,
            TextAlign::Center,
        );
    } else {
        let bg = if hovered {
            pal.control_hover
        } else {
            pal.control_bg
        };
        fill_rounded_rect(bgra, r, rad, bg);
        stroke_rounded_rect(bgra, r, rad, pal.control_border);
        push_text(
            texts,
            r,
            label,
            12,
            false,
            pal.text_secondary,
            TextAlign::Center,
        );
    }
}

fn draw_fluent_btn(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    label: &str,
    tone: ButtonTone,
    hovered: bool,
    pal: &FluentPalette,
) {
    let rad = if pal.design_theme.is_glass() { 7 } else { 4 };
    let (bg, fg, bordered) = match (tone, hovered) {
        (ButtonTone::Primary, false) => (pal.accent, pal.accent_on, false),
        (ButtonTone::Primary, true) => (pal.accent_hover, pal.accent_on, false),
        (ButtonTone::Danger, false) => (pal.danger_bg, [255, 255, 255, 255], false),
        (ButtonTone::Danger, true) => (pal.danger_hover, [255, 255, 255, 255], false),
        (ButtonTone::Amber, false) => (pal.warn_border, [20, 20, 20, 255], false),
        (ButtonTone::Amber, true) => (
            asrbox_platform_api::lerp_rgba(pal.warn_border, [255, 255, 255, 255], 0.14),
            [20, 20, 20, 255],
            false,
        ),
        (ButtonTone::Secondary, false) => (pal.control_bg, pal.text_primary, true),
        (ButtonTone::Secondary, true) => (pal.control_hover, pal.text_primary, true),
    };
    fill_rounded_rect(bgra, r, rad, bg);
    if bordered {
        stroke_rounded_rect(bgra, r, rad, pal.control_border);
    }
    push_text(texts, r, label, 12, !bordered, fg, TextAlign::Center);
}
