use std::collections::VecDeque;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;

use asrbox_platform_api::WebDavResult;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM};
use windows::Win32::Networking::WinHttp::{
    WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryHeaders, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
    WinHttpWriteData, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
    WINHTTP_ADDREQ_FLAG_ADD, WINHTTP_ADDREQ_FLAG_REPLACE, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub(crate) const WM_WEBDAV_DONE: u32 = WM_APP + 16;
const MAX_UPLOAD_BYTES: u64 = 128 * 1024 * 1024;

static WEBDAV_RESULTS: OnceLock<Mutex<VecDeque<WebDavResult>>> = OnceLock::new();

fn queue() -> &'static Mutex<VecDeque<WebDavResult>> {
    WEBDAV_RESULTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub(crate) fn drain_webdav_results() -> Vec<WebDavResult> {
    let Ok(mut guard) = queue().lock() else {
        return Vec::new();
    };
    guard.drain(..).collect()
}

pub(crate) fn spawn_webdav_test(
    host_hwnd: HWND,
    op_id: u64,
    base_url: String,
    username: String,
    password: String,
) {
    let hwnd_raw = host_hwnd.0 as usize;
    let _ = thread::Builder::new()
        .name("asrbox-webdav".to_owned())
        .spawn(move || {
            let res = run_test(op_id, &base_url, &username, &password);
            post_result(hwnd_raw, res);
        });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_webdav_upload(
    host_hwnd: HWND,
    op_id: u64,
    base_url: String,
    remote_dir: String,
    remote_name: String,
    local_path: PathBuf,
    username: String,
    password: String,
) {
    let hwnd_raw = host_hwnd.0 as usize;
    let _ = thread::Builder::new()
        .name("asrbox-webdav".to_owned())
        .spawn(move || {
            let res = run_upload(
                op_id,
                &base_url,
                &remote_dir,
                &remote_name,
                &local_path,
                &username,
                &password,
            );
            post_result(hwnd_raw, res);
        });
}

fn post_result(hwnd_raw: usize, result: WebDavResult) {
    if let Ok(mut guard) = queue().lock() {
        guard.push_back(result);
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    // SAFETY: PostMessageW is thread-safe and notifies the UI thread of completion.
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_WEBDAV_DONE, WPARAM(0), LPARAM(0));
    }
}

fn run_test(op_id: u64, base_url: &str, username: &str, password: &str) -> WebDavResult {
    let parsed = match parse_https_url(base_url) {
        Ok(p) => p,
        Err(msg) => return err_result(op_id, 0, msg),
    };
    let auth = build_basic_auth(username, password);
    let headers = format!("Authorization: {auth}\r\nDepth: 0\r\n");
    match winhttp_request(
        &parsed.host,
        parsed.port,
        "PROPFIND",
        &parsed.path,
        &headers,
        None,
    ) {
        Ok(code) if (200..=299).contains(&code) => WebDavResult {
            op_id,
            ok: true,
            status_code: code,
            message: format!("WebDAV подключён (HTTP {code})"),
        },
        Ok(401 | 403) => err_result(
            op_id,
            401,
            "Ошибка входа (401/403): проверьте логин и пароль приложения".to_owned(),
        ),
        Ok(code) => err_result(op_id, code, format!("Сервер ответил HTTP {code}")),
        Err(e) => err_result(op_id, 0, format!("Сеть WebDAV: {e}")),
    }
}

fn run_upload(
    op_id: u64,
    base_url: &str,
    remote_dir: &str,
    remote_name: &str,
    local_path: &PathBuf,
    username: &str,
    password: &str,
) -> WebDavResult {
    let parsed = match parse_https_url(base_url) {
        Ok(p) => p,
        Err(msg) => return err_result(op_id, 0, msg),
    };
    let meta = match std::fs::metadata(local_path) {
        Ok(m) => m,
        Err(e) => return err_result(op_id, 0, format!("Файл бэкапа не найден: {e}")),
    };
    if meta.len() > MAX_UPLOAD_BYTES {
        return err_result(op_id, 0, "Архив превышает лимит 128 МБ".to_owned());
    }
    let data = match std::fs::read(local_path) {
        Ok(d) => d,
        Err(e) => return err_result(op_id, 0, format!("Ошибка чтения архива: {e}")),
    };
    let fname = if remote_name.trim().is_empty() {
        local_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("backup.zip")
    } else {
        remote_name.trim()
    };
    let auth = build_basic_auth(username, password);
    let base_prefix = parsed.path.trim_end_matches('/');
    let clean_dir = remote_dir.trim_matches('/');
    let dir_path = if clean_dir.is_empty() {
        format!("{base_prefix}/")
    } else {
        format!("{base_prefix}/{clean_dir}")
    };
    if !clean_dir.is_empty() {
        let mk_hdr = format!("Authorization: {auth}\r\n");
        let _ = winhttp_request(&parsed.host, parsed.port, "MKCOL", &dir_path, &mk_hdr, None);
    }
    let put_path = format!("{}/{}", dir_path.trim_end_matches('/'), fname);
    let put_hdr = format!("Authorization: {auth}\r\nContent-Type: application/zip\r\n");
    match winhttp_request(
        &parsed.host,
        parsed.port,
        "PUT",
        &put_path,
        &put_hdr,
        Some(&data),
    ) {
        Ok(code) if (200..=299).contains(&code) => WebDavResult {
            op_id,
            ok: true,
            status_code: code,
            message: format!(
                "Загружено в облако: {fname} ({:.1} КБ)",
                data.len() as f64 / 1024.0
            ),
        },
        Ok(401 | 403) => err_result(
            op_id,
            401,
            "Ошибка авторизации (401/403): проверьте пароль приложения".to_owned(),
        ),
        Ok(code) => err_result(op_id, code, format!("Ошибка загрузки HTTP {code}")),
        Err(e) => err_result(op_id, 0, format!("Сеть WebDAV: {e}")),
    }
}

fn err_result(op_id: u64, status_code: u16, message: String) -> WebDavResult {
    WebDavResult {
        op_id,
        ok: false,
        status_code,
        message,
    }
}

struct ParsedHttps {
    host: String,
    port: u16,
    path: String,
}

fn parse_https_url(url: &str) -> Result<ParsedHttps, String> {
    let trimmed = url.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .ok_or_else(|| "URL WebDAV должен начинаться с https://".to_owned())?;
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(port) => (h, port),
            Err(_) => (authority, 443),
        },
        None => (authority, 443),
    };
    if host.is_empty() {
        return Err("Пустой хост в URL WebDAV".to_owned());
    }
    Ok(ParsedHttps {
        host: host.to_owned(),
        port,
        path: path.to_owned(),
    })
}

