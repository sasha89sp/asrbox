use std::env;
use std::fs;
use std::path::PathBuf;

const ICON_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 128];
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let res_bytes = build_icon_res_blob();
    let res_path = PathBuf::from(out_dir).join("asrbox_icon.res");
    if fs::write(&res_path, res_bytes).is_ok()
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-bins={}", res_path.display());
    }
}

fn build_icon_res_blob() -> Vec<u8> {
    let mut dibs: Vec<(u32, Vec<u8>)> = Vec::with_capacity(ICON_SIZES.len());
    for &size in &ICON_SIZES {
        dibs.push((size, build_rt_icon_dib(size)));
    }
    let mut res = Vec::with_capacity(110_000);
    // Standard 32-byte Win32 32-bit .res header (ordinal TYPE=0, NAME=0, DataSize=0).
    write_res_entry(&mut res, 0, 0, 0, &[]);
    for (idx, (_, dib)) in dibs.iter().enumerate() {
        write_res_entry(&mut res, RT_ICON, (idx + 1) as u16, 0x1010, dib);
    }
    let group = build_group_icon_dir(&dibs);
    write_res_entry(&mut res, RT_GROUP_ICON, 1, 0x1030, &group);
    res
}

fn write_res_entry(out: &mut Vec<u8>, type_id: u16, name_id: u16, mem_flags: u16, data: &[u8]) {
    let data_size = u32::try_from(data.len()).unwrap_or(0);
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes()); // HeaderSize
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&type_id.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&name_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    out.extend_from_slice(&mem_flags.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // LanguageId (LANG_NEUTRAL)
    out.extend_from_slice(&0u32.to_le_bytes()); // Version
    out.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    out.extend_from_slice(data);
    let pad = (4 - (out.len() & 3)) & 3;
    out.resize(out.len() + pad, 0);
}

fn build_group_icon_dir(dibs: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(6 + dibs.len() * 14);
    out.extend_from_slice(&0u16.to_le_bytes()); // idReserved
    out.extend_from_slice(&1u16.to_le_bytes()); // idType (1 = icon)
    out.extend_from_slice(&(dibs.len() as u16).to_le_bytes());
    for (idx, (size, dib)) in dibs.iter().enumerate() {
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        out.push(dim); // bWidth
        out.push(dim); // bHeight
        out.push(0); // bColorCount
        out.push(0); // bReserved
        out.extend_from_slice(&1u16.to_le_bytes()); // wPlanes
        out.extend_from_slice(&32u16.to_le_bytes()); // wBitCount
        out.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        out.extend_from_slice(&((idx + 1) as u16).to_le_bytes()); // nID
    }
    out
}

fn build_rt_icon_dib(size: u32) -> Vec<u8> {
    let top_down = render_fluent_icon_bgra(size);
    let mask_stride = (size.div_ceil(32) * 4) as usize;
    let xor_bytes = (size as usize) * (size as usize) * 4;
    let and_bytes = mask_stride * (size as usize);
    let mut out = Vec::with_capacity(40 + xor_bytes + and_bytes);
    out.extend_from_slice(&40u32.to_le_bytes()); // biSize
    out.extend_from_slice(&(size as i32).to_le_bytes()); // biWidth
    out.extend_from_slice(&((size * 2) as i32).to_le_bytes()); // biHeight (XOR + AND)
    out.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    out.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    out.extend_from_slice(&0u32.to_le_bytes()); // biCompression (BI_RGB)
    out.extend_from_slice(&((xor_bytes + and_bytes) as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]); // pels/meter + clrUsed + clrImportant

    let row_bytes = (size as usize) * 4;
    for y in (0..size as usize).rev() {
        let start = y * row_bytes;
        out.extend_from_slice(&top_down[start..start + row_bytes]);
    }
    let mut mask_row = vec![0u8; mask_stride];
    for y in (0..size as usize).rev() {
        mask_row.fill(0);
        for x in 0..size as usize {
            let a = top_down[(y * (size as usize) + x) * 4 + 3];
            if a == 0 {
                mask_row[x / 8] |= 1u8 << (7 - (x & 7));
            }
        }
        out.extend_from_slice(&mask_row);
    }
    out
}

fn render_fluent_icon_bgra(size: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (size as usize) * (size as usize) * 4];
    let s = size as f32;
    let margin = s * 0.06;
    let body = s - margin * 2.0;
    let rad = s * 0.24;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let d = rounded_sdf(fx - s * 0.5, fy - s * 0.5, body * 0.5, body * 0.5, rad);
            let cov = (0.6 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let ux = ((fx - margin) / body).clamp(0.0, 1.0);
            let uy = ((fy - margin) / body).clamp(0.0, 1.0);
            let mut rgb = sample_fluent_surface(ux, uy);
            if d > -1.2 {
                let rim = ((d + 1.2) / 1.2).clamp(0.0, 1.0) * 0.30;
                rgb[0] = lerp_u8(rgb[0], 190, rim);
                rgb[1] = lerp_u8(rgb[1], 232, rim);
                rgb[2] = lerp_u8(rgb[2], 255, rim);
            }
            let a = (cov * 255.0).round().clamp(0.0, 255.0) as u8;
            let idx = ((y as usize) * (size as usize) + (x as usize)) * 4;
            buf[idx] = rgb[2];
            buf[idx + 1] = rgb[1];
            buf[idx + 2] = rgb[0];
            buf[idx + 3] = a;
        }
    }
    buf
}

fn sample_fluent_surface(ux: f32, uy: f32) -> [u8; 3] {
    let diag = (ux * 0.45 + uy * 0.55).clamp(0.0, 1.0);
    let bg = [
        lerp_u8(22, 0, diag),
        lerp_u8(136, 84, diag),
        lerp_u8(224, 174, diag),
    ];
    if in_box(ux, uy, 0.17, 0.17, 0.46, 0.46, 0.07) {
        return [255, 255, 255];
    }
    if in_box(ux, uy, 0.54, 0.17, 0.83, 0.46, 0.07) {
        return [140, 224, 255];
    }
    if in_box(ux, uy, 0.17, 0.54, 0.83, 0.83, 0.07) {
        return [230, 244, 255];
    }
    bg
}

fn in_box(ux: f32, uy: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let hx = (x1 - x0) * 0.5;
    let hy = (y1 - y0) * 0.5;
    rounded_sdf(ux - cx, uy - cy, hx, hy, r) <= 0.0
}

fn rounded_sdf(px: f32, py: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = px.abs() - (hx - r);
    let qy = py.abs() - (hy - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (f32::from(a) + (f32::from(b) - f32::from(a)) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}
