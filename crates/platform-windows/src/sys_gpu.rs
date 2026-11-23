use std::ffi::c_void;
use std::mem;

use asrbox_platform_api::GpuTelemetry;
use windows::core::{s, w, Interface};
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter3, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
    DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
};
use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

#[repr(C)]
#[derive(Default)]
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}

#[repr(C)]
#[derive(Default)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

type NvmlInitFn = unsafe extern "C" fn() -> i32;
type NvmlShutdownFn = unsafe extern "C" fn() -> i32;
type NvmlGetHandleFn = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
type NvmlGetUtilFn = unsafe extern "C" fn(*mut c_void, *mut NvmlUtilization) -> i32;
type NvmlGetMemFn = unsafe extern "C" fn(*mut c_void, *mut NvmlMemory) -> i32;
type NvmlGetTempFn = unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32;
type NvmlGetFanFn = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;
type NvmlGetClockFn = unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32;
type NvmlGetPowerFn = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;
type NvmlGetThrottleFn = unsafe extern "C" fn(*mut c_void, *mut u64) -> i32;

struct NvmlHandle {
    module: HMODULE,
    device: *mut c_void,
    shutdown: Option<NvmlShutdownFn>,
    get_util: Option<NvmlGetUtilFn>,
    get_mem: Option<NvmlGetMemFn>,
    get_temp: Option<NvmlGetTempFn>,
    get_fan: Option<NvmlGetFanFn>,
    get_clock: Option<NvmlGetClockFn>,
    get_power: Option<NvmlGetPowerFn>,
    get_throttle: Option<NvmlGetThrottleFn>,
}

impl NvmlHandle {
    fn try_load() -> Option<Self> {
        // SAFETY: Dynamically loading optional nvml.dll from NVIDIA driver if present.
        unsafe {
            let module = LoadLibraryW(w!("nvml.dll")).ok()?;
            let init: NvmlInitFn = load_sym(module, s!("nvmlInit_v2"))?;
            let get_handle: NvmlGetHandleFn =
                load_sym(module, s!("nvmlDeviceGetHandleByIndex_v2"))?;
            if init() != 0 {
                let _ = FreeLibrary(module);
                return None;
            }
            let mut device: *mut c_void = std::ptr::null_mut();
            if get_handle(0, &mut device) != 0 || device.is_null() {
                if let Some(sh) = load_sym::<NvmlShutdownFn>(module, s!("nvmlShutdown")) {
                    let _ = sh();
                }
                let _ = FreeLibrary(module);
                return None;
            }
            Some(Self {
                module,
                device,
                shutdown: load_sym(module, s!("nvmlShutdown")),
                get_util: load_sym(module, s!("nvmlDeviceGetUtilizationRates")),
                get_mem: load_sym(module, s!("nvmlDeviceGetMemoryInfo")),
                get_temp: load_sym(module, s!("nvmlDeviceGetTemperature")),
                get_fan: load_sym(module, s!("nvmlDeviceGetFanSpeed")),
                get_clock: load_sym(module, s!("nvmlDeviceGetClockInfo")),
                get_power: load_sym(module, s!("nvmlDeviceGetPowerUsage")),
                get_throttle: load_sym(module, s!("nvmlDeviceGetCurrentClocksThrottleReasons")),
            })
        }
    }

