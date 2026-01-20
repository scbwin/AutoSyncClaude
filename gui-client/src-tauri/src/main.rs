// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod grpc;
mod proto;
mod state;

use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    // 初始化日志（设置日志级别为 DEBUG 以便调试）
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    tauri::Builder::default()
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
