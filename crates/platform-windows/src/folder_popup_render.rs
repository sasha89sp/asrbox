use std::path::PathBuf;

use asrbox_platform_api::{
    BgraFrame, FluentPalette, PixelPoint, PixelRect, RgbaImage, TextAlign, WidgetFrame,
    WidgetTextCmd,
};

use crate::widget_text::composite_widget_frame;

pub(crate) const POPUP_SHADOW: i32 = 8;
const POPUP_RADIUS: f32 = 12.0;
const PANEL_PAD: i32 = 10;
const HEADER_H: i32 = 34;
const TILE_W: i32 = 76;
const TILE_H: i32 = 72;
const TILE_GAP: i32 = 6;
const MAX_COLS: usize = 4;

pub(crate) struct FolderEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub icon: Option<RgbaImage>,
}

#[derive(Clone, Copy)]
pub(crate) struct TileSlot {
    pub entry_index: usize,
    pub rect: PixelRect,
}

#[derive(Clone)]
pub(crate) struct FolderPopupLayout {
    pub total_w: u32,
    pub total_h: u32,
    pub panel_x: i32,
    pub panel_y: i32,
    pub panel_w: i32,
    pub panel_h: i32,
    pub explorer_btn: PixelRect,
    pub tiles: Vec<TileSlot>,
}

pub(crate) fn compute_folder_popup_layout(entries: &[FolderEntry]) -> FolderPopupLayout {
    let count = entries.len();
    let cols = if count == 0 {
        3
    } else {
        count.clamp(2, MAX_COLS)
    };
    let rows = if count == 0 { 1 } else { count.div_ceil(cols) };
    let grid_w = (cols as i32) * TILE_W + (cols.saturating_sub(1) as i32) * TILE_GAP;
    let grid_h = if count == 0 {
        56
    } else {
        (rows as i32) * TILE_H + (rows.saturating_sub(1) as i32) * TILE_GAP
    };
    let panel_w = (grid_w + PANEL_PAD * 2).max(248);
    let panel_h = HEADER_H + 8 + grid_h + PANEL_PAD * 2;
    let total_w = (panel_w + POPUP_SHADOW * 2) as u32;
    let total_h = (panel_h + POPUP_SHADOW * 2) as u32;

    let explorer_btn = PixelRect {
        x: POPUP_SHADOW + panel_w - PANEL_PAD - 96,
        y: POPUP_SHADOW + 8,
        width: 96,
        height: 24,
    };

    let mut tiles = Vec::with_capacity(count);
    let grid_x0 = POPUP_SHADOW + PANEL_PAD;
    let grid_y0 = POPUP_SHADOW + HEADER_H + 8;
    for idx in 0..count {
        let col = (idx % cols) as i32;
        let row = (idx / cols) as i32;
        tiles.push(TileSlot {
            entry_index: idx,
            rect: PixelRect {
                x: grid_x0 + col * (TILE_W + TILE_GAP),
                y: grid_y0 + row * (TILE_H + TILE_GAP),
                width: TILE_W as u32,
                height: TILE_H as u32,
            },
        });
    }

    FolderPopupLayout {
        total_w,
        total_h,
        panel_x: POPUP_SHADOW,
        panel_y: POPUP_SHADOW,
        panel_w,
        panel_h,
        explorer_btn,
        tiles,
    }
}

pub(crate) fn clamp_popup_origin(
    anchor: PixelRect,
    layout: &FolderPopupLayout,
    area: PixelRect,
) -> PixelPoint {
    let center_x = anchor.x + (anchor.width as i32) / 2;
    let min_x = area.x + 8 - POPUP_SHADOW;
    let max_x = (area.x + area.width as i32 - layout.panel_w - 8 - POPUP_SHADOW).max(min_x);
    let x = (center_x - layout.panel_w / 2 - POPUP_SHADOW).clamp(min_x, max_x);

    let above_panel_y = anchor.y - layout.panel_h - 8;
    let min_panel_y = area.y + 8;
    let max_panel_y = (area.y + area.height as i32 - layout.panel_h - 8).max(min_panel_y);
    let panel_y = if above_panel_y >= min_panel_y {
        above_panel_y
    } else {
        (anchor.y + anchor.height as i32 + 8).clamp(min_panel_y, max_panel_y)
    };
    PixelPoint {
        x,
        y: panel_y - POPUP_SHADOW,
    }
}

