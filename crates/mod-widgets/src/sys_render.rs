use asrbox_platform_api::{
    BgraFrame, LocalDateTime, PixelPoint, SysTelemetry, TextAlign, WidgetFrame,
};

use crate::date::{days_from_civil, weekday_mon0};
use crate::draw::{
    draw_hline, draw_resize_grip, draw_vline, fill_macos_glass_card, fill_rounded_rect,
};
use crate::sys_sections::{
    draw_cpu_section, draw_gpu_section, draw_net_section, draw_ram_disk_section, Canvas,
};

pub const SYSMON_HEADER_H: i32 = 34;

pub(crate) const TEXT_BRIGHT: [u8; 4] = [238, 240, 244, 255];
pub(crate) const TEXT_MAIN: [u8; 4] = [218, 222, 230, 255];
pub(crate) const TEXT_MUTED: [u8; 4] = [148, 155, 168, 255];
pub(crate) const TEXT_TAG: [u8; 4] = [140, 147, 160, 220];
pub(crate) const TEXT_CYAN: [u8; 4] = [242, 146, 100, 255];
pub(crate) const TEXT_GREEN: [u8; 4] = [104, 199, 148, 255];
pub(crate) const TEXT_AMBER: [u8; 4] = [234, 179, 84, 255];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SysHitTarget {
    #[default]
    None,
    HeaderDrag,
    SettingsBtn,
    RefreshBtn,
}

pub struct SysRenderInput<'a> {
    pub width: u32,
    pub height: u32,
    pub opacity_percent: u8,
    pub refresh_ms: u32,
    pub now: LocalDateTime,
    pub data: &'a SysTelemetry,
    pub hovered: SysHitTarget,
}

pub fn hit_test_sysmon(pt: PixelPoint, width: u32, height: u32) -> SysHitTarget {
    let (w, h) = (width as i32, height as i32);
    if pt.x < 0 || pt.y < 0 || pt.x >= w || pt.y >= h {
        return SysHitTarget::None;
    }
    if pt.y < SYSMON_HEADER_H {
        if pt.x >= w - 34 && pt.x < w - 10 {
            return SysHitTarget::RefreshBtn;
        }
        if pt.x >= w - 60 && pt.x < w - 36 {
            return SysHitTarget::SettingsBtn;
        }
        return SysHitTarget::HeaderDrag;
    }
    SysHitTarget::None
}

pub fn render_sysmon_widget(inp: &SysRenderInput<'_>) -> WidgetFrame {
    let w = inp.width.max(340);
    let h = inp.height.max(300);
    let (wi, hi) = (w as i32, h as i32);
    let mut bgra = BgraFrame::transparent(w, h);
    let mut texts = Vec::with_capacity(64);

    let pct = u16::from(inp.opacity_percent.clamp(20, 100));
    let div = [255, 255, 255, ((pct * 30) / 100).clamp(14, 38) as u8];

    fill_macos_glass_card(&mut bgra, inp.opacity_percent);
    {
        let mut c = Canvas {
            frame: &mut bgra,
            texts: &mut texts,
        };
        draw_top_bar(&mut c, inp, wi);
        draw_hline(c.frame, SYSMON_HEADER_H, div);

        let body_h = hi - SYSMON_HEADER_H - 8;
        let cpu_h = (body_h * 19 / 100).clamp(64, 102);
        let gpu_h = (body_h * 17 / 100).clamp(58, 92);
        let ram_h = (body_h * 21 / 100).clamp(72, 118);
        let net_h = (body_h * 16 / 100).clamp(54, 86);

        let y_cpu = SYSMON_HEADER_H;
        draw_cpu_section(&mut c, inp, wi, y_cpu, cpu_h);
        let y_gpu = y_cpu + cpu_h;
        draw_hline(c.frame, y_gpu, div);
        draw_gpu_section(&mut c, inp, wi, y_gpu, gpu_h);
        let y_ram = y_gpu + gpu_h;
        draw_hline(c.frame, y_ram, div);
        draw_ram_disk_section(&mut c, inp, wi, y_ram, ram_h);
        let y_net = y_ram + ram_h;
        draw_hline(c.frame, y_net, div);
        draw_net_section(&mut c, inp, wi, y_net, net_h);
        let y_live = y_net + net_h;
        draw_hline(c.frame, y_live, div);
        draw_live_procs(&mut c, inp, wi, y_live, hi - y_live - 6, div);
    }

    draw_resize_grip(&mut bgra);
    WidgetFrame { bgra, texts }
}

fn draw_top_bar(c: &mut Canvas<'_>, inp: &SysRenderInput<'_>, w: i32) {
    let dt = inp.now;
    let days = days_from_civil(dt.year, dt.month, dt.day);
    let wd = weekday_ru(weekday_mon0(days));
    let wk = iso_week_number(dt.year, days);
    let up = format_uptime(inp.data.uptime_sec);
    let plan = if inp.data.power_plan.is_empty() {
        "BAL"
    } else {
        &inp.data.power_plan
    };
    let watts = inp
        .data
        .system_watts
        .map(|pw| format!(" • ~{pw} Вт"))
        .unwrap_or_default();
    let line = format!(
        "{:02}.{:02}.{:04} • {wd} • W{wk:02} • {:02}:{:02} • Up: {up} • [{plan}]{watts} • Err: {}",
        dt.day, dt.month, dt.year, dt.hour, dt.minute, inp.data.err_count
    );
    c.text(
        [14, 7, w - 76, 20],
        line,
        12,
        false,
        TEXT_MAIN,
        TextAlign::Left,
    );

    let btns = [
        (SysHitTarget::SettingsBtn, w - 60, "⚙"),
        (SysHitTarget::RefreshBtn, w - 34, "↻"),
    ];
    for (target, bx, glyph) in btns {
        let hov = inp.hovered == target;
        if hov {
            fill_rounded_rect(c.frame, bx, 6, 22, 22, 6.0, [255, 255, 255, 28]);
        }
        let col = if hov { TEXT_BRIGHT } else { TEXT_TAG };
        c.text([bx, 6, 22, 22], glyph, 13, false, col, TextAlign::Center);
    }
}

