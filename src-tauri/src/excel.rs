/// Excel 导出模块 —— 将采集数据输出为 .xlsx 文件

use anyhow::{Context, Result};
use rust_xlsxwriter::{Format, Workbook, Worksheet};

use crate::crawler::orchestrator::CrawlProgress;

// ============================================================================
// 公共入口：在采集结束后生成 Excel
// ============================================================================

/// 从 aweme_details.jsonl 和评论数据生成 Excel
pub fn generate_excel(output_dir: &str, progress: &CrawlProgress) -> Result<()> {
    let mut wb = Workbook::new();

    // Sheet 1: 视频汇总
    let sheet1 = wb.add_worksheet();
    sheet1.set_name("视频汇总")?;
    write_video_sheet(sheet1, output_dir)?;

    if progress.fetched_comments > 0 {
        let sheet2 = wb.add_worksheet();
        sheet2.set_name("评论明细")?;
        write_comments_sheet(sheet2, output_dir)?;
    }

    let xlsx_path = format!("{}/采集汇总.xlsx", output_dir);
    wb.save(&xlsx_path)
        .with_context(|| format!("保存 Excel 失败: {}", xlsx_path))?;

    log::info!("Excel 已保存: {}", xlsx_path);
    Ok(())
}

// ============================================================================
// 视频汇总 Sheet
// ============================================================================

fn write_video_sheet(sheet: &mut Worksheet, output_dir: &str) -> Result<()> {
    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0x4472C4))
        .set_font_color(rust_xlsxwriter::Color::White);

    let headers = [
        "序号", "视频ID", "视频描述", "作者昵称", "作者ID",
        "点赞数", "评论数", "分享数", "收藏数", "播放数",
        "发布时间", "视频时长", "视频链接", "视频标签",
    ];

    // 设置列宽
    let col_widths = [6.0, 20.0, 40.0, 18.0, 22.0, 10.0, 10.0, 10.0, 10.0, 10.0, 18.0, 10.0, 40.0, 25.0];
    for (i, w) in col_widths.iter().enumerate() {
        sheet.set_column_width(i as u16, *w)?;
    }

    // 写入表头
    for (i, h) in headers.iter().enumerate() {
        sheet.write_with_format(0, i as u16, *h, &header_fmt)?;
    }

    // 读取 jsonl 文件
    let jsonl_path = format!("{}/aweme_details.jsonl", output_dir);
    let content = std::fs::read_to_string(&jsonl_path)
        .with_context(|| format!("读取 jsonl 文件失败: {}", jsonl_path))?;

    let mut row = 1u32;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let aweme: serde_json::Value = serde_json::from_str(line)?;

        // 序号
        sheet.write(row, 0, row)?;

        // 视频ID
        sheet.write(row, 1, str_val(&aweme, "aweme_id"))?;

        // 视频描述
        sheet.write(row, 2, str_val(&aweme, "desc"))?;

        // 作者昵称
        let author = aweme.get("author");
        sheet.write(row, 3, author.and_then(|a| a.get("nickname")).and_then(|v| v.as_str()).unwrap_or(""))?;

        // 作者ID
        sheet.write(row, 4, author.and_then(|a| a.get("sec_uid")).and_then(|v| v.as_str()).unwrap_or(""))?;

        // 统计数据
        let stats = aweme.get("statistics");
        let digg = stats.and_then(|s| s.get("digg_count")).and_then(|v| v.as_u64()).unwrap_or(0);
        let comment = stats.and_then(|s| s.get("comment_count")).and_then(|v| v.as_u64()).unwrap_or(0);
        let share = stats.and_then(|s| s.get("share_count")).and_then(|v| v.as_u64()).unwrap_or(0);
        let collect = stats.and_then(|s| s.get("collect_count")).and_then(|v| v.as_u64()).unwrap_or(0);
        let play = stats.and_then(|s| s.get("play_count")).and_then(|v| v.as_u64()).unwrap_or(0);

        sheet.write(row, 5, digg)?;
        sheet.write(row, 6, comment)?;
        sheet.write(row, 7, share)?;
        sheet.write(row, 8, collect)?;
        sheet.write(row, 9, play)?;

        // 发布时间
        let ts = aweme.get("create_time").and_then(|v| v.as_u64()).unwrap_or(0);
        if ts > 0 {
            let time_str = timestamp_to_str(ts);
            sheet.write(row, 10, &time_str)?;
        }

        // 视频时长 (ms → s)
        let duration = aweme.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);
        sheet.write(row, 11, format!("{}s", duration / 1000))?;

        // 视频链接
        let aweme_id = str_val(&aweme, "aweme_id");
        let link = format!("https://www.douyin.com/video/{}", aweme_id);
        sheet.write(row, 12, &link)?;

        // 视频标签
        let tags = aweme.get("text_extra")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.get("hashtag_name").and_then(|v| v.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        sheet.write(row, 13, &tags)?;

        row += 1;
    }

    Ok(())
}

