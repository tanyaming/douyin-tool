import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ===== 类型定义 =====

export interface CrawlConfig {
  mode: "Search" | "Detail" | "Creator";
  keywords?: string[];
  video_urls?: string[];
  creator_urls?: string[];
  max_videos: number;
  max_comments: number;
  enable_comments: boolean;
  enable_sub_comments: boolean;
  enable_media: boolean;
  sort_type?: "General" | "MostLike" | "Latest";
  publish_time?: "Unlimited" | "OneDay" | "OneWeek" | "SixMonths";
  output_dir: string;
  sleep_secs: number;
}

export interface CrawlProgress {
  status: "Idle" | "Running" | "Paused" | "Completed" | { Error: string };
  current_keyword: string;
  total_videos: number;
  fetched_videos: number;
  total_comments: number;
  fetched_comments: number;
  downloaded_media: number;
  errors: string[];
  output_dir: string;
}

export interface LoginCookies {
  cookies: string;
  logged_in: boolean;
}

// ===== API 调用 =====

export async function startCrawl(
  cookies: string,
  userAgent: string,
  msToken: string | null,
  config: CrawlConfig
): Promise<string> {
  return await invoke("start_crawl", {
    cookies,
    userAgent,
    msToken,
    config,
  });
}

export async function stopCrawl(): Promise<string> {
  return await invoke("stop_crawl");
}

export async function parseVideoUrl(url: string): Promise<{
  aweme_id: string;
  url_type: string;
}> {
  return await invoke("parse_video_url", { url });
}

export async function parseCreatorUrl(url: string): Promise<{
  sec_user_id: string;
}> {
  return await invoke("parse_creator_url", { url });
}

export async function openLoginWindow(): Promise<string> {
  return await invoke("open_login_window");
}

export async function closeLoginWindow(): Promise<string> {
  return await invoke("close_login_window");
}

export async function getLoginCookies(): Promise<LoginCookies> {
  return await invoke("get_login_cookies");
}

export async function pickDirectory(): Promise<string | null> {
  return await invoke("pick_directory");
}

export async function openOutputDir(path: string): Promise<void> {
  return await invoke("open_output_dir", { path });
}

// ===== 事件监听 =====

export function onProgress(
  callback: (progress: CrawlProgress) => void
): Promise<() => void> {
  return listen<CrawlProgress>("crawl-progress", (event) => {
    callback(event.payload);
  });
}

export function onError(callback: (error: string) => void): Promise<() => void> {
  return listen<string>("crawl-error", (event) => {
    callback(event.payload);
  });
}
