/// 抖音扫码登录模块
///
/// 通过 SSO API 获取二维码 + 轮询检测扫码状态

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::{Result, anyhow, Context};
use base64::Engine;

/// 登录状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginStatus {
    pub is_logged_in: bool,
    pub cookies: Option<String>,
    pub error: Option<String>,
}

/// 二维码信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QRCodeInfo {
    pub image_base64: String,
    pub ticket: String,
    pub expires_in_secs: u64,
}

/// SSO 登录客户端
pub struct SsoLoginClient;

impl SsoLoginClient {
    /// 获取登录二维码
    /// API: https://sso.douyin.com/get_qrcode/
    pub async fn get_qrcode() -> Result<QRCodeInfo> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .context("创建 HTTP 客户端失败")?;

        let url = "https://sso.douyin.com/get_qrcode/?next=https%3A%2F%2Fwww.douyin.com%2F&service=https:%2F%2Fwww.douyin.com";

        let resp = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .header("Referer", "https://www.douyin.com/")
            .send()
            .await
            .context("请求二维码接口失败")?;

        let json: Value = resp.json().await.context("解析二维码响应失败")?;

        let data = json.get("data")
            .ok_or_else(|| anyhow!("响应中缺少 data 字段。响应: {}", json))?;

        let error_code = data.get("error_code").and_then(|v| v.as_u64()).unwrap_or(0);
        if error_code != 0 {
            let desc = data.get("description").and_then(|v| v.as_str()).unwrap_or("未知错误");
            return Err(anyhow!("二维码接口返回错误: {} (code: {})", desc, error_code));
        }

        // 二维码可能是 base64 字符串或 URL
        let qrcode_b64 = data.get("qrcode")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("未找到 qrcode 字段"))?;

        let token = data.get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("未找到 token 字段"))?
            .to_string();

        // qrcode 已经是 base64 编码的 PNG 图片
        let image_base64 = if qrcode_b64.starts_with("data:") {
            qrcode_b64.to_string()
        } else if qrcode_b64.starts_with("http") {
            // 如果是 URL，下载后转 base64
            let img_resp = client.get(qrcode_b64).send().await?;
            let img_bytes = img_resp.bytes().await?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&img_bytes);
            format!("data:image/png;base64,{}", b64)
        } else {
            format!("data:image/png;base64,{}", qrcode_b64)
        };

        Ok(QRCodeInfo {
            image_base64,
            ticket: token,
            expires_in_secs: 300,
        })
    }

    /// 检查二维码是否被扫码
    /// API: https://sso.douyin.com/check_qrconnect/
    pub async fn check_qrcode_status(ticket: &str) -> Result<LoginStatus> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .context("创建 HTTP 客户端失败")?;

        let url = format!(
            "https://sso.douyin.com/check_qrconnect/?token={}&service=https:%2F%2Fwww.douyin.com",
            ticket
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .header("Referer", "https://www.douyin.com/")
            .send()
            .await
            .context("检查二维码状态失败")?;

        // 在消费 body 前提取 headers
        let response_cookies = resp.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect::<Vec<_>>()
            .join("; ");

        let json: Value = resp.json().await.context("解析状态响应失败")?;

        let error_code = json.get("error_code")
            .or_else(|| json.get("data").and_then(|d| d.get("error_code")))
            .and_then(|v| v.as_u64())
            .unwrap_or(999);

        let _status = json.get("data")
            .and_then(|d| d.get("status"))
            .and_then(|v| v.as_str());

        match error_code {
            0 => {
                // 扫码成功，从重定向URL提取cookie
                // 注意：check_qrconnect 成功后会返回 redirect_url，
                // 需要访问该 URL 获得最终 cookie
                let redirect_url = json.get("data")
                    .and_then(|d| d.get("redirect_url"))
                    .and_then(|v| v.as_str());

                if let Some(redirect) = redirect_url {
                    let final_resp = client
                        .get(redirect)
                        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
                        .send()
                        .await?;

                    // 从 Set-Cookie headers 提取 cookie
                    let cookies = final_resp.headers()
                        .get_all("set-cookie")
                        .iter()
                        .filter_map(|v| v.to_str().ok())
                        .collect::<Vec<_>>()
                        .join("; ");

                    return Ok(LoginStatus {
                        is_logged_in: true,
                        cookies: Some(cookies),
                        error: None,
                    });
                }

                // 如果没有 redirect_url，使用之前提取的 cookies
                Ok(LoginStatus {
                    is_logged_in: true,
                    cookies: if response_cookies.is_empty() { None } else { Some(response_cookies) },
                    error: None,
                })
            }
            2 => {
                // code 2: 等待扫码，还没扫
                Ok(LoginStatus {
                    is_logged_in: false,
                    cookies: None,
                    error: None,
                })
            }
            3 => {
                // code 3: 已扫码，等待确认
                Ok(LoginStatus {
                    is_logged_in: false,
                    cookies: None,
                    error: None,
                })
            }
            4 => {
                // code 4: 二维码过期
                Ok(LoginStatus {
                    is_logged_in: false,
                    cookies: None,
                    error: Some("二维码已过期，请刷新".to_string()),
                })
            }
            _ => {
                let msg = json.get("data")
                    .and_then(|d| d.get("description"))
                    .or_else(|| json.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("未知状态");
                Ok(LoginStatus {
                    is_logged_in: false,
                    cookies: None,
                    error: Some(format!("{} (code: {})", msg, error_code)),
                })
            }
        }
    }
}

/// 解析 Cookie 字符串
pub fn parse_cookie_string(cookie_str: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some(idx) = part.find('=') {
            let key = part[..idx].trim().to_string();
            let value = part[idx + 1..].trim().to_string();
            map.insert(key, value);
        }
    }
    map
}
