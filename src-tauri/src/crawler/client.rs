
use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::signer::Signer;

/// 搜索频道类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchChannel {
    General,     // 综合
    Video,        // 视频
    User,         // 用户
    Live,         // 直播
}

impl SearchChannel {
    pub fn value(&self) -> &str {
        match self {
            SearchChannel::General => "aweme_general",
            SearchChannel::Video => "aweme_video_web",
            SearchChannel::User => "aweme_user_web",
            SearchChannel::Live => "aweme_live",
        }
    }
}

/// 排序类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortType {
    General,      // 综合
    MostLike,     // 最多点赞
    Latest,       // 最新
}

impl SortType {
    pub fn value(&self) -> u8 {
        match self {
            SortType::General => 0,
            SortType::MostLike => 1,
            SortType::Latest => 2,
        }
    }
}

/// 发布时间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublishTime {
    Unlimited,    // 不限
    OneDay,       // 一天内
    OneWeek,      // 一周内
    SixMonths,    // 半年内
}

impl PublishTime {
    pub fn value(&self) -> u32 {
        match self {
            PublishTime::Unlimited => 0,
            PublishTime::OneDay => 1,
            PublishTime::OneWeek => 7,
            PublishTime::SixMonths => 180,
        }
    }
}

/// 抖音 API 客户端
pub struct DouyinClient {
    client: reqwest::Client,
    signer: Signer,
    base_url: String,
    cookies: String,
}

