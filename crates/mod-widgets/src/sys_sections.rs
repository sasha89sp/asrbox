use asrbox_platform_api::{BgraFrame, FluentPalette, TextAlign, WidgetTextCmd};

use crate::draw::{fill_glow_bar, fill_pill_badge};
use crate::sys_render::{fmt_mem, truncate, SysRenderInput};

pub(crate) struct Canvas<'a> {
    pub frame: &'a mut BgraFrame,
    pub texts: &'a mut Vec<WidgetTextCmd>,
    pub pal: &'a FluentPalette,
}

impl Canvas<'_> {
    pub(crate) fn text(
        &mut self,
        r: [i32; 4],
        s: impl Into<String>,
        sz: i32,
        bold: bool,
        col: [u8; 4],
        align: TextAlign,
    ) {
        if r[2] <= 4 || r[3] <= 4 {
            return;
        }
        self.texts.push(WidgetTextCmd {
            x: r[0],
            y: r[1],
            width: r[2],
            height: r[3],
            text: s.into(),
            font_size_px: sz,
            bold,
            color_rgba: col,
            align,
        });
    }

    pub(crate) fn l(&mut self, x: i32, y: i32, w: i32, s: impl Into<String>) {
        let col = self.pal.text_primary;
        self.text([x, y, w, 18], s, 12, false, col, TextAlign::Left);
    }

    pub(crate) fn lc(&mut self, x: i32, y: i32, w: i32, s: impl Into<String>, col: [u8; 4]) {
        self.text([x, y, w, 18], s, 12, false, col, TextAlign::Left);
    }

    pub(crate) fn hdr(&mut self, w: i32, y: i32, title: &str, tag: &str) {
        let bright = self.pal.text_primary;
        let muted = self.pal.text_muted;
        self.text(
            [14, y + 6, 220, 18],
            title,
            12,
            true,
            bright,
            TextAlign::Left,
        );
        if !tag.is_empty() {
            self.text(
                [w - 134, y + 6, 120, 18],
                tag,
                11,
                false,
                muted,
                TextAlign::Right,
            );
        }
    }
}

pub(crate) fn draw_cpu_section(
    c: &mut Canvas<'_>,
    inp: &SysRenderInput<'_>,
    w: i32,
    y: i32,
    h: i32,
) {
    let cpu = &inp.data.cpu;
    let step = ((h - 24) / 3).clamp(16, 24);
    c.hdr(w, y, "ПРОЦЕССОР", "CPU");

    let (r1, mid) = (y + 24, (w * 46 / 100).max(205));
    c.l(14, r1, 96, format!("Нагрузка: {:>2}%", cpu.usage_pct));
    let bar_h = if c.pal.design_theme.is_glass() { 5 } else { 7 };
    fill_glow_bar(
        c.frame,
        106,
        r1 + 6,
        (mid - 116).clamp(50, 140),
        bar_h,
        cpu.usage_pct,
        c.pal,
    );
    let temp = match (cpu.temp_c, cpu.tdp_watts, cpu.fan_rpm) {
        (Some(t), Some(pw), Some(rpm)) => format!("Темп/TDP: {t}°C | {pw} Вт | {rpm} RPM"),
        (Some(t), Some(pw), None) => format!("Темп/TDP: {t}°C | {pw} Вт"),
        (Some(t), None, _) => format!("Темп: {t}°C"),
        _ => format!("Обновление: {:.1} с", f64::from(inp.refresh_ms) / 1000.0),
    };
    c.l(mid, r1, w - mid - 14, temp);

    let r2 = r1 + step;
    let freq = if cpu.cur_mhz > 0 {
        let cur = f64::from(cpu.cur_mhz) / 1000.0;
        let mx = f64::from(cpu.max_mhz.max(cpu.cur_mhz)) / 1000.0;
        format!("Частота: {cur:.2} ГГц / {mx:.2} ГГц")
    } else {
        "Частота: —".to_owned()
    };
    c.l(14, r2, mid - 20, freq);
    let thr = if cpu.throttled { "Да" } else { "Нет" };
    let tcol = if cpu.throttled {
        c.pal.warn_text
    } else {
        c.pal.text_primary
    };
    let cores = format!(
        "Ядра/Потоки: {} / {} | [THR: {thr}]",
        cpu.logical_cores, cpu.active_threads
    );
    c.lc(mid, r2, w - mid - 14, cores, tcol);

    let r3 = r2 + step;
    if !inp.data.gpu.name.is_empty() {
        let hint = format!("GPU: {}", truncate(&inp.data.gpu.name, 26));
        let muted = c.pal.text_secondary;
        c.text(
            [14, r3, mid - 20, 18],
            hint,
            11,
            false,
            muted,
            TextAlign::Left,
        );
    }
    let top = cpu
        .top_proc
        .as_ref()
        .map(|p| format!("Топ CPU: {} ({}%)", p.name, p.cpu_pct))
        .unwrap_or_else(|| "Топ CPU: —".to_owned());
    c.l(mid, r3, w - mid - 14, top);
}