pub(crate) fn render_folder_popup(
    folder_title: &str,
    entries: &[FolderEntry],
    layout: &FolderPopupLayout,
    palette: &FluentPalette,
    hovered_tile: Option<usize>,
    hovered_explorer: bool,
) -> Result<BgraFrame, String> {
    let mut bgra = BgraFrame::transparent(layout.total_w, layout.total_h);
    let mut texts = Vec::new();
    draw_glass_panel(&mut bgra, layout, palette);

    texts.push(WidgetTextCmd {
        x: layout.panel_x + PANEL_PAD + 2,
        y: layout.panel_y + 6,
        width: (layout.explorer_btn.x - (layout.panel_x + PANEL_PAD + 10)).max(60),
        height: 26,
        text: folder_title.to_owned(),
        font_size_px: 13,
        bold: true,
        color_rgba: palette.text_primary,
        align: TextAlign::Left,
    });

    let btn = layout.explorer_btn;
    let btn_bg = if hovered_explorer {
        palette.menu_hover
    } else if palette.is_dark {
        [255, 255, 255, 16]
    } else {
        [15, 23, 42, 14]
    };
    fill_rounded_rect(
        &mut bgra,
        btn.x,
        btn.y,
        btn.width as i32,
        btn.height as i32,
        6.0,
        btn_bg,
    );
    texts.push(WidgetTextCmd {
        x: btn.x,
        y: btn.y,
        width: btn.width as i32,
        height: btn.height as i32,
        text: "↗ Проводник".to_owned(),
        font_size_px: 11,
        bold: false,
        color_rgba: if hovered_explorer {
            palette.accent
        } else {
            palette.text_secondary
        },
        align: TextAlign::Center,
    });

    fill_hline(
        &mut bgra,
        layout.panel_x + 8,
        layout.panel_x + layout.panel_w - 8,
        layout.panel_y + HEADER_H + 2,
        palette.menu_divider,
    );

    if entries.is_empty() {
        texts.push(WidgetTextCmd {
            x: layout.panel_x + PANEL_PAD,
            y: layout.panel_y + HEADER_H + 16,
            width: layout.panel_w - PANEL_PAD * 2,
            height: 36,
            text: "Папка пуста".to_owned(),
            font_size_px: 12,
            bold: false,
            color_rgba: palette.text_muted,
            align: TextAlign::Center,
        });
    } else {
        for slot in &layout.tiles {
            let Some(entry) = entries.get(slot.entry_index) else {
                continue;
            };
            let is_hot = hovered_tile == Some(slot.entry_index);
            draw_tile(&mut bgra, &mut texts, slot.rect, entry, palette, is_hot);
        }
    }

    composite_widget_frame(&WidgetFrame { bgra, texts })
}

fn draw_tile(
    bgra: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    r: PixelRect,
    entry: &FolderEntry,
    palette: &FluentPalette,
    hovered: bool,
) {
    if hovered {
        fill_rounded_rect(
            bgra,
            r.x,
            r.y,
            r.width as i32,
            r.height as i32,
            8.0,
            palette.menu_hover,
        );
    }
    let icon_sz = 32;
    let ix = r.x + (r.width as i32 - icon_sz) / 2;
    let iy = r.y + 8;
    if let Some(ref img) = entry.icon {
        blit_rgba_scaled(bgra, ix, iy, icon_sz as u32, icon_sz as u32, img);
    } else {
        let fallback_bg = if entry.is_dir {
            [245, 185, 66, 210]
        } else {
            palette.accent
        };
        fill_rounded_rect(bgra, ix, iy, icon_sz, icon_sz, 7.0, fallback_bg);
    }
    texts.push(WidgetTextCmd {
        x: r.x + 3,
        y: r.y + 44,
        width: (r.width as i32) - 6,
        height: 22,
        text: entry.name.clone(),
        font_size_px: 11,
        bold: false,
        color_rgba: palette.text_primary,
        align: TextAlign::Center,
    });
}