// ============================================================================
// 评论明细 Sheet
// ============================================================================

fn write_comments_sheet(sheet: &mut Worksheet, output_dir: &str) -> Result<()> {
    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0x4472C4))
        .set_font_color(rust_xlsxwriter::Color::White);

    let headers = [
        "序号", "视频ID", "视频描述", "评论ID",
        "评论内容", "评论者昵称", "评论者ID", "点赞数",
        "回复数", "评论时间", "是否置顶",
    ];

    let col_widths = [6.0, 20.0, 40.0, 20.0, 50.0, 18.0, 22.0, 10.0, 10.0, 18.0, 8.0];
    for (i, w) in col_widths.iter().enumerate() {
        sheet.set_column_width(i as u16, *w)?;
    }

    for (i, h) in headers.iter().enumerate() {
        sheet.write_with_format(0, i as u16, *h, &header_fmt)?;
    }

    let mut row = 1u32;

    // 遍历每个视频目录
    let dirs = std::fs::read_dir(output_dir)
        .with_context(|| format!("读取输出目录失败: {}", output_dir))?;

    for entry in dirs {
        let entry = entry?;
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if !dir_name.starts_with("aweme_") {
            continue;
        }
        let aweme_id = dir_name.strip_prefix("aweme_").unwrap_or("");

        // 读取 detail.json 获取视频描述
        let detail_path = format!("{}/{}/detail.json", output_dir, dir_name);
        let desc = if let Ok(content) = std::fs::read_to_string(&detail_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                val.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // 读取 comments.json
        let comments_path = format!("{}/{}/comments.json", output_dir, dir_name);
        let comments_content = match std::fs::read_to_string(&comments_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let comments: serde_json::Value = match serde_json::from_str(&comments_content) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let comment_list = match comments.as_array() {
            Some(arr) => arr,
            None => continue,
        };

        for comment in comment_list {
            // 序号
            sheet.write(row, 0, row)?;

            // 视频ID
            sheet.write(row, 1, aweme_id)?;

            // 视频描述
            sheet.write(row, 2, &desc)?;

            // 评论ID
            sheet.write(row, 3, str_val(comment, "cid"))?;

            // 评论内容
            sheet.write(row, 4, str_val(comment, "text"))?;

            // 评论者
            let user = comment.get("user");
            sheet.write(row, 5, user.and_then(|u| u.get("nickname")).and_then(|v| v.as_str()).unwrap_or(""))?;
            sheet.write(row, 6, user.and_then(|u| u.get("sec_uid")).and_then(|v| v.as_str()).unwrap_or(""))?;

            // 点赞数
            let digg = comment.get("digg_count").and_then(|v| v.as_u64()).unwrap_or(0);
            sheet.write(row, 7, digg)?;

            // 回复数
            let reply = comment.get("reply_comment_total").and_then(|v| v.as_u64()).unwrap_or(0);
            sheet.write(row, 8, reply)?;

            // 评论时间
            let ts = comment.get("create_time").and_then(|v| v.as_u64()).unwrap_or(0);
            if ts > 0 {
                sheet.write(row, 9, timestamp_to_str(ts))?;
            }

            // 是否置顶
            let stick = comment.get("is_stick").and_then(|v| v.as_u64()).unwrap_or(0) == 1;
            sheet.write(row, 10, if stick { "是" } else { "否" })?;

            row += 1;
        }
    }

    Ok(())
}

// ============================================================================
// 辅助函数
// ============================================================================

fn str_val(val: &serde_json::Value, key: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn timestamp_to_str(ts: u64) -> String {
    use chrono::TimeZone;
    if let Some(dt) = chrono::Utc.timestamp_opt(ts as i64, 0).single() {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        String::new()
    }
}
