use std::rc::Rc;

use asrbox_platform_api::SysTelemetry;
use slint::{ModelRc, SharedString, VecModel};

use crate::generated::ui::{SlintSysDriveRow, SlintSysProcRow, SysMonitorWindow};
use crate::widgets_sysmon::SysMonSlintState;
use crate::win_os::local_time_now;

pub(crate) fn sync_sysmon_ui(s: &SysMonSlintState) {
    let ui = &s.ui;
    let d = &s.data;
    let now = local_time_now();
    let up_h = d.uptime_sec / 3600;
    let watts = d
        .system_watts
        .map_or_else(String::new, |w| format!(" · ~{w} Вт"));
    ui.set_header_meta(SharedString::from(format!(
        "{:02}.{:02}.{} · {:02}:{:02} · Up: {}д {}ч · [{}]{watts}",
        now.day,
        now.month,
        now.year,
        now.hour,
        now.minute,
        up_h / 24,
        up_h % 24,
        d.power_plan
    )));
    sync_cpu_section(ui, d, s.cfg.refresh_interval_ms);
    sync_gpu_section(ui, d);
    sync_ram_and_drives(ui, d);
    sync_net_and_procs(ui, d);
}

fn sync_cpu_section(ui: &SysMonitorWindow, d: &SysTelemetry, refresh_ms: u32) {
    let cpu = &d.cpu;
    ui.set_cpu_pct(i32::from(cpu.usage_pct));
    let temp = match (cpu.temp_c, cpu.tdp_watts, cpu.fan_rpm) {
        (Some(t), Some(pw), Some(rpm)) => format!("Темп/TDP: {t}°C | {pw} Вт | {rpm} RPM"),
        (Some(t), Some(pw), None) => format!("Темп/TDP: {t}°C | {pw} Вт"),
        (Some(t), None, _) => format!("Темп: {t}°C"),
        _ => format!("Обновление: {:.1} с", f64::from(refresh_ms) / 1000.0),
    };
    ui.set_cpu_temp_line(SharedString::from(temp));
    let freq = if cpu.cur_mhz > 0 {
        let cur = f64::from(cpu.cur_mhz) / 1000.0;
        let mx = f64::from(cpu.max_mhz.max(cpu.cur_mhz)) / 1000.0;
        format!("Частота: {cur:.2} / {mx:.2} ГГц")
    } else {
        "Частота: —".to_owned()
    };
    ui.set_cpu_freq_line(SharedString::from(freq));
    let thr = if cpu.throttled { "Да" } else { "Нет" };
    ui.set_cpu_throttled(cpu.throttled);
    ui.set_cpu_cores_line(SharedString::from(format!(
        "Ядра/Потоки: {} / {} | [THR: {thr}]",
        cpu.logical_cores, cpu.active_threads
    )));
    let gpu_hint = if d.gpu.name.is_empty() {
        "GPU: —".to_owned()
    } else {
        format!("GPU: {}", d.gpu.name)
    };
    ui.set_cpu_gpu_hint(SharedString::from(gpu_hint));
    let top_cpu = cpu.top_proc.as_ref().map_or_else(
        || "Топ CPU: —".to_owned(),
        |p| format!("Топ CPU: {} ({}%)", p.name, p.cpu_pct),
    );
    ui.set_cpu_top_line(SharedString::from(top_cpu));
}

fn sync_gpu_section(ui: &SysMonitorWindow, d: &SysTelemetry) {
    let gpu = &d.gpu;
    let hz = if gpu.display_hz > 0 {
        format!("Экран: {} Гц", gpu.display_hz)
    } else {
        "GPU".to_owned()
    };
    ui.set_gpu_hz_badge(SharedString::from(hz));
    ui.set_gpu_pct(i32::from(gpu.usage_pct.unwrap_or(0)));
    let load = gpu
        .usage_pct
        .map_or_else(|| "Нагрузка: —".to_owned(), |u| format!("Нагрузка: {u}%"));
    ui.set_gpu_load_text(SharedString::from(load));
    let clk = gpu
        .clock_mhz
        .map_or_else(|| "Частота: —".to_owned(), |m| format!("Частота: {m} МГц"));
    ui.set_gpu_clock_text(SharedString::from(clk));
    let vpct = if gpu.vram_total_mb > 0 {
        ((u64::from(gpu.vram_used_mb) * 100) / u64::from(gpu.vram_total_mb)).min(100) as i32
    } else {
        0
    };
    ui.set_vram_pct(vpct);
    let vlbl = if gpu.vram_total_mb > 0 {
        let u = f64::from(gpu.vram_used_mb) / 1024.0;
        let t = f64::from(gpu.vram_total_mb) / 1024.0;
        format!("VRAM: {u:.1} / {t:.0} ГБ")
    } else {
        "VRAM: —".to_owned()
    };
    ui.set_gpu_vram_text(SharedString::from(vlbl));
    let ts = gpu
        .temp_c
        .map_or_else(|| "—".to_owned(), |t| format!("{t}°C"));
    let fs = gpu
        .fan_pct
        .map_or_else(|| "—".to_owned(), |f| format!("{f}%"));
    let gthr = if gpu.throttled { "Да" } else { "Нет" };
    ui.set_gpu_temp_text(SharedString::from(format!(
        "Темп/Fan: {ts} | {fs} | [THR: {gthr}]"
    )));
}

