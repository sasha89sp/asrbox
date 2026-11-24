use asrbox_platform_api::{
    BgraFrame, FluentPalette, PixelPoint, TextAlign, WidgetFrame, WidgetTextCmd,
};

use crate::draw::{
    arrow_color, draw_calendar_badge, draw_hline, draw_resize_grip, draw_triangle_down,
    draw_triangle_up, fill_macos_glass_card, fill_pill_badge, fill_rounded_rect, push_txt,
};
use crate::storage::{DateFormatMode, PeriodFilter};

pub const HEADER_HEIGHT: i32 = 50;
pub const FILTER_HEIGHT: i32 = 36;
pub const ROW_HEIGHT: i32 = 52;
pub const FOOTER_HEIGHT: i32 = 38;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetHitTarget {
    HeaderDrag,
    TitleLink,
    AddEventBtn,
    SettingsBtn,
    RefreshBtn,
    FilterTab(PeriodFilter),
    FormatCycleBtn,
    EventRow(usize),
    ScrollUpBtn,
    ScrollDownBtn,
    None,
}

#[derive(Debug, Clone)]
pub struct DisplayRow {
    pub title: String,
    pub subtitle: String,
    pub bar_rgb: [u8; 3],
    pub tooltip: String,
    pub is_past: bool,
}

pub struct RenderView<'a> {
    pub width: u32,
    pub visible_rows: u8,
    pub opacity_percent: u8,
    pub title: &'a str,
    pub period: PeriodFilter,
    pub date_format: DateFormatMode,
    pub rows: &'a [DisplayRow],
    pub hovered: WidgetHitTarget,
    pub footer_text: &'a str,
    pub footer_error: bool,
    pub can_scroll_up: bool,
    pub can_scroll_down: bool,
    pub is_loading: bool,
    pub palette: FluentPalette,
}

pub fn widget_height(visible_rows: u8) -> u32 {
    let rows = i32::from(visible_rows.clamp(3, 16));
    (HEADER_HEIGHT + FILTER_HEIGHT + rows * ROW_HEIGHT + FOOTER_HEIGHT) as u32
}

pub fn rows_for_height(height: u32) -> u8 {
    let chrome = HEADER_HEIGHT + FILTER_HEIGHT + FOOTER_HEIGHT;
    let body = (height as i32 - chrome).max(ROW_HEIGHT * 3);
    let rows = (body + ROW_HEIGHT / 2) / ROW_HEIGHT;
    rows.clamp(3, 16) as u8
}

pub fn hit_test_widget(
    pt: PixelPoint,
    width: u32,
    visible_rows: u8,
    row_count: usize,
) -> WidgetHitTarget {
    let w = width as i32;
    let h = widget_height(visible_rows) as i32;
    if pt.x < 0 || pt.y < 0 || pt.x >= w || pt.y >= h {
        return WidgetHitTarget::None;
    }
    if pt.y < HEADER_HEIGHT {
        return hit_test_header(pt, w);
    }
    let filter_bottom = HEADER_HEIGHT + FILTER_HEIGHT;
    if pt.y < filter_bottom {
        return hit_test_filter_bar(pt, w);
    }
    let footer_top = h - FOOTER_HEIGHT;
    if pt.y >= footer_top {
        if pt.x >= w - 58 && pt.x < w - 32 {
            return WidgetHitTarget::ScrollUpBtn;
        }
        if pt.x >= w - 32 && pt.x < w - 6 {
            return WidgetHitTarget::ScrollDownBtn;
        }
        return WidgetHitTarget::None;
    }
    let idx = ((pt.y - filter_bottom) / ROW_HEIGHT) as usize;
    if idx < row_count {
        WidgetHitTarget::EventRow(idx)
    } else {
        WidgetHitTarget::None
    }
}

fn hit_test_header(pt: PixelPoint, w: i32) -> WidgetHitTarget {
    if (w - 34..w - 8).contains(&pt.x) {
        WidgetHitTarget::RefreshBtn
    } else if (w - 64..w - 38).contains(&pt.x) {
        WidgetHitTarget::SettingsBtn
    } else if (w - 94..w - 68).contains(&pt.x) {
        WidgetHitTarget::AddEventBtn
    } else if (12..190).contains(&pt.x) {
        WidgetHitTarget::TitleLink
    } else {
        WidgetHitTarget::HeaderDrag
    }
}

