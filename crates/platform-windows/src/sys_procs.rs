use std::collections::HashMap;
use std::ffi::c_void;
use std::mem;
use std::time::Instant;

use asrbox_platform_api::{CpuTelemetry, DriveTelemetry, ProcUsage, RamTelemetry};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Power::{CallNtPowerInformation, ProcessorInformation};
use windows::Win32::System::ProcessStatus::{
    K32GetPerformanceInfo, K32GetProcessMemoryInfo, PERFORMANCE_INFORMATION,
    PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::SystemInformation::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};
use windows::Win32::System::Threading::{
    GetProcessTimes, GetSystemTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::shell::encode_wide_str;
use crate::sys_telemetry::{query_disk_space, read_disk_perf, DiskPrev};

const DRIVE_FIXED: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ProcessorPowerInfo {
    number: u32,
    max_mhz: u32,
    current_mhz: u32,
    mhz_limit: u32,
    max_idle_state: u32,
    current_idle_state: u32,
}

pub(crate) struct ProcAndDiskSampler {
    logical_cores: u16,
    prev_idle: u64,
    prev_sys_total: u64,
    prev_proc_ticks: HashMap<u32, u64>,
    prev_disks: HashMap<char, DiskPrev>,
    last_sample: Instant,
}

impl ProcAndDiskSampler {
    pub(crate) fn new() -> Self {
        let cores = query_logical_cores();
        let (idle, total) = read_sys_times();
        Self {
            logical_cores: cores,
            prev_idle: idle,
            prev_sys_total: total,
            prev_proc_ticks: HashMap::new(),
            prev_disks: HashMap::new(),
            last_sample: Instant::now(),
        }
    }

    pub(crate) fn sample(
        &mut self,
        err_count: &mut u32,
    ) -> (
        CpuTelemetry,
        RamTelemetry,
        Vec<DriveTelemetry>,
        Vec<ProcUsage>,
        Vec<ProcUsage>,
    ) {
        let elapsed_sec = self.last_sample.elapsed().as_secs_f64().max(0.1);
        self.last_sample = Instant::now();

        let (idle, total) = read_sys_times();
        let d_idle = idle.saturating_sub(self.prev_idle);
        let d_total = total.saturating_sub(self.prev_sys_total);
        self.prev_idle = idle;
        self.prev_sys_total = total;
        let cpu_pct = if d_total > 0 {
            let busy = d_total.saturating_sub(d_idle);
            ((u128::from(busy) * 100) / u128::from(d_total)).min(100) as u8
        } else {
            0
        };

        let (cur_mhz, max_mhz, throttled) = query_cpu_freq(self.logical_cores);
        let (top_cpu, top_ram, total_threads) = self.sample_processes(d_total, err_count);
        let cpu = CpuTelemetry {
            usage_pct: cpu_pct,
            cur_mhz,
            max_mhz,
            logical_cores: self.logical_cores,
            active_threads: total_threads,
            temp_c: None,
            tdp_watts: None,
            fan_rpm: None,
            throttled,
            top_proc: top_cpu.first().cloned(),
        };

        let mut ram = query_ram();
        ram.top_proc = top_ram.first().cloned();
        let drives = self.sample_drives(elapsed_sec);
        (cpu, ram, drives, top_cpu, top_ram)
    }

    fn sample_processes(
        &mut self,
        d_sys_total: u64,
        err_count: &mut u32,
    ) -> (Vec<ProcUsage>, Vec<ProcUsage>, u32) {
        let mut next_ticks: HashMap<u32, u64> = HashMap::new();
        let mut by_name: HashMap<String, (u64, u64)> = HashMap::new();
        let mut total_threads = 0u32;

        // SAFETY: ToolHelp snapshot handle is closed before returning; PROCESSENTRY32W.dwSize is initialized.
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                *err_count = err_count.saturating_add(1);
                return (Vec::new(), Vec::new(), 0);
            };
            let mut pe = PROCESSENTRY32W {
                dwSize: u32::try_from(mem::size_of::<PROCESSENTRY32W>()).unwrap_or(0),
                ..Default::default()
            };
            if Process32FirstW(snap, &mut pe).is_ok() {
                loop {
                    let pid = pe.th32ProcessID;
                    total_threads = total_threads.saturating_add(pe.cntThreads);
                    if pid > 0 {
                        let name = utf16_z_to_string(&pe.szExeFile);
                        if let Some((ticks, mem_bytes)) = read_proc_usage(pid) {
                            let delta = self
                                .prev_proc_ticks
                                .get(&pid)
                                .map_or(0, |prev| ticks.saturating_sub(*prev));
                            next_ticks.insert(pid, ticks);
                            let entry = by_name.entry(name).or_insert((0, 0));
                            entry.0 = entry.0.saturating_add(delta);
                            entry.1 = entry.1.saturating_add(mem_bytes);
                        }
                    }
                    if Process32NextW(snap, &mut pe).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
        }
        self.prev_proc_ticks = next_ticks;
        build_top_lists(by_name, d_sys_total, total_threads)
    }

    fn sample_drives(&mut self, elapsed_sec: f64) -> Vec<DriveTelemetry> {
        // SAFETY: GetLogicalDrives returns a 26-bit drive presence bitmask.
        let mask = unsafe { GetLogicalDrives() };
        let mut drives = Vec::new();
        for bit in 0..26u32 {
            if (mask & (1 << bit)) == 0 {
                continue;
            }
            let letter = (b'A' + bit as u8) as char;
            let root = format!("{letter}:\\");
            let root_w = encode_wide_str(&root);
            // SAFETY: root_w is a NUL-terminated UTF-16 drive root path.
            let dtype = unsafe { GetDriveTypeW(PCWSTR(root_w.as_ptr())) };
            if dtype != DRIVE_FIXED {
                continue;
            }
            let (free_gb, total_gb) = query_disk_space(&root_w);
            let (busy_pct, read_t, write_t) = self.query_disk_io(letter, elapsed_sec);
            drives.push(DriveTelemetry {
                letter,
                busy_pct,
                read_tenths_mb_s: read_t,
                write_tenths_mb_s: write_t,
                free_gb,
                total_gb,
                temp_c: None,
            });
            if drives.len() >= 3 {
                break;
            }
        }
        drives
    }

    fn query_disk_io(&mut self, letter: char, elapsed_sec: f64) -> (u8, u32, u32) {
        let Some(cur) = read_disk_perf(letter) else {
            return (0, 0, 0);
        };
        let prev = self.prev_disks.insert(letter, cur);
        let Some(p) = prev else {
            return (0, 0, 0);
        };
        let d_read = cur.bytes_read.saturating_sub(p.bytes_read).max(0) as f64;
        let d_write = cur.bytes_written.saturating_sub(p.bytes_written).max(0) as f64;
        let d_time = cur
            .read_time
            .saturating_sub(p.read_time)
            .saturating_add(cur.write_time.saturating_sub(p.write_time))
            .max(0) as f64;
        let read_tenths = ((d_read / (1024.0 * 1024.0) / elapsed_sec) * 10.0).round() as u32;
        let write_tenths = ((d_write / (1024.0 * 1024.0) / elapsed_sec) * 10.0).round() as u32;
        let busy_pct = ((d_time / (elapsed_sec * 10_000_000.0)) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u8;
        (busy_pct, read_tenths, write_tenths)
    }
}

fn build_top_lists(
    by_name: HashMap<String, (u64, u64)>,
    d_sys_total: u64,
    total_threads: u32,
) -> (Vec<ProcUsage>, Vec<ProcUsage>, u32) {
    let mut rows: Vec<ProcUsage> = by_name
        .into_iter()
        .map(|(name, (d_ticks, mem_b))| {
            let cpu_pct = if d_sys_total > 0 {
                ((u128::from(d_ticks) * 100) / u128::from(d_sys_total)).min(100) as u8
            } else {
                0
            };
            ProcUsage {
                name,
                cpu_pct,
                mem_mb: (mem_b / (1024 * 1024)) as u32,
            }
        })
        .collect();

    rows.sort_by(|a, b| {
        b.cpu_pct
            .cmp(&a.cpu_pct)
            .then_with(|| b.mem_mb.cmp(&a.mem_mb))
    });
    let top_cpu: Vec<ProcUsage> = rows.iter().take(5).cloned().collect();

    rows.sort_by(|a, b| {
        b.mem_mb
            .cmp(&a.mem_mb)
            .then_with(|| b.cpu_pct.cmp(&a.cpu_pct))
    });
    let top_ram: Vec<ProcUsage> = rows.into_iter().take(5).collect();
    (top_cpu, top_ram, total_threads)
}

unsafe fn read_proc_usage(pid: u32) -> Option<(u64, u64)> {
    let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let (mut c, mut e, mut k, mut u) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    let ticks = if GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u).is_ok() {
        ft_u64(k).saturating_add(ft_u64(u))
    } else {
        0
    };
    let mut pmc = PROCESS_MEMORY_COUNTERS {
        cb: u32::try_from(mem::size_of::<PROCESS_MEMORY_COUNTERS>()).unwrap_or(0),
        ..Default::default()
    };
    let mem_b = if K32GetProcessMemoryInfo(h, &mut pmc, pmc.cb).as_bool() {
        pmc.WorkingSetSize as u64
    } else {
        0
    };
    let _ = CloseHandle(h);
    Some((ticks, mem_b))
}

fn query_logical_cores() -> u16 {
    let mut si = SYSTEM_INFO::default();
    // SAFETY: GetSystemInfo writes to valid stack SYSTEM_INFO.
    unsafe {
        GetSystemInfo(&mut si);
    }
    u16::try_from(si.dwNumberOfProcessors.max(1)).unwrap_or(1)
}

fn query_cpu_freq(cores: u16) -> (u32, u32, bool) {
    let mut infos = vec![ProcessorPowerInfo::default(); usize::from(cores.max(1))];
    let byte_len = u32::try_from(infos.len() * mem::size_of::<ProcessorPowerInfo>()).unwrap_or(0);
    // SAFETY: Output buffer length matches `infos` array of PROCESSOR_POWER_INFORMATION structs.
    let status = unsafe {
        CallNtPowerInformation(
            ProcessorInformation,
            None,
            0,
            Some(infos.as_mut_ptr() as *mut c_void),
            byte_len,
        )
    };
    if status.0 != 0 || infos.is_empty() {
        return (0, 0, false);
    }
    let mut cur_sum = 0u64;
    let mut max_mhz = 0u32;
    let mut throttled = false;
    for info in &infos {
        cur_sum += u64::from(info.current_mhz);
        max_mhz = max_mhz.max(info.max_mhz);
        if info.mhz_limit > 0 && info.max_mhz > 0 && info.mhz_limit + 250 < info.max_mhz {
            throttled = true;
        }
    }
    let cur_avg = (cur_sum / infos.len() as u64) as u32;
    (cur_avg, max_mhz, throttled)
}

fn query_ram() -> RamTelemetry {
    let mut ms = MEMORYSTATUSEX {
        dwLength: u32::try_from(mem::size_of::<MEMORYSTATUSEX>()).unwrap_or(0),
        ..Default::default()
    };
    let mut pi = PERFORMANCE_INFORMATION {
        cb: u32::try_from(mem::size_of::<PERFORMANCE_INFORMATION>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: GlobalMemoryStatusEx and K32GetPerformanceInfo write to valid initialized structs.
    unsafe {
        let _ = GlobalMemoryStatusEx(&mut ms);
        let _ = K32GetPerformanceInfo(&mut pi, pi.cb);
    }
    let mb = 1024u64 * 1024;
    let total_mb = (ms.ullTotalPhys / mb) as u32;
    let avail_mb = (ms.ullAvailPhys / mb) as u32;
    let used_mb = total_mb.saturating_sub(avail_mb);
    let page_sz = pi.PageSize as u64;
    let cache_mb = ((pi.SystemCache as u64).saturating_mul(page_sz) / mb) as u32;
    let commit_mb = ((pi.CommitTotal as u64).saturating_mul(page_sz) / mb) as u32;
    RamTelemetry {
        usage_pct: ms.dwMemoryLoad.min(100) as u8,
        used_mb,
        total_mb,
        cache_mb,
        pagefile_used_mb: commit_mb.saturating_sub(used_mb),
        top_proc: None,
    }
}

fn read_sys_times() -> (u64, u64) {
    let (mut idle, mut kern, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: GetSystemTimes writes to valid stack FILETIME structs.
    unsafe {
        let _ = GetSystemTimes(Some(&mut idle), Some(&mut kern), Some(&mut user));
    }
    let i = ft_u64(idle);
    let total = ft_u64(kern).saturating_add(ft_u64(user));
    (i, total)
}

fn ft_u64(ft: FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

fn utf16_z_to_string(slice: &[u16]) -> String {
    let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..end])
}
