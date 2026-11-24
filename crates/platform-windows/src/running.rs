use std::collections::HashSet;
use std::path::{Path, PathBuf};

use windows::core::{Interface, BOOL, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, STGM_READ,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowLongW,
    GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow,
    ShowWindow, ASFW_ANY, GWL_EXSTYLE, SW_RESTORE, SW_SHOW, WS_EX_TOOLWINDOW,
};

use crate::shell::{encode_wide_str, launch_path};

pub(crate) fn resolve_target_exe_name(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if ext == "exe" {
        return path
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase());
    }
    if ext == "lnk" {
        let resolved = resolve_lnk_target(path)?;
        let res_ext = resolved.extension()?.to_string_lossy().to_ascii_lowercase();
        if res_ext == "exe" {
            return resolved
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase());
        }
    }
    None
}

pub(crate) fn query_running_exe_names() -> HashSet<String> {
    let mut names = HashSet::new();
    let ptr = &mut names as *mut HashSet<String> as isize;
    // SAFETY: EnumWindows invokes callback synchronously while `names` is alive on the stack.
    unsafe {
        let _ = EnumWindows(Some(collect_running_cb), LPARAM(ptr));
    }
    names
}

pub(crate) fn activate_or_launch_target(path: &Path, focus_running: bool) -> Result<bool, String> {
    if focus_running {
        if let Some(exe) = resolve_target_exe_name(path) {
            if let Some(hwnd) = find_window_by_exe(&exe) {
                focus_existing_window(hwnd);
                return Ok(true);
            }
        }
    }
    launch_path(path)?;
    Ok(false)
}

fn focus_existing_window(hwnd: HWND) {
    // SAFETY: Restoring and bringing an existing application window to foreground from a WS_EX_NOACTIVATE dock.
    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        let _ = BringWindowToTop(hwnd);
        if SetForegroundWindow(hwnd).as_bool() {
            return;
        }
        let fg = GetForegroundWindow();
        let fg_tid = if !fg.is_invalid() {
            GetWindowThreadProcessId(fg, None)
        } else {
            0
        };
        let cur_tid = GetCurrentThreadId();
        let tgt_tid = GetWindowThreadProcessId(hwnd, None);
        let attach_fg = fg_tid != 0 && fg_tid != cur_tid;
        let attach_tgt = tgt_tid != 0 && tgt_tid != cur_tid && tgt_tid != fg_tid;
        if attach_fg {
            let _ = AttachThreadInput(cur_tid, fg_tid, true);
        }
        if attach_tgt {
            let _ = AttachThreadInput(cur_tid, tgt_tid, true);
        }
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if attach_tgt {
            let _ = AttachThreadInput(cur_tid, tgt_tid, false);
        }
        if attach_fg {
            let _ = AttachThreadInput(cur_tid, fg_tid, false);
        }
    }
}

struct FindWindowCtx<'a> {
    target_exe: &'a str,
    found: Option<HWND>,
}

fn find_window_by_exe(target_exe: &str) -> Option<HWND> {
    let mut ctx = FindWindowCtx {
        target_exe,
        found: None,
    };
    let ptr = &mut ctx as *mut FindWindowCtx<'_> as isize;
    // SAFETY: EnumWindows executes synchronously while `ctx` stays pinned on the stack.
    unsafe {
        let _ = EnumWindows(Some(find_window_cb), LPARAM(ptr));
    }
    ctx.found
}

unsafe extern "system" fn collect_running_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !is_app_window(hwnd) {
        return BOOL(1);
    }
    if let Some(exe) = hwnd_exe_name(hwnd) {
        let set = &mut *(lparam.0 as *mut HashSet<String>);
        set.insert(exe);
    }
    BOOL(1)
}

unsafe extern "system" fn find_window_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !is_app_window(hwnd) {
        return BOOL(1);
    }
    let ctx = &mut *(lparam.0 as *mut FindWindowCtx<'_>);
    if let Some(exe) = hwnd_exe_name(hwnd) {
        if exe == ctx.target_exe {
            ctx.found = Some(hwnd);
            return BOOL(0);
        }
    }
    BOOL(1)
}

unsafe fn is_app_window(hwnd: HWND) -> bool {
    if !IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() {
        return false;
    }
    let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    if (ex & WS_EX_TOOLWINDOW.0) != 0 {
        return false;
    }
    GetWindowTextLengthW(hwnd) > 0
}

unsafe fn hwnd_exe_name(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return None;
    }
    let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buf = [0u16; 520];
    let mut len = u32::try_from(buf.len()).unwrap_or(520);
    let ok =
        QueryFullProcessImageNameW(proc, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
            .is_ok();
    let _ = CloseHandle(proc);
    if !ok || len == 0 {
        return None;
    }
    let full = String::from_utf16_lossy(&buf[..len as usize]);
    Path::new(&full)
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
}

fn resolve_lnk_target(lnk_path: &Path) -> Option<PathBuf> {
    let wide = encode_wide_str(&lnk_path.to_string_lossy());
    // SAFETY: Standard COM IShellLinkW resolution with balanced CoInitializeEx/CoUninitialize.
    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let need_uninit = init.is_ok();
        let result = (|| -> Option<PathBuf> {
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
            let persist: IPersistFile = link.cast().ok()?;
            persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;
            let mut out = [0u16; 520];
            link.GetPath(&mut out, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
                .ok()?;
            let end = out.iter().position(|&c| c == 0).unwrap_or(out.len());
            if end == 0 {
                return None;
            }
            Some(PathBuf::from(String::from_utf16_lossy(&out[..end])))
        })();
        if need_uninit {
            CoUninitialize();
        }
        result
    }
}
