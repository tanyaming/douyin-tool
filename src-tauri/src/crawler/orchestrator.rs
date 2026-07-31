/// 爬虫编排器
///
/// 管理爬取任务的整个生命周期，包括：
/// - 关键词搜索模式
/// - 指定视频模式
/// - 创作者爬取模式
/// - 进度追踪与上报
/// - 数据存储

use std::sync::Arc;
use std::io::Write as IoWrite;
use tokio::sync::{mpsc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::{Result, anyhow};
use tauri::{AppHandle, Manager};

use super::client::{DouyinClient, SortType, PublishTime};
use super::url_parser::{parse_video_url, parse_creator_url, UrlType};

/// 爬取模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlMode {
    Search,     // 关键词搜索
    Detail,     // 指定视频
    Creator,    // 创作者
}

/// 爬取配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub mode: CrawlMode,
    pub keywords: Option<Vec<String>>,      // 关键词列表（Search 模式）
    pub video_urls: Option<Vec<String>>,    // 视频 URL 列表（Detail 模式）
    pub creator_urls: Option<Vec<String>>,  // 创作者 URL 列表（Creator 模式）
    pub max_videos: u32,                     // 最大视频数
    pub max_comments: u32,                   // 每条视频最大评论数
    pub enable_comments: bool,               // 是否获取评论
    pub enable_sub_comments: bool,           // 是否获取二级评论
    pub enable_media: bool,                  // 是否下载媒体
    pub sort_type: Option<SortType>,         // 排序（Search 模式）
    pub publish_time: Option<PublishTime>,   // 发布时间（Search 模式）
    pub output_dir: String,                  // 输出目录
    pub sleep_secs: u32,                     // 请求间隔秒数
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            mode: CrawlMode::Search,
            keywords: None,
            video_urls: None,
            creator_urls: None,
            max_videos: 15,
            max_comments: 10,
            enable_comments: true,
            enable_sub_comments: false,
            enable_media: false,
            sort_type: Some(SortType::General),
            publish_time: Some(PublishTime::Unlimited),
            output_dir: "./data".to_string(),
            sleep_secs: 2,
        }
    }
}

/// 爬取进度
#[derive(Debug, Clone, Serialize)]
pub struct CrawlProgress {
    pub status: ProgressStatus,
    pub current_keyword: String,
    pub total_videos: u32,
    pub fetched_videos: u32,
    pub total_comments: u32,
    pub fetched_comments: u32,
    pub downloaded_media: u32,
    pub errors: Vec<String>,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum ProgressStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Error(String),
}

/// 爬虫编排器
pub struct CrawlerOrchestrator {
    client: DouyinClient,
    config: CrawlConfig,
    progress: Arc<Mutex<CrawlProgress>>,
    progress_tx: mpsc::UnboundedSender<CrawlProgress>,
    /// 用于浏览器搜索的 AppHandle 和共享结果存储
    app_handle: Option<AppHandle>,
    #[allow(dead_code)]
    search_results: Option<Arc<Mutex<Option<Value>>>>,
}

impl CrawlerOrchestrator {
    pub fn new(
        client: DouyinClient,
        config: CrawlConfig,
        progress_tx: mpsc::UnboundedSender<CrawlProgress>,
    ) -> Self {
        Self::with_browser(client, config, progress_tx, None, None)
    }

    /// 带浏览器搜索支持的构造函数
    pub fn with_browser(
        client: DouyinClient,
        config: CrawlConfig,
        progress_tx: mpsc::UnboundedSender<CrawlProgress>,
        app_handle: Option<AppHandle>,
        search_results: Option<Arc<Mutex<Option<Value>>>>,
    ) -> Self {
        let progress = Arc::new(Mutex::new(CrawlProgress {
            status: ProgressStatus::Idle,
            current_keyword: String::new(),
            total_videos: 0,
            fetched_videos: 0,
            total_comments: 0,
            fetched_comments: 0,
            downloaded_media: 0,
            errors: Vec::new(),
            output_dir: config.output_dir.clone(),
        }));

        Self {
            client,
            config,
            progress,
            progress_tx,
            app_handle,
            search_results,
        }
    }