pub(crate) fn draw_gpu_section(
    c: &mut Canvas<'_>,
    inp: &SysRenderInput<'_>,
    w: i32,
    y: i32,
    h: i32,
) {
    let gpu = &inp.data.gpu;
    let step = ((h - 26) / 2).clamp(17, 25);
    c.hdr(w, y, "ГРАФИКА", "");

    let badge = if gpu.display_hz > 0 {
        format!("Экран: {} Гц", gpu.display_hz)
    } else {
        "GPU".to_owned()
    };
    let bw = 114;
    fill_pill_badge(c.frame, w - bw - 14, y + 5, bw, 20, c.pal);
    let acc = c.pal.accent;
    c.text(
        [w - bw - 14, y + 6, bw, 18],
        badge,
        11,
        true,
        acc,
        TextAlign::Center,
    );

    let (r1, mid) = (y + 27, (w * 42 / 100).max(190));
    let vx = (w * 72 / 100).max(mid + 120);
    let load = gpu.usage_pct.map_or_else(
        || "Нагрузка: —".to_owned(),
        |u| format!("Нагрузка: {u:>2}%"),
    );
    c.l(14, r1, 96, load);
    let bar_h = if c.pal.design_theme.is_glass() { 5 } else { 7 };
    fill_glow_bar(
        c.frame,
        106,
        r1 + 6,
        (mid - 114).clamp(40, 110),
        bar_h,
        gpu.usage_pct.unwrap_or(0),
        c.pal,
    );

    let clk = gpu
        .clock_mhz
        .map_or_else(|| "Частота: —".to_owned(), |m| format!("Частота: {m} МГц"));
    c.l(mid, r1, vx - mid - 6, clk);

    let vpct = if gpu.vram_total_mb > 0 {
        ((u64::from(gpu.vram_used_mb) * 100) / u64::from(gpu.vram_total_mb)).min(100) as u8
    } else {
        0
    };
    c.l(vx, r1, 46, "VRAM:");
    fill_glow_bar(
        c.frame,
        vx + 44,
        r1 + 6,
        (w - vx - 58).clamp(32, 110),
        bar_h,
        vpct,
        c.pal,
    );

    let r2 = r1 + step;
    let vlbl = if gpu.vram_total_mb > 0 {
        let u = f64::from(gpu.vram_used_mb) / 1024.0;
        let t = f64::from(gpu.vram_total_mb) / 1024.0;
        format!("VRAM: {u:.1}/{t:.0}G")
    } else {
        "VRAM: —".to_owned()
    };
    c.l(14, r2, mid - 18, vlbl);
    let ts = gpu
        .temp_c
        .map_or_else(|| "—".to_owned(), |t| format!("{t}°C"));
    let fs = gpu
        .fan_pct
        .map_or_else(|| "—".to_owned(), |f| format!("{f}%"));
    let thr = if gpu.throttled { "Да" } else { "Нет" };
    c.l(
        mid,
        r2,
        w - mid - 14,
        format!("Темп/Fan: {ts} | {fs} | [THR: {thr}]"),
    );
}

