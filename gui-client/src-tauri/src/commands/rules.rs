use crate::config::ConfigManager;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

#[tauri::command]
pub async fn list_rules(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Value, String> {
    let manager = config_manager.lock().await;
    let rules = manager.get_rules().await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(rules).unwrap())
}

#[tauri::command]
pub async fn add_rule(
    name: String,
    rule_type: String,
    pattern: String,
    file_type: Option<String>,
    priority: i32,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<(), String> {
    let manager = config_manager.lock().await;
    manager
        .add_rule(name, rule_type, pattern, file_type, priority)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_rule(
    rule_id: String,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<(), String> {
    let manager = config_manager.lock().await;
    manager
        .remove_rule(rule_id)
        .await
        .map_err(|e| e.to_string())
}
