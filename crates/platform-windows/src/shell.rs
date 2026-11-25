use std::ffi::OsString;
use std::mem;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_ALLOWMULTISELECT, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
    OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, ShellExecuteW, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS,
    SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::running::resolve_lnk_target;

pub(crate) fn pick_icon_dialog(owner: HWND) -> Result<Option<PathBuf>, String> {
    // SAFETY: Release any active mouse capture before running a modal Win32 dialog.
    unsafe {
        let _ = ReleaseCapture();
    }
    let filter = encode_wide_str(concat!(
        "Иконки (*.ico;*.png;*.exe;*.dll)\0",
        "*.ico;*.png;*.exe;*.dll\0",
        "Все файлы (*.*)\0*.*\0",
    ));
    let mut file_buf = vec![0u16; 1024];
    let mut ofn = OPENFILENAMEW {
        lStructSize: u32::try_from(mem::size_of::<OPENFILENAMEW>()).unwrap_or(0),
        hwndOwner: owner,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(file_buf.as_mut_ptr()),
        nMaxFile: u32::try_from(file_buf.len()).unwrap_or(1024),
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    // SAFETY: `filter` and `file_buf` stay valid during the modal GetOpenFileNameW call.
    if !unsafe { GetOpenFileNameW(&mut ofn).as_bool() } {
        return Ok(None);
    }
    let len = file_buf
        .iter()
        .position(|&ch| ch == 0)
        .unwrap_or(file_buf.len());
    if len == 0 {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(OsString::from_wide(&file_buf[..len]))))
}

pub(crate) fn pick_sync_files_dialog(owner: HWND) -> Result<Vec<PathBuf>, String> {
    // SAFETY: Release any active mouse capture before running a modal Win32 dialog.
    unsafe {
        let _ = ReleaseCapture();
    }
    let filter = encode_wide_str("Все файлы (*.*)\0*.*\0");
    let mut file_buf = vec![0u16; 32_768];
    let mut ofn = OPENFILENAMEW {
        lStructSize: u32::try_from(mem::size_of::<OPENFILENAMEW>()).unwrap_or(0),
        hwndOwner: owner,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(file_buf.as_mut_ptr()),
        nMaxFile: u32::try_from(file_buf.len()).unwrap_or(32_768),
        Flags: OFN_ALLOWMULTISELECT
            | OFN_EXPLORER
            | OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    // SAFETY: `filter` and `file_buf` stay valid during modal multi-select GetOpenFileNameW.
    if !unsafe { GetOpenFileNameW(&mut ofn).as_bool() } {
        return Ok(Vec::new());
    }
    let mut parts: Vec<OsString> = Vec::new();
    let mut start = 0usize;
    while start < file_buf.len() {
        let Some(rel_end) = file_buf[start..].iter().position(|&ch| ch == 0) else {
            break;
        };
        if rel_end == 0 {
            break;
        }
        parts.push(OsString::from_wide(&file_buf[start..start + rel_end]));
        start += rel_end + 1;
    }
    match parts.len() {
        0 => Ok(Vec::new()),
        1 => Ok(vec![PathBuf::from(&parts[0])]),
        _ => {
            let dir = PathBuf::from(&parts[0]);
            Ok(parts[1..].iter().map(|name| dir.join(name)).collect())
        }
    }
}

pub(crate) fn pick_sync_folder_dialog(owner: HWND) -> Result<Option<PathBuf>, String> {
    // SAFETY: Release mouse capture and initialize STA COM for IFileOpenDialog.
    unsafe {
        let _ = ReleaseCapture();
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let need_uninit = hr.is_ok();
        let res = run_folder_dialog(owner);
        if need_uninit {
            CoUninitialize();
        }
        res
    }
}

unsafe fn run_folder_dialog(owner: HWND) -> Result<Option<PathBuf>, String> {
    let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
        .map_err(|e| format!("CoCreateInstance(FileOpenDialog): {e}"))?;
    let opts = dialog
        .GetOptions()
        .map_err(|e| format!("GetOptions: {e}"))?;
    dialog
        .SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)
        .map_err(|e| format!("SetOptions: {e}"))?;
    if dialog.Show(Some(owner)).is_err() {
        return Ok(None);
    }
    let item = dialog.GetResult().map_err(|e| format!("GetResult: {e}"))?;
    let pwstr = item
        .GetDisplayName(SIGDN_FILESYSPATH)
        .map_err(|e| format!("GetDisplayName: {e}"))?;
    let path_str = pwstr.to_string().unwrap_or_default();
    CoTaskMemFree(Some(pwstr.0.cast()));
    if path_str.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(path_str)))
    }
}