impl DouyinClient {
    pub fn new(cookies: String, user_agent: String, ms_token: Option<String>) -> Result<Self> {
        let signer = if let Some(token) = ms_token {
            Signer::new(user_agent.clone()).with_ms_token(token)
        } else {
            Signer::new(user_agent.clone())
        };

        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, HeaderValue::from_str(&cookies)?);
        headers.insert(USER_AGENT, HeaderValue::from_str(&user_agent)?);
        headers.insert("Host", HeaderValue::from_static("www.douyin.com"));
        headers.insert("Origin", HeaderValue::from_static("https://www.douyin.com"));
        headers.insert("Referer", HeaderValue::from_static("https://www.douyin.com/"));
        headers.insert("Content-Type", HeaderValue::from_static("application/json;charset=UTF-8"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            signer,
            base_url: "https://www.douyin.com".to_string(),
            cookies,
        })
    }

    /// 关键词搜索
    pub async fn search_by_keyword(
        &self,
        keyword: &str,
        offset: u32,
        sort_type: SortType,
        publish_time: PublishTime,
        search_id: &str,
    ) -> Result<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("search_channel".to_string(), SearchChannel::General.value().to_string());
        params.insert("enable_history".to_string(), "1".to_string());
        params.insert("keyword".to_string(), keyword.to_string());
        params.insert("search_source".to_string(), "tab_search".to_string());
        params.insert("query_correct_type".to_string(), "1".to_string());
        params.insert("is_filter_search".to_string(), "0".to_string());
        params.insert("from_group_id".to_string(), "7378810571505847586".to_string());
        params.insert("offset".to_string(), offset.to_string());
        params.insert("count".to_string(), "15".to_string());
        params.insert("need_filter_settings".to_string(), "1".to_string());
        params.insert("list_type".to_string(), "multi".to_string());
        params.insert("search_id".to_string(), search_id.to_string());

        // 添加筛选条件
        if sort_type.value() != 0 || publish_time.value() != 0 {
            let filter = serde_json::json!({
                "sort_type": sort_type.value().to_string(),
                "publish_time": publish_time.value().to_string(),
            });
            params.insert("filter_selected".to_string(), filter.to_string());
            params.insert("is_filter_search".to_string(), "1".to_string());
        }

        // 添加公共参数
        for (k, v) in self.signer.common_params() {
            params.insert(k.to_string(), v);
        }

        let url = format!("{}/aweme/v1/web/general/search/single/", self.base_url);
        let resp = self.client.get(&url).query(&params).send().await?;
        let json: Value = resp.json().await?;
        Ok(json)
    }

    /// 获取视频详情
    pub async fn get_video_detail(&self, aweme_id: &str) -> Result<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("aweme_id".to_string(), aweme_id.to_string());

        for (k, v) in self.signer.common_params() {
            params.insert(k.to_string(), v);
        }

        let url = format!("{}/aweme/v1/web/aweme/detail/", self.base_url);
        let resp = self.client.get(&url).query(&params).send().await?;
        let json: Value = resp.json().await?;
        Ok(json.get("aweme_detail").cloned().unwrap_or(Value::Null))
    }

    /// 获取视频评论
    pub async fn get_comments(&self, aweme_id: &str, cursor: u32) -> Result<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("aweme_id".to_string(), aweme_id.to_string());
        params.insert("cursor".to_string(), cursor.to_string());
        params.insert("count".to_string(), "20".to_string());
        params.insert("item_type".to_string(), "0".to_string());

        for (k, v) in self.signer.common_params() {
            params.insert(k.to_string(), v);
        }

        let url = format!("{}/aweme/v1/web/comment/list/", self.base_url);
        let resp = self.client.get(&url).query(&params).send().await?;
        let status = resp.status();
        let text = resp.text().await.context("读取评论响应失败")?;
        if text.trim().is_empty() {
            return Err(anyhow!("评论API返回空响应 (HTTP {})", status));
        }
        let json: Value = serde_json::from_str(&text)
            .with_context(|| format!("评论响应JSON解析失败，HTTP {}, 原始响应前200字符: {}", status, &text[..text.len().min(200)]))?;
        Ok(json)
    }

    /// 获取子评论（二级回复）
    pub async fn get_sub_comments(
        &self,
        aweme_id: &str,
        comment_id: &str,
        cursor: u32,
    ) -> Result<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("comment_id".to_string(), comment_id.to_string());
        params.insert("cursor".to_string(), cursor.to_string());
        params.insert("count".to_string(), "20".to_string());
        params.insert("item_type".to_string(), "0".to_string());
        params.insert("item_id".to_string(), aweme_id.to_string());

        for (k, v) in self.signer.common_params() {
            params.insert(k.to_string(), v);
        }

        let url = format!("{}/aweme/v1/web/comment/list/reply/", self.base_url);
        let resp = self.client.get(&url).query(&params).send().await?;
        let status = resp.status();
        let text = resp.text().await.context("读取子评论响应失败")?;
        if text.trim().is_empty() {
            return Err(anyhow!("子评论API返回空响应 (HTTP {})", status));
        }
        let json: Value = serde_json::from_str(&text)
            .with_context(|| format!("子评论响应JSON解析失败，HTTP {}, 原始响应前200字符: {}", status, &text[..text.len().min(200)]))?;
        Ok(json)
    }

    /// 获取用户信息
    pub async fn get_user_info(&self, sec_user_id: &str) -> Result<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("sec_user_id".to_string(), sec_user_id.to_string());
        params.insert("publish_video_strategy_type".to_string(), "2".to_string());
        params.insert("personal_center_strategy".to_string(), "1".to_string());

        for (k, v) in self.signer.common_params() {
            params.insert(k.to_string(), v);
        }

        let url = format!("{}/aweme/v1/web/user/profile/other/", self.base_url);
        let resp = self.client.get(&url).query(&params).send().await?;
        let json: Value = resp.json().await?;
        Ok(json)
    }

    /// 获取用户作品列表
    pub async fn get_user_posts(&self, sec_user_id: &str, max_cursor: &str) -> Result<Value> {
        let mut params = std::collections::HashMap::new();
        params.insert("sec_user_id".to_string(), sec_user_id.to_string());
        params.insert("count".to_string(), "18".to_string());
        params.insert("max_cursor".to_string(), max_cursor.to_string());
        params.insert("locate_query".to_string(), "false".to_string());
        params.insert("publish_video_strategy_type".to_string(), "2".to_string());

        for (k, v) in self.signer.common_params() {
            params.insert(k.to_string(), v);
        }

        let url = format!("{}/aweme/v1/web/aweme/post/", self.base_url);
        let resp = self.client.get(&url).query(&params).send().await?;
        let json: Value = resp.json().await?;
        Ok(json)
    }

    /// 下载媒体文件（视频或图片）
    pub async fn download_media(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.client
            .get(url)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await?;
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// 解析短链接
    pub async fn resolve_short_url(&self, short_url: &str) -> Result<Option<String>> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let resp = client.get(short_url).send().await?;
        if resp.status().is_redirection() {
            Ok(resp.headers().get("location")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }
}
