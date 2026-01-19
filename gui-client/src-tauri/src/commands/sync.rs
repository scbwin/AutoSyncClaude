use crate::config::ConfigManager;
use crate::state::SyncState;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use tracing::{debug, error, info, warn};

/// 文件同步状态
#[derive(Debug, Clone, serde::Serialize)]
struct FileSyncInfo {
    path: String,
    hash: String,
    size: u64,
    modified: bool,
}

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
    let mode_clone = mode.clone();

    // 启动后台同步任务
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_sync_task(mode_clone, claude_dir, sync_state_inner).await {
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

    // 加载本地文件状态缓存
    let cached_states = load_file_cache(&claude_dir)?;

    // 扫描文件并计算哈希
    let files_with_hash = scan_files_with_hash(&claude_dir).await?;

    if files_with_hash.is_empty() {
        warn!("没有找到可同步的文件");
        update_sync_state(&sync_state, 100.0, 0, 0).await;
        mark_sync_complete(&sync_state, &claude_dir, 0).await?;
        return Ok(());
    }

    let total_files_count = files_with_hash.len();

    // 找出需要同步的文件（哈希不同的文件）
    let files_to_sync: Vec<_> = files_with_hash
        .into_iter()
        .filter(|(path, hash, _size)| {
            cached_states
                .get(path)
                .map_or(true, |cached| cached != hash)
        })
        .collect();

    info!("总文件数: {}, 需要同步: {}", total_files_count, files_to_sync.len());

    if files_to_sync.is_empty() {
        info!("所有文件都是最新的，无需同步");
        update_sync_state(&sync_state, 100.0, 0, 0).await;
        mark_sync_complete(&sync_state, &claude_dir, 0).await?;
        return Ok(());
    }

    let total_files = files_to_sync.len() as f64;

    // 同步有变化的文件
    for (index, (file_path, _hash, _size)) in files_to_sync.iter().enumerate() {
        let progress = ((index + 1) as f64 / total_files) * 100.0;

        debug!("同步文件 [{}/{}]: {:?}", index + 1, files_to_sync.len(), file_path);

        // TODO: 实际的文件同步逻辑
        // 1. 调用 gRPC 客户端上报文件变更
        // 2. 上传文件内容（如果需要）

        // 更新进度
        update_sync_state(&sync_state, progress, index + 1, 0).await;

        // 模拟处理延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // 同步完成后重新扫描并更新缓存
    let all_files = scan_files_with_hash(&claude_dir).await?;
    save_file_cache(&claude_dir, &all_files)?;

    info!("同步完成: {} 个文件", files_to_sync.len());

    mark_sync_complete(&sync_state, &claude_dir, files_to_sync.len()).await?;

    Ok(())
}

/// 获取待同步文件列表
#[tauri::command]
pub async fn get_pending_files(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Value, String> {
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
        return Ok(serde_json::json!([]));
    }

    // 加载本地文件状态缓存
    let cached_states = load_file_cache(&claude_dir).unwrap_or_default();

    // 扫描文件并计算哈希
    let files_with_hash = scan_files_with_hash(&claude_dir).await?;

    // 找出需要同步的文件
    let pending_files: Vec<FileSyncInfo> = files_with_hash
        .into_iter()
        .map(|(path, hash, size)| {
            let cached_hash = cached_states.get(&path);
            FileSyncInfo {
                path: path.clone(),
                hash: hash.clone(),
                size,
                modified: cached_hash.map_or(true, |h| h != &hash),
            }
        })
        .collect();

    Ok(serde_json::to_value(pending_files).unwrap())
}

/// 扫描文件并计算哈希
async fn scan_files_with_hash(claude_dir: &Path) -> Result<Vec<(String, String, u64)>, String> {
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

                    // 计算文件哈希
                    if let Ok((hash, size)) = calculate_file_hash(&path) {
                        // 获取相对路径
                        if let Ok(rel_path) = path.strip_prefix(claude_dir) {
                            let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");
                            files.push((rel_path_str, hash, size));
                        }
                    }
                }
            }
        }
    }

    // 排序以获得一致的顺序
    files.sort();

    Ok(files)
}

