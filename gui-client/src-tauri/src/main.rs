// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod grpc;
mod proto;
mod state;

use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() {
    // 初始化日志 - 同时输出到控制台和文件
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("claude-sync")
        .join("logs");

    // 创建日志目录
    std::fs::create_dir_all(&log_dir).unwrap_or_else(|e| {
        eprintln!("Failed to create log directory: {}", e);
    });

    // 文件日志（每天轮转）
    let file_appender = tracing_appender::rolling::daily(&log_dir, "claude-sync");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 确保 guard 在程序运行期间保持存活，防止日志提前停止
    std::mem::forget(guard);

    // 使用 Registry 组合多个 layer：控制台日志 + 文件日志
    tracing_subscriber::registry()
        .with(
            // 控制台日志层
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .with_writer(std::io::stdout),
        )
        .with(
            // 文件日志层
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_writer(non_blocking),
        )
        .init();

    // 记录日志目录位置，方便用户查找
    println!("Log directory: {}", log_dir.display());

    // 创建系统托盘菜单
    let show = CustomMenuItem::new("show", "显示窗口");
    let hide = CustomMenuItem::new("hide", "隐藏到托盘");
    let quit = CustomMenuItem::new("quit", "退出");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    tauri::Builder::default()
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick {
                position: _,
                size: _,
                ..
            } => {
                // 左键点击托盘图标：显示窗口
                let window = app.get_window("main").unwrap();
                window.show().unwrap();
                window.set_focus().unwrap();
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "show" => {
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
                "hide" => {
                    let window = app.get_window("main").unwrap();
                    window.hide().unwrap();
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            },
            _ => {}
        })
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // 窗口关闭请求：隐藏到托盘而不是真正关闭
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(|app| {
            // 初始化应用状态
            let handle = app.handle();
            let config_manager = Arc::new(Mutex::new(config::ConfigManager::new()));
            let sync_state = Arc::new(Mutex::new(state::SyncState::new()));

            // 存储到应用状态
            app.manage(config_manager);
            app.manage(sync_state);

            // 启动后台同步任务（如果配置了自动启动）
            let _app_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                // TODO: 根据配置决定是否自动启动同步
                tracing::info!("GUI 应用已启动");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::init_config,
            commands::config::get_config,
            commands::config::update_config,
            commands::auth::register,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_status,
            commands::auth::check_connection,
            commands::sync::start_sync,
            commands::sync::stop_sync,
            commands::sync::get_sync_status,
            commands::sync::get_pending_files,
            commands::sync::load_sync_state_from_disk,
            commands::sync::get_file_tree,
            commands::sync::get_ignore_patterns,
            commands::sync::add_ignore_pattern,
            commands::sync::remove_ignore_pattern,
            commands::sync::delete_file_from_server,
            commands::sync::get_debug_info,
            commands::rules::list_rules,
            commands::rules::add_rule,
            commands::rules::remove_rule,
            commands::devices::list_devices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
