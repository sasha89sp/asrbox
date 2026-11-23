use asrbox_platform_api::{BgraFrame, PixelPoint, TextAlign, WidgetFrame, WidgetTextCmd};

use crate::draw::{
    draw_calendar_badge, draw_hline, draw_triangle_down, draw_triangle_up, fill_rounded_card,
    fill_rounded_rect,
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
}

pub struct RenderView<'a> {
    pub width: u32,
    pub visible_rows: u8,
    pub acrylic: bool,
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
}

pub fn widget_height(visible_rows: u8) -> u32 {
    let rows = i32::from(visible_rows.clamp(3, 12));
    (HEADER_HEIGHT + FILTER_HEIGHT + rows * ROW_HEIGHT + FOOTER_HEIGHT) as u32
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
    if pt.x >= w - 34 && pt.x < w - 8 {
        return WidgetHitTarget::RefreshBtn;
    }
    if pt.x >= w - 64 && pt.x < w - 38 {
        return WidgetHitTarget::SettingsBtn;
    }
    if pt.x >= w - 94 && pt.x < w - 68 {
        return WidgetHitTarget::AddEventBtn;
    }
    if pt.x >= 12 && pt.x < 190 {
        return WidgetHitTarget::TitleLink;
    }
    WidgetHitTarget::HeaderDrag
}

fn hit_test_filter_bar(pt: PixelPoint, w: i32) -> WidgetHitTarget {
    if pt.x >= w - 98 {
        return WidgetHitTarget::FormatCycleBtn;
    }
    if pt.x >= 10 && pt.x < 68 {
        WidgetHitTarget::FilterTab(PeriodFilter::Today)
    } else if pt.x >= 68 && pt.x < 98 {
        WidgetHitTarget::FilterTab(PeriodFilter::Days7)
    } else if pt.x >= 98 && pt.x < 134 {
        WidgetHitTarget::FilterTab(PeriodFilter::Days30)
    } else if pt.x >= 134 && pt.x < 170 {
        WidgetHitTarget::FilterTab(PeriodFilter::All)
    } else {
        WidgetHitTarget::None
    }
}

pub fn render_calendar_widget(view: &RenderView<'_>) -> WidgetFrame {
    let w = view.width;
    let h = widget_height(view.visible_rows);
    let mut bgra = BgraFrame::transparent(w, h);
    let mut texts = Vec::with_capacity(32);

    let bg_alpha = if view.acrylic { 226 } else { 244 };
    fill_rounded_card(&mut bgra, [19, 20, 24, bg_alpha], [255, 255, 255, 26]);
    draw_header(&mut bgra, &mut texts, view, w as i32);
    let div = [255, 255, 255, 18];
    draw_hline(&mut bgra, HEADER_HEIGHT, div);
    draw_filter_bar(&mut bgra, &mut texts, view, w as i32);
    draw_hline(&mut bgra, HEADER_HEIGHT + FILTER_HEIGHT, div);
    draw_event_rows(&mut bgra, &mut texts, view, w as i32);

    let footer_y = h as i32 - FOOTER_HEIGHT;
    draw_hline(&mut bgra, footer_y, div);
    draw_footer(&mut bgra, &mut texts, view, w as i32, footer_y);

    WidgetFrame { bgra, texts }
}

fn draw_header(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    view: &RenderView<'_>,
    w: i32,
) {
    draw_calendar_badge(frame, 14, 14);
    texts.push(WidgetTextCmd {
        x: 44,
        y: 12,
        width: w - 144,
        height: 26,
        text: view.title.to_owned(),
        font_size_px: 15,
        bold: true,
        color_rgba: [243, 244, 246, 255],
        align: TextAlign::Left,
    });

    let refresh_glyph = if view.is_loading { "…" } else { "↻" };
    let buttons = [
        (
            WidgetHitTarget::AddEventBtn,
            w - 94,
            "+",
            [52, 211, 153, 255],
            19,
            true,
        ),
        (
            WidgetHitTarget::SettingsBtn,
            w - 64,
            "⚙",
            [156, 163, 175, 255],
            16,
            false,
        ),
        (
            WidgetHitTarget::RefreshBtn,
            w - 34,
            refresh_glyph,
            [156, 163, 175, 255],
            16,
            false,
        ),
    ];
    for (target, bx, glyph, color, size, bold) in buttons {
        if view.hovered == target {
            fill_rounded_rect(frame, bx, 12, 26, 26, 6.0, [255, 255, 255, 24]);
        }
        texts.push(WidgetTextCmd {
            x: bx,
            y: 11,
            width: 26,
            height: 28,
            text: glyph.to_owned(),
            font_size_px: size,
            bold,
            color_rgba: color,
            align: TextAlign::Center,
        });
    }
}

