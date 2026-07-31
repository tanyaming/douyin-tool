/// 抖音 a_bogus 签名生成模块
/// 
/// 通过调用内嵌的 douyin.js 生成 a_bogus 签名参数
/// 使用 quickjs-rs 或直接翻译 JS 算法为 Rust

use rand::Rng;

/// 生成随机 webid（19位）
pub fn generate_webid() -> String {
    let mut rng = rand::thread_rng();
    let mut webid = String::with_capacity(19);
    
    for i in 0..19 {
        let c = match i {
            8 | 13 | 18 | 23 => '-',
            _ => {
                let hex = "0123456789abcdef";
                let idx = rng.gen_range(0..16);
                hex.chars().nth(idx).unwrap_or('0')
            }
        };
        if c != '-' {
            webid.push(c);
        } else {
            let digit = rng.gen_range(0..10);
            webid.push_str(&digit.to_string());
        }
    }
    
    // 确保长度是19位
    while webid.len() > 19 {
        webid.pop();
    }
    while webid.len() < 19 {
        webid.push_str(&rng.gen_range(0..10).to_string());
    }
    
    webid
}

/// 生成 a_bogus 签名的简化实现
/// 
/// 抖音的 a_bogus 参数是通过对 URL 参数进行加密签名生成的
/// 完整实现需要调用 JS 引擎执行 douyin.js
/// 
/// 这里有3种实现策略：
/// 1. 翻译 JS → Rust（稳定，工作量大）
/// 2. 内嵌 quickjs 执行 JS（推荐，和 MediaCrawler 一致）
/// 3. 通过 WebView 注入 JS 获取（最简单，CDP 模式已实现）
pub struct Signer {
    /// msToken（从 localStorage 获取）
    pub ms_token: Option<String>,
    /// 用户 UA
    pub user_agent: String,
}

impl Signer {
    pub fn new(user_agent: String) -> Self {
        Self {
            ms_token: None,
            user_agent,
        }
    }

    pub fn with_ms_token(mut self, token: String) -> Self {
        self.ms_token = Some(token);
        self
    }

    /// 生成公共参数
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
            ("webid", generate_webid()),
        ];
        
        if let Some(ref token) = self.ms_token {
            params.push(("msToken", token.clone()));
        }
        
        params
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
}
