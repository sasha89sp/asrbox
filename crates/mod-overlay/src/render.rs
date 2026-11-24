use asrbox_platform_api::{
    BgraFrame, FluentPalette, OverlayTelemetry, TextAlign, WidgetFrame, WidgetTextCmd,
};

use crate::config::{
    OverlayConfig, MAX_OVERLAY_HEIGHT, MAX_OVERLAY_WIDTH, MIN_OVERLAY_HEIGHT, MIN_OVERLAY_WIDTH,
};
use crate::draw::{fill_circle, fill_overlay_card, fill_progress_bar, fill_rounded_rect, push_txt};

const COL_WHITE: [u8; 4] = [242, 244, 248, 255];
const COL_MUTED: [u8; 4] = [148, 156, 170, 255];
const COL_SUB: [u8; 4] = [174, 182, 198, 255];
const COL_CYAN: [u8; 4] = [56, 178, 216, 255];
const COL_CYAN_BAR: [u8; 4] = [34, 184, 207, 245];
const COL_GREEN: [u8; 4] = [76, 195, 138, 255];
const COL_GREEN_BAR: [u8; 4] = [58, 194, 128, 245];
const COL_AMBER: [u8; 4] = [230, 180, 108, 255];
const COL_CORAL: [u8; 4] = [232, 122, 72, 255];
const COL_CORAL_TXT: [u8; 4] = [244, 146, 100, 255];
const COL_TRACK: [u8; 4] = [255, 255, 255, 28];

struct LayoutScale {
    w: i32,
    pad_x: i32,
    inner_w: i32,
    col_val: i32,
    col_load: i32,
    font_main: i32,
    font_lbl: i32,
    font_sm: i32,
    bar_h: i32,
    row_h: i32,
    top_y: i32,
}

impl LayoutScale {
    fn from_size(w: u32, h: u32) -> Self {
        let (wi, hi) = (w as i32, h as i32);
        let s = ((w as f32) / 320.0)
            .min((h as f32) / 188.0)
            .clamp(0.68, 2.1);
        let pad_x = (12.0 * s).round() as i32;
        let pad_y = (8.0 * s).round() as i32;
        let inner_w = (wi - pad_x * 2).max(120);
        Self {
            w: wi,
            pad_x,
            inner_w,
            col_val: pad_x + (inner_w as f32 * 0.27).round() as i32,
            col_load: pad_x + (inner_w as f32 * 0.54).round() as i32,
            font_main: (21.0 * s).round().clamp(13.0, 42.0) as i32,
            font_lbl: (19.0 * s).round().clamp(12.0, 38.0) as i32,
            font_sm: (13.0 * s).round().clamp(10.0, 26.0) as i32,
            bar_h: (6.0 * s).round().clamp(4.0, 12.0) as i32,
            row_h: ((hi - pad_y * 2) / 5).max(22),
            top_y: pad_y,
        }
    }

    fn lbl(&self, t: &mut Vec<WidgetTextCmd>, y: i32, h: i32, text: &str, col: [u8; 4]) {
        push_txt(
            t,
            (self.pad_x, y, 76, h),
            text,
            (self.font_lbl, false),
            col,
            TextAlign::Left,
        );
    }

    fn val(
        &self,
        t: &mut Vec<WidgetTextCmd>,
        rect: (i32, i32, i32, i32),
        text: &str,
        col: [u8; 4],
    ) {
        push_txt(t, rect, text, (self.font_main, true), col, TextAlign::Left);
    }
}

pub fn render_game_overlay(
    cfg: &OverlayConfig,
    snap: &OverlayTelemetry,
    setup_mode: bool,
    pal: &FluentPalette,
) -> WidgetFrame {
    let w = cfg.width.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
    let h = cfg.height.clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT);
    let mut bgra = BgraFrame::transparent(w, h);
    let mut texts = Vec::with_capacity(20);
    fill_overlay_card(&mut bgra, cfg.opacity_percent, setup_mode, pal);

    let sc = LayoutScale::from_size(w, h);
    draw_fps_row(&mut bgra, &mut texts, &sc, snap, setup_mode, pal);
    draw_gpu_row(&mut bgra, &mut texts, &sc, snap);
    draw_cpu_row(&mut bgra, &mut texts, &sc, snap);
    draw_ram_row(&mut bgra, &mut texts, &sc, snap);
    draw_net_row(&mut bgra, &mut texts, &sc, snap);
    WidgetFrame { bgra, texts }
}

