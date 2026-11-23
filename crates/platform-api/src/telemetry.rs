#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetSizeBounds {
    pub min_w: u32,
    pub min_h: u32,
    pub max_w: u32,
    pub max_h: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProcUsage {
    pub name: String,
    pub cpu_pct: u8,
    pub mem_mb: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DriveTelemetry {
    pub letter: char,
    pub busy_pct: u8,
    pub read_tenths_mb_s: u32,
    pub write_tenths_mb_s: u32,
    pub free_gb: u32,
    pub total_gb: u32,
    pub temp_c: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CpuTelemetry {
    pub usage_pct: u8,
    pub cur_mhz: u32,
    pub max_mhz: u32,
    pub logical_cores: u16,
    pub active_threads: u32,
    pub temp_c: Option<i16>,
    pub tdp_watts: Option<u16>,
    pub fan_rpm: Option<u32>,
    pub throttled: bool,
    pub top_proc: Option<ProcUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GpuTelemetry {
    pub name: String,
    pub usage_pct: Option<u8>,
    pub clock_mhz: Option<u32>,
    pub vram_used_mb: u32,
    pub vram_total_mb: u32,
    pub temp_c: Option<i16>,
    pub fan_pct: Option<u8>,
    pub fan_rpm: Option<u32>,
    pub power_watts: Option<u16>,
    pub throttled: bool,
    pub display_hz: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RamTelemetry {
    pub usage_pct: u8,
    pub used_mb: u32,
    pub total_mb: u32,
    pub cache_mb: u32,
    pub pagefile_used_mb: u32,
    pub top_proc: Option<ProcUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetTelemetry {
    pub ip_v4: String,
    pub vpn_active: bool,
    pub rx_kbps: u32,
    pub tx_kbps: u32,
    pub gw_ping_ms: Option<u16>,
    pub inet_ping_ms: Option<u16>,
    pub inet_loss_pct: u8,
    pub inet_jitter_ms: u16,
    pub timer_res_us: u32,
    pub sample_us: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SysTelemetry {
    pub uptime_sec: u64,
    pub power_plan: String,
    pub system_watts: Option<u16>,
    pub err_count: u32,
    pub cpu: CpuTelemetry,
    pub gpu: GpuTelemetry,
    pub ram: RamTelemetry,
    pub drives: Vec<DriveTelemetry>,
    pub net: NetTelemetry,
    pub top_cpu: Vec<ProcUsage>,
    pub top_ram: Vec<ProcUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverlayTelemetry {
    pub fps: Option<u32>,
    pub fps_low_1pct: Option<u32>,
    pub display_hz: u32,
    pub gpu_temp_c: Option<i16>,
    pub gpu_usage_pct: Option<u8>,
    pub vram_used_mb: u32,
    pub vram_total_mb: u32,
    pub cpu_temp_c: Option<i16>,
    pub cpu_usage_pct: u8,
    pub ram_used_mb: u32,
    pub ram_total_mb: u32,
    pub ram_usage_pct: u8,
    pub net_ping_ms: Option<u16>,
    pub net_loss_tenths_pct: u16,
    pub game_active: bool,
    pub game_process: String,
    pub monitor_rect: Option<crate::PixelRect>,
}