fn winhttp_request(
    host: &str,
    port: u16,
    verb: &str,
    path: &str,
    headers: &str,
    body: Option<&[u8]>,
) -> Result<u16, String> {
    let host_w = wide_null(host);
    let verb_w = wide_null(verb);
    let path_w = wide_null(path);
    let hdr_w: Vec<u16> = headers.encode_utf16().collect();

    // SAFETY: All WinHTTP handles are validated for null and closed before returning.
    unsafe {
        let mut session = WinHttpOpen(
            w!("asrbox/0.0.2"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            session = WinHttpOpen(
                w!("asrbox/0.0.2"),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            );
        }
        if session.is_null() {
            return Err(format!("WinHttpOpen ({:?})", GetLastError()));
        }
        let _ = WinHttpSetTimeouts(session, 10_000, 10_000, 30_000, 60_000);
        let connect = WinHttpConnect(session, PCWSTR(host_w.as_ptr()), port, 0);
        if connect.is_null() {
            let code = GetLastError();
            let _ = WinHttpCloseHandle(session);
            return Err(format!("WinHttpConnect ({code:?})"));
        }
        let req = WinHttpOpenRequest(
            connect,
            PCWSTR(verb_w.as_ptr()),
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
        let res = execute_request(req, &hdr_w, body);
        let _ = WinHttpCloseHandle(req);
        let _ = WinHttpCloseHandle(connect);
        let _ = WinHttpCloseHandle(session);
        res
    }
}

unsafe fn execute_request(
    req: *mut c_void,
    hdr_w: &[u16],
    body: Option<&[u8]>,
) -> Result<u16, String> {
    if !hdr_w.is_empty() {
        let _ = WinHttpAddRequestHeaders(
            req,
            hdr_w,
            WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE,
        );
    }
    let total_len = body.map_or(0u32, |b| u32::try_from(b.len()).unwrap_or(u32::MAX));
    WinHttpSendRequest(req, None, None, 0, total_len, 0)
        .map_err(|e| format!("SendRequest: {e}"))?;
    if let Some(bytes) = body {
        for chunk in bytes.chunks(65_536) {
            let mut written = 0u32;
            WinHttpWriteData(
                req,
                Some(chunk.as_ptr().cast::<c_void>()),
                chunk.len() as u32,
                &mut written,
            )
            .map_err(|e| format!("WriteData: {e}"))?;
        }
    }
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
    Ok(u16::try_from(status).unwrap_or(0))
}

fn build_basic_auth(username: &str, password: &str) -> String {
    let raw = format!("{username}:{password}");
    format!("Basic {}", base64_encode(raw.as_bytes()))
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = if chunk.len() > 1 {
            u32::from(chunk[1])
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            u32::from(chunk[2])
        } else {
            0
        };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
