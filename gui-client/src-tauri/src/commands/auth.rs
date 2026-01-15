use crate::config::ConfigManager;
use crate::state::SyncState;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

#[tauri::command]
pub async fn login(
    _email: String,
    _password: String,
    _device_name: Option<String>,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    _sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;
    let _config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    // TODO: 调用实际的登录逻辑
    // 这里需要集成 client 模块中的登录功能

    let response = serde_json::json!({
        "user_id": "temp-user-id",
        "device_id": "temp-device-id",
        "access_token": "temp-token",
        "message": "登录成功"
    });

    Ok(response)
}

#[tauri::command]
pub async fn logout(
    _config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    _sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    // TODO: 实现登出逻辑
    Ok(())
}

#[tauri::command]
pub async fn get_status(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;

    let is_logged_in = manager.is_logged_in().await.map_err(|e| e.to_string())?;

    let status = serde_json::json!({
        "logged_in": is_logged_in,
        "user_id": manager.get_user_id().await.unwrap_or_default(),
        "device_id": manager.get_device_id().await.unwrap_or_default()
    });

    Ok(status)
}