fn draw_glass_panel(bgra: &mut BgraFrame, layout: &FolderPopupLayout, palette: &FluentPalette) {
    let w = bgra.width as i32;
    let h = bgra.height as i32;
    let cx = layout.panel_x as f32 + layout.panel_w as f32 * 0.5;
    let cy = layout.panel_y as f32 + layout.panel_h as f32 * 0.5;
    let hx = layout.panel_w as f32 * 0.5;
    let hy = layout.panel_h as f32 * 0.5;
    let shadow_alpha = if palette.is_dark { 88.0f32 } else { 44.0f32 };
    let top_spec = if palette.is_dark {
        [255u8, 255, 255, 44]
    } else {
        [255u8, 255, 255, 220]
    };
    for y in 0..h {
        let is_top = y == layout.panel_y + 1;
        for x in 0..w {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let d_sh = rounded_sdf(fx - cx, fy - (cy + 2.0), hx, hy, POPUP_RADIUS);
            if d_sh > 0.0 && d_sh < POPUP_SHADOW as f32 {
                let t = 1.0 - (d_sh / POPUP_SHADOW as f32);
                blend_px(bgra, x, y, [0, 0, 0, (t * t * shadow_alpha).round() as u8]);
            }
            let d = rounded_sdf(fx - cx, fy - cy, hx, hy, POPUP_RADIUS);
            if d <= 0.5 {
                let cov = (0.5 - d).clamp(0.0, 1.0);
                let mut col = if d >= -1.0 {
                    palette.menu_border
                } else {
                    palette.menu_bg
                };
                col[3] = (f32::from(col[3]) * cov).round().clamp(0.0, 255.0) as u8;
                blend_px(bgra, x, y, col);
                if is_top && d < -1.0 {
                    blend_px(bgra, x, y, top_spec);
                }
            }
        }
    }
}

fn blit_rgba_scaled(dst: &mut BgraFrame, dx: i32, dy: i32, dw: u32, dh: u32, src: &RgbaImage) {
    if src.width == 0 || src.height == 0 || dw == 0 || dh == 0 {
        return;
    }
    for py in 0..dh {
        let sy = ((py as f32 + 0.5) * src.height as f32 / dh as f32)
            .floor()
            .clamp(0.0, (src.height - 1) as f32) as u32;
        for px in 0..dw {
            let sx = ((px as f32 + 0.5) * src.width as f32 / dw as f32)
                .floor()
                .clamp(0.0, (src.width - 1) as f32) as u32;
            let s_idx = ((sy as usize) * (src.width as usize) + (sx as usize)) * 4;
            if s_idx + 3 < src.pixels.len() {
                let col = [
                    src.pixels[s_idx],
                    src.pixels[s_idx + 1],
                    src.pixels[s_idx + 2],
                    src.pixels[s_idx + 3],
                ];
                blend_px(dst, dx + px as i32, dy + py as i32, col);
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
    bgra.pixels[idx] =
        (f32::from(rgba[2]) * sa + f32::from(bgra.pixels[idx]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 1] =
        (f32::from(rgba[1]) * sa + f32::from(bgra.pixels[idx + 1]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 2] =
        (f32::from(rgba[0]) * sa + f32::from(bgra.pixels[idx + 2]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 3] =
        (sa * 255.0 + f32::from(bgra.pixels[idx + 3]) * inv).clamp(0.0, 255.0) as u8;
}

fn rounded_sdf(px: f32, py: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = px.abs() - (hx - r);
    let qy = py.abs() - (hy - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}