fn sync_ram_and_drives(ui: &SysMonitorWindow, d: &SysTelemetry) {
    let ram = &d.ram;
    ui.set_ram_pct(i32::from(ram.usage_pct));
    let (ug, tg) = (
        f64::from(ram.used_mb) / 1024.0,
        f64::from(ram.total_mb) / 1024.0,
    );
    let (cg, pg) = (
        f64::from(ram.cache_mb) / 1024.0,
        f64::from(ram.pagefile_used_mb) / 1024.0,
    );
    ui.set_ram_line(SharedString::from(format!(
        "ОЗУ: {}% {ug:.1}/{tg:.0}G (Кэш: {cg:.1}G | PF: {pg:.1}G)",
        ram.usage_pct
    )));
    let top_ram = ram.top_proc.as_ref().map_or_else(
        || "Топ: —".to_owned(),
        |p| format!("Топ: {} ({})", p.name, fmt_mb(p.mem_mb)),
    );
    ui.set_ram_top_line(SharedString::from(top_ram));

    let drives: Vec<SlintSysDriveRow> = d
        .drives
        .iter()
        .take(3)
        .map(|dr| {
            let pct = if dr.busy_pct > 0 {
                dr.busy_pct
            } else if dr.total_gb > 0 {
                let used = u64::from(dr.total_gb.saturating_sub(dr.free_gb));
                ((used * 100) / u64::from(dr.total_gb)).min(100) as u8
            } else {
                0
            };
            let rd = f64::from(dr.read_tenths_mb_s) / 10.0;
            let wr = f64::from(dr.write_tenths_mb_s) / 10.0;
            SlintSysDriveRow {
                label: SharedString::from(format!("Диск {}: {}%", dr.letter, dr.busy_pct)),
                busy_pct: i32::from(pct),
                details: SharedString::from(format!(
                    "Чт {rd:.1} | Зап {wr:.1} МБ/с | Своб. {} ГБ",
                    dr.free_gb
                )),
            }
        })
        .collect();
    ui.set_drives(ModelRc::from(Rc::new(VecModel::from(drives))));
}

fn sync_net_and_procs(ui: &SysMonitorWindow, d: &SysTelemetry) {
    let net = &d.net;
    let ip = if net.ip_v4.is_empty() {
        "IP: —".to_owned()
    } else if net.vpn_active {
        format!("IP: {} [VPN]", net.ip_v4)
    } else {
        format!("IP: {}", net.ip_v4)
    };
    ui.set_net_ip_line(SharedString::from(ip));
    ui.set_net_rx_text(SharedString::from(format!(
        "▼ {:.1} Мбит/с",
        f64::from(net.rx_kbps) / 1000.0
    )));
    ui.set_net_tx_text(SharedString::from(format!(
        "▲ {:.1} Мбит/с",
        f64::from(net.tx_kbps) / 1000.0
    )));
    let gw = net
        .gw_ping_ms
        .map_or_else(|| "—".to_owned(), |m| format!("{m} мс"));
    let inet = net.inet_ping_ms.map_or_else(
        || "—".to_owned(),
        |m| format!("{m} мс ({}% J:{})", net.inet_loss_pct, net.inet_jitter_ms),
    );
    let tmr = if net.timer_res_us > 0 {
        format!("{:.2} мс", f64::from(net.timer_res_us) / 1000.0)
    } else {
        "—".to_owned()
    };
    ui.set_net_latency_line(SharedString::from(format!(
        "GW: {gw} | 1.1.1.1: {inet} | {} мкс | Таймер ОС: {tmr}",
        net.sample_us
    )));

    let mut procs = Vec::new();
    for idx in 0..5 {
        let (cn, cv) = d.top_cpu.get(idx).map_or_else(
            || ("—".to_owned(), String::new()),
            |p| (p.name.clone(), format!("{}%", p.cpu_pct)),
        );
        let (rn, rv) = d.top_ram.get(idx).map_or_else(
            || ("—".to_owned(), String::new()),
            |p| (p.name.clone(), fmt_mb(p.mem_mb)),
        );
        procs.push(SlintSysProcRow {
            cpu_name: SharedString::from(cn),
            cpu_val: SharedString::from(cv),
            ram_name: SharedString::from(rn),
            ram_val: SharedString::from(rv),
        });
    }
    ui.set_top_procs(ModelRc::from(Rc::new(VecModel::from(procs))));
}

fn fmt_mb(mb: u32) -> String {
    if mb >= 1024 {
        format!("{:.1} ГБ", f64::from(mb) / 1024.0)
    } else {
        format!("{mb} МБ")
    }
}