pub(crate) fn launch_path(path: &Path) -> Result<(), String> {
    let verb = encode_wide_str("open");
    let target: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    shell_exec(&verb, &target, None)
}

pub(crate) fn open_url_str(url: &str) -> Result<(), String> {
    let verb = encode_wide_str("open");
    let target = encode_wide_str(url);
    shell_exec(&verb, &target, None)
}

pub(crate) fn open_in_notepad(path: &Path) -> Result<(), String> {
    let verb = encode_wide_str("open");
    let exe = encode_wide_str("notepad.exe");
    let args = encode_wide_str(&format!("\"{}\"", path.display()));
    shell_exec(&verb, &exe, Some(&args))
}

pub(crate) fn open_in_explorer(path: &Path) -> Result<(), String> {
    let verb = encode_wide_str("open");
    let exe = encode_wide_str("explorer.exe");
    let args = encode_wide_str(&format!("\"{}\"", path.display()));
    shell_exec(&verb, &exe, Some(&args))
}

pub(crate) fn open_item_location(path: &Path) -> Result<(), String> {
    let resolved = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        .then(|| resolve_lnk_target(path))
        .flatten()
        .filter(|target| target.exists() || target.parent().is_some_and(Path::exists));
    let effective = resolved.as_deref().unwrap_or(path);
    if effective.is_dir() {
        return open_in_explorer(effective);
    }
    if effective.exists() {
        let verb = encode_wide_str("open");
        let exe = encode_wide_str("explorer.exe");
        let args = encode_wide_str(&format!("/select,\"{}\"", effective.display()));
        return shell_exec(&verb, &exe, Some(&args));
    }
    if let Some(parent) = effective.parent().filter(|p| p.exists()) {
        return open_in_explorer(parent);
    }
    Err(format!("Путь не найден: {}", effective.display()))
}

fn shell_exec(verb: &[u16], file: &[u16], params: Option<&[u16]>) -> Result<(), String> {
    let param_ptr = params.map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr()));
    // SAFETY: All UTF-16 slices are NUL-terminated and live across ShellExecuteW.
    let res = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            param_ptr,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if (res.0 as usize) <= 32 {
        return Err(format!("ShellExecuteW failed with code {}", res.0 as usize));
    }
    Ok(())
}

pub(crate) fn encode_wide_str(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn copy_text_to_clipboard(owner: HWND, text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    const CF_UNICODETEXT: u32 = 13;

    let wide = encode_wide_str(text);
    let byte_len = wide.len().saturating_mul(2);
    // SAFETY: Standard Win32 clipboard sequence; global memory transfers ownership on success.
    unsafe {
        OpenClipboard(Some(owner)).map_err(|e| format!("OpenClipboard failed: {e}"))?;
        let _ = EmptyClipboard();
        let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, byte_len) else {
            let _ = CloseClipboard();
            return Err("GlobalAlloc failed".to_owned());
        };
        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            let _ = GlobalFree(Some(hmem));
            let _ = CloseClipboard();
            return Err("GlobalLock failed".to_owned());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr.cast::<u16>(), wide.len());
        let _ = GlobalUnlock(hmem);
        let set = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(hmem.0)));
        let _ = CloseClipboard();
        if set.is_err() {
            let _ = GlobalFree(Some(hmem));
            return Err("SetClipboardData failed".to_owned());
        }
    }
    Ok(())
}

pub(crate) fn launch_system_tool(cmd: &str, args: &str) -> Result<(), String> {
    let verb = encode_wide_str("open");
    let target = encode_wide_str(cmd);
    let params = (!args.is_empty()).then(|| encode_wide_str(args));
    shell_exec(&verb, &target, params.as_deref())
}

pub(crate) fn random_bytes_win(len: usize) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{
        BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    };

    let mut buf = vec![0u8; len];
    if len == 0 {
        return Ok(buf);
    }
    // SAFETY: `buf` is valid writable memory; using system-preferred CSPRNG without custom handle.
    let status = unsafe { BCryptGenRandom(None, &mut buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status.is_err() {
        return Err(format!("BCryptGenRandom failed: {status:?}"));
    }
    Ok(buf)
}
