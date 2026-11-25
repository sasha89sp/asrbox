use std::fs::File;
use std::io::Write;
use std::path::Path;

use asrbox_platform_api::LocalDateTime;

use crate::crypto::winzip_aes256_encrypt;

pub struct ZipEntry {
    pub name: String,
    pub data: Vec<u8>,
}

/// Writes a standard uncompressed (STORE, method 0) ZIP archive readable by Windows Explorer.
pub fn write_store_zip(
    path: &Path,
    entries: &[ZipEntry],
    now: LocalDateTime,
) -> Result<u64, String> {
    write_zip_with_encryption(path, entries, now, None)
}

/// Writes a ZIP archive — either plain STORE (method 0) or WinZip AE-2 AES-256 (method 99)
/// compatible with 7-Zip, WinRAR, and WinZip when `aes` is `Some((password, salts))`.
pub fn write_zip_with_encryption(
    path: &Path,
    entries: &[ZipEntry],
    now: LocalDateTime,
    aes: Option<(&str, &[[u8; 16]])>,
) -> Result<u64, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create_dir_all failed: {e}"))?;
    }
    let tmp = path.with_extension("zip.tmp");
    let mut file = File::create(&tmp).map_err(|e| format!("create zip tmp failed: {e}"))?;
    let (dos_time, dos_date) = to_dos_datetime(now);

    let mut central = Vec::new();
    let mut offset: u32 = 0;

    for (idx, entry) in entries.iter().enumerate() {
        let name_bytes = entry.name.as_bytes();
        let name_len = u16::try_from(name_bytes.len())
            .map_err(|_| format!("zip entry name too long: {}", entry.name))?;
        let raw_len = u32::try_from(entry.data.len())
            .map_err(|_| format!("zip entry > 4 GiB: {}", entry.name))?;

        let (ver_needed, flags, method, crc, comp_len, extra, payload) =
            if let Some((password, salts)) = aes {
                let salt = salts
                    .get(idx)
                    .copied()
                    .ok_or_else(|| "missing CSPRNG salt for encrypted zip entry".to_owned())?;
                let enc = winzip_aes256_encrypt(password.as_bytes(), salt, &entry.data);
                let mut body = Vec::with_capacity(16 + 2 + enc.ciphertext.len() + 10);
                body.extend_from_slice(&enc.salt);
                body.extend_from_slice(&enc.pw_verify);
                body.extend_from_slice(&enc.ciphertext);
                body.extend_from_slice(&enc.auth_code);
                let c_len = u32::try_from(body.len())
                    .map_err(|_| format!("encrypted zip entry > 4 GiB: {}", entry.name))?;
                // WinZip AE-2 extra field (0x9901, 7 bytes payload: ver=2, "AE", strength=3 (AES-256), actual_method=0 STORE)
                let extra_ae2: Vec<u8> = vec![
                    0x01, 0x99, 0x07, 0x00, 0x02, 0x00, b'A', b'E', 0x03, 0x00, 0x00,
                ];
                (51u16, 0x0801u16, 99u16, 0u32, c_len, extra_ae2, Some(body))
            } else {
                let c = crc32_ieee(&entry.data);
                (20u16, 0x0800u16, 0u16, c, raw_len, Vec::new(), None)
            };
        let extra_len = u16::try_from(extra.len()).unwrap_or(0);

        let mut lfh = Vec::with_capacity(30 + name_bytes.len() + extra.len());
        lfh.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        lfh.extend_from_slice(&ver_needed.to_le_bytes());
        lfh.extend_from_slice(&flags.to_le_bytes());
        lfh.extend_from_slice(&method.to_le_bytes());
        lfh.extend_from_slice(&dos_time.to_le_bytes());
        lfh.extend_from_slice(&dos_date.to_le_bytes());
        lfh.extend_from_slice(&crc.to_le_bytes());
        lfh.extend_from_slice(&comp_len.to_le_bytes());
        lfh.extend_from_slice(&raw_len.to_le_bytes());
        lfh.extend_from_slice(&name_len.to_le_bytes());
        lfh.extend_from_slice(&extra_len.to_le_bytes());
        lfh.extend_from_slice(name_bytes);
        lfh.extend_from_slice(&extra);

        file.write_all(&lfh)
            .map_err(|e| format!("write lfh: {e}"))?;
        let data_slice = payload.as_deref().unwrap_or(&entry.data);
        file.write_all(data_slice)
            .map_err(|e| format!("write entry data: {e}"))?;

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&ver_needed.to_le_bytes());
        central.extend_from_slice(&ver_needed.to_le_bytes());
        central.extend_from_slice(&flags.to_le_bytes());
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&dos_time.to_le_bytes());
        central.extend_from_slice(&dos_date.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&comp_len.to_le_bytes());
        central.extend_from_slice(&raw_len.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&extra_len.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);
        central.extend_from_slice(&extra);

        offset = offset
            .checked_add(lfh.len() as u32)
            .and_then(|v| v.checked_add(comp_len))
            .ok_or_else(|| "zip archive exceeds 4 GiB limit".to_owned())?;
    }

    let cd_size = u32::try_from(central.len()).map_err(|_| "central dir too big".to_owned())?;
    let count = u16::try_from(entries.len()).map_err(|_| "too many zip entries".to_owned())?;
    file.write_all(&central)
        .map_err(|e| format!("write central dir: {e}"))?;

    let mut eocd = Vec::with_capacity(22);
    eocd.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    eocd.extend_from_slice(&0u16.to_le_bytes());
    eocd.extend_from_slice(&0u16.to_le_bytes());
    eocd.extend_from_slice(&count.to_le_bytes());
    eocd.extend_from_slice(&count.to_le_bytes());
    eocd.extend_from_slice(&cd_size.to_le_bytes());
    eocd.extend_from_slice(&offset.to_le_bytes());
    eocd.extend_from_slice(&0u16.to_le_bytes());
    file.write_all(&eocd)
        .map_err(|e| format!("write eocd: {e}"))?;
    file.sync_all().map_err(|e| format!("sync zip: {e}"))?;
    drop(file);

    let total_bytes = u64::from(offset) + u64::from(cd_size) + 22;
    let _ = std::fs::remove_file(path);
    std::fs::rename(&tmp, path).map_err(|e| format!("rename zip: {e}"))?;
    Ok(total_bytes)
}

