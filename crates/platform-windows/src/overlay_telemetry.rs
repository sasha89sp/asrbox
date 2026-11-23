use std::collections::VecDeque;
use std::ffi::c_void;
use std::mem;
use std::sync::{Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Instant;

use asrbox_platform_api::OverlayTelemetry;
use windows::Win32::Foundation::{FILETIME, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetCompositionTimingInfo, DWM_TIMING_INFO};
use windows::Win32::NetworkManagement::IpHelper::{
    IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

use crate::overlay_game::detect_foreground_game;
use crate::sys_gpu::GpuSampler;

pub(crate) const WM_OVERLAY_DONE: u32 = WM_APP + 17;

const PING_HISTORY_LEN: usize = 40;
const PING_INTERVAL_MS: u128 = 2000;
const FPS_HISTORY_LEN: usize = 60;

struct OverlaySignal {
    pending: bool,
    full_sample: bool,
    extra_games: Vec<String>,
    target_hwnd: usize,
}

static SIGNAL: OnceLock<(Mutex<OverlaySignal>, Condvar)> = OnceLock::new();
static RESULTS: OnceLock<Mutex<VecDeque<OverlayTelemetry>>> = OnceLock::new();

pub(crate) fn request_overlay_sample(host_hwnd: HWND, full_sample: bool, extra_games: &[String]) {
    let pair = SIGNAL.get_or_init(|| {
        let state = (
            Mutex::new(OverlaySignal {
                pending: false,
                full_sample,
                extra_games: extra_games.to_vec(),
                target_hwnd: host_hwnd.0 as usize,
            }),
            Condvar::new(),
        );
        spawn_overlay_worker();
        state
    });
    if let Ok(mut g) = pair.0.lock() {
        g.target_hwnd = host_hwnd.0 as usize;
        g.full_sample = g.full_sample || full_sample;
        if g.extra_games != extra_games {
            g.extra_games = extra_games.to_vec();
        }
        if !g.pending {
            g.pending = true;
            pair.1.notify_one();
        }
    }
}

pub(crate) fn drain_overlay_results() -> Vec<OverlayTelemetry> {
    let queue = RESULTS.get_or_init(|| Mutex::new(VecDeque::new()));
    queue
        .lock()
        .map(|mut g| g.drain(..).collect())
        .unwrap_or_default()
}

fn spawn_overlay_worker() {
    thread::spawn(|| {
        let mut gpu = GpuSampler::new();
        let mut cpu = LightCpuSampler::new();
        let mut fps = DwmFpsSampler::new();
        let mut net = LightNetSampler::new();

        while let Some(pair) = SIGNAL.get() {
            let (target_hwnd, want_full, extra_games) = {
                let Ok(mut g) = pair.0.lock() else {
                    break;
                };
                while !g.pending {
                    let Ok(next) = pair.1.wait(g) else {
                        return;
                    };
                    g = next;
                }
                g.pending = false;
                let full = g.full_sample;
                g.full_sample = false;
                (g.target_hwnd, full, g.extra_games.clone())
            };

            let game = detect_foreground_game(&extra_games);
            let do_full = want_full || game.active;
            let snap = if do_full {
                let gstat = gpu.sample();
                let (cur_fps, low_1pct) = fps.sample();
                let cpu_pct = cpu.sample();
                let (ram_used, ram_total, ram_pct) = query_ram_light();
                let (ping_ms, loss_tenths) = net.sample();
                OverlayTelemetry {
                    fps: cur_fps,
                    fps_low_1pct: low_1pct,
                    display_hz: gstat.display_hz,
                    gpu_temp_c: gstat.temp_c,
                    gpu_usage_pct: gstat.usage_pct,
                    vram_used_mb: gstat.vram_used_mb,
                    vram_total_mb: gstat.vram_total_mb,
                    cpu_temp_c: None,
                    cpu_usage_pct: cpu_pct,
                    ram_used_mb: ram_used,
                    ram_total_mb: ram_total,
                    ram_usage_pct: ram_pct,
                    net_ping_ms: ping_ms,
                    net_loss_tenths_pct: loss_tenths,
                    game_active: game.active,
                    game_process: game.process_name,
                    monitor_rect: game.monitor_rect,
                }
            } else {
                fps.reset();
                OverlayTelemetry {
                    game_active: game.active,
                    game_process: game.process_name,
                    monitor_rect: game.monitor_rect,
                    ..OverlayTelemetry::default()
                }
            };

            let queue = RESULTS.get_or_init(|| Mutex::new(VecDeque::new()));
            if let Ok(mut q) = queue.lock() {
                if q.len() >= 2 {
                    q.pop_front();
                }
                q.push_back(snap);
            }
            if target_hwnd != 0 {
                let hwnd = HWND(target_hwnd as *mut c_void);
                // SAFETY: PostMessageW is thread-safe and delivers WM_OVERLAY_DONE to host_hwnd.
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_OVERLAY_DONE, WPARAM(0), LPARAM(0));
                }
            }
        }
    });
}

struct LightCpuSampler {
    prev_idle: u64,
    prev_total: u64,
}

impl LightCpuSampler {
    fn new() -> Self {
        let (i, t) = read_sys_times();
        Self {
            prev_idle: i,
            prev_total: t,
        }
    }

