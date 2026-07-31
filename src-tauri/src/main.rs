// 防止控制台窗口在 Windows 上出现
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    douyin_collector_lib::run();
}
