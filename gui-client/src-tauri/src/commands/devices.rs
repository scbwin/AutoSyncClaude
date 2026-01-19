use crate::config::ConfigManager;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use uuid::Uuid;

/// 设备信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub is_current: bool,
    pub last_seen: String,
    pub status: String,
}

#[tauri::command]
pub async fn list_devices(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    // 获取或生成设备 ID
    let device_id = get_or_create_device_id(&config).await?;

    // 获取设备名称
    let device_name = get_device_name().await;

    // 当前设备
    let current_device = DeviceInfo {
        id: device_id.clone(),
        name: device_name.clone(),
        device_type: get_device_type().await,
        is_current: true,
        last_seen: chrono::Utc::now().to_rfc3339(),
        status: "online".to_string(),
    };

    // TODO: 从服务器获取其他设备列表
    // 目前只返回当前设备
    let devices = vec![current_device];

    Ok(serde_json::json!({
        "devices": devices
    }))
}

/// 获取或创建设备 ID
async fn get_or_create_device_id(config: &Value) -> Result<String, String> {
    // 尝试从配置中读取设备 ID
    if let Some(device_id) = config.get("device_id").and_then(|v| v.as_str()) {
        if !device_id.is_empty() {
            return Ok(device_id.to_string());
        }
    }

    // 如果没有，创建新的设备 ID
    let device_id = Uuid::new_v4().to_string();

    // TODO: 保存到配置文件

    Ok(device_id)
}

/// 获取设备名称
async fn get_device_name() -> String {
    // 获取计算机名称
    match std::env::var("COMPUTERNAME") {
        Ok(name) => format!("{}-Claude-Sync", name),
        Err(_) => {
            // Windows 失败，尝试其他方法
            match gethostname::gethostname().to_str() {
                Some(hostname) => format!("{}-Claude-Sync", hostname),
                None => "Unknown-Device".to_string(),
            }
        }
    }
}

/// 获取设备类型
async fn get_device_type() -> String {
    if cfg!(windows) {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    }
}
