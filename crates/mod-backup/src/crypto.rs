/// Pure-Rust WinZip AE-2 AES-256 encryption primitives (SHA-1, HMAC-SHA1,
/// PBKDF2-HMAC-SHA1, AES-256-CTR with WinZip LE counter, and 10-byte auth tag).
/// Zero external crates and zero `unsafe`.
const AES_SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const RCON: [u32; 8] = [
    0x0100_0000,
    0x0200_0000,
    0x0400_0000,
    0x0800_0000,
    0x1000_0000,
    0x2000_0000,
    0x4000_0000,
    0x8000_0000,
];

pub struct WinZipAesPayload {
    pub salt: [u8; 16],
    pub pw_verify: [u8; 2],
    pub ciphertext: Vec<u8>,
    pub auth_code: [u8; 10],
}

pub fn winzip_aes256_encrypt(
    password: &[u8],
    salt: [u8; 16],
    plaintext: &[u8],
) -> WinZipAesPayload {
    let mut dk = [0u8; 66];
    pbkdf2_hmac_sha1(password, &salt, 1000, &mut dk);
    let mut enc_key = [0u8; 32];
    enc_key.copy_from_slice(&dk[0..32]);
    let auth_key = &dk[32..64];
    let pw_verify = [dk[64], dk[65]];

    let aes = Aes256::new(&enc_key);
    let mut ciphertext = plaintext.to_vec();
    aes.apply_winzip_ctr(&mut ciphertext);

    let mac = hmac_sha1(auth_key, &ciphertext);
    let mut auth_code = [0u8; 10];
    auth_code.copy_from_slice(&mac[..10]);
    WinZipAesPayload {
        salt,
        pw_verify,
        ciphertext,
        auth_code,
    }
}

pub fn winzip_aes256_decrypt(
    password: &[u8],
    salt: [u8; 16],
    pw_verify: [u8; 2],
    ciphertext: &[u8],
    auth_code: [u8; 10],
) -> Result<Vec<u8>, String> {
    let mut dk = [0u8; 66];
    pbkdf2_hmac_sha1(password, &salt, 1000, &mut dk);
    if [dk[64], dk[65]] != pw_verify {
        return Err("password verification mismatch".to_owned());
    }
    let mac = hmac_sha1(&dk[32..64], ciphertext);
    let mut diff = 0u8;
    for i in 0..10 {
        diff |= mac[i] ^ auth_code[i];
    }
    if diff != 0 {
        return Err("HMAC-SHA1 authentication failed".to_owned());
    }
    let mut enc_key = [0u8; 32];
    enc_key.copy_from_slice(&dk[0..32]);
    let aes = Aes256::new(&enc_key);
    let mut out = ciphertext.to_vec();
    aes.apply_winzip_ctr(&mut out);
    Ok(out)
}

pub struct Aes256 {
    rk: [[u8; 16]; 15],
}

impl Aes256 {
    pub fn new(key: &[u8; 32]) -> Self {
        let mut w = [0u32; 60];
        for i in 0..8 {
            w[i] = u32::from_be_bytes([key[4 * i], key[4 * i + 1], key[4 * i + 2], key[4 * i + 3]]);
        }
        for i in 8..60 {
            let mut temp = w[i - 1];
            if i % 8 == 0 {
                temp = sub_word(temp.rotate_left(8)) ^ RCON[(i / 8) - 1];
            } else if i % 8 == 4 {
                temp = sub_word(temp);
            }
            w[i] = w[i - 8] ^ temp;
        }
        let mut rk = [[0u8; 16]; 15];
        for (round, slot) in rk.iter_mut().enumerate() {
            for col in 0..4 {
                let bytes = w[round * 4 + col].to_be_bytes();
                slot[col * 4..col * 4 + 4].copy_from_slice(&bytes);
            }
        }
        Self { rk }
    }

    pub fn encrypt_block(&self, block: [u8; 16]) -> [u8; 16] {
        let mut s = block;
        add_round_key(&mut s, &self.rk[0]);
        for rk in &self.rk[1..14] {
            sub_bytes(&mut s);
            shift_rows(&mut s);
            mix_columns(&mut s);
            add_round_key(&mut s, rk);
        }
        sub_bytes(&mut s);
        shift_rows(&mut s);
        add_round_key(&mut s, &self.rk[14]);
        s
    }

    /// WinZip AES-CTR mode uses a 16-byte little-endian counter starting at 1.
    pub fn apply_winzip_ctr(&self, data: &mut [u8]) {
        let mut ctr: u128 = 1;
        for chunk in data.chunks_mut(16) {
            let keystream = self.encrypt_block(ctr.to_le_bytes());
            for (b, k) in chunk.iter_mut().zip(keystream.iter()) {
                *b ^= *k;
            }
            ctr = ctr.wrapping_add(1);
        }
    }
}

fn sub_word(w: u32) -> u32 {
    let b = w.to_be_bytes();
    u32::from_be_bytes([
        AES_SBOX[usize::from(b[0])],
        AES_SBOX[usize::from(b[1])],
        AES_SBOX[usize::from(b[2])],
        AES_SBOX[usize::from(b[3])],
    ])
}

fn add_round_key(s: &mut [u8; 16], rk: &[u8; 16]) {
    for i in 0..16 {
        s[i] ^= rk[i];
    }
}

