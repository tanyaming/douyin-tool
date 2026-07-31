/// RC4 加密 — 来自 MediaCrawler douyin.js 的 Rust 翻译

pub fn rc4_encrypt(plaintext: &str, key: &str) -> String {
    let mut s: Vec<u8> = (0..256).map(|i| i as u8).collect();

    let key_bytes = key.as_bytes();
    let mut j: usize = 0;
    for i in 0..256 {
        j = (j + s[i] as usize + key_bytes[i % key_bytes.len()] as usize) % 256;
        s.swap(i, j);
    }

    let plain_bytes = plaintext.as_bytes();
    let mut i: usize = 0;
    j = 0;
    let mut cipher: Vec<u8> = Vec::with_capacity(plain_bytes.len());
    for k in 0..plain_bytes.len() {
        i = (i + 1) % 256;
        j = (j + s[i] as usize) % 256;
        s.swap(i, j);
        let t = (s[i] as usize + s[j] as usize) % 256;
        cipher.push(s[t] ^ plain_bytes[k]);
    }

    String::from_utf8_lossy(&cipher).to_string()
}
