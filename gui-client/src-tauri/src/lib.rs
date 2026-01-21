#![cfg_attr(mobile, tauri::mobile_entry_point)]

mod commands;
mod config;
mod grpc;
mod proto;
mod state;

use tauri::Manager;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use tracing_subscriber::prelude::*;

// 全局日志 Guard
static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

pub fn run() {
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
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // 窗口关闭时隐藏而不是退出（如果有托盘）
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(|app| {
            let handle = app.handle();
            let config_manager = Arc::new(Mutex::new(config::ConfigManager::new()));
            let sync_state = Arc::new(Mutex::new(state::SyncState::new()));

            app.manage(config_manager);
            app.manage(sync_state);

            // 创建系统托盘
            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let show = MenuItemBuilder::with_id("show", "显示").build(app)?;
            let hide = MenuItemBuilder::with_id("hide", "隐藏").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&show, &hide, &quit])
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "hide" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            tracing::info!("Claude Sync GUI 已启动");
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
