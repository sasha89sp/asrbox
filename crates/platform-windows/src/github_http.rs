use std::collections::VecDeque;
use std::ffi::c_void;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;

use asrbox_platform_api::GitHubFetchResult;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM};
use windows::Win32::Networking::WinHttp::{
    WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
    WINHTTP_ADDREQ_FLAG_ADD, WINHTTP_FLAG_SECURE, WINHTTP_QUERY_FLAG_NUMBER,
    WINHTTP_QUERY_STATUS_CODE,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub(crate) const WM_GITHUB_DONE: u32 = WM_APP + 26;
const MAX_ZIPBALL_BYTES: u64 = 256 * 1024 * 1024;

static GH_RESULTS: OnceLock<Mutex<VecDeque<GitHubFetchResult>>> = OnceLock::new();

fn queue() -> &'static Mutex<VecDeque<GitHubFetchResult>> {
    GH_RESULTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_github_zipball(
    host_hwnd: HWND,
    op_id: u64,
    repo: String,
    branch: String,
    token: String,
    dest_path: PathBuf,
    include_in_pending_backup: bool,
) {
    let hwnd_raw = host_hwnd.0 as usize;
    let _ = thread::Builder::new()
        .name("asrbox-github".to_owned())
        .spawn(move || {
            let res = fetch_zipball_blocking(
                op_id,
                &repo,
                &branch,
                &token,
                &dest_path,
                include_in_pending_backup,
            );
            if let Ok(mut guard) = queue().lock() {
                guard.push_back(res);
            }
            let hwnd = HWND(hwnd_raw as *mut c_void);
            // SAFETY: PostMessageW is thread-safe and wakes the UI message loop.
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_GITHUB_DONE, WPARAM(0), LPARAM(0));
            }
        });
}

pub(crate) fn drain_github_results() -> Vec<GitHubFetchResult> {
    let Ok(mut guard) = queue().lock() else {
        return Vec::new();
    };
    guard.drain(..).collect()
}

fn fetch_zipball_blocking(
    op_id: u64,
    repo: &str,
    branch: &str,
    token: &str,
    dest_path: &PathBuf,
    include_in_pending_backup: bool,
) -> GitHubFetchResult {
    let path = format!("/repos/{repo}/zipball/{branch}");
    match winhttp_download_github(&path, token, dest_path) {
        Ok((status, bytes)) if (200..300).contains(&status) => GitHubFetchResult {
            op_id,
            ok: true,
            status_code: status,
            message: format!("Скачано {repo}@{branch} ({} КБ)", bytes.div_ceil(1024)),
            archive_path: Some(dest_path.clone()),
            include_in_pending_backup,
        },
        Ok((status, _)) => {
            let _ = fs::remove_file(dest_path);
            let hint = match status {
                401 => "неверный токен GitHub (401)",
                403 => "доступ запрещён или лимит API (403)",
                404 => "репозиторий или ветка не найдены (404)",
                _ => "ошибка HTTP",
            };
            GitHubFetchResult {
                op_id,
                ok: false,
                status_code: status,
                message: format!("GitHub: {hint}"),
                archive_path: None,
                include_in_pending_backup,
            }
        }
        Err(err) => {
            let _ = fs::remove_file(dest_path);
            GitHubFetchResult {
                op_id,
                ok: false,
                status_code: 0,
                message: format!("GitHub: {err}"),
                archive_path: None,
                include_in_pending_backup,
            }
        }
    }
}

fn winhttp_download_github(
    path: &str,
    token: &str,
    dest_path: &PathBuf,
) -> Result<(u16, u64), String> {
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all: {e}"))?;
    }
    let path_w = wide_null(path);
    let mut headers =
        String::from("Accept: application/vnd.github+json\r\nX-GitHub-Api-Version: 2022-11-28\r\n");
    if !token.trim().is_empty() {
        headers.push_str("Authorization: Bearer ");
        headers.push_str(token.trim());
        headers.push_str("\r\n");
    }
    let headers_w: Vec<u16> = headers.encode_utf16().collect();

    // SAFETY: All WinHTTP handles are null-checked and closed before returning.
    unsafe {
        let mut session = WinHttpOpen(
            w!("asrbox/0.0.8"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            session = WinHttpOpen(
                w!("asrbox/0.0.8"),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            );
        }
        if session.is_null() {
            return Err(format!("WinHttpOpen ({:?})", GetLastError()));
        }
        let _ = WinHttpSetTimeouts(session, 15_000, 15_000, 30_000, 60_000);
        let connect = WinHttpConnect(session, w!("api.github.com"), 443, 0);
        if connect.is_null() {
            let code = GetLastError();
            let _ = WinHttpCloseHandle(session);
            return Err(format!("WinHttpConnect ({code:?})"));
        }
        let req = WinHttpOpenRequest(
            connect,
            w!("GET"),
            PCWSTR(path_w.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        );
        if req.is_null() {
            let code = GetLastError();
            let _ = WinHttpCloseHandle(connect);
            let _ = WinHttpCloseHandle(session);
            return Err(format!("WinHttpOpenRequest ({code:?})"));
        }
        let _ = WinHttpAddRequestHeaders(req, &headers_w, WINHTTP_ADDREQ_FLAG_ADD);
        let res = stream_response_to_file(req, dest_path);
        let _ = WinHttpCloseHandle(req);
        let _ = WinHttpCloseHandle(connect);
        let _ = WinHttpCloseHandle(session);
        res
    }
}

unsafe fn stream_response_to_file(
    req: *mut c_void,
    dest_path: &PathBuf,
) -> Result<(u16, u64), String> {
    WinHttpSendRequest(req, None, None, 0, 0, 0).map_err(|e| format!("SendRequest: {e}"))?;
    WinHttpReceiveResponse(req, std::ptr::null_mut())
        .map_err(|e| format!("ReceiveResponse: {e}"))?;

    let mut status: u32 = 0;
    let mut status_len = std::mem::size_of::<u32>() as u32;
    WinHttpQueryHeaders(
        req,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        PCWSTR::null(),
        Some(&mut status as *mut u32 as *mut c_void),
        &mut status_len,
        std::ptr::null_mut(),
    )
    .map_err(|e| format!("QueryHeaders: {e}"))?;
    let code = u16::try_from(status).unwrap_or(0);
    if !(200..300).contains(&code) {
        return Ok((code, 0));
    }

    let tmp_path = dest_path.with_extension("zip.part");
    let mut file = File::create(&tmp_path).map_err(|e| format!("create zip.part: {e}"))?;
    let mut chunk = vec![0u8; 32_768];
    let mut total = 0u64;
    loop {
        let mut read: u32 = 0;
        WinHttpReadData(
            req,
            chunk.as_mut_ptr().cast(),
            chunk.len() as u32,
            &mut read,
        )
        .map_err(|e| format!("ReadData: {e}"))?;
        if read == 0 {
            break;
        }
        let take = (read as usize).min(chunk.len());
        total = total.saturating_add(take as u64);
        if total > MAX_ZIPBALL_BYTES {
            drop(file);
            let _ = fs::remove_file(&tmp_path);
            return Err("архив репозитория превышает 256 МБ".to_owned());
        }
        file.write_all(&chunk[..take])
            .map_err(|e| format!("write zip.part: {e}"))?;
    }
    file.flush().map_err(|e| format!("flush zip.part: {e}"))?;
    drop(file);
    let _ = fs::remove_file(dest_path);
    fs::rename(&tmp_path, dest_path).map_err(|e| format!("rename zipball: {e}"))?;
    Ok((code, total))
}

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
