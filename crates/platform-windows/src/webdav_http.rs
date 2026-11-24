use std::collections::VecDeque;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;

use asrbox_platform_api::{WebDavOpKind, WebDavResult};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub(crate) use crate::webdav_wire::{
    build_basic_auth, ensure_remote_dirs, join_remote_path, parse_https_url,
    percent_encode_segment, winhttp_request, ParsedHttps,
};

pub(crate) const WM_WEBDAV_DONE: u32 = WM_APP + 16;
pub(crate) const MAX_UPLOAD_BYTES: u64 = 128 * 1024 * 1024;

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

pub(crate) fn post_result(hwnd_raw: usize, result: WebDavResult) {
    if let Ok(mut guard) = queue().lock() {
        guard.push_back(result);
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    // SAFETY: PostMessageW is thread-safe and notifies the UI thread of completion.
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_WEBDAV_DONE, WPARAM(0), LPARAM(0));
    }
}

pub(crate) fn spawn_webdav_test(
    host_hwnd: HWND,
    op_id: u64,
    server_id: String,
    base_url: String,
    username: String,
    password: String,
) {
    let hwnd_raw = host_hwnd.0 as usize;
    let _ = thread::Builder::new()
        .name("asrbox-webdav".to_owned())
        .spawn(move || {
            let res = run_test(op_id, server_id, &base_url, &username, &password);
            post_result(hwnd_raw, res);
        });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_webdav_upload(
    host_hwnd: HWND,
    op_id: u64,
    server_id: String,
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
                server_id,
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

fn run_test(
    op_id: u64,
    server_id: String,
    base_url: &str,
    username: &str,
    password: &str,
) -> WebDavResult {
    let parsed = match parse_https_url(base_url) {
        Ok(p) => p,
        Err(msg) => return err_result(op_id, WebDavOpKind::Test, server_id, 0, msg),
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
        false,
    ) {
        Ok((code, _)) if (200..=299).contains(&code) => WebDavResult {
            op_id,
            kind: WebDavOpKind::Test,
            server_id,
            ok: true,
            status_code: code,
            message: format!("WebDAV подключён (HTTP {code})"),
            remote_path: String::new(),
            entries: Vec::new(),
        },
        Ok((401 | 403, _)) => err_result(
            op_id,
            WebDavOpKind::Test,
            server_id,
            401,
            "Ошибка входа (401/403): проверьте логин и пароль приложения".to_owned(),
        ),
        Ok((code, _)) => err_result(
            op_id,
            WebDavOpKind::Test,
            server_id,
            code,
            format!("Сервер ответил HTTP {code}"),
        ),
        Err(e) => err_result(
            op_id,
            WebDavOpKind::Test,
            server_id,
            0,
            format!("Сеть WebDAV: {e}"),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_upload(
    op_id: u64,
    server_id: String,
    base_url: &str,
    remote_dir: &str,
    remote_name: &str,
    local_path: &PathBuf,
    username: &str,
    password: &str,
) -> WebDavResult {
    let parsed = match parse_https_url(base_url) {
        Ok(p) => p,
        Err(msg) => return err_result(op_id, WebDavOpKind::Upload, server_id, 0, msg),
    };
    let meta = match std::fs::metadata(local_path) {
        Ok(m) => m,
        Err(e) => {
            return err_result(
                op_id,
                WebDavOpKind::Upload,
                server_id,
                0,
                format!("Файл бэкапа не найден: {e}"),
            )
        }
    };
    if meta.len() > MAX_UPLOAD_BYTES {
        return err_result(
            op_id,
            WebDavOpKind::Upload,
            server_id,
            0,
            "Архив превышает лимит 128 МБ".to_owned(),
        );
    }
    let data = match std::fs::read(local_path) {
        Ok(d) => d,
        Err(e) => {
            return err_result(
                op_id,
                WebDavOpKind::Upload,
                server_id,
                0,
                format!("Ошибка чтения архива: {e}"),
            )
        }
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
    ensure_remote_dirs(&parsed, remote_dir, &auth);
    let dir_path = join_remote_path(&parsed.path, remote_dir);
    let put_path = format!(
        "{}/{}",
        dir_path.trim_end_matches('/'),
        percent_encode_segment(fname)
    );
    let put_hdr = format!("Authorization: {auth}\r\nContent-Type: application/zip\r\n");
    match winhttp_request(
        &parsed.host,
        parsed.port,
        "PUT",
        &put_path,
        &put_hdr,
        Some(&data),
        false,
    ) {
        Ok((code, _)) if (200..=299).contains(&code) => WebDavResult {
            op_id,
            kind: WebDavOpKind::Upload,
            server_id,
            ok: true,
            status_code: code,
            message: format!(
                "Загружено в облако: {fname} ({:.1} КБ)",
                data.len() as f64 / 1024.0
            ),
            remote_path: remote_dir.to_owned(),
            entries: Vec::new(),
        },
        Ok((401 | 403, _)) => err_result(
            op_id,
            WebDavOpKind::Upload,
            server_id,
            401,
            "Ошибка авторизации (401/403): проверьте пароль приложения".to_owned(),
        ),
        Ok((code, _)) => err_result(
            op_id,
            WebDavOpKind::Upload,
            server_id,
            code,
            format!("Ошибка загрузки HTTP {code}"),
        ),
        Err(e) => err_result(
            op_id,
            WebDavOpKind::Upload,
            server_id,
            0,
            format!("Сеть WebDAV: {e}"),
        ),
    }
}

pub(crate) fn err_result(
    op_id: u64,
    kind: WebDavOpKind,
    server_id: String,
    status_code: u16,
    message: String,
) -> WebDavResult {
    WebDavResult {
        op_id,
        kind,
        server_id,
        ok: false,
        status_code,
        message,
        remote_path: String::new(),
        entries: Vec::new(),
    }
}