fn sub_bytes(s: &mut [u8; 16]) {
    for b in s.iter_mut() {
        *b = AES_SBOX[usize::from(*b)];
    }
}

fn shift_rows(s: &mut [u8; 16]) {
    let t = *s;
    s[1] = t[5];
    s[5] = t[9];
    s[9] = t[13];
    s[13] = t[1];

    s[2] = t[10];
    s[6] = t[14];
    s[10] = t[2];
    s[14] = t[6];

    s[3] = t[15];
    s[7] = t[3];
    s[11] = t[7];
    s[15] = t[11];
}

fn xtime(a: u8) -> u8 {
    (a << 1) ^ if (a & 0x80) != 0 { 0x1b } else { 0 }
}

fn mix_columns(s: &mut [u8; 16]) {
    for c in 0..4 {
        let i = c * 4;
        let (a0, a1, a2, a3) = (s[i], s[i + 1], s[i + 2], s[i + 3]);
        let t = a0 ^ a1 ^ a2 ^ a3;
        s[i] = a0 ^ t ^ xtime(a0 ^ a1);
        s[i + 1] = a1 ^ t ^ xtime(a1 ^ a2);
        s[i + 2] = a2 ^ t ^ xtime(a2 ^ a3);
        s[i + 3] = a3 ^ t ^ xtime(a3 ^ a0);
    }
}

pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 80];
    let mut off = 0usize;
    while off + 64 <= padded.len() {
        let chunk = &padded[off..off + 64];
        off += 64;
        for (i, slot) in w[..16].iter_mut().enumerate() {
            *slot = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

pub fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..20].copy_from_slice(&sha1(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(64 + msg.len());
    for &b in &k {
        inner.push(b ^ 0x36);
    }
    inner.extend_from_slice(msg);
    let inner_hash = sha1(&inner);

    let mut outer = [0u8; 84];
    for (i, &b) in k.iter().enumerate() {
        outer[i] = b ^ 0x5c;
    }
    outer[64..84].copy_from_slice(&inner_hash);
    sha1(&outer)
}

pub fn pbkdf2_hmac_sha1(password: &[u8], salt: &[u8], iterations: u32, out: &mut [u8]) {
    let mut block_idx = 1u32;
    let mut offset = 0usize;
    let mut salt_blk = Vec::with_capacity(salt.len() + 4);
    while offset < out.len() {
        salt_blk.clear();
        salt_blk.extend_from_slice(salt);
        salt_blk.extend_from_slice(&block_idx.to_be_bytes());
        let mut u = hmac_sha1(password, &salt_blk);
        let mut t = u;
        for _ in 1..iterations {
            u = hmac_sha1(password, &u);
            for (tb, ub) in t.iter_mut().zip(u.iter()) {
                *tb ^= *ub;
            }
        }
        let take = (out.len() - offset).min(20);
        out[offset..offset + take].copy_from_slice(&t[..take]);
        offset += take;
        block_idx = block_idx.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_hmac_pbkdf2_and_aes256_match_standard_vectors() {
        assert_eq!(
            sha1(b"abc"),
            [
                0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78, 0x50,
                0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d
            ]
        );
        // RFC 2202 test case 2
        assert_eq!(
            hmac_sha1(b"Jefe", b"what do ya want for nothing?"),
            [
                0xef, 0xfc, 0xdf, 0x6a, 0xe5, 0xeb, 0x2f, 0xa2, 0xd2, 0x74, 0x16, 0xd5, 0xf1, 0x84,
                0xdf, 0x9c, 0x25, 0x9a, 0x7c, 0x79
            ]
        );
        // RFC 6070 test case 1
        let mut dk = [0u8; 20];
        pbkdf2_hmac_sha1(b"password", b"salt", 1, &mut dk);
        assert_eq!(
            dk,
            [
                0x0c, 0x60, 0xc8, 0x0f, 0x96, 0x1f, 0x0e, 0x71, 0xf3, 0xa9, 0xb5, 0x24, 0xaf, 0x60,
                0x12, 0x06, 0x2f, 0xe0, 0x37, 0xa6
            ]
        );
        // NIST FIPS-197 Appendix C.3 AES-256
        let mut key = [0u8; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = i as u8;
        }
        let pt = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let expected_ct = [
            0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49,
            0x60, 0x89,
        ];
        assert_eq!(Aes256::new(&key).encrypt_block(pt), expected_ct);
    }

    #[test]
    fn winzip_aes256_roundtrip_and_wrong_password_rejection() {
        let salt = [7u8; 16];
        let msg = b"asrbox portable backup payload 1234567890!";
        let enc = winzip_aes256_encrypt(b"StrongPass!2026", salt, msg);
        assert_ne!(enc.ciphertext.as_slice(), msg);
        let dec = winzip_aes256_decrypt(
            b"StrongPass!2026",
            enc.salt,
            enc.pw_verify,
            &enc.ciphertext,
            enc.auth_code,
        );
        assert_eq!(dec, Ok(msg.to_vec()));
        let wrong = winzip_aes256_decrypt(
            b"WrongPass",
            enc.salt,
            enc.pw_verify,
            &enc.ciphertext,
            enc.auth_code,
        );
        assert!(wrong.is_err());
    }
}
