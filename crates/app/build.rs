use std::env;
use std::fs;
use std::path::PathBuf;

const ICON_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

const GLYPH_A: [u8; 7] = [
    0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
];
const GLYPH_S: [u8; 7] = [
    0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
];
const GLYPH_R: [u8; 7] = [
    0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
];

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
    let mut res = Vec::with_capacity(380_000);
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
    out.extend_from_slice(&32u32.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&type_id.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&name_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&mem_flags.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(data);
    let pad = (4 - (out.len() & 3)) & 3;
    out.resize(out.len() + pad, 0);
}

fn build_group_icon_dir(dibs: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(6 + dibs.len() * 14);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(dibs.len() as u16).to_le_bytes());
    for (idx, (size, dib)) in dibs.iter().enumerate() {
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        out.push(dim);
        out.push(dim);
        out.push(0);
        out.push(0);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        out.extend_from_slice(&((idx + 1) as u16).to_le_bytes());
    }
    out
}

fn build_rt_icon_dib(size: u32) -> Vec<u8> {
    let top_down = render_asr_glass_icon_bgra(size);
    let mask_stride = (size.div_ceil(32) * 4) as usize;
    let xor_bytes = (size as usize) * (size as usize) * 4;
    let and_bytes = mask_stride * (size as usize);
    let mut out = Vec::with_capacity(40 + xor_bytes + and_bytes);
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(size as i32).to_le_bytes());
    out.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((xor_bytes + and_bytes) as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]);

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

fn render_asr_glass_icon_bgra(size: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (size as usize) * (size as usize) * 4];
    let s = size as f32;
    let rim_w = (s * 0.025).clamp(0.9, 3.2);
    let back_cx = s * 0.43;
    let back_cy = s * 0.43;
    let front_cx = s * 0.56;
    let front_cy = s * 0.56;
    let half = s * 0.36;
    let rad = s * 0.16;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let mut px = [0u8; 4];

            let d_back = rounded_sdf(fx - back_cx, fy - back_cy, half, half, rad);
            let c_back = (0.6 - d_back).clamp(0.0, 1.0);
            if c_back > 0.0 {
                let t = (((fx - (back_cx - half)) + (fy - (back_cy - half))) / (half * 4.0))
                    .clamp(0.0, 1.0);
                let mut col = [
                    lerp_u8(2, 56, t),
                    lerp_u8(132, 189, t),
                    lerp_u8(199, 248, t),
                    (c_back * 238.0).round() as u8,
                ];
                if d_back > -rim_w {
                    col = blend_rgba(col, [224, 242, 254, 96]);
                }
                px = col;
            }

            let d_front = rounded_sdf(fx - front_cx, fy - front_cy, half, half, rad);
            let c_front = (0.6 - d_front).clamp(0.0, 1.0);
            if c_front > 0.0 {
                let t = (((fx - (front_cx - half)) * 0.45 + (fy - (front_cy - half)) * 0.55)
                    / (half * 2.0))
                    .clamp(0.0, 1.0);
                let front_a = (c_front * 224.0).round() as u8;
                let glass = [
                    lerp_u8(206, 94, t),
                    lerp_u8(242, 200, t),
                    lerp_u8(255, 252, t),
                    front_a,
                ];
                px = blend_rgba(px, glass);
                if d_front > -rim_w {
                    let rim_a = (c_front * 195.0).round() as u8;
                    px = blend_rgba(px, [240, 249, 255, rim_a]);
                }
            }

            if px[3] > 0 {
                let idx = ((y as usize) * (size as usize) + (x as usize)) * 4;
                buf[idx] = px[2];
                buf[idx + 1] = px[1];
                buf[idx + 2] = px[0];
                buf[idx + 3] = px[3];
            }
        }
    }
    stamp_asr_text(&mut buf, size, front_cx, front_cy, half);
    buf
}

fn stamp_asr_text(buf: &mut [u8], size: u32, cx: f32, cy: f32, half: f32) {
    let glyphs = [GLYPH_A, GLYPH_S, GLYPH_R];
    let scale = (half * 0.088).max(0.65);
    let total_w = (3.0 * 5.0 + 2.0 * 1.4) * scale;
    let total_h = 7.0 * scale;
    let x0 = cx - total_w * 0.5;
    let y0 = cy - total_h * 0.5;
    let ink = [11u8, 18, 32, 245];
    let step = scale.ceil().max(1.0) as i32;

    for (gi, g) in glyphs.iter().enumerate() {
        let gx = x0 + (gi as f32) * 6.4 * scale;
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5usize {
                if (bits & (1 << (4 - col))) == 0 {
                    continue;
                }
                let px0 = (gx + (col as f32) * scale).floor() as i32;
                let py0 = (y0 + (row as f32) * scale).floor() as i32;
                for dy in 0..step {
                    for dx in 0..step {
                        let tx = px0 + dx;
                        let ty = py0 + dy;
                        if tx >= 0 && ty >= 0 && (tx as u32) < size && (ty as u32) < size {
                            let idx = ((ty as usize) * (size as usize) + (tx as usize)) * 4;
                            let dst = [buf[idx + 2], buf[idx + 1], buf[idx], buf[idx + 3]];
                            let out = blend_rgba(dst, ink);
                            buf[idx] = out[2];
                            buf[idx + 1] = out[1];
                            buf[idx + 2] = out[0];
                            buf[idx + 3] = out[3];
                        }
                    }
                }
            }
        }
    }
}

fn blend_rgba(dst: [u8; 4], src: [u8; 4]) -> [u8; 4] {
    let sa = u32::from(src[3]);
    if sa == 0 {
        return dst;
    }
    if sa == 255 {
        return src;
    }
    let da = u32::from(dst[3]);
    let inv = 255 - sa;
    let out_a = sa + (da * inv + 127) / 255;
    if out_a == 0 {
        return [0; 4];
    }
    let ch = |sc: u8, dc: u8| -> u8 {
        let num = u32::from(sc) * sa + (u32::from(dc) * da * inv + 127) / 255;
        ((num + out_a / 2) / out_a).min(255) as u8
    };
    [
        ch(src[0], dst[0]),
        ch(src[1], dst[1]),
        ch(src[2], dst[2]),
        out_a.min(255) as u8,
    ]
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
