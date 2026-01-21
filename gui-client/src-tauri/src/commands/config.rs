use crate::config::ConfigManager;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

#[tauri::command]
pub async fn init_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;
    manager
        .init_config()
        .await
        .map_err(|e| e.to_string())
        .and_then(|config| {
            serde_json::to_value(config)
                .map_err(|e| format!("序列化配置失败: {}", e))
        })
}

#[tauri::command]
pub async fn get_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;
    manager
        .get_config()
        .await
        .map_err(|e| e.to_string())
        .and_then(|config| {
            serde_json::to_value(config)
                .map_err(|e| format!("序列化配置失败: {}", e))
        })
}

#[tauri::command]
pub async fn update_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    config: Value,
) -> Result<(), String> {
    let manager = config_manager.lock().await;
    manager
        .update_config(config)
        .await
        .map_err(|e| e.to_string())
}
