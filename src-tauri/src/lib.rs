/// Tauri 应用入口和命令定义

pub mod signer;
pub mod crawler;
pub mod storage;

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use serde_json::Value;

use crawler::client::DouyinClient;
use crawler::orchestrator::{CrawlConfig, CrawlerOrchestrator, CrawlProgress};

/// 全局应用状态
pub struct AppState {
    pub orchestrator: Arc<Mutex<Option<CrawlerOrchestrator>>>,
    /// 浏览器搜索结果暂存
    pub search_results: Arc<Mutex<Option<Value>>>,
    /// 登录窗口的 Cookie 字符串（供搜索使用）
    pub login_cookies: Arc<Mutex<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            orchestrator: Arc::new(Mutex::new(None)),
            search_results: Arc::new(Mutex::new(None)),
            login_cookies: Arc::new(Mutex::new(String::new())),
        }
    }
}

/// 启动爬取任务
#[tauri::command]
async fn start_crawl(
    cookies: String,
    user_agent: String,
    ms_token: Option<String>,
    config: CrawlConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    // 创建 HTTP 客户端
    let client = DouyinClient::new(cookies, user_agent, ms_token)
        .map_err(|e| format!("创建客户端失败: {}", e))?;

    // 创建进度通道
    let (tx, mut rx) = mpsc::unbounded_channel::<CrawlProgress>();

    // 创建爬虫编排器（带浏览器搜索支持）
    let orchestrator = CrawlerOrchestrator::with_browser(
        client, config, tx,
        Some(app.clone()),
        Some(state.search_results.clone()),
    );

    // 启动后台爬取任务
    let app_for_error = app.clone();
    tokio::spawn(async move {
        if let Err(e) = orchestrator.start().await {
            let _ = app_for_error.emit("crawl-error", format!("爬取出错: {}", e));
        }
    });

    // 监听进度事件
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            let _ = app.emit("crawl-progress", &progress);
        }
    });

    Ok("爬取任务已启动".to_string())
}

/// 停止爬取
#[tauri::command]
async fn stop_crawl() -> Result<String, String> {
    Ok("已请求停止".to_string())
}

/// 解析视频链接
#[tauri::command]
fn parse_video_url(url: String) -> Result<serde_json::Value, String> {
    match crawler::url_parser::parse_video_url(&url) {
        Ok(info) => Ok(serde_json::json!({
            "aweme_id": info.aweme_id,
            "url_type": format!("{:?}", info.url_type),
        })),
        Err(e) => Err(format!("解析失败: {}", e)),
    }
}

/// 解析创作者链接
#[tauri::command]
fn parse_creator_url(url: String) -> Result<serde_json::Value, String> {
    match crawler::url_parser::parse_creator_url(&url) {
        Ok(info) => Ok(serde_json::json!({
            "sec_user_id": info.sec_user_id,
        })),
        Err(e) => Err(format!("解析失败: {}", e)),
    }
}

/// 生成 webid
#[tauri::command]
fn generate_webid() -> String {
    signer::generate_webid()
}

/// 打开抖音扫码登录窗口
#[tauri::command]
async fn open_login_window(app: AppHandle) -> Result<String, String> {
    // 检查是否已有登录窗口
    if let Some(win) = app.get_webview_window("douyin-login") {
        let _ = win.set_focus();
        return Ok("登录窗口已打开".to_string());
    }

    // 创建新的登录窗口
    let login_window = WebviewWindowBuilder::new(
        &app,
        "douyin-login",
        WebviewUrl::External(
            "https://www.douyin.com/login_page?service=https%3A%2F%2Fwww.douyin.com"
                .parse()
                .map_err(|e| format!("URL 解析失败: {}", e))?,
        ),
    )
    .title("抖音扫码登录")
    .inner_size(680.0, 640.0)
    .resizable(false)
    .center()
    .build()
    .map_err(|e| format!("创建登录窗口失败: {}", e))?;

    // 监听窗口关闭
    let app_handle = app.clone();
    login_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            let _ = app_handle.emit("login-window-closed", ());
        }
    });

    Ok("登录窗口已打开，请扫码登录".to_string())
}

