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
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;
    let _config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    // TODO: 调用实际的登录逻辑
    // 这里需要集成 client 模块中的登录功能

    let user_id = uuid::Uuid::new_v4().to_string();
    let device_id = uuid::Uuid::new_v4().to_string();

    // 保存用户信息到同步状态
    let mut state = sync_state.lock().await;
    state.set_user(user_id.clone(), device_id.clone());
    drop(state);

    let response = serde_json::json!({
        "user_id": user_id,
        "device_id": device_id,
        "access_token": "temp-token",
        "message": "登录成功"
    });

    Ok(response)
}

#[tauri::command]
pub async fn logout(
    _config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    // 清除用户信息
    let mut state = sync_state.lock().await;
    state.user_id = None;
    state.device_id = None;

    tracing::info!("用户已登出");

    Ok(())
}

#[tauri::command]
pub async fn get_status(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;

    let is_logged_in = manager.is_logged_in().await.map_err(|e| e.to_string())?;

    // 从 sync_state 获取用户信息
    let state = sync_state.lock().await;
    let user_id = state.user_id.clone().unwrap_or_default();
    let device_id = state.device_id.clone().unwrap_or_default();
    drop(state);

    let status = serde_json::json!({
        "logged_in": is_logged_in || !user_id.is_empty(),
        "user_id": user_id,
        "device_id": device_id
    });

    Ok(status)
}

/// 检查服务器连接状态
#[tauri::command]
pub async fn check_connection(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    let health_check_url = config["server"]["health_check_address"]
        .as_str()
        .unwrap_or("http://localhost:8181");

    let timeout = config["server"]["timeout"]
        .as_u64()
        .unwrap_or(5);

    // 构建健康检查 URL
    let url = format!("{}/health", health_check_url.trim_end_matches('/'));

    // 创建 HTTP 客户端并设置超时
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| format!("创建客户端失败: {}", e))?;

    // 发送健康检查请求
    let response = client
        .get(&url)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Ok(serde_json::json!({
                    "connected": true,
                    "message": "已连接",
                    "server": health_check_url
                }))
            } else {
                Ok(serde_json::json!({
                    "connected": false,
                    "message": format!("服务器返回错误状态: {}", status),
                    "server": health_check_url
                }))
            }
        }
        Err(e) => {
            Ok(serde_json::json!({
                "connected": false,
                "message": format!("连接失败: {}", e),
                "server": health_check_url
            }))
        }
    }
}