/// 计算文件哈希
fn calculate_file_hash(path: &Path) -> Result<(String, u64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("无法读取文件元数据: {}", e))?;
    let size = metadata.len();

    // 大文件跳过哈希计算（超过 10MB）
    if size > 10 * 1024 * 1024 {
        return Ok((format!("large_{}", size), size));
    }

    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("无法打开文件: {}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let n = file.read(&mut buffer)
            .map_err(|e| format!("读取文件失败: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash = format!("{:x}", hasher.finalize());
    Ok((hash, size))
}

/// 文件缓存文件路径
fn cache_file_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join(".sync-cache.json")
}

/// 加载文件缓存
fn load_file_cache(claude_dir: &Path) -> Result<HashMap<String, String>, String> {
    let cache_path = cache_file_path(claude_dir);

    if !cache_path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&cache_path)
        .map_err(|e| format!("无法读取缓存文件: {}", e))?;

    let cache: HashMap<String, String> = serde_json::from_str(&content)
        .map_err(|e| format!("无法解析缓存文件: {}", e))?;

    Ok(cache)
}

/// 保存文件缓存
fn save_file_cache(claude_dir: &Path, files: &[(String, String, u64)]) -> Result<(), String> {
    let cache_path = cache_file_path(claude_dir);

    let cache: HashMap<String, String> = files
        .iter()
        .map(|(path, hash, _)| (path.clone(), hash.clone()))
        .collect();

    let content = serde_json::to_string_pretty(&cache)
        .map_err(|e| format!("无法序列化缓存: {}", e))?;

    std::fs::write(&cache_path, content)
        .map_err(|e| format!("无法写入缓存文件: {}", e))?;

    Ok(())
}

/// 扫描 Claude 目录中的文件（已弃用，使用 scan_files_with_hash）
async fn scan_claude_files(claude_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let files_with_hash = scan_files_with_hash(claude_dir).await?;
    Ok(files_with_hash
        .into_iter()
        .map(|(path, _hash, _size)| claude_dir.join(path.replace('/', "\\")))
        .collect())
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
async fn mark_sync_complete(
    sync_state: &Arc<Mutex<SyncState>>,
    claude_dir: &Path,
    synced_count: usize,
) -> Result<(), String> {
    let now = chrono::Utc::now();
    let mut state = sync_state.lock().await;
    state.is_syncing = false;
    state.progress = 100.0;
    state.last_sync_time = Some(now);
    state.synced_count = synced_count;

    // 保存到配置文件
    save_sync_state(claude_dir, now, synced_count)?;

    Ok(())
}

/// 保存同步状态到配置文件
fn save_sync_state(claude_dir: &Path, last_sync: chrono::DateTime<Utc>, synced_count: usize) -> Result<(), String> {
    let state_path = claude_dir.join(".sync-state.json");

    let state_data = serde_json::json!({
        "last_sync": last_sync.to_rfc3339(),
        "synced_count": synced_count
    });

    let content = serde_json::to_string_pretty(&state_data)
        .map_err(|e| format!("无法序列化同步状态: {}", e))?;

    std::fs::write(&state_path, content)
        .map_err(|e| format!("无法写入同步状态: {}", e))?;

    Ok(())
}

/// 加载同步状态从配置文件
pub fn load_sync_state(claude_dir: &Path) -> Result<(Option<chrono::DateTime<Utc>>, usize), String> {
    let state_path = claude_dir.join(".sync-state.json");

    if !state_path.exists() {
        return Ok((None, 0));
    }

    let content = std::fs::read_to_string(&state_path)
        .map_err(|e| format!("无法读取同步状态: {}", e))?;

    let state: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("无法解析同步状态: {}", e))?;

    let last_sync = state["last_sync"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());

    let synced_count = state["synced_count"]
        .as_u64()
        .unwrap_or(0) as usize;

    Ok((last_sync, synced_count))
}

#[tauri::command]
pub async fn load_sync_state_from_disk(
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<(), String> {
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    // 获取 Claude 目录路径
    let claude_dir = config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let claude_dir = PathBuf::from(claude_dir);

    // 加载持久化的状态
    let (last_sync, synced_count) = load_sync_state(&claude_dir)?;

    let mut state = sync_state.lock().await;
    state.load_persistent(last_sync, synced_count);

    tracing::info!("加载同步状态: last_sync={:?}, synced_count={}", last_sync, synced_count);

    Ok(())
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