    fn fill_stats(&self, out: &mut GpuTelemetry) {
        // SAFETY: All function pointers were resolved from live `self.module` for valid `self.device`.
        unsafe {
            if let Some(f) = self.get_util {
                let mut u = NvmlUtilization::default();
                if f(self.device, &mut u) == 0 {
                    out.usage_pct = Some(u.gpu.min(100) as u8);
                }
            }
            if let Some(f) = self.get_mem {
                let mut m = NvmlMemory::default();
                if f(self.device, &mut m) == 0 && m.total > 0 {
                    let mb = 1024u64 * 1024;
                    out.vram_used_mb = (m.used / mb) as u32;
                    out.vram_total_mb = (m.total / mb) as u32;
                }
            }
            if let Some(f) = self.get_temp {
                let mut t = 0u32;
                if f(self.device, 0, &mut t) == 0 {
                    out.temp_c = i16::try_from(t).ok();
                }
            }
            if let Some(f) = self.get_fan {
                let mut fan = 0u32;
                if f(self.device, &mut fan) == 0 {
                    out.fan_pct = Some(fan.min(100) as u8);
                }
            }
            if let Some(f) = self.get_clock {
                let mut clk = 0u32;
                if f(self.device, 0, &mut clk) == 0 {
                    out.clock_mhz = Some(clk);
                }
            }
            if let Some(f) = self.get_power {
                let mut mw = 0u32;
                if f(self.device, &mut mw) == 0 {
                    out.power_watts = u16::try_from((mw + 500) / 1000).ok();
                }
            }
            if let Some(f) = self.get_throttle {
                let mut reasons = 0u64;
                if f(self.device, &mut reasons) == 0 {
                    // Bits 0x20 (HW slowdown), 0x40 (HW thermal), 0x80 (HW power brake).
                    out.throttled = (reasons & 0xE0) != 0;
                }
            }
        }
    }
}

impl Drop for NvmlHandle {
    fn drop(&mut self) {
        // SAFETY: Shutting down NVML and unloading nvml.dll once on Drop.
        unsafe {
            if let Some(sh) = self.shutdown {
                let _ = sh();
            }
            let _ = FreeLibrary(self.module);
        }
    }
}

unsafe fn load_sym<T: Copy>(module: HMODULE, name: windows::core::PCSTR) -> Option<T> {
    let raw = GetProcAddress(module, name)?;
    Some(mem::transmute_copy(&raw))
}

pub(crate) struct GpuSampler {
    nvml: Option<NvmlHandle>,
}

impl GpuSampler {
    pub(crate) fn new() -> Self {
        Self {
            nvml: NvmlHandle::try_load(),
        }
    }

    pub(crate) fn sample(&mut self) -> GpuTelemetry {
        let mut out = query_dxgi_gpu();
        out.display_hz = query_display_hz();
        if let Some(nv) = &self.nvml {
            nv.fill_stats(&mut out);
        }
        out
    }
}

fn query_display_hz() -> u32 {
    let mut dm = DEVMODEW {
        dmSize: u16::try_from(mem::size_of::<DEVMODEW>()).unwrap_or(0),
        ..Default::default()
    };
    // SAFETY: EnumDisplaySettingsW writes current display mode into valid DEVMODEW.
    unsafe {
        if EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
            dm.dmDisplayFrequency
        } else {
            0
        }
    }
}

fn query_dxgi_gpu() -> GpuTelemetry {
    // SAFETY: Creating a local DXGI factory and enumerating hardware adapters on background thread.
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else {
            return GpuTelemetry::default();
        };
        let mut idx = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(idx) {
            idx += 1;
            let Ok(desc) = adapter.GetDesc1() else {
                continue;
            };
            if (desc.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32)) != 0 {
                continue;
            }
            let mb = 1024u64 * 1024;
            let total_mb = (desc.DedicatedVideoMemory as u64 / mb) as u32;
            let mut used_mb = 0u32;
            if let Ok(a3) = adapter.cast::<IDXGIAdapter3>() {
                let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                if a3
                    .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info)
                    .is_ok()
                {
                    used_mb = (info.CurrentUsage / mb) as u32;
                }
            }
            let end = desc
                .Description
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(desc.Description.len());
            return GpuTelemetry {
                name: String::from_utf16_lossy(&desc.Description[..end]),
                vram_used_mb: used_mb,
                vram_total_mb: total_mb,
                ..Default::default()
            };
        }
        GpuTelemetry::default()
    }
}