/// 关闭登录窗口
#[tauri::command]
async fn close_login_window(app: AppHandle) -> Result<String, String> {
    if let Some(win) = app.get_webview_window("douyin-login") {
        win.close().map_err(|e| format!("关闭窗口失败: {}", e))?;
    }
    Ok("登录窗口已关闭".to_string())
}

/// 从登录窗口提取 Cookie
#[tauri::command]
async fn get_login_cookies(app: AppHandle) -> Result<serde_json::Value, String> {
    if let Some(win) = app.get_webview_window("douyin-login") {
        // 使用 Tauri v2 的 cookies() 方法获取窗口 Cookie
        let cookies = win.cookies()
            .map_err(|e| format!("获取 Cookie 失败: {}", e))?;
        
        // 拼接为 cookie 字符串
        let cookie_str: String = cookies.iter()
            .map(|c| format!("{}={}", c.name(), c.value()))
            .collect::<Vec<_>>()
            .join("; ");
        
        // 精确匹配 Cookie 名称，避免 passport_auth_mix_state 误判
        let logged_in = cookies.iter().any(|c| {
            c.name() == "sessionid" || c.name() == "sid_tt"
        });
        
        Ok(serde_json::json!({
            "cookies": cookie_str,
            "logged_in": logged_in,
        }))
    } else {
        Err("登录窗口未打开".to_string())
    }
}

/// 接收浏览器搜索结果的回调命令
#[tauri::command]
async fn report_search_results(data: Value, state: State<'_, AppState>) -> Result<(), String> {
    let mut results = state.search_results.lock().await;
    *results = Some(data);
    Ok(())
}

/// 在登录窗口的浏览器环境中执行搜索（利用浏览器的完整签名环境）
#[tauri::command]
async fn browser_search(
    keyword: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    // 获取登录窗口
    let win = match app.get_webview_window("douyin-login") {
        Some(w) => w,
        None => return Err("登录窗口未打开，请先扫码登录或使用 Cookie 模式".to_string()),
    };
    
    // 清空之前的结果
    {
        let mut results = state.search_results.lock().await;
        *results = None;
    }
    
    // 转义关键词中的特殊字符
    let escaped_keyword = keyword.replace('\\', "\\\\").replace('\'', "\\'").replace('"', "\\\"");
    
    // 构建搜索 JS 代码
    let search_js = format!(r#"
(async () => {{
    try {{
        const params = new URLSearchParams({{
            search_channel: 'aweme_general',
            enable_history: '1',
            keyword: "{}",
            search_source: 'tab_search',
            query_correct_type: '1',
            is_filter_search: '0',
            offset: '0',
            count: '15',
            need_filter_settings: '1',
            list_type: 'multi',
        }});
        
        const resp = await fetch('/aweme/v1/web/general/search/single/?' + params, {{
            headers: {{ 'Accept': 'application/json' }},
            credentials: 'include',
        }});
        
        const data = await resp.json();
        window.__TAURI__.core.invoke('report_search_results', {{ data }});
    }} catch (e) {{
        window.__TAURI__.core.invoke('report_search_results', {{ 
            data: {{ error: e.message, status: 'js_error' }} 
        }});
    }}
}})();
"#, escaped_keyword);
    
    // 在登录窗口执行搜索
    win.eval(&search_js).map_err(|e| format!("执行搜索 JS 失败: {}", e))?;
    
    // 等待结果（最多 15 秒）
    for i in 0..150 {{
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let results = state.search_results.lock().await;
        if let Some(ref data) = *results {{
            return Ok(data.clone());
        }}
        if i % 30 == 29 {{
            // 每 3 秒检查一次窗口是否还在
            if app.get_webview_window("douyin-login").is_none() {{
                return Err("登录窗口已关闭".to_string());
            }}
        }}
    }}
    
    Err("搜索超时（15秒无响应）".to_string())
}

/// 选择输出目录
#[tauri::command]
async fn pick_directory(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let dir = app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string());
    Ok(dir)
}

/// 打开输出目录
#[tauri::command]
async fn open_output_dir(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(&path).spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(&path).spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    Ok(())
}

/// 启动 Tauri 应用
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            start_crawl,
            stop_crawl,
            parse_video_url,
            parse_creator_url,
            generate_webid,
            open_login_window,
            close_login_window,
            get_login_cookies,
            browser_search,
            report_search_results,
            pick_directory,
            open_output_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