fn hit_test_filter_bar(pt: PixelPoint, w: i32) -> WidgetHitTarget {
    if pt.x >= w - 98 {
        return WidgetHitTarget::FormatCycleBtn;
    }
    match pt.x {
        10..68 => WidgetHitTarget::FilterTab(PeriodFilter::Today),
        68..98 => WidgetHitTarget::FilterTab(PeriodFilter::Days7),
        98..134 => WidgetHitTarget::FilterTab(PeriodFilter::Days30),
        134..170 => WidgetHitTarget::FilterTab(PeriodFilter::All),
        _ => WidgetHitTarget::None,
    }
}

pub fn render_calendar_widget(view: &RenderView<'_>) -> WidgetFrame {
    let w = view.width;
    let h = widget_height(view.visible_rows);
    let mut bgra = BgraFrame::transparent(w, h);
    let mut texts = Vec::with_capacity(32);
    let pal = &view.palette;

    let pct = u16::from(view.opacity_percent.clamp(20, 100));
    let div_a = ((pct * 220) / 100).clamp(60, 235) as u8;
    let div = [pal.divider[0], pal.divider[1], pal.divider[2], div_a];

    fill_macos_glass_card(&mut bgra, view.opacity_percent, pal);
    draw_header(&mut bgra, &mut texts, view, w as i32);
    draw_hline(&mut bgra, HEADER_HEIGHT, div);
    draw_filter_bar(&mut bgra, &mut texts, view, w as i32);
    draw_hline(&mut bgra, HEADER_HEIGHT + FILTER_HEIGHT, div);
    draw_event_rows(&mut bgra, &mut texts, view, w as i32);

    let footer_y = h as i32 - FOOTER_HEIGHT;
    draw_hline(&mut bgra, footer_y, div);
    draw_footer(&mut bgra, &mut texts, view, w as i32, footer_y);
    draw_resize_grip(&mut bgra, pal);

    WidgetFrame { bgra, texts }
}

fn draw_header(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    view: &RenderView<'_>,
    w: i32,
) {
    let pal = &view.palette;
    draw_calendar_badge(frame, 14, 14, pal);
    push_txt(
        texts,
        (44, 12, w - 144, 26),
        view.title,
        (15, true),
        pal.text_primary,
        TextAlign::Left,
    );
    let refresh_glyph = if view.is_loading { "…" } else { "↻" };
    let buttons = [
        (
            WidgetHitTarget::AddEventBtn,
            w - 94,
            "+",
            pal.accent,
            (19, true),
        ),
        (
            WidgetHitTarget::SettingsBtn,
            w - 64,
            "⚙",
            pal.text_secondary,
            (16, false),
        ),
        (
            WidgetHitTarget::RefreshBtn,
            w - 34,
            refresh_glyph,
            pal.text_secondary,
            (16, false),
        ),
    ];
    let hov_bg = [pal.menu_hover[0], pal.menu_hover[1], pal.menu_hover[2], 200];
    for (target, bx, glyph, color, font) in buttons {
        if view.hovered == target {
            fill_rounded_rect(frame, bx, 12, 26, 26, 6.0, hov_bg);
        }
        push_txt(
            texts,
            (bx, 11, 26, 28),
            glyph,
            font,
            color,
            TextAlign::Center,
        );
    }
}

fn draw_filter_bar(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    view: &RenderView<'_>,
    w: i32,
) {
    let pal = &view.palette;
    let y = HEADER_HEIGHT + 4;
    let hov_bg = [pal.menu_hover[0], pal.menu_hover[1], pal.menu_hover[2], 190];
    let tabs = [
        (PeriodFilter::Today, 12, 54, "Сегодня"),
        (PeriodFilter::Days7, 70, 26, "7д"),
        (PeriodFilter::Days30, 100, 32, "30д"),
        (PeriodFilter::All, 136, 30, "Все"),
    ];
    for (tab, tx, tw, label) in tabs {
        let active = view.period == tab;
        let hovered = view.hovered == WidgetHitTarget::FilterTab(tab);
        if active {
            fill_pill_badge(frame, tx - 5, y + 2, tw + 10, 24, pal);
        } else if hovered {
            fill_rounded_rect(frame, tx - 4, y + 2, tw + 8, 24, 6.0, hov_bg);
        }
        let color = if active {
            pal.accent
        } else if hovered {
            pal.text_primary
        } else {
            pal.text_secondary
        };
        push_txt(
            texts,
            (tx, y, tw, 28),
            label,
            (12, active),
            color,
            TextAlign::Center,
        );
    }

    let fmt_hovered = view.hovered == WidgetHitTarget::FormatCycleBtn;
    if fmt_hovered {
        fill_rounded_rect(frame, w - 98, y + 2, 88, 24, 5.0, hov_bg);
    }
    let fmt_col = if fmt_hovered {
        pal.text_primary
    } else {
        pal.text_secondary
    };
    push_txt(
        texts,
        (w - 96, y, 84, 28),
        view.date_format.label(),
        (12, false),
        fmt_col,
        TextAlign::Right,
    );
}