    fn sample(&mut self) -> u8 {
        let (idle, total) = read_sys_times();
        let d_idle = idle.saturating_sub(self.prev_idle);
        let d_total = total.saturating_sub(self.prev_total);
        self.prev_idle = idle;
        self.prev_total = total;
        if d_total == 0 {
            return 0;
        }
        let busy = d_total.saturating_sub(d_idle);
        ((u128::from(busy) * 100) / u128::from(d_total)).min(100) as u8
    }
}

struct DwmFpsSampler {
    prev_frames: Option<u64>,
    prev_ts: Instant,
    history: VecDeque<u32>,
}

impl DwmFpsSampler {
    fn new() -> Self {
        Self {
            prev_frames: None,
            prev_ts: Instant::now(),
            history: VecDeque::with_capacity(FPS_HISTORY_LEN),
        }
    }

    fn reset(&mut self) {
        self.prev_frames = None;
        self.history.clear();
    }

    fn sample(&mut self) -> (Option<u32>, Option<u32>) {
        let mut info = DWM_TIMING_INFO {
            cbSize: u32::try_from(mem::size_of::<DWM_TIMING_INFO>()).unwrap_or(0),
            ..Default::default()
        };
        // SAFETY: Passing properly sized DWM_TIMING_INFO to read-only DwmGetCompositionTimingInfo.
        let ok = unsafe { DwmGetCompositionTimingInfo(HWND::default(), &mut info).is_ok() };
        if !ok {
            return (None, None);
        }
        let frames = info.cFramesDisplayed;
        let now = Instant::now();
        let dt = now.duration_since(self.prev_ts).as_secs_f64();
        let prev = self.prev_frames.replace(frames);
        self.prev_ts = now;

        let Some(prev_f) = prev else {
            return (None, None);
        };
        if dt < 0.05 || frames <= prev_f {
            return (None, None);
        }
        let cur_fps = (((frames - prev_f) as f64) / dt).round().clamp(1.0, 999.0) as u32;
        if self.history.len() >= FPS_HISTORY_LEN {
            self.history.pop_front();
        }
        self.history.push_back(cur_fps);

        let mut sorted: Vec<u32> = self.history.iter().copied().collect();
        sorted.sort_unstable();
        let low = sorted.first().copied();
        (Some(cur_fps), low)
    }
}

struct LightNetSampler {
    last_ping_ts: Option<Instant>,
    history: VecDeque<Option<u16>>,
}

impl LightNetSampler {
    fn new() -> Self {
        Self {
            last_ping_ts: None,
            history: VecDeque::with_capacity(PING_HISTORY_LEN),
        }
    }

    fn sample(&mut self) -> (Option<u16>, u16) {
        let due = self
            .last_ping_ts
            .is_none_or(|t| t.elapsed().as_millis() >= PING_INTERVAL_MS);
        if due {
            self.last_ping_ts = Some(Instant::now());
            let ip = u32::from_ne_bytes([1, 1, 1, 1]);
            let rtt = icmp_ping_ipv4(ip, 250);
            if self.history.len() >= PING_HISTORY_LEN {
                self.history.pop_front();
            }
            self.history.push_back(rtt);
        }
        if self.history.is_empty() {
            return (None, 0);
        }
        let last_ok = self.history.iter().rev().copied().flatten().next();
        let lost = self.history.iter().filter(|v| v.is_none()).count();
        let loss_tenths = ((lost * 1000) / self.history.len()).min(1000) as u16;
        (last_ok, loss_tenths)
    }
}

fn query_ram_light() -> (u32, u32, u8) {
    let mut ms = MEMORYSTATUSEX {
        dwLength: u32::try_from(mem::size_of::<MEMORYSTATUSEX>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: GlobalMemoryStatusEx writes to initialized stack MEMORYSTATUSEX.
    unsafe {
        let _ = GlobalMemoryStatusEx(&mut ms);
    }
    let mb = 1024u64 * 1024;
    let total = (ms.ullTotalPhys / mb) as u32;
    let avail = (ms.ullAvailPhys / mb) as u32;
    (
        total.saturating_sub(avail),
        total,
        ms.dwMemoryLoad.min(100) as u8,
    )
}

fn read_sys_times() -> (u64, u64) {
    let (mut idle, mut kern, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: GetSystemTimes writes into valid stack FILETIME values.
    unsafe {
        let _ = GetSystemTimes(Some(&mut idle), Some(&mut kern), Some(&mut user));
    }
    let i = ft_u64(idle);
    (i, ft_u64(kern).saturating_add(ft_u64(user)))
}

fn ft_u64(ft: FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

fn icmp_ping_ipv4(dest_ip: u32, timeout_ms: u32) -> Option<u16> {
    // SAFETY: Opening ICMP handle, sending 8-byte echo into sized reply buffer, closing handle.
    unsafe {
        let h = IcmpCreateFile().ok()?;
        let payload = [0x61u8; 8];
        let reply_size = mem::size_of::<ICMP_ECHO_REPLY>() + payload.len() + 16;
        let mut reply_buf = vec![0u8; reply_size];
        let count = IcmpSendEcho(
            h,
            dest_ip,
            payload.as_ptr() as *const c_void,
            payload.len() as u16,
            None,
            reply_buf.as_mut_ptr() as *mut c_void,
            reply_buf.len() as u32,
            timeout_ms,
        );
        let _ = IcmpCloseHandle(h);
        if count == 0 {
            return None;
        }
        let reply = &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY);
        if reply.Status == 0 {
            u16::try_from(reply.RoundTripTime).ok()
        } else {
            None
        }
    }
}