fn draw_filter_bar(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    view: &RenderView<'_>,
    w: i32,
) {
    let y = HEADER_HEIGHT + 4;
    let tabs = [
        (PeriodFilter::Today, 12, 54, "Сегодня"),
        (PeriodFilter::Days7, 70, 26, "7д"),
        (PeriodFilter::Days30, 100, 32, "30д"),
        (PeriodFilter::All, 136, 30, "Все"),
    ];
    for (tab, tx, tw, label) in tabs {
        let active = view.period == tab;
        let hovered = view.hovered == WidgetHitTarget::FilterTab(tab);
        if hovered && !active {
            fill_rounded_rect(frame, tx - 3, y + 2, tw + 6, 24, 5.0, [255, 255, 255, 16]);
        }
        let color = if active {
            [78, 161, 255, 255]
        } else if hovered {
            [229, 231, 235, 255]
        } else {
            [148, 155, 168, 255]
        };
        texts.push(WidgetTextCmd {
            x: tx,
            y,
            width: tw,
            height: 28,
            text: label.to_owned(),
            font_size_px: 12,
            bold: active,
            color_rgba: color,
            align: TextAlign::Left,
        });
    }

    let fmt_hovered = view.hovered == WidgetHitTarget::FormatCycleBtn;
    if fmt_hovered {
        fill_rounded_rect(frame, w - 98, y + 2, 88, 24, 5.0, [255, 255, 255, 16]);
    }
    texts.push(WidgetTextCmd {
        x: w - 96,
        y,
        width: 84,
        height: 28,
        text: view.date_format.label().to_owned(),
        font_size_px: 12,
        bold: false,
        color_rgba: if fmt_hovered {
            [229, 231, 235, 255]
        } else {
            [148, 155, 168, 255]
        },
        align: TextAlign::Right,
    });
}

fn draw_event_rows(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    view: &RenderView<'_>,
    w: i32,
) {
    let base_y = HEADER_HEIGHT + FILTER_HEIGHT;
    let max_rows = usize::from(view.visible_rows.clamp(3, 12));
    for (idx, row) in view.rows.iter().take(max_rows).enumerate() {
        let ry = base_y + (idx as i32) * ROW_HEIGHT;
        if view.hovered == WidgetHitTarget::EventRow(idx) {
            let tint = [255, 255, 255, 15];
            fill_rounded_rect(frame, 6, ry + 3, w - 12, ROW_HEIGHT - 6, 8.0, tint);
        }
        let bar = [row.bar_rgb[0], row.bar_rgb[1], row.bar_rgb[2], 255];
        fill_rounded_rect(frame, 14, ry + 10, 4, 32, 2.0, bar);

        texts.push(WidgetTextCmd {
            x: 28,
            y: ry + 7,
            width: w - 42,
            height: 22,
            text: row.title.clone(),
            font_size_px: 14,
            bold: true,
            color_rgba: [243, 244, 246, 255],
            align: TextAlign::Left,
        });
        texts.push(WidgetTextCmd {
            x: 28,
            y: ry + 28,
            width: w - 42,
            height: 18,
            text: row.subtitle.clone(),
            font_size_px: 12,
            bold: false,
            color_rgba: [152, 160, 173, 255],
            align: TextAlign::Left,
        });
    }
}

fn draw_footer(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    view: &RenderView<'_>,
    w: i32,
    footer_y: i32,
) {
    let color = if view.footer_error {
        [245, 158, 11, 255]
    } else {
        [136, 144, 158, 255]
    };
    texts.push(WidgetTextCmd {
        x: 14,
        y: footer_y + 8,
        width: w - 76,
        height: 22,
        text: view.footer_text.to_owned(),
        font_size_px: 11,
        bold: false,
        color_rgba: color,
        align: TextAlign::Left,
    });

    let up_color = arrow_color(
        view.can_scroll_up,
        view.hovered == WidgetHitTarget::ScrollUpBtn,
    );
    let dn_color = arrow_color(
        view.can_scroll_down,
        view.hovered == WidgetHitTarget::ScrollDownBtn,
    );
    draw_triangle_up(frame, w - 45, footer_y + 15, up_color);
    draw_triangle_down(frame, w - 20, footer_y + 15, dn_color);
}

fn arrow_color(enabled: bool, hovered: bool) -> [u8; 4] {
    match (enabled, hovered) {
        (false, _) => [86, 92, 104, 140],
        (true, true) => [243, 244, 246, 255],
        (true, false) => [156, 163, 175, 255],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_frame_and_hit_tests_buttons() {
        let rows = vec![DisplayRow {
            title: "ОЗОН: карта".to_owned(),
            subtitle: "Сегодня · весь день".to_owned(),
            bar_rgb: [52, 211, 153],
            tooltip: "ОЗОН: карта".to_owned(),
        }];
        let view = RenderView {
            width: 336,
            visible_rows: 8,
            acrylic: true,
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
