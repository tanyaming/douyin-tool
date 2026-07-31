/// 存储模块
///
/// 负责数据输出到 JSONL 文件，支持：
/// - 视频详情 JSONL
/// - 评论 JSONL
/// - 数据汇总和导出

use std::fs::{self, OpenOptions};
use std::io::Write;
use serde_json::Value;
use anyhow::Result;

pub struct JsonlStore {
    base_dir: String,
}

impl JsonlStore {
    pub fn new(base_dir: String) -> Self {
        Self { base_dir }
    }

    /// 确保目录存在
    fn ensure_dir(&self, sub: &str) -> Result<String> {
        let dir = if sub.is_empty() {
            self.base_dir.clone()
        } else {
            format!("{}/{}", self.base_dir, sub)
        };
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// 写入 JSONL 文件（追加模式）
    pub fn append(&self, filename: &str, data: &Value) -> Result<()> {
        let dir = self.ensure_dir("")?;
        let path = format!("{}/{}", dir, filename);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let line = serde_json::to_string(data)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// 批量写入 JSONL
    pub fn append_batch(&self, filename: &str, data_list: &[Value]) -> Result<()> {
        for data in data_list {
            self.append(filename, data)?;
        }
        Ok(())
    }

    /// 写入 JSON 文件（覆盖模式）
    pub fn write_json(&self, subdir: &str, filename: &str, data: &Value) -> Result<()> {
        let dir = self.ensure_dir(subdir)?;
        let path = format!("{}/{}", dir, filename);
        fs::write(&path, serde_json::to_string_pretty(data)?)?;
        Ok(())
    }

    /// 写入二进制文件
    pub fn write_binary(&self, subdir: &str, filename: &str, data: &[u8]) -> Result<()> {
        let dir = self.ensure_dir(subdir)?;
        let path = format!("{}/{}", dir, filename);
        fs::write(&path, data)?;
        Ok(())
    }

    /// 获取输出目录
    pub fn output_dir(&self) -> &str {
        &self.base_dir
    }
}

/// 汇总统计
#[derive(Debug, serde::Serialize)]
pub struct DataSummary {
    pub total_videos: u32,
    pub total_comments: u32,
    pub total_media_files: u32,
    pub output_dir: String,
}

impl DataSummary {
    pub fn from_dir(base_dir: &str) -> Result<Self> {
        let mut total_videos = 0u32;
        let mut total_comments = 0u32;
        let mut total_media = 0u32;

        // 统计 JSONL 行数
        let jsonl_path = format!("{}/aweme_details.jsonl", base_dir);
        if let Ok(content) = fs::read_to_string(&jsonl_path) {
            total_videos = content.lines().count() as u32;
        }

        // 统计媒体文件
        for entry in fs::read_dir(base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if dir_name.starts_with("aweme_") {
                    for file in fs::read_dir(&path)? {
                        let file = file?;
                        let name = file.file_name().to_str().unwrap_or("").to_string();
                        if name.ends_with(".jpeg") || name.ends_with(".mp4") {
                            total_media += 1;
                        }
                        if name == "comments.json" {
                            if let Ok(content) = fs::read_to_string(file.path()) {
                                total_comments += content.matches('\n').count() as u32;
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            total_videos,
            total_comments,
            total_media_files: total_media,
            output_dir: base_dir.to_string(),
        })
    }
}