fn draw_live_procs(
    c: &mut Canvas<'_>,
    inp: &SysRenderInput<'_>,
    w: i32,
    y: i32,
    h: i32,
    div: [u8; 4],
) {
    c.hdr(w, y, "ТОП-5 ПРОЦЕССОВ В МОМЕНТЕ", "LIVE");
    let mid = w / 2 - 6;
    let hy = y + 23;
    c.text(
        [mid - 58, hy, 48, 16],
        "CPU",
        11,
        false,
        TEXT_TAG,
        TextAlign::Right,
    );
    c.text(
        [w - 68, hy, 54, 16],
        "ОЗУ",
        11,
        false,
        TEXT_TAG,
        TextAlign::Right,
    );

    let ly = hy + 16;
    let avail = (h - 42).max(16);
    let max_r = (avail / 17).clamp(1, 5) as usize;
    let step = (avail / max_r as i32).clamp(15, 22);
    draw_vline(c.frame, mid, ly, ly + (max_r as i32) * step, div);

    for i in 0..max_r {
        let ry = ly + (i as i32) * step;
        if let Some(p) = inp.data.top_cpu.get(i) {
            c.l(14, ry, mid - 66, &p.name);
            c.text(
                [mid - 56, ry, 46, 17],
                format!("{}%", p.cpu_pct),
                12,
                false,
                TEXT_BRIGHT,
                TextAlign::Right,
            );
        }
        if let Some(p) = inp.data.top_ram.get(i) {
            c.l(mid + 14, ry, w - mid - 94, &p.name);
            c.text(
                [w - 78, ry, 64, 17],
                fmt_mem(p.mem_mb),
                12,
                false,
                TEXT_BRIGHT,
                TextAlign::Right,
            );
        }
    }
}

pub(crate) fn fmt_mem(mb: u32) -> String {
    if mb >= 1024 {
        format!("{:.1} ГБ", f64::from(mb) / 1024.0)
    } else {
        format!("{mb} МБ")
    }
}

fn format_uptime(sec: u64) -> String {
    let days = sec / 86_400;
    let hrs = (sec % 86_400) / 3600;
    let mins = (sec % 3600) / 60;
    if days > 0 {
        format!("{days}д {hrs}ч")
    } else {
        format!("{hrs}ч {mins}м")
    }
}

fn weekday_ru(mon0: u8) -> &'static str {
    match mon0 {
        0 => "ПН",
        1 => "ВТ",
        2 => "СР",
        3 => "ЧТ",
        4 => "ПТ",
        5 => "СБ",
        _ => "ВС",
    }
}

fn iso_week_number(year: i32, cur_days: i64) -> u32 {
    let jan1 = days_from_civil(year, 1, 1);
    let doy = (cur_days - jan1).max(0) as u32;
    let wd = u32::from(weekday_mon0(cur_days));
    (((doy + 10).saturating_sub(wd)) / 7).clamp(1, 53)
}

pub(crate) fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        let head: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{head}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asrbox_platform_api::{DriveTelemetry, ProcUsage};

    #[test]
    fn renders_sysmon_widget_at_multiple_sizes() {
        let mut data = SysTelemetry {
            uptime_sec: 3 * 86_400 + 14 * 3600,
            power_plan: "BAL".to_owned(),
            ..SysTelemetry::default()
        };
        data.cpu.usage_pct = 42;
        data.cpu.cur_mhz = 4650;
        data.cpu.max_mhz = 5100;
        data.cpu.logical_cores = 16;
        data.cpu.active_threads = 2400;
        data.gpu.display_hz = 144;
        data.ram.usage_pct = 61;
        data.ram.used_mb = 19_968;
        data.ram.total_mb = 32_768;
        data.drives.push(DriveTelemetry {
            letter: 'C',
            busy_pct: 14,
            read_tenths_mb_s: 1450,
            write_tenths_mb_s: 420,
            free_gb: 124,
            total_gb: 500,
            temp_c: None,
        });
        data.top_cpu.push(ProcUsage {
            name: "chrome.exe".to_owned(),
            cpu_pct: 18,
            mem_mb: 800,
        });
        let now = LocalDateTime {
            year: 2026,
            month: 11,
            day: 23,
            hour: 4,
            minute: 20,
            second: 0,
            utc_offset_min: 180,
        };
        for (w, h) in [(340, 300), (560, 492), (760, 620)] {
            let frame = render_sysmon_widget(&SysRenderInput {
                width: w,
                height: h,
                opacity_percent: 88,
                refresh_ms: 2000,
                now,
                data: &data,
                hovered: SysHitTarget::None,
            });
            assert_eq!(frame.bgra.width, w);
            assert_eq!(frame.bgra.height, h);
            assert!(!frame.texts.is_empty());
        }
    }
}
