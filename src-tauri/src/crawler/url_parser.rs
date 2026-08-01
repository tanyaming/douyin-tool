/// URL 解析工具
///
/// 支持多种抖音视频链接格式：
/// 1. https://www.douyin.com/video/7525082444551310602
/// 2. https://www.douyin.com/user/xxx?modal_id=7525082444551310602
/// 3. https://v.douyin.com/iF12345ABC/（短链，需解析）
/// 4. 纯数字 ID

use regex::Regex;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUrlInfo {
    pub aweme_id: String,
    pub url_type: UrlType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UrlType {
    Normal,     // 正常视频链接
    Short,      // 短链（需要解析）
    Modal,      // 带 modal_id 参数
    RawId,      // 纯数字 ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorUrlInfo {
    pub sec_user_id: String,
}

impl VideoUrlInfo {
    pub fn new(aweme_id: String, url_type: UrlType) -> Self {
        Self { aweme_id, url_type }
    }
}

/// 解析视频链接
pub fn parse_video_url(input: &str) -> Result<VideoUrlInfo> {
    let input = input.trim();

    // 1. 纯数字 ID
    if input.chars().all(|c| c.is_ascii_digit()) && !input.is_empty() {
        return Ok(VideoUrlInfo::new(input.to_string(), UrlType::RawId));
    }

    // 2. 短链（排除已知的长链接格式）
    if input.contains("v.douyin.com") {
        return Ok(VideoUrlInfo::new(input.to_string(), UrlType::Short));
    }

    // 3. 检查 modal_id 参数
    if let Ok(url) = Url::parse(input) {
        for (k, v) in url.query_pairs() {
            if k == "modal_id" && !v.is_empty() {
                return Ok(VideoUrlInfo::new(v.to_string(), UrlType::Modal));
            }
        }
    }

    // 4. 标准视频链接: /video/{id}
    let video_re = Regex::new(r"/video/(\d+)")?;
    if let Some(caps) = video_re.captures(input) {
        if let Some(id) = caps.get(1) {
            return Ok(VideoUrlInfo::new(id.as_str().to_string(), UrlType::Normal));
        }
    }

    Err(anyhow!("无法解析视频链接: {}", input))
}

/// 解析创作者主页链接
pub fn parse_creator_url(input: &str) -> Result<CreatorUrlInfo> {
    let input = input.trim();

    // 1. 纯 sec_user_id（以 MS4wLjABAAAA 开头）
    if input.starts_with("MS4wLjABAAAA") || (!input.starts_with("http") && !input.contains("douyin.com")) {
        return Ok(CreatorUrlInfo {
            sec_user_id: input.to_string(),
        });
    }

    // 2. 创作者主页链接: /user/{sec_user_id}
    let user_re = Regex::new(r"/user/([^/?]+)")?;
    if let Some(caps) = user_re.captures(input) {
        if let Some(id) = caps.get(1) {
            return Ok(CreatorUrlInfo {
                sec_user_id: id.as_str().to_string(),
            });
        }
    }

    Err(anyhow!("无法解析创作者链接: {}", input))
}

/// 提取 URL 参数到 HashMap
pub fn extract_url_params(url_str: &str) -> Result<std::collections::HashMap<String, String>> {
    let url = Url::parse(url_str)?;
    let mut map = std::collections::HashMap::new();
    for (k, v) in url.query_pairs() {
        map.insert(k.to_string(), v.to_string());
    }
    Ok(map)
}
