use serde_json::Value;

#[tauri::command]
pub async fn list_devices() -> Result<Value, String> {
    // TODO: 实现获取设备列表的逻辑
    let devices = serde_json::json!({
        "devices": []
    });

    Ok(devices)
}