fn draw_fps_row(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    sc: &LayoutScale,
    snap: &OverlayTelemetry,
    setup_mode: bool,
    pal: &FluentPalette,
) {
    let (y, h) = (sc.top_y, sc.row_h);
    sc.lbl(texts, y, h, "FPS", COL_MUTED);
    let fps = snap.fps.map_or_else(|| "—".to_owned(), |v| v.to_string());
    sc.val(texts, (sc.col_val, y, 86, h), &fps, COL_WHITE);

    let sub = match (snap.fps_low_1pct, snap.display_hz) {
        (Some(low), _) => format!("1% {low}"),
        (None, hz) if hz > 0 => format!("{hz}Hz"),
        _ => "1% —".to_owned(),
    };
    let sub_y = y + (sc.font_main - sc.font_sm) / 3;
    push_txt(
        texts,
        (sc.col_load - 6, sub_y, 110, h),
        &sub,
        (sc.font_sm, false),
        COL_SUB,
        TextAlign::Left,
    );

    if setup_mode {
        let bw = (sc.font_sm * 6).clamp(62, 110);
        let bh = (sc.font_sm + 6).clamp(16, 26);
        let (bx, by) = (sc.w - sc.pad_x - bw, y + (h - bh) / 2);
        let acc = pal.accent;
        fill_rounded_rect(frame, (bx, by, bw, bh), 5.0, [acc[0], acc[1], acc[2], 70]);
        push_txt(
            texts,
            (bx, by, bw, bh),
            "Настройка",
            ((sc.font_sm - 2).max(9), true),
            acc,
            TextAlign::Center,
        );
    } else {
        let dot_col = if snap.game_active {
            COL_GREEN
        } else {
            COL_MUTED
        };
        let r = (sc.bar_h as f32 * 0.65).clamp(3.0, 6.0);
        fill_circle(
            frame,
            sc.w - sc.pad_x - (r as i32 + 2),
            y + h / 2,
            r,
            dot_col,
        );
    }
}

fn draw_gpu_row(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    sc: &LayoutScale,
    snap: &OverlayTelemetry,
) {
    let y = sc.top_y + sc.row_h;
    let text_h = (sc.row_h - sc.bar_h - 3).max(16);
    let temp = fmt_opt(snap.gpu_temp_c, "°C");
    let load = fmt_opt(snap.gpu_usage_pct, "%");
    draw_temp_load_prefix(texts, sc, y, text_h, "GPU", COL_CYAN, &temp, &load);

    let vram_w = (sc.inner_w as f32 * 0.30).round() as i32;
    let vram_x = sc.w - sc.pad_x - vram_w;
    let sm = (sc.font_sm - 1).max(9);
    let half = (text_h / 2).max(10);
    let vram_gb = format_gb(snap.vram_used_mb, false);
    push_txt(
        texts,
        (vram_x, y - 1, vram_w, half),
        "VRAM",
        (sm, false),
        COL_CYAN,
        TextAlign::Right,
    );
    push_txt(
        texts,
        (vram_x, y + half - 2, vram_w, half + 2),
        &vram_gb,
        (sm, true),
        COL_CYAN,
        TextAlign::Right,
    );

    let pct = snap.gpu_usage_pct.unwrap_or_else(|| {
        if snap.vram_total_mb > 0 {
            ((u64::from(snap.vram_used_mb) * 100) / u64::from(snap.vram_total_mb)).min(100) as u8
        } else {
            0
        }
    });
    let bar_y = y + sc.row_h - sc.bar_h - 2;
    fill_progress_bar(
        frame,
        (sc.pad_x, bar_y, sc.inner_w, sc.bar_h),
        pct,
        COL_CYAN_BAR,
        COL_TRACK,
    );
}

