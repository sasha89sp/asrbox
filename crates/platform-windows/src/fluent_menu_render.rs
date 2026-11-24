use asrbox_platform_api::{
    BgraFrame, FluentMenuItem, FluentMenuSpec, FluentPalette, TextAlign, WidgetFrame, WidgetTextCmd,
};

pub(crate) use crate::fluent_menu_layout::{
    clamp_menu_origin, clamp_submenu_origin, measure_menu, MenuHitRow, MenuLayout, HOVER_RADIUS,
    MENU_PAD, MENU_QUICK_H, MENU_RADIUS, MENU_SEP_H, MENU_SHADOW,
};
use crate::widget_text::composite_widget_frame;

pub(crate) fn render_fluent_menu(
    spec: &FluentMenuSpec,
    layout: &MenuLayout,
    palette: &FluentPalette,
    hovered_row: Option<usize>,
    hovered_quick: Option<usize>,
    open_sub_row: Option<usize>,
) -> Result<BgraFrame, String> {
    let mut bgra = BgraFrame::transparent(layout.total_w, layout.total_h);
    let mut texts = Vec::new();
    draw_panel_with_shadow(&mut bgra, layout, palette);

    if !spec.quick_actions.is_empty() {
        draw_quick_row(&mut bgra, &mut texts, spec, layout, palette, hovered_quick);
        let sep_y = MENU_SHADOW + MENU_PAD + MENU_QUICK_H + MENU_SEP_H / 2;
        fill_hline(
            &mut bgra,
            layout.panel_x + 8,
            layout.panel_x + layout.panel_w - 8,
            sep_y,
            palette.menu_divider,
        );
    }

    for row in &layout.rows {
        let Some(item) = spec.items.get(row.item_index) else {
            continue;
        };
        let is_hot = hovered_row == Some(row.item_index) || open_sub_row == Some(row.item_index);
        draw_menu_row(&mut bgra, &mut texts, layout, row, item, palette, is_hot);
    }

    composite_widget_frame(&WidgetFrame { bgra, texts })
}

fn draw_quick_row(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    spec: &FluentMenuSpec,
    layout: &MenuLayout,
    palette: &FluentPalette,
    hovered_quick: Option<usize>,
) {
    for (idx, (btn, qb)) in layout
        .quick_btns
        .iter()
        .zip(spec.quick_actions.iter())
        .enumerate()
    {
        if hovered_quick == Some(idx) && btn.enabled {
            fill_rounded_rect(
                bgra,
                btn.x,
                btn.y,
                btn.w,
                btn.h,
                HOVER_RADIUS,
                palette.menu_hover,
            );
        }
        let col = if btn.enabled {
            palette.text_primary
        } else {
            palette.text_muted
        };
        texts.push(WidgetTextCmd {
            x: btn.x,
            y: btn.y,
            width: btn.w,
            height: btn.h,
            text: qb.glyph.clone(),
            font_size_px: 14,
            bold: false,
            color_rgba: col,
            align: TextAlign::Center,
        });
    }
}

fn draw_menu_row(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    layout: &MenuLayout,
    row: &MenuHitRow,
    item: &FluentMenuItem,
    palette: &FluentPalette,
    is_hot: bool,
) {
    match item {
        FluentMenuItem::Separator => {
            let cy = row.y + row.h / 2;
            fill_hline(
                bgra,
                layout.panel_x + 8,
                layout.panel_x + layout.panel_w - 8,
                cy,
                palette.menu_divider,
            );
        }
        FluentMenuItem::Action {
            label,
            icon,
            shortcut,
            checked,
            enabled,
            ..
        } => {
            if is_hot && *enabled {
                fill_rounded_rect(
                    bgra,
                    layout.panel_x + MENU_PAD,
                    row.y + 1,
                    layout.panel_w - MENU_PAD * 2,
                    row.h - 2,
                    HOVER_RADIUS,
                    palette.menu_hover,
                );
            }
            let fg = if *enabled {
                palette.text_primary
            } else {
                palette.text_muted
            };
            let glyph = if *checked {
                Some((
                    "✓",
                    if *enabled {
                        palette.accent
                    } else {
                        palette.text_muted
                    },
                ))
            } else {
                icon.as_deref().map(|g| (g, fg))
            };
            if let Some((g, gcol)) = glyph {
                texts.push(WidgetTextCmd {
                    x: layout.panel_x + 12,
                    y: row.y,
                    width: 18,
                    height: row.h,
                    text: g.to_owned(),
                    font_size_px: 13,
                    bold: *checked,
                    color_rgba: gcol,
                    align: TextAlign::Center,
                });
            }
            let right_reserve = if shortcut.is_some() { 74 } else { 14 };
            texts.push(WidgetTextCmd {
                x: layout.panel_x + 36,
                y: row.y,
                width: (layout.panel_w - 36 - right_reserve).max(40),
                height: row.h,
                text: label.clone(),
                font_size_px: 13,
                bold: false,
                color_rgba: fg,
                align: TextAlign::Left,
            });
            if let Some(sc) = shortcut {
                texts.push(WidgetTextCmd {
                    x: layout.panel_x + layout.panel_w - 78,
                    y: row.y,
                    width: 64,
                    height: row.h,
                    text: sc.clone(),
                    font_size_px: 11,
                    bold: false,
                    color_rgba: palette.text_muted,
                    align: TextAlign::Right,
                });
            }
        }
        FluentMenuItem::Submenu {
            label,
            icon,
            enabled,
            ..
        } => {
            if is_hot && *enabled {
                fill_rounded_rect(
                    bgra,
                    layout.panel_x + MENU_PAD,
                    row.y + 1,
                    layout.panel_w - MENU_PAD * 2,
                    row.h - 2,
                    HOVER_RADIUS,
                    palette.menu_hover,
                );
            }
            let fg = if *enabled {
                palette.text_primary
            } else {
                palette.text_muted
            };
            if let Some(g) = icon {
                texts.push(WidgetTextCmd {
                    x: layout.panel_x + 12,
                    y: row.y,
                    width: 18,
                    height: row.h,
                    text: g.clone(),
                    font_size_px: 13,
                    bold: false,
                    color_rgba: fg,
                    align: TextAlign::Center,
                });
            }
            texts.push(WidgetTextCmd {
                x: layout.panel_x + 36,
                y: row.y,
                width: (layout.panel_w - 64).max(40),
                height: row.h,
                text: label.clone(),
                font_size_px: 13,
                bold: false,
                color_rgba: fg,
                align: TextAlign::Left,
            });
            texts.push(WidgetTextCmd {
                x: layout.panel_x + layout.panel_w - 26,
                y: row.y,
                width: 16,
                height: row.h,
                text: "›".to_owned(),
                font_size_px: 15,
                bold: false,
                color_rgba: palette.text_muted,
                align: TextAlign::Right,
            });
        }
    }
}

