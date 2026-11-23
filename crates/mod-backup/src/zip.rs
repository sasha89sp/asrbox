use std::fs::File;
use std::io::Write;
use std::path::Path;

use asrbox_platform_api::LocalDateTime;

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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create_dir_all failed: {e}"))?;
    }
    let tmp = path.with_extension("zip.tmp");
    let mut file = File::create(&tmp).map_err(|e| format!("create zip tmp failed: {e}"))?;
    let (dos_time, dos_date) = to_dos_datetime(now);

    let mut central = Vec::new();
    let mut offset: u32 = 0;

    for entry in entries {
        let name_bytes = entry.name.as_bytes();
        let name_len = u16::try_from(name_bytes.len())
            .map_err(|_| format!("zip entry name too long: {}", entry.name))?;
        let data_len = u32::try_from(entry.data.len())
            .map_err(|_| format!("zip entry > 4 GiB: {}", entry.name))?;
        let crc = crc32_ieee(&entry.data);

        // Local file header (30 bytes + filename)
        let mut lfh = Vec::with_capacity(30 + name_bytes.len());
        lfh.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        lfh.extend_from_slice(&20u16.to_le_bytes()); // version needed: 2.0
        lfh.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 filename flag
        lfh.extend_from_slice(&0u16.to_le_bytes()); // method: STORE
        lfh.extend_from_slice(&dos_time.to_le_bytes());
        lfh.extend_from_slice(&dos_date.to_le_bytes());
        lfh.extend_from_slice(&crc.to_le_bytes());
        lfh.extend_from_slice(&data_len.to_le_bytes());
        lfh.extend_from_slice(&data_len.to_le_bytes());
        lfh.extend_from_slice(&name_len.to_le_bytes());
        lfh.extend_from_slice(&0u16.to_le_bytes());
        lfh.extend_from_slice(name_bytes);

        file.write_all(&lfh)
            .map_err(|e| format!("write lfh: {e}"))?;
        file.write_all(&entry.data)
            .map_err(|e| format!("write entry data: {e}"))?;

        // Central directory entry (46 bytes + filename)
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 flag
        central.extend_from_slice(&0u16.to_le_bytes()); // STORE
        central.extend_from_slice(&dos_time.to_le_bytes());
        central.extend_from_slice(&dos_date.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&data_len.to_le_bytes());
        central.extend_from_slice(&data_len.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra len
        central.extend_from_slice(&0u16.to_le_bytes()); // comment len
        central.extend_from_slice(&0u16.to_le_bytes()); // disk number
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attr
        central.extend_from_slice(&0u32.to_le_bytes()); // external attr
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);

        offset = offset
            .checked_add(lfh.len() as u32)
            .and_then(|v| v.checked_add(data_len))
            .ok_or_else(|| "zip archive exceeds 4 GiB limit".to_owned())?;
    }

    let cd_size = u32::try_from(central.len()).map_err(|_| "central dir too big".to_owned())?;
    let count = u16::try_from(entries.len()).map_err(|_| "too many zip entries".to_owned())?;
    file.write_all(&central)
        .map_err(|e| format!("write central dir: {e}"))?;

    // End of central directory (22 bytes)
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

    #[test]
    fn crc32_matches_standard_vector() {
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
    }
}