pub(crate) fn draw_ram_disk_section(
    c: &mut Canvas<'_>,
    inp: &SysRenderInput<'_>,
    w: i32,
    y: i32,
    h: i32,
) {
    let ram = &inp.data.ram;
    c.hdr(w, y, "ПАМЯТЬ И ДИСКИ", "RAM / STORAGE");

    let max_d = if h >= 104 { 3 } else { 2 };
    let rows = (1 + inp.data.drives.len().min(max_d).max(1)) as i32;
    let step = ((h - 24) / rows).clamp(17, 25);

    let r1 = y + 24;
    let (ug, tg) = (
        f64::from(ram.used_mb) / 1024.0,
        f64::from(ram.total_mb) / 1024.0,
    );
    let (cg, pg) = (
        f64::from(ram.cache_mb) / 1024.0,
        f64::from(ram.pagefile_used_mb) / 1024.0,
    );
    let ram_s = format!(
        "ОЗУ: {:>2}%  {ug:.1}/{tg:.0}G (Кэш: {cg:.1}G | PF: {pg:.1}G)",
        ram.usage_pct
    );
    let bx = (w * 51 / 100).max(245);
    let bw = (w * 15 / 100).clamp(44, 96);
    c.l(14, r1, bx - 18, ram_s);
    let bar_h = if c.pal.design_theme.is_glass() { 5 } else { 7 };
    fill_glow_bar(c.frame, bx, r1 + 6, bw, bar_h, ram.usage_pct, c.pal);
    if let Some(p) = &ram.top_proc {
        let top = format!("Топ: {} ({})", truncate(&p.name, 14), fmt_mem(p.mem_mb));
        let tx = bx + bw + 8;
        c.l(tx, r1, w - tx - 14, top);
    }

    for (idx, d) in inp.data.drives.iter().take(max_d).enumerate() {
        let ry = r1 + ((idx as i32) + 1) * step;
        let pct = if d.busy_pct > 0 {
            d.busy_pct
        } else if d.total_gb > 0 {
            let used = u64::from(d.total_gb.saturating_sub(d.free_gb));
            ((used * 100) / u64::from(d.total_gb)).min(100) as u8
        } else {
            0
        };
        c.l(14, ry, 86, format!("Диск {}: {:>2}%", d.letter, d.busy_pct));
        fill_glow_bar(c.frame, 102, ry + 6, 54, bar_h, pct, c.pal);
        let (rd, wr) = (
            f64::from(d.read_tenths_mb_s) / 10.0,
            f64::from(d.write_tenths_mb_s) / 10.0,
        );
        let det = format!(
            "| Чтение: {rd:.1} МБ/с | Запись: {wr:.1} МБ/с | Свободно: {} ГБ",
            d.free_gb
        );
        c.l(164, ry, w - 178, det);
    }
}

pub(crate) fn draw_net_section(
    c: &mut Canvas<'_>,
    inp: &SysRenderInput<'_>,
    w: i32,
    y: i32,
    h: i32,
) {
    let net = &inp.data.net;
    let step = ((h - 24) / 2).clamp(17, 24);
    c.hdr(w, y, "СЕТЬ И ЗАДЕРЖКИ", "NET / LATENCY");

    let (r1, mid) = (y + 24, (w * 36 / 100).max(162));
    let ip = if net.ip_v4.is_empty() {
        "IP: —".to_owned()
    } else if net.vpn_active {
        format!("IP: {} [VPN]", net.ip_v4)
    } else {
        format!("IP: {}", net.ip_v4)
    };
    c.l(14, r1, mid - 18, ip);
    c.l(mid, r1, 60, "Трафик:");
    let acc = c.pal.accent;
    let ok = c.pal.success;
    c.lc(
        mid + 54,
        r1,
        112,
        format!("▼ {:.1} Мбит/с", f64::from(net.rx_kbps) / 1000.0),
        acc,
    );
    c.lc(
        mid + 166,
        r1,
        w - mid - 180,
        format!("▲ {:.1} Мбит/с", f64::from(net.tx_kbps) / 1000.0),
        ok,
    );

    let r2 = r1 + step;
    let gw = net
        .gw_ping_ms
        .map_or_else(|| "—".to_owned(), |m| format!("{m} мс"));
    let inet = net.inet_ping_ms.map_or_else(
        || "—".to_owned(),
        |m| {
            format!(
                "{m} мс ({}% Jitter {})",
                net.inet_loss_pct, net.inet_jitter_ms
            )
        },
    );
    let tmr = if net.timer_res_us > 0 {
        format!("{:.2} мс", f64::from(net.timer_res_us) / 1000.0)
    } else {
        "—".to_owned()
    };
    let st = if net.sample_us < 15_000 {
        "Норма"
    } else {
        "Нагр."
    };
    let l2 = format!(
        "GW: {gw} | 1.1.1.1: {inet} | {} мкс ({st}) | Таймер ОС: {tmr}",
        net.sample_us
    );
    c.l(14, r2, w - 28, l2);
}
