use std::ffi::c_void;

use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
    CRED_TYPE_GENERIC,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

use crate::shell::encode_wide_str;

const RUN_SUBKEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("asrbox");

pub(crate) fn is_autostart_enabled() -> bool {
    let mut hkey = HKEY::default();
    // SAFETY: Opening HKCU Run key read-only and closing it before return.
    unsafe {
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            RUN_SUBKEY,
            Some(0),
            KEY_QUERY_VALUE,
            &mut hkey,
        ) != ERROR_SUCCESS
        {
            return false;
        }
        let mut size = 0u32;
        let status = RegQueryValueExW(hkey, RUN_VALUE, None, None, None, Some(&mut size));
        let _ = RegCloseKey(hkey);
        status == ERROR_SUCCESS && size > 2
    }
}

pub(crate) fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    let mut hkey = HKEY::default();
    // SAFETY: Creating/opening HKCU Run key for current user and closing it before return.
    unsafe {
        let st = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_SUBKEY,
            Some(0),
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut hkey,
            None,
        );
        if st != ERROR_SUCCESS {
            return Err(format!("RegCreateKeyExW failed: {st:?}"));
        }
        let res = if enabled {
            let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
            let quoted = format!("\"{}\"", exe.display());
            let wide = encode_wide_str(&quoted);
            let bytes = std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2);
            let wr = RegSetValueExW(hkey, RUN_VALUE, Some(0), REG_SZ, Some(bytes));
            if wr == ERROR_SUCCESS {
                Ok(())
            } else {
                Err(format!("RegSetValueExW failed: {wr:?}"))
            }
        } else {
            let del = RegDeleteValueW(hkey, RUN_VALUE);
            if del == ERROR_SUCCESS || del == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                Err(format!("RegDeleteValueW failed: {del:?}"))
            }
        };
        let _ = RegCloseKey(hkey);
        res
    }
}

pub(crate) fn cred_save(target: &str, username: &str, secret: &str) -> Result<(), String> {
    let mut target_w = encode_wide_str(target);
    let mut user_w = encode_wide_str(username);
    let secret_utf16: Vec<u16> = secret.encode_utf16().collect();
    let mut secret_bytes: Vec<u8> = secret_utf16.iter().flat_map(|u| u.to_le_bytes()).collect();
    let blob_len = u32::try_from(secret_bytes.len()).map_err(|_| "secret too long".to_owned())?;

    let cred = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target_w.as_mut_ptr()),
        CredentialBlobSize: blob_len,
        CredentialBlob: if secret_bytes.is_empty() {
            std::ptr::null_mut()
        } else {
            secret_bytes.as_mut_ptr()
        },
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(user_w.as_mut_ptr()),
        ..Default::default()
    };

    // SAFETY: All pointers in CREDENTIALW point to live local Vec buffers during CredWriteW.
    let res = unsafe { CredWriteW(&cred, 0) };
    // Zeroize transient secret buffer in memory
    secret_bytes.fill(0);
    res.map_err(|e| format!("CredWriteW failed: {e}"))
}

pub(crate) fn cred_load(target: &str) -> Result<Option<(String, String)>, String> {
    let target_w = encode_wide_str(target);
    let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: CredReadW allocates *mut CREDENTIALW which is always freed with CredFree.
    unsafe {
        if let Err(err) = CredReadW(
            PCWSTR(target_w.as_ptr()),
            CRED_TYPE_GENERIC,
            Some(0),
            &mut ptr,
        ) {
            if err.code() == ERROR_NOT_FOUND.to_hresult() {
                return Ok(None);
            }
            return Err(format!("CredReadW failed: {err}"));
        }
        if ptr.is_null() {
            return Ok(None);
        }
        let cred = &*ptr;
        let user = if cred.UserName.is_null() {
            String::new()
        } else {
            cred.UserName.to_string().unwrap_or_default()
        };
        let secret = if !cred.CredentialBlob.is_null() && cred.CredentialBlobSize > 0 {
            let raw =
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
            let words: Vec<u16> = raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&c| u16::from_le_bytes(c))
                .collect();
            String::from_utf16_lossy(&words)
        } else {
            String::new()
        };
        CredFree(ptr.cast::<c_void>());
        Ok(Some((user, secret)))
    }
}

pub(crate) fn cred_delete(target: &str) -> Result<(), String> {
    let target_w = encode_wide_str(target);
    // SAFETY: target_w is NUL-terminated UTF-16 valid during synchronous CredDeleteW.
    unsafe {
        match CredDeleteW(PCWSTR(target_w.as_ptr()), CRED_TYPE_GENERIC, Some(0)) {
            Ok(()) => Ok(()),
            Err(err) if err.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
            Err(err) => Err(format!("CredDeleteW failed: {err}")),
        }
    }
}
