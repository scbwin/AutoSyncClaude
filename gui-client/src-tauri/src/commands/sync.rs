use crate::config::ConfigManager;
use crate::state::SyncState;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use tracing::{debug, error, info, warn};

#[tauri::command]
pub async fn start_sync(
    mode: String,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<String, String> {
    let mut state = sync_state.lock().await;

    if state.is_syncing {
        return Err("同步已在运行中".to_string());
    }

    state.is_syncing = true;
    state.sync_mode = Some(mode.clone());
    state.synced_count = 0;
    state.failed_count = 0;
    state.progress = 0.0;

    // 释放锁，让后台任务可以运行
    drop(state);

    // 获取配置
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    // 获取 Claude 目录路径
    let claude_dir = config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let claude_dir = PathBuf::from(claude_dir);

    if !claude_dir.exists() {
        error!("Claude 目录不存在: {:?}", claude_dir);
        let mut state = sync_state.lock().await;
        state.is_syncing = false;
        return Err(format!("Claude 目录不存在: {:?}", claude_dir));
    }

    info!("开始 {} 同步，目录: {:?}", mode, claude_dir);

    // 克隆状态用于后台更新
    let sync_state_inner = sync_state.inner().clone();

    // 启动后台同步任务
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_sync_task(mode, claude_dir, sync_state_inner).await {
            error!("同步任务失败: {}", e);
        }
    });

    Ok(format!("已启动 {} 模式同步", mode))
}

/// 运行同步任务
async fn run_sync_task(
    _mode: String,
    claude_dir: PathBuf,
    sync_state: Arc<Mutex<SyncState>>,
) -> Result<(), String> {
    info!("同步任务开始运行");

    // 扫描文件
    let files = scan_claude_files(&claude_dir).await?;

    if files.is_empty() {
        warn!("没有找到可同步的文件");
        update_sync_state(&sync_state, 100.0, 0, 0).await;
        mark_sync_complete(&sync_state).await;
        return Ok(());
    }

    info!("找到 {} 个文件待同步", files.len());

    // 获取服务器地址
    // TODO: 从配置中获取实际的 gRPC 服务器地址并调用

    let total_files = files.len() as f64;

    // 模拟同步过程
    for (index, file_path) in files.iter().enumerate() {
        let progress = ((index + 1) as f64 / total_files) * 100.0;

        debug!("同步文件 [{}/{}]: {:?}", index + 1, files.len(), file_path);

        // TODO: 实际的文件同步逻辑
        // 1. 计算文件哈希
        // 2. 调用 gRPC 客户端上报文件变更
        // 3. 上传文件内容（如果需要）

        // 更新进度
        update_sync_state(&sync_state, progress, index + 1, 0).await;

        // 模拟处理延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    info!("同步完成: {} 个文件", files.len());

    mark_sync_complete(&sync_state).await;

    Ok(())
}

/// 扫描 Claude 目录中的文件
async fn scan_claude_files(claude_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    // 默认排除的目录
    let exclude_dirs = vec![
        "cache",
        "downloads",
        "image-cache",
        "file-history",
        "shell-snapshots",
        "statsig",
        "blob",
        "zdocs",
    ];

    // 默认排除的文件扩展名
    let exclude_exts = vec![
        "tmp", "log", "swp", "DS_Store", "pdb", "exe", "dll", "so", "dylib",
        "rlib", "rmeta", "o", "a", "lib",
    ];

    // 只包含的文件扩展名
    let allowed_exts = vec![
        "json", "md", "txt", "toml", "yaml", "yml", "rs", "js", "ts", "py",
        "sh", "bat", "zsh", "fish", "env", "proto",
    ];

    // 递归遍历目录
    let mut stack = vec![claude_dir.to_path_buf()];

    while let Some(current_dir) = stack.pop() {
        // 读取目录条目
        let entries = match std::fs::read_dir(&current_dir) {
            Ok(entries) => entries,
            Err(e) => {
                debug!("无法读取目录 {:?}: {}", current_dir, e);
                continue;
            }
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let file_name = entry.file_name();

            // 跳过隐藏文件
            if file_name.to_string_lossy().starts_with('.') {
                continue;
            }

            if path.is_dir() {
                // 检查是否在排除目录中
                let dir_name = file_name.to_string_lossy();
                if exclude_dirs.contains(&dir_name.as_ref()) {
                    debug!("跳过排除目录: {:?}", path);
                    continue;
                }
                stack.push(path);
            } else if path.is_file() {
                // 检查文件扩展名
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();

                    // 跳过排除的扩展名
                    if exclude_exts.contains(&ext_str.as_str()) {
                        debug!("跳过排除的文件: {:?}", path);
                        continue;
                    }

                    // 只包含允许的扩展名
                    if !allowed_exts.contains(&ext_str.as_str()) {
                        debug!("跳过不支持类型的文件: {:?}", path);
                        continue;
                    }

                    files.push(path);
                }
            }
        }
    }

    // 排序以获得一致的顺序
    files.sort();

    Ok(files)
}

/// 更新同步状态
async fn update_sync_state(
    sync_state: &Arc<Mutex<SyncState>>,
    progress: f64,
    synced_count: usize,
    failed_count: usize,
) {
    let mut state = sync_state.lock().await;
    state.progress = progress;
    state.synced_count = synced_count;
    state.failed_count = failed_count;
}

/// 标记同步完成
async fn mark_sync_complete(sync_state: &Arc<Mutex<SyncState>>) {
    let mut state = sync_state.lock().await;
    state.is_syncing = false;
    state.progress = 100.0;
    state.last_sync_time = Some(chrono::Utc::now());
}

#[tauri::command]
pub async fn stop_sync(
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    let mut state = sync_state.lock().await;
    state.is_syncing = false;
    state.sync_mode = None;

    // TODO: 停止正在运行的同步任务

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
