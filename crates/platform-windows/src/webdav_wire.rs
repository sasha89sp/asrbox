use std::ffi::c_void;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Networking::WinHttp::{
    WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryDataAvailable, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetTimeouts, WinHttpWriteData, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
    WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_ADDREQ_FLAG_ADD, WINHTTP_ADDREQ_FLAG_REPLACE,
    WINHTTP_FLAG_SECURE, WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub(crate) struct ParsedHttps {
    pub host: String,
    pub port: u16,
    pub path: String,
}

pub(crate) fn ensure_remote_dirs(parsed: &ParsedHttps, remote_dir: &str, auth: &str) {
    let mk_hdr = format!("Authorization: {auth}\r\n");
    let mut acc = String::new();
    for seg in remote_dir
        .trim()
        .replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty())
    {
        acc.push('/');
        acc.push_str(seg);
        let full = join_remote_path(&parsed.path, &acc);
        let _ = winhttp_request(
            &parsed.host,
            parsed.port,
            "MKCOL",
            &full,
            &mk_hdr,
            None,
            false,
        );
    }
}

pub(crate) fn join_remote_path(base_path: &str, remote_dir: &str) -> String {
    let base = base_path.trim_end_matches('/');
    let encoded_parts: Vec<String> = remote_dir
        .trim()
        .replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty())
        .map(percent_encode_segment)
        .collect();
    if encoded_parts.is_empty() {
        if base.is_empty() {
            "/".to_owned()
        } else {
            format!("{base}/")
        }
    } else {
        format!("{base}/{}", encoded_parts.join("/"))
    }
}

pub(crate) fn percent_encode_segment(seg: &str) -> String {
    let mut out = String::with_capacity(seg.len());
    for &b in seg.as_bytes() {
        let keep = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub(crate) fn parse_https_url(url: &str) -> Result<ParsedHttps, String> {
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

pub(crate) fn winhttp_request(
    host: &str,
    port: u16,
    verb: &str,
    path: &str,
    headers: &str,
    body: Option<&[u8]>,
    read_response: bool,
) -> Result<(u16, Vec<u8>), String> {
    let host_w = wide_null(host);
    let verb_w = wide_null(verb);
    let path_w = wide_null(path);
    let hdr_w: Vec<u16> = headers.encode_utf16().collect();

    // SAFETY: All WinHTTP handles are validated for null and closed before returning.
    unsafe {
        let mut session = WinHttpOpen(
            w!("asrbox/0.0.5"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            session = WinHttpOpen(
                w!("asrbox/0.0.5"),
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
        let res = execute_request(req, &hdr_w, body, read_response);
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
    read_response: bool,
) -> Result<(u16, Vec<u8>), String> {
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

    let mut resp_body = Vec::new();
    if read_response {
        loop {
            let mut avail = 0u32;
            if WinHttpQueryDataAvailable(req, &mut avail).is_err() || avail == 0 {
                break;
            }
            let take = (avail as usize).min(MAX_RESPONSE_BYTES.saturating_sub(resp_body.len()));
            if take == 0 {
                break;
            }
            let start = resp_body.len();
            resp_body.resize(start + take, 0);
            let mut read = 0u32;
            if WinHttpReadData(
                req,
                resp_body[start..].as_mut_ptr().cast::<c_void>(),
                take as u32,
                &mut read,
            )
            .is_err()
                || read == 0
            {
                resp_body.truncate(start);
                break;
            }
            resp_body.truncate(start + read as usize);
        }
    }
    Ok((u16::try_from(status).unwrap_or(0), resp_body))
}

pub(crate) fn build_basic_auth(username: &str, password: &str) -> String {
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
