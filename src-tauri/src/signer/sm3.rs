/// GB/T 32905-2016 SM3 密码杂凑算法
/// 来自 MediaCrawler douyin.js 中 SM3 的 Rust 翻译

pub struct Sm3 {
    reg: [u32; 8],
    chunk: Vec<u8>,
    size: usize,
}

impl Sm3 {
    pub fn new() -> Self {
        let mut sm3 = Self {
            reg: [0; 8],
            chunk: Vec::new(),
            size: 0,
        };
        sm3.reset();
        sm3
    }

    fn reset(&mut self) {
        self.reg = [
            0x7380166f, 0x4914b2b9, 0x172442d7, 0xda8a0600,
            0xa96f30bc, 0x163138aa, 0xe38dee4d, 0xb0fb0e4e,
        ];
        self.chunk.clear();
        self.size = 0;
    }

    pub fn hash(&mut self, input: &str) -> Vec<u8> {
        self.reset();
        self.write_string(input);
        self.fill();
        let chunk = self.chunk.clone();
        for offset in (0..chunk.len()).step_by(64) {
            self.compress(&chunk[offset..offset + 64]);
        }
        let mut result = vec![0u8; 32];
        for i in 0..8 {
            let v = self.reg[i];
            result[4 * i] = ((v >> 24) & 0xff) as u8;
            result[4 * i + 1] = ((v >> 16) & 0xff) as u8;
            result[4 * i + 2] = ((v >> 8) & 0xff) as u8;
            result[4 * i + 3] = (v & 0xff) as u8;
        }
        self.reset();
        result
    }

    fn write_string(&mut self, s: &str) {
        let bytes: Vec<u8> = s.bytes().collect();
        self.write(&bytes);
    }

    fn write(&mut self, data: &[u8]) {
        self.size += data.len();
        let space = 64 - self.chunk.len();
        if data.len() < space {
            self.chunk.extend_from_slice(data);
        } else {
            self.chunk.extend_from_slice(&data[..space]);
            let mut offset = space;
            while self.chunk.len() >= 64 {
                // Clone chunk for compression
                let chunk_clone = self.chunk.clone();
                self.compress(&chunk_clone[..64]);
                if offset < data.len() {
                    let end = (offset + 64).min(data.len());
                    self.chunk = data[offset..end].to_vec();
                    offset += 64;
                } else {
                    self.chunk.clear();
                    break;
                }
            }
        }
    }

    fn fill(&mut self) {
        let total_bits = (self.size * 8) as u64;
        self.chunk.push(0x80);
        let pad = self.chunk.len() % 64;
        let zeros = if 64 - pad < 8 { 64 - pad + 56 } else { 56 - pad };
        self.chunk.extend(std::iter::repeat(0u8).take(zeros));
        for i in (0..8).rev() {
            self.chunk.push(((total_bits >> (8 * i)) & 0xff) as u8);
        }
    }

    fn compress(&mut self, data: &[u8]) {
        let w = expand(data);
        let mut a = self.reg.clone();

        for j in 0..64usize {
            let j_u32 = j as u32;
            let ss1 = rotl(rotl(a[0], 12).wrapping_add(a[4]).wrapping_add(rotl(tj(j_u32), j_u32 % 32)), 7);
            let ss2 = ss1 ^ rotl(a[0], 12);
            let tt1 = ffj(j_u32, a[0], a[1], a[2]).wrapping_add(a[3]).wrapping_add(ss2).wrapping_add(w[j + 68]);
            let tt2 = ggj(j_u32, a[4], a[5], a[6]).wrapping_add(a[7]).wrapping_add(ss1).wrapping_add(w[j]);
            a[3] = a[2];
            a[2] = rotl(a[1], 9);
            a[1] = a[0];
            a[0] = tt1;
            a[7] = a[6];
            a[6] = rotl(a[5], 19);
            a[5] = a[4];
            a[4] = p0(tt2);
        }

        for i in 0..8 {
            self.reg[i] ^= a[i];
        }
    }
}

fn rotl(x: u32, n: u32) -> u32 {
    (x << n) | (x >> (32 - n))
}

fn tj(j: u32) -> u32 {
    if j < 16 { 0x79cc4519 } else { 0x7a879d8a }
}

fn ffj(j: u32, x: u32, y: u32, z: u32) -> u32 {
    if j < 16 { x ^ y ^ z } else { (x & y) | (x & z) | (y & z) }
}

fn ggj(j: u32, x: u32, y: u32, z: u32) -> u32 {
    if j < 16 { x ^ y ^ z } else { (x & y) | (!x & z) }
}

fn p0(x: u32) -> u32 {
    x ^ rotl(x, 9) ^ rotl(x, 17)
}

fn p1(x: u32) -> u32 {
    x ^ rotl(x, 15) ^ rotl(x, 23)
}

fn expand(data: &[u8]) -> Vec<u32> {
    let mut w = vec![0u32; 132];

    for i in 0..16 {
        w[i] = ((data[4 * i] as u32) << 24)
            | ((data[4 * i + 1] as u32) << 16)
            | ((data[4 * i + 2] as u32) << 8)
            | (data[4 * i + 3] as u32);
    }

    for i in 16..68 {
        w[i] = p1(w[i - 16] ^ w[i - 9] ^ rotl(w[i - 3], 15)) ^ rotl(w[i - 13], 7) ^ w[i - 6];
    }

    for i in 0..64 {
        w[i + 68] = w[i] ^ w[i + 4];
    }

    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sm3() {
        let mut sm3 = Sm3::new();
        let result = sm3.hash("abc");
        // SM3("abc") = 66c7f0f4 62eeedd9 d1f2d46b dc10e4e2 4167c487 5cf2f7a2 297da02b 8f4ba8e0
        let expected = [
            0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9,
            0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10, 0xe4, 0xe2,
            0x41, 0x67, 0xc4, 0x87, 0x5c, 0xf2, 0xf7, 0xa2,
            0x29, 0x7d, 0xa0, 0x2b, 0x8f, 0x4b, 0xa8, 0xe0,
        ];
        assert_eq!(result, expected);
    }
}
