use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};
use std::thread;

use asrbox_platform_api::HttpFetchResult;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM};
use windows::Win32::Networking::WinHttp::{
    WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
    WINHTTP_ADDREQ_FLAG_ADD, WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub(crate) const WM_HTTP_DONE: u32 = WM_APP + 11;
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

static HTTP_RESULTS: OnceLock<Mutex<VecDeque<(u64, HttpFetchResult)>>> = OnceLock::new();

fn queue() -> &'static Mutex<VecDeque<(u64, HttpFetchResult)>> {
    HTTP_RESULTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub(crate) fn spawn_http_get(host_hwnd: HWND, request_id: u64, url: String) {
    spawn_http_get_headers(host_hwnd, request_id, url, Vec::new());
}

pub(crate) fn spawn_http_get_headers(
    host_hwnd: HWND,
    request_id: u64,
    url: String,
    headers: Vec<(String, String)>,
) {
    let hwnd_raw = host_hwnd.0 as usize;
    let _ = thread::Builder::new()
        .name("asrbox-http".to_owned())
        .spawn(move || {
            let result = fetch_blocking(&url, &headers);
            if let Ok(mut guard) = queue().lock() {
                guard.push_back((request_id, result));
            }
            let hwnd = HWND(hwnd_raw as *mut c_void);
            // SAFETY: PostMessageW is thread-safe and notifies the UI loop of completion.
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_HTTP_DONE, WPARAM(0), LPARAM(0));
            }
        });
}

pub(crate) fn drain_http_results() -> Vec<(u64, HttpFetchResult)> {
    let Ok(mut guard) = queue().lock() else {
        return Vec::new();
    };
    guard.drain(..).collect()
}

struct ParsedUrl {
    secure: bool,
    host: String,
    port: u16,
    path: String,
}

fn parse_http_url(url: &str) -> Result<ParsedUrl, String> {
    let (secure, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Err("неподдерживаемая схема URL".to_owned());
    };
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };
    let default_port = if secure { 443 } else { 80 };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(port) => (h, port),
            Err(_) => (authority, default_port),
        },
        None => (authority, default_port),
    };
    if host.is_empty() {
        return Err("пустой хост в URL".to_owned());
    }
    Ok(ParsedUrl {
        secure,
        host: host.to_owned(),
        port,
        path: path.to_owned(),
    })
}

fn fetch_blocking(url: &str, headers: &[(String, String)]) -> HttpFetchResult {
    let parsed = match parse_http_url(url) {
        Ok(p) => p,
        Err(err) => {
            return HttpFetchResult {
                status_code: 0,
                body: Vec::new(),
                error: Some(err),
            };
        }
    };
    match winhttp_get(&parsed, headers) {
        Ok((status_code, body)) => HttpFetchResult {
            status_code,
            body,
            error: None,
        },
        Err(err) => HttpFetchResult {
            status_code: 0,
            body: Vec::new(),
            error: Some(err),
        },
    }
}

fn winhttp_get(parsed: &ParsedUrl, headers: &[(String, String)]) -> Result<(u16, Vec<u8>), String> {
    let host_w = wide_null(&parsed.host);
    let path_w = wide_null(&parsed.path);
    let flags = if parsed.secure {
        WINHTTP_FLAG_SECURE
    } else {
        WINHTTP_OPEN_REQUEST_FLAGS(0)
    };

    // SAFETY: All WinHTTP handles are validated for null and closed before returning.
    unsafe {
        let mut session = WinHttpOpen(
            w!("asrbox/0.0.1"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            session = WinHttpOpen(
                w!("asrbox/0.0.1"),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            );
        }
        if session.is_null() {
            return Err(format!("WinHttpOpen ({:?})", GetLastError()));
        }
        let _ = WinHttpSetTimeouts(session, 10_000, 10_000, 15_000, 20_000);
        let connect = WinHttpConnect(session, PCWSTR(host_w.as_ptr()), parsed.port, 0);
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
            flags,
        );
        if req.is_null() {
            let code = GetLastError();
            let _ = WinHttpCloseHandle(connect);
            let _ = WinHttpCloseHandle(session);
            return Err(format!("WinHttpOpenRequest ({code:?})"));
        }
        if !headers.is_empty() {
            let mut raw = String::new();
            for (k, v) in headers {
                raw.push_str(k);
                raw.push_str(": ");
                raw.push_str(v);
                raw.push_str("\r\n");
            }
            let hdrs_w: Vec<u16> = raw.encode_utf16().collect();
            let _ = WinHttpAddRequestHeaders(req, &hdrs_w, WINHTTP_ADDREQ_FLAG_ADD);
        }
        let res = send_and_read(req);
        let _ = WinHttpCloseHandle(req);
        let _ = WinHttpCloseHandle(connect);
        let _ = WinHttpCloseHandle(session);
        res
    }
}

unsafe fn send_and_read(req: *mut c_void) -> Result<(u16, Vec<u8>), String> {
    WinHttpSendRequest(req, None, None, 0, 0, 0).map_err(|err| format!("SendRequest: {err}"))?;
    WinHttpReceiveResponse(req, std::ptr::null_mut())
        .map_err(|err| format!("ReceiveResponse: {err}"))?;

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
    .map_err(|err| format!("QueryHeaders: {err}"))?;

    let mut body = Vec::new();
    let mut chunk = vec![0u8; 16_384];
    loop {
        let mut read: u32 = 0;
        WinHttpReadData(
            req,
            chunk.as_mut_ptr().cast(),
            chunk.len() as u32,
            &mut read,
        )
        .map_err(|err| format!("ReadData: {err}"))?;
        if read == 0 {
            break;
        }
        let take = (read as usize).min(chunk.len());
        if body.len().saturating_add(take) > MAX_RESPONSE_BYTES {
            return Err("файл календаря превышает 32 МБ".to_owned());
        }
        body.extend_from_slice(&chunk[..take]);
    }
    Ok((u16::try_from(status).unwrap_or(0), body))
}

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
