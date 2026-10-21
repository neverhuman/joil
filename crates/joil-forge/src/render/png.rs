//! Minimal PNG writer: per-row adaptive filtering and DEFLATE with greedy LZ77 and fixed
//! Huffman codes. No dependencies; good compression on rendered images with smooth
//! gradients and flat regions.

const CRC_POLY: u32 = 0xEDB8_8320;

fn crc32(data: &[u8], mut crc: u32) -> u32 {
    crc = !crc;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ CRC_POLY
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

struct Bits {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl Bits {
    fn put(&mut self, value: u32, count: u32) {
        self.acc |= (value as u64) << self.n;
        self.n += count;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }

    /// Huffman codes are sent most-significant bit first.
    fn code(&mut self, code: u32, len: u32) {
        let mut r = 0;
        for i in 0..len {
            r |= ((code >> i) & 1) << (len - 1 - i);
        }
        self.put(r, len);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

fn lit(b: &mut Bits, v: u32) {
    match v {
        0..=143 => b.code(0x30 + v, 8),
        144..=255 => b.code(0x190 + v - 144, 9),
        256..=279 => b.code(v - 256, 7),
        _ => b.code(0xC0 + v - 280, 8),
    }
}

const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn emit_match(b: &mut Bits, len: u32, dist: u32) {
    let li = LEN_BASE
        .iter()
        .rposition(|x| *x <= len)
        .expect("length >= 3");
    lit(b, 257 + li as u32);
    b.put(len - LEN_BASE[li], LEN_EXTRA[li]);
    let di = DIST_BASE
        .iter()
        .rposition(|x| *x <= dist)
        .expect("distance >= 1");
    b.code(di as u32, 5);
    b.put(dist - DIST_BASE[di], DIST_EXTRA[di]);
}

/// zlib stream (fixed-Huffman DEFLATE).
pub fn zlib(data: &[u8]) -> Vec<u8> {
    const WINDOW: usize = 32768;
    const HBITS: usize = 15;
    let mut head = vec![usize::MAX; 1 << HBITS];
    let mut prev = vec![usize::MAX; WINDOW];
    let hash = |i: usize| -> usize {
        let v = (data[i] as u32) << 16 | (data[i + 1] as u32) << 8 | data[i + 2] as u32;
        (v.wrapping_mul(2_654_435_761) >> (32 - HBITS)) as usize
    };
    let mut b = Bits {
        out: vec![0x78, 0x01],
        acc: 0,
        n: 0,
    };
    b.put(1, 1); // final block
    b.put(1, 2); // fixed Huffman
    let mut i = 0;
    let n = data.len();
    let insert = |i: usize, head: &mut [usize], prev: &mut [usize]| {
        if i + 2 < n {
            let h = hash(i);
            prev[i % WINDOW] = head[h];
            head[h] = i;
        }
    };
    while i < n {
        let mut best = (0usize, 0usize);
        if i + 2 < n {
            let mut cand = head[hash(i)];
            let mut chain = 48;
            while cand != usize::MAX && i - cand < WINDOW && chain > 0 {
                let max = (n - i).min(258);
                let mut l = 0;
                while l < max && data[cand + l] == data[i + l] {
                    l += 1;
                }
                if l > best.0 {
                    best = (l, i - cand);
                    if l == max {
                        break;
                    }
                }
                let nxt = prev[cand % WINDOW];
                if nxt == usize::MAX || nxt >= cand {
                    break;
                }
                cand = nxt;
                chain -= 1;
            }
        }
        if best.0 >= 3 {
            emit_match(&mut b, best.0 as u32, best.1 as u32);
            for k in 0..best.0 {
                insert(i + k, &mut head, &mut prev);
            }
            i += best.0;
        } else {
            lit(&mut b, data[i] as u32);
            insert(i, &mut head, &mut prev);
            i += 1;
        }
    }
    lit(&mut b, 256);
    let mut out = b.finish();
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut c = Vec::with_capacity(4 + data.len());
    c.extend_from_slice(kind);
    c.extend_from_slice(data);
    out.extend_from_slice(&c);
    out.extend_from_slice(&crc32(&c, 0).to_be_bytes());
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i16 + b as i16 - c as i16;
    let (pa, pb, pc) = (
        (p - a as i16).abs(),
        (p - b as i16).abs(),
        (p - c as i16).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Encode an 8-bit RGB image (`rgb.len() == 3 w h`).
pub fn encode_rgb(w: usize, h: usize, rgb: &[u8]) -> Vec<u8> {
    let stride = 3 * w;
    let mut raw = Vec::with_capacity((stride + 1) * h);
    let zero = vec![0u8; stride];
    for y in 0..h {
        let row = &rgb[y * stride..(y + 1) * stride];
        let up = if y == 0 {
            &zero[..]
        } else {
            &rgb[(y - 1) * stride..y * stride]
        };
        let mut best: (u64, u8, Vec<u8>) = (u64::MAX, 0, Vec::new());
        for f in 0..5u8 {
            let mut line = Vec::with_capacity(stride);
            for x in 0..stride {
                let a = if x >= 3 { row[x - 3] } else { 0 };
                let c = if x >= 3 { up[x - 3] } else { 0 };
                let v = match f {
                    0 => row[x],
                    1 => row[x].wrapping_sub(a),
                    2 => row[x].wrapping_sub(up[x]),
                    3 => row[x].wrapping_sub(((a as u16 + up[x] as u16) / 2) as u8),
                    _ => row[x].wrapping_sub(paeth(a, up[x], c)),
                };
                line.push(v);
            }
            let cost: u64 = line.iter().map(|v| (*v as i8).unsigned_abs() as u64).sum();
            if cost < best.0 {
                best = (cost, f, line);
            }
        }
        raw.push(best.1);
        raw.extend_from_slice(&best.2);
    }
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference inflate for fixed-Huffman streams (test-only) to check round trips.
    fn inflate_fixed(z: &[u8]) -> Vec<u8> {
        let data = &z[2..z.len() - 4];
        let mut pos = 0usize;
        let bit = |pos: &mut usize| -> u32 {
            let b = (data[*pos / 8] >> (*pos % 8)) & 1;
            *pos += 1;
            b as u32
        };
        let bits =
            |pos: &mut usize, n: u32| -> u32 { (0..n).fold(0, |acc, i| acc | (bit(pos) << i)) };
        assert_eq!(bits(&mut pos, 3), 0b011);
        let mut out: Vec<u8> = Vec::new();
        loop {
            // decode fixed literal/length code
            let mut code = 0u32;
            let mut len = 0;
            let sym = loop {
                code = (code << 1) | bit(&mut pos);
                len += 1;
                match len {
                    7 if code <= 0x17 => break code + 256,
                    8 if (0x30..=0xBF).contains(&code) => break code - 0x30,
                    8 if (0xC0..=0xC7).contains(&code) => break code - 0xC0 + 280,
                    9 if (0x190..=0x1FF).contains(&code) => break code - 0x190 + 144,
                    _ => {}
                }
            };
            if sym < 256 {
                out.push(sym as u8);
            } else if sym == 256 {
                break;
            } else {
                let li = (sym - 257) as usize;
                let l = LEN_BASE[li] + bits(&mut pos, LEN_EXTRA[li]);
                let mut dc = 0u32;
                for _ in 0..5 {
                    dc = (dc << 1) | bit(&mut pos);
                }
                let d = DIST_BASE[dc as usize] + bits(&mut pos, DIST_EXTRA[dc as usize]);
                for _ in 0..l {
                    let v = out[out.len() - d as usize];
                    out.push(v);
                }
            }
        }
        out
    }

    #[test]
    fn deflate_round_trips() {
        let mut data = Vec::new();
        for i in 0..20000u32 {
            data.push((i % 251) as u8 ^ ((i / 97) as u8));
            if i % 7 == 0 {
                data.extend_from_slice(b"abcabcabcabc");
            }
        }
        let z = zlib(&data);
        assert_eq!(inflate_fixed(&z), data);
        assert_eq!(
            u32::from_be_bytes(z[z.len() - 4..].try_into().unwrap()),
            adler32(&data)
        );
        assert!(z.len() < data.len());
    }

    #[test]
    fn png_has_valid_chunks() {
        let png = encode_rgb(4, 3, &[200u8; 36]);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }
}
