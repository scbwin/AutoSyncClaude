// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod grpc;
mod proto;
mod state;

use tauri::Manager;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use tracing_subscriber::prelude::*;

// 全局日志 Guard，确保其在程序整个生命周期内存活
static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

#[tokio::main]
async fn main() {
    // 初始化日志
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("claude-sync")
        .join("logs");

    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "claude-sync");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!("Claude Sync GUI 启动");

    tauri::Builder::default()
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // 正常关闭窗口
            }
            _ => {}
        })
        .setup(|app| {
            let handle = app.handle();
            let config_manager = Arc::new(Mutex::new(config::ConfigManager::new()));
            let sync_state = Arc::new(Mutex::new(state::SyncState::new()));

            app.manage(config_manager);
            app.manage(sync_state);

            // 启动后台任务
            let _app_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                tracing::info!("Claude Sync GUI 已启动");
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

    tracing::info!("Tauri 应用已退出");
}
