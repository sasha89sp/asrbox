use std::path::{Path, PathBuf};
use std::thread;

use asrbox_platform_api::{WebDavOpKind, WebDavResult};
use windows::Win32::Foundation::HWND;

use crate::webdav_http::{
    build_basic_auth, ensure_remote_dirs, err_result, join_remote_path, parse_https_url,
    percent_encode_segment, post_result, winhttp_request, ParsedHttps, MAX_UPLOAD_BYTES,
};
use crate::webdav_xml::{normalize_dir, parse_propfind_xml};

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_webdav_list_dir(
    host_hwnd: HWND,
    op_id: u64,
    server_id: String,
    base_url: String,
    remote_dir: String,
    username: String,
    password: String,
) {
    let hwnd_raw = host_hwnd.0 as usize;
    let _ = thread::Builder::new()
        .name("asrbox-webdav-ls".to_owned())
        .spawn(move || {
            post_result(
                hwnd_raw,
                run_list_dir(
                    op_id,
                    server_id,
                    &base_url,
                    &remote_dir,
                    &username,
                    &password,
                ),
            );
        });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_webdav_mkcol(
    host_hwnd: HWND,
    op_id: u64,
    server_id: String,
    base_url: String,
    remote_dir: String,
    username: String,
    password: String,
) {
    let hwnd_raw = host_hwnd.0 as usize;
    let _ = thread::Builder::new()
        .name("asrbox-webdav-mkcol".to_owned())
        .spawn(move || {
            post_result(
                hwnd_raw,
                run_mkcol(
                    op_id,
                    server_id,
                    &base_url,
                    &remote_dir,
                    &username,
                    &password,
                ),
            );
        });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_webdav_sync_path(
    host_hwnd: HWND,
    op_id: u64,
    server_id: String,
    base_url: String,
    remote_root: String,
    local_path: PathBuf,
    username: String,
    password: String,
) {
    let hwnd_raw = host_hwnd.0 as usize;
    let _ = thread::Builder::new()
        .name("asrbox-webdav-sync".to_owned())
        .spawn(move || {
            post_result(
                hwnd_raw,
                run_sync_path(
                    op_id,
                    server_id,
                    &base_url,
                    &remote_root,
                    &local_path,
                    &username,
                    &password,
                ),
            );
        });
}

fn run_list_dir(
    op_id: u64,
    server_id: String,
    base_url: &str,
    remote_dir: &str,
    username: &str,
    password: &str,
) -> WebDavResult {
    let parsed = match parse_https_url(base_url) {
        Ok(p) => p,
        Err(msg) => return err_result(op_id, WebDavOpKind::ListDir, server_id, 0, msg),
    };
    let auth = build_basic_auth(username, password);
    let path = join_remote_path(&parsed.path, remote_dir);
    let headers = format!("Authorization: {auth}\r\nDepth: 1\r\nContent-Type: application/xml\r\n");
    let body = br#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/><d:getcontentlength/></d:prop></d:propfind>"#;
    match winhttp_request(
        &parsed.host,
        parsed.port,
        "PROPFIND",
        &path,
        &headers,
        Some(body),
        true,
    ) {
        Ok((code, raw)) if (200..=299).contains(&code) => {
            let xml = String::from_utf8_lossy(&raw);
            let entries = parse_propfind_xml(&xml, &parsed.path, remote_dir);
            WebDavResult {
                op_id,
                kind: WebDavOpKind::ListDir,
                server_id,
                ok: true,
                status_code: code,
                message: format!("Папок/файлов: {}", entries.len()),
                remote_path: normalize_dir(remote_dir),
                entries,
            }
        }
        Ok((401 | 403, _)) => err_result(
            op_id,
            WebDavOpKind::ListDir,
            server_id,
            401,
            "Ошибка доступа (401/403)".to_owned(),
        ),
        Ok((code, _)) => err_result(
            op_id,
            WebDavOpKind::ListDir,
            server_id,
            code,
            format!("Ошибка чтения папки HTTP {code}"),
        ),
        Err(e) => err_result(
            op_id,
            WebDavOpKind::ListDir,
            server_id,
            0,
            format!("Сеть WebDAV: {e}"),
        ),
    }
}

fn run_mkcol(
    op_id: u64,
    server_id: String,
    base_url: &str,
    remote_dir: &str,
    username: &str,
    password: &str,
) -> WebDavResult {
    let parsed = match parse_https_url(base_url) {
        Ok(p) => p,
        Err(msg) => return err_result(op_id, WebDavOpKind::MkCol, server_id, 0, msg),
    };
    let auth = build_basic_auth(username, password);
    let path = join_remote_path(&parsed.path, remote_dir);
    let headers = format!("Authorization: {auth}\r\n");
    match winhttp_request(
        &parsed.host,
        parsed.port,
        "MKCOL",
        &path,
        &headers,
        None,
        false,
    ) {
        Ok((code, _)) if (200..=299).contains(&code) || code == 405 => WebDavResult {
            op_id,
            kind: WebDavOpKind::MkCol,
            server_id,
            ok: true,
            status_code: code,
            message: format!("Папка создана: {}", normalize_dir(remote_dir)),
            remote_path: normalize_dir(remote_dir),
            entries: Vec::new(),
        },
        Ok((code, _)) => err_result(
            op_id,
            WebDavOpKind::MkCol,
            server_id,
            code,
            format!("Не удалось создать папку (HTTP {code})"),
        ),
        Err(e) => err_result(
            op_id,
            WebDavOpKind::MkCol,
            server_id,
            0,
            format!("Сеть WebDAV: {e}"),
        ),
    }
}

fn run_sync_path(
    op_id: u64,
    server_id: String,
    base_url: &str,
    remote_root: &str,
    local_path: &Path,
    username: &str,
    password: &str,
) -> WebDavResult {
    let parsed = match parse_https_url(base_url) {
        Ok(p) => p,
        Err(msg) => return err_result(op_id, WebDavOpKind::SyncUpload, server_id, 0, msg),
    };
    if !local_path.exists() {
        return err_result(
            op_id,
            WebDavOpKind::SyncUpload,
            server_id,
            0,
            format!("Путь не найден: {}", local_path.display()),
        );
    }
    let auth = build_basic_auth(username, password);
    let mut uploaded = 0usize;
    let mut skipped = 0usize;
    let base_name = local_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("sync");
    let sync_root = format!("{}/sync", normalize_dir(remote_root).trim_end_matches('/'));
    if local_path.is_file() {
        ensure_remote_dirs(&parsed, &sync_root, &auth);
        match upload_one_file(&parsed, &sync_root, base_name, local_path, &auth) {
            Ok(()) => uploaded += 1,
            Err(e) => return err_result(op_id, WebDavOpKind::SyncUpload, server_id, 0, e),
        }
    } else {
        let dir_remote = format!("{sync_root}/{base_name}");
        ensure_remote_dirs(&parsed, &dir_remote, &auth);
        if let Err(e) = sync_dir_recursive(
            &parsed,
            local_path,
            &dir_remote,
            &auth,
            &mut uploaded,
            &mut skipped,
            0,
        ) {
            return err_result(op_id, WebDavOpKind::SyncUpload, server_id, 0, e);
        }
    }
    WebDavResult {
        op_id,
        kind: WebDavOpKind::SyncUpload,
        server_id,
        ok: true,
        status_code: 201,
        message: format!("Синхронизировано файлов: {uploaded} (пропущено: {skipped})"),
        remote_path: sync_root,
        entries: Vec::new(),
    }
}

fn sync_dir_recursive(
    parsed: &ParsedHttps,
    local_dir: &Path,
    remote_dir: &str,
    auth: &str,
    uploaded: &mut usize,
    skipped: &mut usize,
    depth: u8,
) -> Result<(), String> {
    if depth > 6 {
        return Ok(());
    }
    let rd = std::fs::read_dir(local_dir).map_err(|e| format!("read_dir: {e}"))?;
    for entry in rd.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if p.is_dir() {
            let sub = format!("{}/{name}", remote_dir.trim_end_matches('/'));
            ensure_remote_dirs(parsed, &sub, auth);
            sync_dir_recursive(parsed, &p, &sub, auth, uploaded, skipped, depth + 1)?;
        } else if p.is_file() {
            let size = std::fs::metadata(&p).map_or(0, |m| m.len());
            if size > MAX_UPLOAD_BYTES {
                *skipped += 1;
                continue;
            }
            upload_one_file(parsed, remote_dir, name, &p, auth)?;
            *uploaded += 1;
        }
    }
    Ok(())
}

fn upload_one_file(
    parsed: &ParsedHttps,
    remote_dir: &str,
    file_name: &str,
    local_path: &Path,
    auth: &str,
) -> Result<(), String> {
    let data = std::fs::read(local_path).map_err(|e| format!("чтение {file_name}: {e}"))?;
    let dir_path = join_remote_path(&parsed.path, remote_dir);
    let put_path = format!(
        "{}/{}",
        dir_path.trim_end_matches('/'),
        percent_encode_segment(file_name)
    );
    let hdr = format!("Authorization: {auth}\r\nContent-Type: application/octet-stream\r\n");
    let (code, _) = winhttp_request(
        &parsed.host,
        parsed.port,
        "PUT",
        &put_path,
        &hdr,
        Some(&data),
        false,
    )?;
    if (200..=299).contains(&code) {
        Ok(())
    } else {
        Err(format!("PUT {file_name}: HTTP {code}"))
    }
}