fn draw_event_rows(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    view: &RenderView<'_>,
    w: i32,
) {
    let pal = &view.palette;
    let base_y = HEADER_HEIGHT + FILTER_HEIGHT;
    let max_rows = usize::from(view.visible_rows.clamp(3, 16));
    let hov_bg = [pal.menu_hover[0], pal.menu_hover[1], pal.menu_hover[2], 180];
    for (idx, row) in view.rows.iter().take(max_rows).enumerate() {
        let ry = base_y + (idx as i32) * ROW_HEIGHT;
        if view.hovered == WidgetHitTarget::EventRow(idx) {
            fill_rounded_rect(frame, 6, ry + 3, w - 12, ROW_HEIGHT - 6, 8.0, hov_bg);
        }
        let bar_a = if row.is_past { 118 } else { 255 };
        let bar = [row.bar_rgb[0], row.bar_rgb[1], row.bar_rgb[2], bar_a];
        fill_rounded_rect(frame, 14, ry + 10, 4, 32, 2.0, bar);
        let (title_col, sub_col) = if row.is_past {
            (pal.text_secondary, pal.text_muted)
        } else {
            (pal.text_primary, pal.text_secondary)
        };
        let l = TextAlign::Left;
        push_txt(
            texts,
            (28, ry + 7, w - 42, 22),
            &row.title,
            (14, !row.is_past),
            title_col,
            l,
        );
        push_txt(
            texts,
            (28, ry + 28, w - 42, 18),
            &row.subtitle,
            (12, false),
            sub_col,
            l,
        );
    }
}

fn draw_footer(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    view: &RenderView<'_>,
    w: i32,
    footer_y: i32,
) {
    let pal = &view.palette;
    let color = if view.footer_error {
        pal.warn_text
    } else {
        pal.text_muted
    };
    push_txt(
        texts,
        (14, footer_y + 8, w - 76, 22),
        view.footer_text,
        (11, false),
        color,
        TextAlign::Left,
    );
    let up = arrow_color(
        view.can_scroll_up,
        view.hovered == WidgetHitTarget::ScrollUpBtn,
        pal,
    );
    let dn = arrow_color(
        view.can_scroll_down,
        view.hovered == WidgetHitTarget::ScrollDownBtn,
        pal,
    );
    draw_triangle_up(frame, w - 45, footer_y + 15, up);
    draw_triangle_down(frame, w - 20, footer_y + 15, dn);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_frame_and_hit_tests_buttons() {
        let rows = vec![DisplayRow {
            title: "ОЗОН: карта".to_owned(),
            subtitle: "Сегодня · весь день".to_owned(),
            bar_rgb: [0, 103, 192],
            tooltip: "ОЗОН: карта".to_owned(),
            is_past: false,
        }];
        let view = RenderView {
            width: 336,
            visible_rows: 8,
            opacity_percent: 88,
            title: "Google Календарь",
            period: PeriodFilter::Days30,
            date_format: DateFormatMode::Normal,
            rows: &rows,
            hovered: WidgetHitTarget::EventRow(0),
            footer_text: "Обн. 01:01 · 1–1 из 1",
            footer_error: false,
            can_scroll_up: false,
            can_scroll_down: false,
            is_loading: false,
            palette: FluentPalette::default(),
        };
        let frame = render_calendar_widget(&view);
        assert_eq!(frame.bgra.width, 336);
        assert_eq!(frame.bgra.height, widget_height(8));
        assert!(!frame.texts.is_empty());
        let add_hit = hit_test_widget(PixelPoint { x: 255, y: 20 }, 336, 8, 1);
        assert_eq!(add_hit, WidgetHitTarget::AddEventBtn);
        let row_hit = hit_test_widget(PixelPoint { x: 40, y: 100 }, 336, 8, 1);
        assert_eq!(row_hit, WidgetHitTarget::EventRow(0));
    }
}
