use std::ffi::OsString;
use std::mem;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub(crate) fn pick_icon_dialog(owner: HWND) -> Result<Option<PathBuf>, String> {
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
