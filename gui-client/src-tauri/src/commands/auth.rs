use crate::config::{AuthTokens, ConfigManager};
use crate::grpc::auth_client::AuthClient;
use crate::state::SyncState;
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

/// 获取设备类型
fn get_device_type() -> String {
    #[cfg(target_os = "windows")]
    return "Windows".to_string();
    #[cfg(target_os = "macos")]
    return "MacOS".to_string();
    #[cfg(target_os = "linux")]
    return "Linux".to_string();
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return "Other".to_string();
}

/// 获取设备指纹
fn get_device_fingerprint() -> String {
    use sha2::{Digest, Sha256};
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let mut hasher = Sha256::new();
    hasher.update(hostname.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 保存认证 Token
async fn save_tokens(
    config_manager: &Arc<Mutex<ConfigManager>>,
    result: &crate::grpc::auth_client::LoginResult,
) -> Result<(), String> {
    let manager = config_manager.lock().await;
    let tokens = AuthTokens {
        access_token: result.access_token.clone(),
        refresh_token: result.refresh_token.clone(),
        user_id: result.user_id.clone(),
        device_id: result.device_id.clone(),
        expires_at: result.expires_at,
    };
    manager
        .save_auth_tokens(tokens)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 清除认证 Token
async fn clear_tokens(config_manager: &Arc<Mutex<ConfigManager>>) -> Result<(), String> {
    let manager = config_manager.lock().await;
    manager
        .clear_auth_tokens()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 用户注册
#[tauri::command]
pub async fn register(
    username: String,
    email: String,
    password: String,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Value, String> {
    // 验证输入
    if username.len() < 3 {
        return Err("用户名至少需要3个字符".to_string());
    }
    if username.len() > 20 {
        return Err("用户名最多20个字符".to_string());
    }
    if !email.contains('@') || !email.contains('.') {
        return Err("请输入有效的邮箱地址".to_string());
    }
    if password.len() < 8 {
        return Err("密码至少需要8个字符".to_string());
    }

    // 获取服务器地址
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    let server_address = config["server"]["address"]
        .as_str()
        .unwrap_or("http://localhost:50051")
        .to_string();
    drop(manager);

    // 调用 gRPC 注册
    let mut client = AuthClient::new(server_address);
    let result = client
        .register(username.clone(), email.clone(), password)
        .await?;

    if !result.success {
        return Err(result.message);
    }

    tracing::info!("用户注册成功: {}", username);

    Ok(json!({
        "success": true,
        "message": result.message,
        "user_id": result.user_id
    }))
}

/// 用户登录
#[tauri::command]
pub async fn login(
    email: String,
    password: String,
    device_name: Option<String>,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Value, String> {
    // 验证输入
    if !email.contains('@') || !email.contains('.') {
        return Err("请输入有效的邮箱地址".to_string());
    }
    if password.is_empty() {
        return Err("请输入密码".to_string());
    }

    // 获取服务器地址
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    let server_address = config["server"]["address"]
        .as_str()
        .unwrap_or("http://localhost:50051")
        .to_string();
    drop(manager);

    // 设备信息
    let device_name = device_name
        .unwrap_or_else(|| gethostname::gethostname().to_string_lossy().to_string());
    let device_type = get_device_type();
    let device_fingerprint = get_device_fingerprint();

    // 调用 gRPC 登录
    let mut client = AuthClient::new(server_address);
    let result = client
        .login(email.clone(), password, device_name, device_type, device_fingerprint)
        .await?;

    if !result.success {
        return Err(result.message);
    }

    // 保存 tokens
    save_tokens(&config_manager, &result).await?;

    // 更新状态
    let mut state = sync_state.lock().await;
    state.set_user(result.user_id.clone(), result.device_id.clone());
    drop(state);

    tracing::info!("用户登录成功: {}", email);

    Ok(json!({
        "success": true,
        "user_id": result.user_id,
        "device_id": result.device_id,
        "message": "登录成功"
    }))
}

/// 用户登出
#[tauri::command]
pub async fn logout(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    // 尝试调用服务器登出（如果需要）
    // let manager = config_manager.lock().await;
    // if let Ok(Some(tokens)) = manager.get_auth_tokens().await {
    //     let config = manager.get_config().await.map_err(|e| e.to_string())?;
    //     let server_address = config["server"]["address"].as_str().unwrap_or("http://localhost:50051");
    //     let mut client = AuthClient::new(server_address.to_string());
    //     let _ = client.logout(tokens.refresh_token).await;
    // }

    // 清除本地 token
    clear_tokens(&config_manager).await?;

    // 清除状态
    let mut state = sync_state.lock().await;
    state.user_id = None;
    state.device_id = None;

    tracing::info!("用户已登出");

    Ok(())
}

/// 获取登录状态
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

    // 只有当 user_id 非空时才认为是已登录
    let actually_logged_in = is_logged_in && !user_id.is_empty();

    // 将空字符串转换为 null，方便前端处理
    let user_id_value = if user_id.is_empty() { serde_json::Value::Null } else { json!(user_id) };
    let device_id_value = if device_id.is_empty() { serde_json::Value::Null } else { json!(device_id) };

    let status = json!({
        "logged_in": actually_logged_in,
        "user_id": user_id_value,
        "device_id": device_id_value
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
        .unwrap_or("http://localhost:8080");

    let timeout = config["server"]["timeout"].as_u64().unwrap_or(5);

    // 构建健康检查 URL
    let url = format!("{}/health", health_check_url.trim_end_matches('/'));

    // 创建 HTTP 客户端并设置超时
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| format!("创建客户端失败: {}", e))?;

    // 发送健康检查请求
    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Ok(json!({
                    "connected": true,
                    "message": "已连接",
                    "server": health_check_url
                }))
            } else {
                Ok(json!({
                    "connected": false,
                    "message": format!("服务器返回错误状态: {}", status),
                    "server": health_check_url
                }))
            }
        }
        Err(e) => Ok(json!({
            "connected": false,
            "message": format!("连接失败: {}", e),
            "server": health_check_url
        })),
    }
}
