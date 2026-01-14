use crate::state::SyncState;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

#[tauri::command]
pub async fn start_sync(
    mode: String,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<String, String> {
    let mut state = sync_state.lock().await;

    if state.is_syncing {
        return Err("同步已在运行中".to_string());
    }

    state.is_syncing = true;
    state.sync_mode = Some(mode.clone());

    // TODO: 启动实际的同步任务
    // 这里需要集成 client 模块中的同步引擎

    Ok(format!("已启动 {} 模式同步", mode))
}

#[tauri::command]
pub async fn stop_sync(
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    let mut state = sync_state.lock().await;
    state.is_syncing = false;
    state.sync_mode = None;

    // TODO: 停止同步任务

    Ok(())
}

#[tauri::command]
pub async fn get_sync_status(
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Value, String> {
    let state = sync_state.lock().await;

    let status = serde_json::json!({
        "is_syncing": state.is_syncing,
        "mode": state.sync_mode,
        "last_sync": state.last_sync_time,
        "synced_files": state.synced_count,
        "failed_files": state.failed_count,
        "progress": state.progress
    });

    Ok(status)
}