fn draw_panel_with_shadow(bgra: &mut BgraFrame, layout: &MenuLayout, palette: &FluentPalette) {
    let w = bgra.width as i32;
    let h = bgra.height as i32;
    let cx = layout.panel_x as f32 + layout.panel_w as f32 * 0.5;
    let cy = layout.panel_y as f32 + layout.panel_h as f32 * 0.5;
    let hx = layout.panel_w as f32 * 0.5;
    let hy = layout.panel_h as f32 * 0.5;
    let shadow_alpha = if palette.is_dark { 84.0f32 } else { 42.0f32 };

    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let d_sh = rounded_sdf(fx - cx, fy - (cy + 2.0), hx, hy, MENU_RADIUS);
            if d_sh > 0.0 && d_sh < MENU_SHADOW as f32 {
                let t = 1.0 - (d_sh / MENU_SHADOW as f32);
                let a = (t * t * shadow_alpha).round() as u8;
                blend_px(bgra, x, y, [0, 0, 0, a]);
            }
            let d = rounded_sdf(fx - cx, fy - cy, hx, hy, MENU_RADIUS);
            if d <= 0.5 {
                let cov = (0.5 - d).clamp(0.0, 1.0);
                let mut col = if d >= -1.0 {
                    palette.menu_border
                } else {
                    palette.menu_bg
                };
                col[3] = ((f32::from(col[3]) * cov).round()).clamp(0.0, 255.0) as u8;
                blend_px(bgra, x, y, col);
            }
        }
    }
}

fn fill_rounded_rect(
    bgra: &mut BgraFrame,
    rx: i32,
    ry: i32,
    rw: i32,
    rh: i32,
    rad: f32,
    rgba: [u8; 4],
) {
    let cx = rx as f32 + rw as f32 * 0.5;
    let cy = ry as f32 + rh as f32 * 0.5;
    let hx = rw as f32 * 0.5;
    let hy = rh as f32 * 0.5;
    for y in ry.max(0)..(ry + rh).min(bgra.height as i32) {
        for x in rx.max(0)..(rx + rw).min(bgra.width as i32) {
            let d = rounded_sdf(x as f32 + 0.5 - cx, y as f32 + 0.5 - cy, hx, hy, rad);
            if d <= 0.5 {
                let cov = (0.5 - d).clamp(0.0, 1.0);
                let a = (f32::from(rgba[3]) * cov).round() as u8;
                blend_px(bgra, x, y, [rgba[0], rgba[1], rgba[2], a]);
            }
        }
    }
}

fn fill_hline(bgra: &mut BgraFrame, x0: i32, x1: i32, y: i32, rgba: [u8; 4]) {
    if y < 0 || y >= bgra.height as i32 {
        return;
    }
    for x in x0.max(0)..x1.min(bgra.width as i32) {
        blend_px(bgra, x, y, rgba);
    }
}

fn blend_px(bgra: &mut BgraFrame, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= bgra.width as i32 || y >= bgra.height as i32 || rgba[3] == 0 {
        return;
    }
    let idx = ((y as usize) * (bgra.width as usize) + (x as usize)) * 4;
    let sa = f32::from(rgba[3]) / 255.0;
    let inv = 1.0 - sa;
    let sb = f32::from(rgba[2]) * sa;
    let sg = f32::from(rgba[1]) * sa;
    let sr = f32::from(rgba[0]) * sa;
    bgra.pixels[idx] = (sb + f32::from(bgra.pixels[idx]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 1] = (sg + f32::from(bgra.pixels[idx + 1]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 2] = (sr + f32::from(bgra.pixels[idx + 2]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 3] =
        (sa * 255.0 + f32::from(bgra.pixels[idx + 3]) * inv).clamp(0.0, 255.0) as u8;
}

fn rounded_sdf(px: f32, py: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = px.abs() - (hx - r);
    let qy = py.abs() - (hy - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}
