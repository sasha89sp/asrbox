use std::collections::VecDeque;
use std::ffi::c_void;
use std::mem;
use std::sync::{Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Instant;

use asrbox_platform_api::SysTelemetry;
use windows::core::{s, w, GUID, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, LocalFree, HLOCAL, HWND, LPARAM, WPARAM};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{DISK_PERFORMANCE, IOCTL_DISK_PERFORMANCE};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Power::PowerGetActiveScheme;
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

use crate::shell::encode_wide_str;
use crate::sys_gpu::GpuSampler;
use crate::sys_net::NetSampler;
use crate::sys_procs::ProcAndDiskSampler;

pub(crate) const WM_TELEMETRY_DONE: u32 = WM_APP + 15;

const GUID_MAX_POWER_SAVINGS: GUID = GUID::from_u128(0xa1841308_3541_4fab_bc81_f71556f20b4a);
const GUID_MIN_POWER_SAVINGS: GUID = GUID::from_u128(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);

struct WorkerSignal {
    pending: bool,
    target_hwnd: usize,
}

static SIGNAL: OnceLock<(Mutex<WorkerSignal>, Condvar)> = OnceLock::new();
static RESULTS: OnceLock<Mutex<VecDeque<SysTelemetry>>> = OnceLock::new();

pub(crate) fn request_telemetry_sample(host_hwnd: HWND) {
    let pair = SIGNAL.get_or_init(|| {
        let pair_ref = (
            Mutex::new(WorkerSignal {
                pending: false,
                target_hwnd: host_hwnd.0 as usize,
            }),
            Condvar::new(),
        );
        spawn_worker();
        pair_ref
    });
    if let Ok(mut guard) = pair.0.lock() {
        guard.target_hwnd = host_hwnd.0 as usize;
        if !guard.pending {
            guard.pending = true;
            pair.1.notify_one();
        }
    }
}

pub(crate) fn drain_telemetry_results() -> Vec<SysTelemetry> {
    let queue = RESULTS.get_or_init(|| Mutex::new(VecDeque::new()));
    queue
        .lock()
        .map(|mut g| g.drain(..).collect())
        .unwrap_or_default()
}

fn spawn_worker() {
    thread::spawn(|| {
        let mut procs = ProcAndDiskSampler::new();
        let mut gpu = GpuSampler::new();
        let mut net = NetSampler::new();

        while let Some(pair) = SIGNAL.get() {
            let target_hwnd = {
                let Ok(mut guard) = pair.0.lock() else {
                    break;
                };
                while !guard.pending {
                    let Ok(next) = pair.1.wait(guard) else {
                        return;
                    };
                    guard = next;
                }
                guard.pending = false;
                guard.target_hwnd
            };

            let t0 = Instant::now();
            let mut err_count = 0u32;
            let (cpu, ram, drives, top_cpu, top_ram) = procs.sample(&mut err_count);
            let gpu_stat = gpu.sample();
            let mut net_stat = net.sample();
            net_stat.timer_res_us = query_timer_resolution_us();
            net_stat.sample_us = u32::try_from(t0.elapsed().as_micros()).unwrap_or(0);

            let system_watts = match (cpu.tdp_watts, gpu_stat.power_watts) {
                (Some(c), Some(g)) => Some(c.saturating_add(g)),
                (None, Some(g)) => Some(g),
                (Some(c), None) => Some(c),
                (None, None) => None,
            };
            let snap = SysTelemetry {
                uptime_sec: query_uptime_sec(),
                power_plan: query_power_plan(),
                system_watts,
                err_count,
                cpu,
                gpu: gpu_stat,
                ram,
                drives,
                net: net_stat,
                top_cpu,
                top_ram,
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
                // SAFETY: PostMessageW is thread-safe and delivers WM_TELEMETRY_DONE to host_hwnd.
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_TELEMETRY_DONE, WPARAM(0), LPARAM(0));
                }
            }
        }
    });
}

fn query_uptime_sec() -> u64 {
    // SAFETY: GetTickCount64 has no preconditions and returns milliseconds since boot.
    unsafe { GetTickCount64() / 1000 }
}

fn query_power_plan() -> String {
    let mut ptr: *mut GUID = std::ptr::null_mut();
    // SAFETY: PowerGetActiveScheme allocates a GUID freed immediately via LocalFree.
    unsafe {
        if PowerGetActiveScheme(None, &mut ptr).0 != 0 || ptr.is_null() {
            return "BAL".to_owned();
        }
        let guid = *ptr;
        let _ = LocalFree(Some(HLOCAL(ptr as *mut c_void)));
        if guid == GUID_MIN_POWER_SAVINGS {
            "PERF".to_owned()
        } else if guid == GUID_MAX_POWER_SAVINGS {
            "ECO".to_owned()
        } else {
            "BAL".to_owned()
        }
    }
}

type NtQueryTimerResFn = unsafe extern "system" fn(*mut u32, *mut u32, *mut u32) -> i32;

fn query_timer_resolution_us() -> u32 {
    // SAFETY: Resolving NtQueryTimerResolution from already-loaded ntdll.dll and calling with valid u32 pointers.
    unsafe {
        let Ok(hmod) = GetModuleHandleW(w!("ntdll.dll")) else {
            return 0;
        };
        let Some(proc) = GetProcAddress(hmod, s!("NtQueryTimerResolution")) else {
            return 0;
        };
        let func: NtQueryTimerResFn = std::mem::transmute(proc);
        let (mut min_r, mut max_r, mut cur_r) = (0u32, 0u32, 0u32);
        if func(&mut min_r, &mut max_r, &mut cur_r) == 0 {
            cur_r / 10
        } else {
            0
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct DiskPrev {
    pub bytes_read: i64,
    pub bytes_written: i64,
    pub read_time: i64,
    pub write_time: i64,
}

pub(crate) fn read_disk_perf(letter: char) -> Option<DiskPrev> {
    let path = encode_wide_str(&format!("\\\\.\\{letter}:"));
    // SAFETY: Opening volume handle with 0 access for IOCTL_DISK_PERFORMANCE and closing immediately.
    unsafe {
        let h = CreateFileW(
            PCWSTR(path.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
        .ok()?;
        let mut perf = DISK_PERFORMANCE::default();
        let mut returned = 0u32;
        let ok = DeviceIoControl(
            h,
            IOCTL_DISK_PERFORMANCE,
            None,
            0,
            Some(&mut perf as *mut DISK_PERFORMANCE as *mut c_void),
            u32::try_from(mem::size_of::<DISK_PERFORMANCE>()).unwrap_or(0),
            Some(&mut returned),
            None,
        )
        .is_ok();
        let _ = CloseHandle(h);
        if ok {
            Some(DiskPrev {
                bytes_read: perf.BytesRead,
                bytes_written: perf.BytesWritten,
                read_time: perf.ReadTime,
                write_time: perf.WriteTime,
            })
        } else {
            None
        }
    }
}

pub(crate) fn query_disk_space(root_w: &[u16]) -> (u32, u32) {
    let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: root_w is NUL-terminated UTF-16 and output pointers are valid stack u64s.
    unsafe {
        let _ = GetDiskFreeSpaceExW(
            PCWSTR(root_w.as_ptr()),
            Some(&mut avail),
            Some(&mut total),
            Some(&mut free),
        );
    }
    let gb = 1024u64 * 1024 * 1024;
    ((avail / gb) as u32, (total / gb) as u32)
}
