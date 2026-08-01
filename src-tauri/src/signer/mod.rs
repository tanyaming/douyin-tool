/// 抖音 a_bogus 签名生成模块
///
/// 完整实现 MediaCrawler douyin.js 中的 sign/sign_datail/sign_reply 算法
/// 纯 Rust 翻译（无 JS 引擎依赖）

mod sm3;
mod rc4;

use rand::Rng;
use sm3::Sm3;
use std::time::{SystemTime, UNIX_EPOCH};

/// 生成随机 webid（19位）
pub fn generate_webid() -> String {
    let mut rng = rand::thread_rng();
    let mut webid = String::with_capacity(19);

    for _ in 0..19 {
        let digit = rng.gen_range(0..10);
        webid.push_str(&digit.to_string());
    }
    webid
}

/// 生成随机字符（用于 a_bogus 前缀随机部分）
fn gener_random(random: u32, option: &[u8; 2]) -> Vec<u8> {
    vec![
        (random as u8 & 0x55) | (option[0] & 0xAA),
        (random as u8 & 0xAA) | (option[0] & 0x55),
        ((random >> 8) as u8 & 0x55) | (option[1] & 0xAA),
        ((random >> 8) as u8 & 0xAA) | (option[1] & 0x55),
    ]
}

fn generate_random_str() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = Vec::new();
    bytes.extend(gener_random(rng.gen_range(0..10000), &[3, 45]));
    bytes.extend(gener_random(rng.gen_range(0..10000), &[1, 0]));
    bytes.extend(gener_random(rng.gen_range(0..10000), &[1, 5]));
    String::from_utf8_lossy(&bytes).to_string()
}

/// 自定义 Base64 编码（s4 表）
fn result_encrypt(data: &str, table_id: &str) -> String {
    let tables: std::collections::HashMap<&str, &str> = [
        ("s0", "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/="),
        ("s1", "Dkdpgh4ZKsQB80/Mfvw36XI1R25+WUAlEi7NLboqYTOPuzmFjJnryx9HVGcaStCe="),
        ("s2", "Dkdpgh4ZKsQB80/Mfvw36XI1R25-WUAlEi7NLboqYTOPuzmFjJnryx9HVGcaStCe="),
        ("s3", "ckdp1h4ZKsUB80/Mfvw36XIgR25+WQAlEi7NLboqYTOPuzmFjJnryx9HVGDaStCe"),
        ("s4", "Dkdpgh2ZmsQB80/MfvV36XI1R45-WUAlEixNLwoqYTOPuzKFjJnry79HbGcaStCe"),
    ].iter().cloned().collect();

    let table = tables[table_id];
    let constant: [u32; 3] = [16515072, 258048, 4032];
    let _bytes = data.as_bytes();

    let mut result = String::new();
    let mut lound = 0;
    let mut long_int = get_long_int(lound, data);

    for i in 0..(data.len() / 3 * 4) {
        if i / 4 != lound {
            lound = i / 4;
            long_int = get_long_int(lound, data);
        }
        let temp = match i % 4 {
            0 => (long_int & constant[0]) >> 18,
            1 => (long_int & constant[1]) >> 12,
            2 => (long_int & constant[2]) >> 6,
            _ => long_int & 63,
        } as usize;
        result.push(table.chars().nth(temp).unwrap_or('A'));
    }
    result
}