fn draw_cpu_row(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    sc: &LayoutScale,
    snap: &OverlayTelemetry,
) {
    let y = sc.top_y + sc.row_h * 2;
    let text_h = (sc.row_h - sc.bar_h - 3).max(16);
    let temp = fmt_opt(snap.cpu_temp_c, "°C");
    let load = format!("{}%", snap.cpu_usage_pct.min(100));
    draw_temp_load_prefix(texts, sc, y, text_h, "CPU", COL_GREEN, &temp, &load);
    let bar_y = y + sc.row_h - sc.bar_h - 2;
    fill_progress_bar(
        frame,
        (sc.pad_x, bar_y, sc.inner_w, sc.bar_h),
        snap.cpu_usage_pct,
        COL_GREEN_BAR,
        COL_TRACK,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_temp_load_prefix(
    texts: &mut Vec<WidgetTextCmd>,
    sc: &LayoutScale,
    y: i32,
    text_h: i32,
    label: &str,
    label_col: [u8; 4],
    temp: &str,
    load: &str,
) {
    sc.lbl(texts, y, text_h, label, label_col);
    sc.val(texts, (sc.col_val, y, 90, text_h), temp, COL_WHITE);
    sc.val(texts, (sc.col_load, y, 80, text_h), load, COL_WHITE);
}

fn draw_ram_row(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    sc: &LayoutScale,
    snap: &OverlayTelemetry,
) {
    let y = sc.top_y + sc.row_h * 3;
    let thin_h = (sc.bar_h / 2).clamp(3, 6);
    let text_h = (sc.row_h - thin_h - 4).max(16);
    sc.lbl(texts, y, text_h, "RAM", COL_MUTED);
    let ram = format_gb(snap.ram_used_mb, true);
    sc.val(texts, (sc.col_val, y, 140, text_h), &ram, COL_WHITE);

    let pill_w = (sc.bar_h * 2 + 2).clamp(10, 22);
    let pill_h = sc.bar_h.clamp(5, 10);
    let pill_x = sc.w - sc.pad_x - pill_w;
    let pill_y = y + (text_h - pill_h) / 2;
    fill_rounded_rect(
        frame,
        (pill_x, pill_y, pill_w, pill_h),
        pill_h as f32 * 0.5,
        COL_CYAN_BAR,
    );

    let bar_y = y + sc.row_h - thin_h - 2;
    fill_progress_bar(
        frame,
        (sc.pad_x, bar_y, sc.inner_w, thin_h),
        snap.ram_usage_pct,
        [118, 130, 148, 190],
        [255, 255, 255, 22],
    );
}

fn draw_net_row(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    sc: &LayoutScale,
    snap: &OverlayTelemetry,
) {
    let (y, h) = (sc.top_y + sc.row_h * 4, sc.row_h);
    sc.lbl(texts, y, h, "NET", COL_MUTED);
    let (ping_str, ping_col) = match snap.net_ping_ms {
        Some(ms) if ms < 50 => (format!("{ms}ms"), COL_GREEN),
        Some(ms) if ms < 110 => (format!("{ms}ms"), COL_AMBER),
        Some(ms) => (format!("{ms}ms"), COL_CORAL),
        None => ("—ms".to_owned(), COL_MUTED),
    };
    sc.val(texts, (sc.col_val, y, 110, h), &ping_str, ping_col);

    let pl_str = format!(
        "{}.{}% PL",
        snap.net_loss_tenths_pct / 10,
        snap.net_loss_tenths_pct % 10
    );
    let pill_w = (sc.inner_w as f32 * 0.39).round().clamp(78.0, 190.0) as i32;
    let pill_h = (sc.row_h - 6).clamp(18, 38);
    let pill_x = sc.w - sc.pad_x - pill_w;
    let pill_y = y + (h - pill_h) / 2;

    if snap.net_loss_tenths_pct > 0 {
        fill_rounded_rect(
            frame,
            (pill_x, pill_y, pill_w, pill_h),
            7.0,
            [168, 62, 38, 115],
        );
        let r = (pill_h as f32 * 0.31).clamp(5.0, 10.0);
        let (cx, cy) = (pill_x + (r as i32) + 7, pill_y + pill_h / 2);
        fill_circle(frame, cx, cy, r, COL_CORAL);
        let d = (r * 2.0) as i32;
        push_txt(
            texts,
            (cx - r as i32, cy - r as i32, d, d),
            "!",
            ((sc.font_sm - 1).max(9), true),
            [24, 24, 28, 255],
            TextAlign::Center,
        );
        let tx = cx + (r as i32) + 4;
        let tw = (pill_x + pill_w - tx - 4).max(32);
        push_txt(
            texts,
            (tx, pill_y, tw, pill_h),
            &pl_str,
            (sc.font_sm, true),
            COL_CORAL_TXT,
            TextAlign::Center,
        );
    } else {
        fill_rounded_rect(
            frame,
            (pill_x, pill_y, pill_w, pill_h),
            7.0,
            [255, 255, 255, 16],
        );
        push_txt(
            texts,
            (pill_x, pill_y, pill_w, pill_h),
            &pl_str,
            (sc.font_sm, false),
            COL_MUTED,
            TextAlign::Center,
        );
    }
}

fn fmt_opt<T: std::fmt::Display>(val: Option<T>, suffix: &str) -> String {
    val.map_or_else(|| format!("—{suffix}"), |v| format!("{v}{suffix}"))
}

fn format_gb(mb: u32, spaced: bool) -> String {
    let tenths = ((u64::from(mb) * 10 + 512) / 1024) as u32;
    let sep = if spaced { " " } else { "" };
    format!("{}.{}{sep}GB", tenths / 10, tenths % 10)
}