pub fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320u32 & mask);
        }
    }
    !crc
}

fn to_dos_datetime(dt: LocalDateTime) -> (u16, u16) {
    let year = (dt.year.clamp(1980, 2107) - 1980) as u16;
    let month = u16::from(dt.month.clamp(1, 12));
    let day = u16::from(dt.day.clamp(1, 31));
    let hour = u16::from(dt.hour.min(23));
    let min = u16::from(dt.minute.min(59));
    let sec = u16::from((dt.second.min(59)) / 2);
    let dos_time = (hour << 11) | (min << 5) | sec;
    let dos_date = (year << 9) | (month << 5) | day;
    (dos_time, dos_date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::winzip_aes256_decrypt;

    #[test]
    fn crc32_matches_standard_vector() {
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn encrypted_zip_ae2_header_and_decrypt_work() {
        let out = std::env::temp_dir().join("asrbox_ae2_test.zip");
        let now = LocalDateTime {
            year: 2026,
            month: 11,
            day: 25,
            hour: 10,
            minute: 0,
            second: 0,
            utc_offset_min: 180,
        };
        let entries = [ZipEntry {
            name: "secret.txt".to_owned(),
            data: b"hello encrypted asrbox".to_vec(),
        }];
        let salts = [[42u8; 16]];
        let res = write_zip_with_encryption(&out, &entries, now, Some(("p@ss123", &salts)));
        assert!(res.is_ok());
        let raw = std::fs::read(&out).unwrap_or_default();
        let _ = std::fs::remove_file(&out);
        assert!(raw.starts_with(b"PK\x03\x04"));
        // Method = 99 (0x0063) at offset 8..10
        assert_eq!(u16::from_le_bytes([raw[8], raw[9]]), 99);
        // CRC = 0 for AE-2 at offset 14..18
        assert_eq!(&raw[14..18], &[0, 0, 0, 0]);
        let name_len = usize::from(u16::from_le_bytes([raw[26], raw[27]]));
        let extra_len = usize::from(u16::from_le_bytes([raw[28], raw[29]]));
        assert_eq!(extra_len, 11);
        let payload_start = 30 + name_len + extra_len;
        let comp_len = u32::from_le_bytes([raw[18], raw[19], raw[20], raw[21]]) as usize;
        let payload = &raw[payload_start..payload_start + comp_len];
        let mut salt = [0u8; 16];
        salt.copy_from_slice(&payload[..16]);
        let pw_v = [payload[16], payload[17]];
        let ct = &payload[18..payload.len() - 10];
        let mut mac = [0u8; 10];
        mac.copy_from_slice(&payload[payload.len() - 10..]);
        let plain = winzip_aes256_decrypt(b"p@ss123", salt, pw_v, ct, mac);
        assert_eq!(plain, Ok(b"hello encrypted asrbox".to_vec()));
    }
}