    /// 启动爬取
    pub async fn start(&self) -> Result<()> {
        self.update_progress(|p| p.status = ProgressStatus::Running).await;

        match self.config.mode {
            CrawlMode::Search => self.run_search().await?,
            CrawlMode::Detail => self.run_detail().await?,
            CrawlMode::Creator => self.run_creator().await?,
        }

        let progress = self.get_progress().await;
        if progress.fetched_videos == 0 {
            let err_msg = if progress.errors.is_empty() {
                "爬取完成但未获取到任何视频。请检查登录状态、网络连接或搜索关键词".to_string()
            } else {
                format!("爬取完成但未获取到任何视频。错误详情: {}", progress.errors.join("; "))
            };
            return Err(anyhow!("{}", err_msg));
        }

        self.update_progress(|p| p.status = ProgressStatus::Completed).await;
        Ok(())
    }

    /// 关键词搜索模式
    async fn run_search(&self) -> Result<()> {
        let keywords = self.config.keywords.clone()
            .ok_or_else(|| anyhow!("Search 模式需要提供关键词"))?;

        for keyword in &keywords {
            self.update_progress(|p| {
                p.current_keyword = keyword.clone();
            }).await;

            let mut offset = 0u32;
            let mut search_id = String::new();
            let limit = 15u32; // 每页15条

            loop {
                if self.config.max_videos > 0
                    && self.get_progress().await.fetched_videos >= self.config.max_videos {
                    break;
                }

                // 检查登录窗口是否存在
                let has_browser = self.app_handle.as_ref()
                    .and_then(|app| app.get_webview_window("douyin-login"))
                    .is_some();

                eprintln!("[Crawler] keyword='{}' offset={} has_browser={}", keyword, offset, has_browser);

                let search_result = if has_browser {
                    // 有登录窗口 → 走浏览器（完整签名环境）
                    self.browser_search(keyword, offset, &search_id).await
                } else {
                    // 无登录窗口 → 返回明确错误，提示用户先扫码登录
                    Err(anyhow!("请先通过「扫码登录」打开登录窗口。仅 Cookie 无法获取签名参数，搜索会失败。"))
                };

                match search_result {
                    Ok(resp) => {
                        let data = resp.get("data").cloned();
                        let extra = resp.get("extra").cloned();

                        if data.is_none() {
                            self.add_error(format!("关键词 '{}' 搜索结果为空，跳过", keyword)).await;
                            break; // 当前关键词无结果，继续下一个关键词
                        }

                        let aweme_list = data.unwrap();
                        if aweme_list.as_array().map_or(true, |a| a.is_empty()) {
                            break; // 当前关键词无更多结果，继续下一个关键词
                        }

                        search_id = extra
                            .and_then(|e| e.get("logid").cloned())
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_default();

                        // 处理每条视频
                        if let Some(items) = aweme_list.as_array() {
                            for item in items {
                                let aweme_info = item
                                    .get("aweme_info")
                                    .or_else(|| item.get("aweme_mix_info")
                                        .and_then(|m| m.get("mix_items"))
                                        .and_then(|m| m.get(0)))
                                    .cloned();

                                if let Some(aweme) = aweme_info {
                                    self.process_aweme(&aweme).await?;
                                }
                            }
                        }

                        offset += limit;
                        self.sleep().await;
                    }
                    Err(e) => {
                        self.add_error(format!("搜索失败: {}", e)).await;
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// 指定视频模式
    async fn run_detail(&self) -> Result<()> {
        let urls = self.config.video_urls.clone()
            .ok_or_else(|| anyhow!("Detail 模式需要提供视频 URL"))?;

        let mut aweme_ids = Vec::new();

        for url in &urls {
            match parse_video_url(url) {
                Ok(info) => {
                    if info.url_type == UrlType::Short {
                        // 短链解析
                        match self.client.resolve_short_url(&info.aweme_id).await {
                            Ok(Some(resolved)) => {
                                match parse_video_url(&resolved) {
                                    Ok(resolved_info) => {
                                        aweme_ids.push(resolved_info.aweme_id);
                                    }
                                    Err(e) => {
                                        self.add_error(format!("短链解析失败: {}", e)).await;
                                    }
                                }
                            }
                            Ok(None) => {
                                self.add_error(format!("短链无重定向: {}", url)).await;
                            }
                            Err(e) => {
                                self.add_error(format!("短链请求失败: {}", e)).await;
                            }
                        }
                    } else {
                        aweme_ids.push(info.aweme_id);
                    }
                }
                Err(e) => {
                    self.add_error(format!("URL 解析失败 '{}': {}", url, e)).await;
                }
            }
        }

        // 获取视频详情
        for aweme_id in &aweme_ids {
            match self.client.get_video_detail(aweme_id).await {
                Ok(detail) => {
                    self.process_aweme(&detail).await?;
                    self.sleep().await;
                }
                Err(e) => {
                    self.add_error(format!("获取视频 {} 详情失败: {}", aweme_id, e)).await;
                }
            }
        }

        Ok(())
    }

    /// 创作者爬取模式
    async fn run_creator(&self) -> Result<()> {
        let urls = self.config.creator_urls.clone()
            .ok_or_else(|| anyhow!("Creator 模式需要提供创作者 URL"))?;

        for url in &urls {
            let sec_user_id = match parse_creator_url(url) {
                Ok(info) => info.sec_user_id,
                Err(e) => {
                    self.add_error(format!("创作者 URL 解析失败 '{}': {}", url, e)).await;
                    continue;
                }
            };

            // 获取用户信息
            match self.client.get_user_info(&sec_user_id).await {
                Ok(user_info) => {
                    self.process_user_info(&user_info).await?;
                }
                Err(e) => {
                    self.add_error(format!("获取用户信息失败: {}", e)).await;
                }
            }

            // 获取用户所有作品
            let mut max_cursor = String::new();
            loop {
                match self.client.get_user_posts(&sec_user_id, &max_cursor).await {
                    Ok(resp) => {
                        let aweme_list = resp.get("aweme_list").cloned();
                        let has_more = resp.get("has_more")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);

                        if let Some(list) = aweme_list.and_then(|l| l.as_array().cloned()) {
                            for aweme in &list {
                                self.process_aweme(aweme).await?;
                            }
                        }

                        max_cursor = resp.get("max_cursor")
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_default();

                        if has_more == 0 || max_cursor.is_empty() {
                            break;
                        }

                        self.sleep().await;
                    }
                    Err(e) => {
                        self.add_error(format!("获取用户作品失败: {}", e)).await;
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// 处理单个视频数据
    async fn process_aweme(&self, aweme: &Value) -> Result<()> {
        let aweme_id = aweme.get("aweme_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // 保存视频详情
        self.save_aweme_detail(&aweme_id, aweme).await?;

        self.increment_progress(|p| p.fetched_videos += 1).await;

        // 获取评论
        if self.config.enable_comments {
            self.fetch_comments(&aweme_id).await?;
        }

        // 下载媒体
        if self.config.enable_media {
            self.download_media_files(&aweme_id, aweme).await?;
        }

        Ok(())
    }

    /// 保存用户信息
    async fn process_user_info(&self, info: &Value) -> Result<()> {
        let user_id = info.get("sec_uid")
            .or_else(|| info.get("uid"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let output_dir = format!("{}/creator_{}", self.config.output_dir, user_id);
        std::fs::create_dir_all(&output_dir)?;

        let json_path = format!("{}/user_info.json", output_dir);
        std::fs::write(&json_path, serde_json::to_string_pretty(info)?)?;

        Ok(())
    }

    /// 保存视频详情
    async fn save_aweme_detail(&self, aweme_id: &str, aweme: &Value) -> Result<()> {
        let output_dir = format!("{}/aweme_{}", self.config.output_dir, aweme_id);
        std::fs::create_dir_all(&output_dir)?;

        let json_path = format!("{}/detail.json", output_dir);
        std::fs::write(&json_path, serde_json::to_string_pretty(aweme)?)?;

        // 同时追加到汇总文件
        let summary_path = format!("{}/aweme_details.jsonl", self.config.output_dir);
        let line = serde_json::to_string(aweme)? + "\n";
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&summary_path)?
            .write_all(line.as_bytes())?;

        Ok(())
    }

    /// 获取评论
    async fn fetch_comments(&self, aweme_id: &str) -> Result<()> {
        let mut cursor = 0u32;
        let mut comments: Vec<Value> = Vec::new();

        loop {
            if comments.len() >= self.config.max_comments as usize {
                break;
            }

            // 优先走浏览器路径（完整签名），失败回退 reqwest
            let result = match self.browser_fetch_comments(aweme_id, cursor).await {
                Ok(v) => Ok(v),
                Err(_) => self.client.get_comments(aweme_id, cursor).await,
            };

            match result {
                Ok(resp) => {
                    let comment_list = resp.get("comments").cloned();
                    let has_more = resp.get("has_more")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    if let Some(list) = comment_list.and_then(|l| l.as_array().cloned()) {
                        if list.is_empty() {
                            break;
                        }

                        for comment in &list {
                            comments.push(comment.clone());

                            // 获取二级评论
                            if self.config.enable_sub_comments {
                                if let Some(reply_count) = comment.get("reply_comment_total").and_then(|v| v.as_u64()) {
                                    if reply_count > 0 {
                                        if let Some(cid) = comment.get("cid").and_then(|v| v.as_str()) {
                                            self.fetch_sub_comments(aweme_id, cid).await?;
                                        }
                                    }
                                }
                            }
                        }

                        cursor = resp.get("cursor")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;
                    }

                    if has_more == 0 {
                        break;
                    }

                    self.sleep().await;
                }
                Err(e) => {
                    let err_msg = format!("获取评论失败 (aweme_id={}): {}", aweme_id, e);
                    self.add_error(err_msg).await;
                    break;
                }
            }
        }

        // 保存评论
        if !comments.is_empty() {
            let output_dir = format!("{}/aweme_{}", self.config.output_dir, aweme_id);
            std::fs::create_dir_all(&output_dir)?;
            let json_path = format!("{}/comments.json", output_dir);
            std::fs::write(&json_path, serde_json::to_string_pretty(&comments)?)?;

            self.increment_progress(|p| {
                p.fetched_comments += comments.len() as u32;
            }).await;
        }

        Ok(())
    }

    /// 获取二级评论
    async fn fetch_sub_comments(&self, aweme_id: &str, comment_id: &str) -> Result<()> {
        let mut cursor = 0u32;

        loop {
            match self.client.get_sub_comments(aweme_id, comment_id, cursor).await {
                Ok(resp) => {
                    let comments = resp.get("comments").and_then(|c| c.as_array()).cloned();
                    let has_more = resp.get("has_more").and_then(|v| v.as_u64()).unwrap_or(0);

                    if let Some(list) = comments {
                        if list.is_empty() {
                            break;
                        }
                        self.increment_progress(|p| {
                            p.fetched_comments += list.len() as u32;
                        }).await;
                    }

                    if has_more == 0 {
                        break;
                    }

                    cursor = resp.get("cursor").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    self.sleep().await;
                }
                Err(e) => {
                    self.add_error(format!("获取二级评论失败: {}", e)).await;
                    break;
                }
            }
        }

        Ok(())
    }

    /// 下载媒体文件
    async fn download_media_files(&self, aweme_id: &str, aweme: &Value) -> Result<()> {
        let output_dir = format!("{}/aweme_{}", self.config.output_dir, aweme_id);
        std::fs::create_dir_all(&output_dir)?;

        // 尝试提取图片列表（图文帖子）
        let image_urls = extract_image_list(aweme);
        if !image_urls.is_empty() {
            for (i, url) in image_urls.iter().enumerate() {
                if let Ok(bytes) = self.client.download_media(url).await {
                    let path = format!("{}/{:03}.jpeg", output_dir, i);
                    std::fs::write(&path, bytes)?;
                    self.increment_progress(|p| p.downloaded_media += 1).await;
                }
            }
        }

        // 尝试提取视频
        if let Some(video_url) = extract_video_url(aweme) {
            if let Ok(bytes) = self.client.download_media(&video_url).await {
                let path = format!("{}/video.mp4", output_dir);
                std::fs::write(&path, bytes)?;
                self.increment_progress(|p| p.downloaded_media += 1).await;
            }
        }

        Ok(())
    }

    // ===== 浏览器方法 =====
    
    /// 通过浏览器获取评论（利用登录窗口的完整签名环境）
    async fn browser_fetch_comments(&self, aweme_id: &str, cursor: u32) -> Result<Value> {
        let app = self.app_handle.as_ref()
            .ok_or_else(|| anyhow!("无 AppHandle"))?;
        let win = match app.get_webview_window("douyin-login") {
            Some(w) => w,
            None => return Err(anyhow!("登录窗口未打开")),
        };

        let js = format!(
            r#"(async () => {{ try {{ const r = await fetch('/aweme/v1/web/comment/list/?aweme_id={aweme_id}&cursor={cursor}&count=20&item_type=0', {{ credentials: 'include' }}); const d = await r.json(); document.cookie = 'comment_result=' + btoa(unescape(encodeURIComponent(JSON.stringify(d)))) + ';path=/'; }} catch(e) {{ document.cookie = 'comment_result=' + btoa(unescape(encodeURIComponent(JSON.stringify({{ error: e.message }})))) + ';path=/'; }} }})();"#,
            aweme_id = aweme_id, cursor = cursor
        );
        win.eval(&js).map_err(|e| anyhow!("执行评论 JS 失败: {}", e))?;

        for _ in 0..100 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if app.get_webview_window("douyin-login").is_none() {
                return Err(anyhow!("登录窗口已关闭"));
            }
            if let Ok(cookies) = win.cookies() {
                for c in &cookies {
                    if c.name() == "comment_result" {
                        let raw = c.value();
                        if raw.is_empty() { continue; }
                        use base64::Engine;
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(raw.as_bytes())
                            .map_err(|e| anyhow!("base64: {}", e))?;
                        let json_str = String::from_utf8(decoded)
                            .map_err(|e| anyhow!("utf8: {}", e))?;
                        return Ok(serde_json::from_str(&json_str)?);
                    }
                }
            }
        }
        Err(anyhow!("评论请求超时（10秒无响应）"))
    }

    /// 通过登录窗口的浏览器环境执行搜索（自动携带签名）
    /// 使用 cookie 中转结果（避免依赖 Tauri IPC，后者在外部 URL 中不可用）
    async fn browser_search(&self, keyword: &str, offset: u32, search_id: &str) -> Result<Value> {
        let app = self.app_handle.as_ref()
            .ok_or_else(|| anyhow!("无 AppHandle"))?;
        let win = match app.get_webview_window("douyin-login") {
            Some(w) => w,
            None => return Err(anyhow!("登录窗口未打开")),
        };
        let escaped = keyword.replace('\\', "\\\\").replace('"', "\\\"");
        let _ = win.eval("document.cookie = 'search_result=; path=/'").ok();
        let js = format!(r#"(async () => {{ try {{ const p = new URLSearchParams({{ search_channel: 'aweme_general', keyword: "{}", search_source: 'tab_search', offset: '{}', count: '15', search_id: '{}' }}); const r = await fetch('/aweme/v1/web/general/search/single/?' + p, {{ credentials: 'include' }}); const d = await r.json(); document.cookie = 'search_result=' + btoa(unescape(encodeURIComponent(JSON.stringify(d)))) + ';path=/'; }} catch(e) {{ document.cookie = 'search_result=' + btoa(unescape(encodeURIComponent(JSON.stringify({{ error: e.message }})))) + ';path=/'; }} }})();"#, escaped, offset, search_id);
        win.eval(&js).map_err(|e| anyhow!("执行搜索 JS 失败: {}", e))?;
        for _ in 0..150 { tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; if app.get_webview_window("douyin-login").is_none() { return Err(anyhow!("登录窗口已关闭")); } if let Ok(cookies) = win.cookies() { for c in &cookies { if c.name() == "search_result" { let raw = c.value(); if raw.is_empty() { continue; } use base64::Engine; let decoded = base64::engine::general_purpose::STANDARD.decode(raw.as_bytes()).map_err(|e| anyhow!("base64: {}", e))?; let json_str = String::from_utf8(decoded).map_err(|e| anyhow!("utf8: {}", e))?; return Ok(serde_json::from_str(&json_str)?); } } } } Err(anyhow!("搜索超时（15秒无响应）"))
    }

    async fn update_progress<F>(&self, f: F)
    where
        F: FnOnce(&mut CrawlProgress),
    {
        let mut progress = self.progress.lock().await;
        f(&mut progress);
        let _ = self.progress_tx.send(progress.clone());
    }

    async fn get_progress(&self) -> CrawlProgress {
        self.progress.lock().await.clone()
    }

    async fn increment_progress<F>(&self, f: F)
    where
        F: FnOnce(&mut CrawlProgress),
    {
        let mut progress = self.progress.lock().await;
        f(&mut progress);
        let _ = self.progress_tx.send(progress.clone());
    }

    async fn add_error(&self, msg: String) {
        let mut progress = self.progress.lock().await;
        progress.errors.push(msg);
        let _ = self.progress_tx.send(progress.clone());
    }

    async fn sleep(&self) {
        if self.config.sleep_secs > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(self.config.sleep_secs as u64)).await;
        }
    }
}

/// 从视频详情提取图片列表
fn extract_image_list(aweme: &Value) -> Vec<String> {
    aweme.get("images")
        .and_then(|v| v.as_array())
        .map(|images| {
            images.iter()
                .filter_map(|img| img.get("url_list")
                    .and_then(|u| u.as_array())
                    .and_then(|u| u.first())
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// 从视频详情提取视频URL
fn extract_video_url(aweme: &Value) -> Option<String> {
    // 尝试多种路径
    let candidates = [
        aweme.get("video")?.get("play_addr")?.get("url_list")?.as_array()?.first()?.as_str()?,
        aweme.get("video")?.get("download_addr")?.get("url_list")?.as_array()?.first()?.as_str()?,
    ];
    candidates.into_iter().next().map(|s| s.to_string())
}
