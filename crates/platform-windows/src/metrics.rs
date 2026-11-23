use std::mem;
use std::time::Instant;

use asrbox_platform_api::ProcessMetrics;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetGuiResources, GetProcessTimes, GetSystemTimes, GR_GDIOBJECTS,
    GR_USEROBJECTS,
};

pub(crate) struct ProcessMetricsSampler {
    started_at: Instant,
    prev_proc_ticks: u64,
    prev_sys_ticks: u64,
}

impl ProcessMetricsSampler {
    pub(crate) fn new() -> Self {
        let (proc_t, sys_t) = read_cpu_ticks();
        Self {
            started_at: Instant::now(),
            prev_proc_ticks: proc_t,
            prev_sys_ticks: sys_t,
        }
    }

    pub(crate) fn sample(&mut self) -> ProcessMetrics {
        let (proc_t, sys_t) = read_cpu_ticks();
        let d_proc = proc_t.saturating_sub(self.prev_proc_ticks);
        let d_sys = sys_t.saturating_sub(self.prev_sys_ticks);
        self.prev_proc_ticks = proc_t;
        self.prev_sys_ticks = sys_t;

        let cpu_tenths_pct = if d_sys > 0 {
            let tenths = (u128::from(d_proc).saturating_mul(1000)) / u128::from(d_sys);
            u32::try_from(tenths.min(1000)).unwrap_or(1000)
        } else {
            0
        };

        let (ram_kb, gdi_handles, user_handles) = read_mem_and_handles();
        ProcessMetrics {
            ram_kb,
            cpu_tenths_pct,
            gdi_handles,
            user_handles,
            uptime_sec: self.started_at.elapsed().as_secs(),
        }
    }
}

fn read_cpu_ticks() -> (u64, u64) {
    let mut create = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut p_kern = FILETIME::default();
    let mut p_user = FILETIME::default();
    let mut s_idle = FILETIME::default();
    let mut s_kern = FILETIME::default();
    let mut s_user = FILETIME::default();
    // SAFETY: Querying process and system CPU times with valid stack FILETIME structs.
    unsafe {
        let proc = GetCurrentProcess();
        let _ = GetProcessTimes(proc, &mut create, &mut exit, &mut p_kern, &mut p_user);
        let _ = GetSystemTimes(Some(&mut s_idle), Some(&mut s_kern), Some(&mut s_user));
    }
    let proc_total = ft_to_u64(p_kern).saturating_add(ft_to_u64(p_user));
    let sys_total = ft_to_u64(s_kern).saturating_add(ft_to_u64(s_user));
    (proc_total, sys_total)
}

fn read_mem_and_handles() -> (u64, u32, u32) {
    let mut pmc = PROCESS_MEMORY_COUNTERS {
        cb: u32::try_from(mem::size_of::<PROCESS_MEMORY_COUNTERS>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: Reading current process working set and GUI handle counts.
    unsafe {
        let proc = GetCurrentProcess();
        let ram_kb = if K32GetProcessMemoryInfo(proc, &mut pmc, pmc.cb).as_bool() {
            (pmc.WorkingSetSize as u64) / 1024
        } else {
            0
        };
        let gdi = GetGuiResources(proc, GR_GDIOBJECTS);
        let user = GetGuiResources(proc, GR_USEROBJECTS);
        (ram_kb, gdi, user)
    }
}

fn ft_to_u64(ft: FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}