fn get_long_int(round: usize, s: &str) -> u32 {
    let bytes = s.as_bytes();
    let offset = round * 3;
    ((bytes[offset] as u32) << 16)
        | ((bytes.get(offset + 1).copied().unwrap_or(0) as u32) << 8)
        | (bytes.get(offset + 2).copied().unwrap_or(0) as u32)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// 生成 rc4 加密的 bb 字符串（核心算法）
fn generate_rc4_bb_str(
    url_search_params: &str,
    user_agent: &str,
    window_env_str: &str,
    arguments: &[u32; 3],
) -> String {
    let mut sm3 = Sm3::new();
    let start_time = now_ms();

    // Step 1: url_search_params + "cus" 两次 sm3 的结果
    let url_search_params_input = format!("{}cus", url_search_params);
    let url_hash_1 = sm3.hash(&url_search_params_input);
    let url_hash_1_str = String::from_utf8_lossy(&url_hash_1).to_string();
    let url_hash_list = sm3.hash(&url_hash_1_str);

    // Step 2: "cus" 两次 sm3
    let cus_hash_1 = sm3.hash("cus");
    let cus_hash_1_str = String::from_utf8_lossy(&cus_hash_1).to_string();
    let cus = sm3.hash(&cus_hash_1_str);

    // Step 3: UA 处理
    // JS key: String.fromCharCode.apply(null, [0.00390625, 1, Arguments[2]])
    // 0.00390625 -> truncates to 0, so key = [0, 1, Arguments[2]]
    let ua_key_bytes = vec![0u8, 1u8, arguments[2] as u8, 0u8];
    let ua_key_str = String::from_utf8_lossy(&ua_key_bytes).to_string();
    let ua_rc4 = rc4::rc4_encrypt(user_agent, &ua_key_str);
    let ua_encoded = result_encrypt(&ua_rc4, "s3");
    let ua = sm3.hash(&ua_encoded);

    let end_time = now_ms();

    // Build b array
    let mut b = [0u8; 256];

    b[8] = 3;
    // b[10] through b[11..] = end_time (stored as u64 at byte 10)
    b[10] = (end_time & 0xff) as u8;
    b[11] = ((end_time >> 8) & 0xff) as u8;
    b[12] = ((end_time >> 16) & 0xff) as u8;
    b[13] = ((end_time >> 24) & 0xff) as u8;
    b[14] = ((end_time >> 32) & 0xff) as u8;
    b[15] = ((end_time >> 40) & 0xff) as u8;
    // b[15] in JS is the pageId/aid structure, but we simplify

    b[16] = (start_time & 0xff) as u8;
    b[17] = ((start_time >> 8) & 0xff) as u8;
    b[18] = ((start_time >> 16) & 0xff) as u8;
    b[19] = ((start_time >> 24) & 0xff) as u8;
    b[20] = ((start_time >> 32) & 0xff) as u8;
    b[21] = ((start_time >> 40) & 0xff) as u8;

    b[18 + 8] = 44; // b[18] in original JS = 44

    // Arguments encoding
    b[26] = ((arguments[0] >> 24) & 0xff) as u8;
    b[27] = ((arguments[0] >> 16) & 0xff) as u8;
    b[28] = ((arguments[0] >> 8) & 0xff) as u8;
    b[29] = (arguments[0] & 0xff) as u8;

    b[30] = ((arguments[1] / 256) & 0xff) as u8;
    b[31] = (arguments[1] % 256) as u8;
    b[32] = ((arguments[1] >> 24) & 0xff) as u8;
    b[33] = ((arguments[1] >> 16) & 0xff) as u8;

    b[34] = ((arguments[2] >> 24) & 0xff) as u8;
    b[35] = ((arguments[2] >> 16) & 0xff) as u8;
    b[36] = ((arguments[2] >> 8) & 0xff) as u8;
    b[37] = (arguments[2] & 0xff) as u8;

    // url_search_params hash
    if url_hash_list.len() > 22 {
        b[38] = url_hash_list[21];
        b[39] = url_hash_list[22];
    }
    if cus.len() > 22 {
        b[40] = cus[21];
        b[41] = cus[22];
    }
    if ua.len() > 24 {
        b[42] = ua[23];
        b[43] = ua[24];
    }

    // end_time encoding
    b[44] = ((end_time >> 24) & 0xff) as u8;
    b[45] = ((end_time >> 16) & 0xff) as u8;
    b[46] = ((end_time >> 8) & 0xff) as u8;
    b[47] = (end_time & 0xff) as u8;
    b[48] = b[8];
    b[49] = ((end_time >> 32) & 0xff) as u8;
    b[50] = ((end_time >> 40) & 0xff) as u8;

    // pageId (6241) and aid (6383)
    let page_id: u32 = 6241;
    let aid: u32 = 6383;
    b[51] = page_id as u8;
    b[52] = ((page_id >> 24) & 0xff) as u8;
    b[53] = ((page_id >> 16) & 0xff) as u8;
    b[54] = ((page_id >> 8) & 0xff) as u8;
    b[55] = (page_id & 0xff) as u8;

    b[56] = aid as u8;
    b[57] = (aid & 0xff) as u8;
    b[58] = ((aid >> 8) & 0xff) as u8;
    b[59] = ((aid >> 16) & 0xff) as u8;
    b[60] = ((aid >> 24) & 0xff) as u8;

    // window_env_list
    let window_env_list: Vec<u8> = window_env_str.bytes().collect();
    b[64] = window_env_list.len() as u8;
    b[65] = (b[64] as u16 & 0xff) as u8;
    b[66] = ((b[64] as u16 >> 8) & 0xff) as u8;

    // XOR checksum
    b[72] = b[18 + 8 - 8 - 8] ^ b[20] ^ b[26] ^ b[30] ^ b[38] ^ b[40] ^ b[42]
        ^ b[21] ^ b[27] ^ b[31] ^ b[35] ^ b[39] ^ b[41] ^ b[43]
        ^ b[22] ^ b[28] ^ b[32] ^ b[36] ^ b[23] ^ b[29] ^ b[33]
        ^ b[37] ^ b[44] ^ b[45] ^ b[46] ^ b[47] ^ b[48] ^ b[49]
        ^ b[50] ^ (b[16 + 8] & 0xff) ^ (b[16 + 9] & 0xff)
        ^ b[52] ^ b[53] ^ b[54] ^ b[55]
        ^ b[57] ^ b[58] ^ b[59] ^ b[60]
        ^ b[65] ^ b[66] ^ b[70] ^ b[71];

    // Build bb array (same order as JS)
    let mut bb: Vec<u8> = vec![
        b[18 + 8 - 8 - 8], b[20], b[52], b[26], b[30], b[34], b[58], b[38],
        b[40], b[53], b[42], b[21], b[27], b[54], b[55], b[31],
        b[35], b[57], b[39], b[41], b[43], b[22], b[28], b[32],
        b[60], b[36], b[23], b[29], b[33], b[37], b[44], b[45],
        b[59], b[46], b[47], b[48], b[49], b[50], (b[16 + 8] & 0xff), (b[16 + 9] & 0xff),
        b[65], b[66], b[70], b[71],
    ];
    bb.extend(&window_env_list);
    bb.push(b[72]);

    let bb_str = String::from_utf8_lossy(&bb).to_string();
    rc4::rc4_encrypt(&bb_str, "y")
}

/// 主签名函数
fn sign(url_search_params: &str, user_agent: &str, arguments: [u32; 3]) -> String {
    let window_env = if cfg!(target_os = "macos") {
        "2560|39|2560|1320|0|30|0|0|2560|1320|2560|1440|2560|39|24|24|MacIntel"
    } else {
        "1536|747|1536|834|0|30|0|0|1536|834|1536|864|1525|747|24|24|Win32"
    };

    let random_part = generate_random_str();
    let bb_part = generate_rc4_bb_str(url_search_params, user_agent, window_env, &arguments);
    let combined = format!("{}{}", random_part, bb_part);
    format!("{}=", result_encrypt(&combined, "s4"))
}

/// 为详情页 API 生成 a_bogus
pub fn sign_datail(params: &str, user_agent: &str) -> String {
    sign(params, user_agent, [0, 1, 14])
}

/// 为回复/评论 API 生成 a_bogus
pub fn sign_reply(params: &str, user_agent: &str) -> String {
    sign(params, user_agent, [0, 1, 8])
}

/// 为搜索 API 生成 a_bogus（搜索不需要 a_bogus，但其他 API 需要）
pub fn sign_search(params: &str, user_agent: &str) -> String {
    sign(params, user_agent, [0, 1, 14])
}

/// Signer 结构体
pub struct Signer {
    pub ms_token: Option<String>,
    pub user_agent: String,
    pub webid: String,
}

impl Signer {
    pub fn new(user_agent: String) -> Self {
        Self {
            ms_token: None,
            user_agent,
            webid: generate_webid(),
        }
    }

    pub fn with_ms_token(mut self, token: String) -> Self {
        self.ms_token = Some(token);
        self
    }

    /// 生成公共参数（不含 a_bogus，在构建 URL 后动态添加）
    pub fn common_params(&self) -> Vec<(&str, String)> {
        let mut params = vec![
            ("device_platform", "webapp".to_string()),
            ("aid", "6383".to_string()),
            ("channel", "channel_pc_web".to_string()),
            ("version_code", "190600".to_string()),
            ("version_name", "19.6.0".to_string()),
            ("update_version_code", "170400".to_string()),
            ("pc_client_type", "1".to_string()),
            ("cookie_enabled", "true".to_string()),
            ("browser_language", "zh-CN".to_string()),
            ("browser_platform", "MacIntel".to_string()),
            ("browser_name", "Chrome".to_string()),
            ("browser_version", "125.0.0.0".to_string()),
            ("browser_online", "true".to_string()),
            ("engine_name", "Blink".to_string()),
            ("os_name", "Mac OS".to_string()),
            ("os_version", "10.15.7".to_string()),
            ("cpu_core_num", "8".to_string()),
            ("device_memory", "8".to_string()),
            ("engine_version", "109.0".to_string()),
            ("platform", "PC".to_string()),
            ("screen_width", "2560".to_string()),
            ("screen_height", "1440".to_string()),
            ("effective_type", "4g".to_string()),
            ("round_trip_time", "50".to_string()),
            ("webid", self.webid.clone()),
        ];

        if let Some(ref token) = self.ms_token {
            params.push(("msToken", token.clone()));
        }

        params
    }

    /// 为请求 URL 附加 a_bogus 签名
    /// uri: 如 "/aweme/v1/web/comment/list/"
    /// query: 已经 URL-encoded 的查询字符串
    pub fn sign_uri(&self, uri: &str, query: &str) -> String {
        if uri.contains("reply") {
            return sign_reply(query, &self.user_agent);
        }
        // 搜索和详情都用 sign_datail
        sign_datail(query, &self.user_agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_webid() {
        let webid = generate_webid();
        assert_eq!(webid.len(), 19);
        assert!(!webid.contains('-'));
    }

    #[test]
    fn test_sign_datail() {
        let params = "aweme_id=12345&device_platform=webapp";
        let ua = "Mozilla/5.0";
        let result = sign_datail(params, ua);
        // 应该以 = 结尾
        assert!(result.ends_with('='));
        assert!(result.len() > 10);
    }

    #[test]
    fn test_rc4() {
        let encrypted = rc4::rc4_encrypt("hello", "key");
        assert!(!encrypted.is_empty());
    }

    #[test]
    fn test_sm3_hash() {
        let mut sm3 = Sm3::new();
        let result = sm3.hash("test");
        assert_eq!(result.len(), 32);
    }
}
