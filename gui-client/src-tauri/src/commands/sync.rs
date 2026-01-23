use crate::config::ConfigManager;
use crate::state::SyncState;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use tracing::{debug, error, info, warn};
use chrono::{DateTime, Utc};

/// 默认排除的目录
const EXCLUDE_DIRS: &[&str] = &[
    "cache",
    "downloads",
    "image-cache",
    "file-history",
    "shell-snapshots",
    "statsig",
    "blob",
    "zdocs",
];

/// 默认排除的文件扩展名
const EXCLUDE_EXTS: &[&str] = &[
    "tmp", "log", "swp", "DS_Store", "pdb", "exe", "dll", "so", "dylib",
    "rlib", "rmeta", "o", "a", "lib",
];

/// 只包含的文件扩展名
const ALLOWED_EXTS: &[&str] = &[
    "json", "md", "txt", "toml", "yaml", "yml", "rs", "js", "ts", "py",
    "sh", "bat", "zsh", "fish", "env", "proto",
];

/// 支持的同步模式
const VALID_SYNC_MODES: &[&str] = &["auto", "manual", "bidirectional", "full", "incremental"];

/// 文件同步状态（已弃用，保留用于兼容）
#[derive(Debug, Clone, serde::Serialize)]
struct FileSyncInfo {
    path: String,
    hash: String,
    size: u64,
    modified: bool,
}

/// 节点类型
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    File,
    Directory,
}

/// 同步状态
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Synced,       // 已同步（本地和服务器都有）
    Pending,      // 待上传（本地有修改）
    NotOnServer,  // 仅本地（本地有，服务器没有）
    OnlyOnServer, // 仅服务器（服务器有，本地没有）
}

/// 文件树节点
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub node_type: NodeType,
    pub sync_status: SyncStatus,
    pub size: u64,
    pub hash: String,
    pub children: Vec<FileTreeNode>,
    pub checked: bool,
    pub exists_on_server: bool,
}

/// 忽略模式信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct IgnorePattern {
    pub pattern: String,
    pub is_active: bool,
}

#[tauri::command]
pub async fn start_sync(
    mode: String,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<String, String> {
    // 验证同步模式
    if !VALID_SYNC_MODES.contains(&mode.as_str()) {
        return Err(format!("无效的同步模式: {}。支持的模式: {}", mode, VALID_SYNC_MODES.join(", ")));
    }

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

    // 获取用户 ID
    let (user_id, device_id) = {
        let state = sync_state.lock().await;
        (
            state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| {
                error!("用户未登录，无法同步");
                "guest".to_string()
            }),
            state.get_device_id().map(|s| s.to_string()).unwrap_or_else(|| {
                uuid::Uuid::new_v4().to_string()
            }),
        )
    };

    info!("使用用户 ID: {}, 设备 ID: {}", user_id, device_id);

    // 加载本地文件状态缓存（按用户隔离）
    let cached_states = load_file_cache(&claude_dir, &user_id)?;

    // 扫描文件并计算哈希
    let files_with_hash = scan_files_with_hash(&claude_dir).await?;

    if files_with_hash.is_empty() {
        warn!("没有找到可同步的文件");
        update_sync_state(&sync_state, 100.0, 0, 0).await;
        mark_sync_complete(&sync_state, &claude_dir, &user_id, 0).await?;
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

    info!("用户 {}: 总文件数: {}, 需要同步: {}", user_id, total_files_count, files_to_sync.len());

    if files_to_sync.is_empty() {
        info!("用户 {}: 所有文件都是最新的，无需同步", user_id);
        update_sync_state(&sync_state, 100.0, 0, 0).await;
        mark_sync_complete(&sync_state, &claude_dir, &user_id, 0).await?;
        return Ok(());
    }

    let total_files = files_to_sync.len() as f64;

    // 同步有变化的文件
    for (index, (file_path, _hash, _size)) in files_to_sync.iter().enumerate() {
        let progress = ((index + 1) as f64 / total_files) * 100.0;

        debug!("同步文件 [{}/{}]: {:?}", index + 1, files_to_sync.len(), file_path);

        // TODO: 实际的文件同步逻辑
        // 1. 调用 gRPC 客户端上报文件变更（附带 user_id）
        // 2. 上传文件内容（如果需要）

        // 更新进度
        update_sync_state(&sync_state, progress, index + 1, 0).await;

        // 模拟处理延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // 同步完成后重新扫描并更新缓存（按用户隔离）
    let all_files = scan_files_with_hash(&claude_dir).await?;
    save_file_cache(&claude_dir, &user_id, &all_files)?;

    info!("用户 {}: 同步完成: {} 个文件", user_id, files_to_sync.len());

    mark_sync_complete(&sync_state, &claude_dir, &user_id, files_to_sync.len()).await?;

    Ok(())
}

/// 获取待同步文件列表
#[tauri::command]
pub async fn get_pending_files(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
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

    // 获取用户 ID
    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    // 加载本地文件状态缓存
    let cached_states = load_file_cache(&claude_dir, &user_id).unwrap_or_default();

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

    serde_json::to_value(pending_files)
        .map_err(|e| format!("序列化文件列表失败: {}", e))
}

/// 扫描文件并计算哈希
async fn scan_files_with_hash(claude_dir: &Path) -> Result<Vec<(String, String, u64)>, String> {
    let mut files = Vec::new();

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
                if EXCLUDE_DIRS.contains(&dir_name.as_ref()) {
                    debug!("跳过排除目录: {:?}", path);
                    continue;
                }
                stack.push(path);
            } else if path.is_file() {
                // 检查文件扩展名
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();

                    // 跳过排除的扩展名
                    if EXCLUDE_EXTS.contains(&ext_str.as_str()) {
                        debug!("跳过排除的文件: {:?}", path);
                        continue;
                    }

                    // 只包含允许的扩展名
                    if !ALLOWED_EXTS.contains(&ext_str.as_str()) {
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

/// 文件缓存文件路径（按用户隔离）
fn cache_file_path(claude_dir: &Path, user_id: &str) -> PathBuf {
    claude_dir.join(format!(".sync-cache-{}.json", user_id))
}

/// 加载文件缓存（按用户隔离）
fn load_file_cache(claude_dir: &Path, user_id: &str) -> Result<HashMap<String, String>, String> {
    let cache_path = cache_file_path(claude_dir, user_id);

    if !cache_path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&cache_path)
        .map_err(|e| format!("无法读取缓存文件: {}", e))?;

    let cache: HashMap<String, String> = serde_json::from_str(&content)
        .map_err(|e| format!("无法解析缓存文件: {}", e))?;

    Ok(cache)
}

/// 保存文件缓存（按用户隔离）
fn save_file_cache(claude_dir: &Path, user_id: &str, files: &[(String, String, u64)]) -> Result<(), String> {
    let cache_path = cache_file_path(claude_dir, user_id);

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
    user_id: &str,
    synced_count: usize,
) -> Result<(), String> {
    let now = chrono::Utc::now();
    let mut state = sync_state.lock().await;
    state.is_syncing = false;
    state.progress = 100.0;
    state.last_sync_time = Some(now);
    state.synced_count = synced_count;

    // 保存到配置文件（按用户隔离）
    save_sync_state(claude_dir, user_id, now, synced_count)?;

    Ok(())
}

/// 保存同步状态到配置文件（按用户隔离）
fn save_sync_state(
    claude_dir: &Path,
    user_id: &str,
    last_sync: DateTime<Utc>,
    synced_count: usize,
) -> Result<(), String> {
    let state_path = claude_dir.join(format!(".sync-state-{}.json", user_id));

    let state_data = serde_json::json!({
        "last_sync": last_sync.to_rfc3339(),
        "synced_count": synced_count,
        "user_id": user_id
    });

    let content = serde_json::to_string_pretty(&state_data)
        .map_err(|e| format!("无法序列化同步状态: {}", e))?;

    std::fs::write(&state_path, content)
        .map_err(|e| format!("无法写入同步状态: {}", e))?;

    Ok(())
}

/// 加载同步状态从配置文件（按用户隔离）
pub fn load_sync_state(claude_dir: &Path, user_id: &str) -> Result<(Option<DateTime<Utc>>, usize), String> {
    let state_path = claude_dir.join(format!(".sync-state-{}.json", user_id));

    if !state_path.exists() {
        return Ok((None, 0));
    }

    let content = std::fs::read_to_string(&state_path)
        .map_err(|e| format!("无法读取同步状态: {}", e))?;

    let state: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("无法解析同步状态: {}", e))?;

    let last_sync = state["last_sync"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc).to_utc());

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

    // 从配置文件中读取用户信息（如果存在）
    let user_id_from_config = config["auth"]
        .get("user_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let device_id_from_config = config["auth"]
        .get("device_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    drop(manager);

    // 获取 Claude 目录路径
    let claude_dir = config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let claude_dir = PathBuf::from(claude_dir);

    // 如果配置文件中有用户信息，设置到 sync_state 中
    if let (Some(uid), Some(did)) = (user_id_from_config, device_id_from_config) {
        let mut state = sync_state.lock().await;
        state.set_user(uid.clone(), did.clone());
        tracing::info!("从配置文件加载用户信息: user_id={}, device_id={}", uid, did);
    }

    // 从状态获取用户 ID
    let user_id = {
        let state = sync_state.lock().await;
        let uid = state.get_user_id().map(|s| s.to_string());
        drop(state);
        uid.unwrap_or_else(|| "guest".to_string())
    };

    // 加载持久化的状态（按用户隔离）
    let (last_sync, synced_count) = load_sync_state(&claude_dir, &user_id)?;

    let mut state = sync_state.lock().await;
    state.load_persistent(last_sync, synced_count);

    tracing::info!("用户 {}: 加载同步状态: last_sync={:?}, synced_count={}", user_id, last_sync, synced_count);

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

/// 获取文件树
#[tauri::command]
pub async fn get_file_tree(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<FileTreeNode, String> {
    // 获取配置
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;

    // 从配置文件中读取类型为 "ignore" 的规则
    let config_rules = config["sync"]["rules"]
        .as_array()
        .unwrap_or(&vec![])
        .clone();

    // 释放配置管理器锁
    drop(manager);

    // 获取 Claude 目录路径
    let claude_dir = config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let claude_dir = PathBuf::from(claude_dir);

    if !claude_dir.exists() {
        return Ok(FileTreeNode {
            name: claude_dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Root")
                .to_string(),
            path: String::new(),
            node_type: NodeType::Directory,
            sync_status: SyncStatus::NotOnServer,
            size: 0,
            hash: String::new(),
            children: Vec::new(),
            checked: true,
            exists_on_server: false,
        });
    }

    // 获取用户 ID
    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    // 加载本地文件状态缓存
    let cached_states = load_file_cache(&claude_dir, &user_id).unwrap_or_default();

    // 加载自定义忽略模式（.sync-ignore-{user_id}.json）
    let mut custom_patterns = load_custom_ignore_patterns(&claude_dir, &user_id)?;

    // 从配置文件的规则中提取类型为 "ignore" 或 "exclude" 的规则
    // exclude 类型的规则用于文件树过滤
    for rule in config_rules {
        if let Some(rule_type) = rule.get("type").and_then(|t| t.as_str()) {
            if rule_type == "ignore" || rule_type == "exclude" {
                if let Some(enabled) = rule.get("enabled").and_then(|e| e.as_bool()) {
                    if enabled {
                        if let Some(pattern) = rule.get("pattern").and_then(|p| p.as_str()) {
                            if !custom_patterns.contains(&pattern.to_string()) {
                                custom_patterns.push(pattern.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    debug!("加载的忽略模式: {:?}", custom_patterns);

    // 构建文件树
    build_file_tree(&claude_dir, &claude_dir, &cached_states, &custom_patterns)
}

/// 构建文件树
fn build_file_tree(
    base_dir: &Path,
    current_dir: &Path,
    cached_states: &HashMap<String, String>,
    ignore_patterns: &[String],
) -> Result<FileTreeNode, String> {
    let dir_name = if current_dir == base_dir {
        current_dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Root")
            .to_string()
    } else {
        current_dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string()
    };

    let rel_path = if current_dir == base_dir {
        String::new()
    } else {
        current_dir
            .strip_prefix(base_dir)
            .unwrap_or(current_dir)
            .to_string_lossy()
            .replace('\\', "/")
    };

    let mut children = Vec::new();
    let mut total_size = 0u64;
    let mut has_modified = false;

    // 读取目录内容
    if let Ok(entries) = std::fs::read_dir(current_dir) {
        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let file_name = entry.file_name();

            // 跳过隐藏文件
            if file_name.to_string_lossy().starts_with('.') {
                continue;
            }

            // 检查忽略模式
            let rel_path_str = if current_dir == base_dir {
                file_name.to_string_lossy().to_string()
            } else {
                format!("{}/{}", rel_path, file_name.to_string_lossy())
            };

            if should_ignore(&rel_path_str, ignore_patterns) {
                debug!("忽略路径: {}", rel_path_str);
                continue;
            }

            if path.is_dir() {
                // 检查是否在默认排除目录中
                let dir_name_str = file_name.to_string_lossy();
                if EXCLUDE_DIRS.contains(&dir_name_str.as_ref()) {
                    continue;
                }
                dirs.push((path, file_name.to_string_lossy().to_string()));
            } else if path.is_file() {
                // 检查文件扩展名
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();

                    if EXCLUDE_EXTS.contains(&ext_str.as_str()) {
                        continue;
                    }

                    if !ALLOWED_EXTS.contains(&ext_str.as_str()) {
                        continue;
                    }

                    files.push((path, file_name.to_string_lossy().to_string()));
                }
            }
        }

        // 排序
        dirs.sort_by(|a, b| a.1.cmp(&b.1));
        files.sort_by(|a, b| a.1.cmp(&b.1));

        // 处理子目录
        for (dir_path, _name) in dirs {
            if let Ok(child_node) = build_file_tree(base_dir, &dir_path, cached_states, ignore_patterns) {
                total_size += child_node.size;
                if child_node.sync_status == SyncStatus::Pending {
                    has_modified = true;
                }
                children.push(child_node);
            }
        }

        // 处理文件
        for (file_path, name) in files {
            if let Ok((hash, size)) = calculate_file_hash(&file_path) {
                let file_rel_path = if current_dir == base_dir {
                    name.clone()
                } else {
                    format!("{}/{}", rel_path, name)
                };

                let cached_hash = cached_states.get(&file_rel_path);
                let sync_status = if let Some(ch) = cached_hash {
                    if ch == &hash {
                        SyncStatus::Synced
                    } else {
                        has_modified = true;
                        SyncStatus::Pending
                    }
                } else {
                    SyncStatus::NotOnServer
                };

                let exists_on_server = cached_hash.is_some();

                total_size += size;

                children.push(FileTreeNode {
                    name,
                    path: file_rel_path,
                    node_type: NodeType::File,
                    sync_status,
                    size,
                    hash,
                    children: Vec::new(),
                    checked: true,
                    exists_on_server,
                });
            }
        }
    }

    // 计算目录的同步状态
    let sync_status = if children.is_empty() {
        SyncStatus::NotOnServer
    } else if has_modified {
        SyncStatus::Pending
    } else {
        // 检查所有子项是否都已同步
        let all_synced = children.iter()
            .all(|c| c.sync_status == SyncStatus::Synced || c.sync_status == SyncStatus::NotOnServer);
        if all_synced && children.iter().any(|c| c.sync_status == SyncStatus::Synced) {
            SyncStatus::Synced
        } else if children.iter().all(|c| c.sync_status == SyncStatus::NotOnServer) {
            SyncStatus::NotOnServer
        } else {
            SyncStatus::Pending
        }
    };

    // 检查目录是否在服务器上存在
    let exists_on_server = children.iter().any(|c| c.exists_on_server);

    Ok(FileTreeNode {
        name: dir_name,
        path: rel_path,
        node_type: NodeType::Directory,
        sync_status,
        size: total_size,
        hash: String::new(),
        children,
        checked: true,
        exists_on_server,
    })
}

/// 检查路径是否应该被忽略
fn should_ignore(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if matches_pattern(path, pattern) {
            debug!("路径 '{}' 匹配忽略模式 '{}'", path, pattern);
            return true;
        }
    }
    false
}

/// 简单的通配符匹配
fn matches_pattern(path: &str, pattern: &str) -> bool {
    let pattern = pattern.trim();
    let result = inner_matches_pattern(path, pattern);
    debug!("检查路径 '{}' 是否匹配模式 '{}': {}", path, pattern, result);
    result
}

/// 内部匹配函数（不含日志）
fn inner_matches_pattern(path: &str, pattern: &str) -> bool {

    // 处理 ** 通配符（匹配任意多级目录）
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0].trim_end_matches('/');
            let suffix = parts[1].trim_start_matches('/');

            if suffix.is_empty() {
                // 模式如 "cache/**" - 匹配以 prefix 开头的所有路径
                // 如果 prefix 为空（模式如 "/**"），则不匹配
                if prefix.is_empty() {
                    return false;
                }
                // 精确匹配或前缀匹配（确保是完整路径段）
                // 例如：debug/** 应该匹配 "debug" 和 "debug/xxx"
                // 但不应该匹配 "desktop"
                return path == prefix || path.starts_with(&format!("{}/", prefix));
            }

            // 检查是否匹配前缀和后缀
            if path.starts_with(prefix) && path.ends_with(suffix) {
                return true;
            }
        }
    }

    // 处理 * 通配符（匹配单级目录或文件名）
    if pattern.contains('*') && !pattern.contains("**") {
        let pattern_regex = pattern
            .replace('.', "\\.")
            .replace('*', "[^/]*")
            .replace('?', "[^/]");
        if let Ok(re) = regex::Regex::new(&format!("^{}$", pattern_regex)) {
            if re.is_match(path) {
                return true;
            }
            // 检查路径的最后一部分是否匹配
            if let Some(last_part) = path.rsplit('/').next() {
                if re.is_match(last_part) {
                    return true;
                }
            }
        }
    }

    // 精确匹配
    if path == pattern || path.starts_with(&format!("{}/", pattern)) {
        return true;
    }

    false
}

/// 加载自定义忽略模式
fn load_custom_ignore_patterns(claude_dir: &Path, user_id: &str) -> Result<Vec<String>, String> {
    let ignore_file = claude_dir.join(format!(".sync-ignore-{}.json", user_id));
    debug!("加载忽略模式，文件路径: {:?}, 存在: {}", ignore_file, ignore_file.exists());

    if !ignore_file.exists() {
        debug!("忽略配置文件不存在，返回空列表");
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&ignore_file)
        .map_err(|e| format!("无法读取忽略配置: {}", e))?;

    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("无法解析忽略配置: {}", e))?;

    let patterns: Vec<String> = data["patterns"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|p| p.as_str().map(|s| s.to_string()))
        .collect();

    debug!("成功加载 {} 个忽略模式: {:?}", patterns.len(), patterns);
    Ok(patterns)
}

/// 保存自定义忽略模式
fn save_custom_ignore_patterns(claude_dir: &Path, user_id: &str, patterns: &[String]) -> Result<(), String> {
    let ignore_file = claude_dir.join(format!(".sync-ignore-{}.json", user_id));

    let data = serde_json::json!({
        "patterns": patterns
    });

    let content = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("无法序列化忽略配置: {}", e))?;

    std::fs::write(&ignore_file, content)
        .map_err(|e| format!("无法写入忽略配置: {}", e))?;

    Ok(())
}

/// 获取忽略模式列表
#[tauri::command]
pub async fn get_ignore_patterns(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Vec<IgnorePattern>, String> {
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    let claude_dir = PathBuf::from(config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or(""));

    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    let patterns = load_custom_ignore_patterns(&claude_dir, &user_id)?;

    Ok(patterns.into_iter()
        .map(|p| IgnorePattern {
            pattern: p,
            is_active: true,
        })
        .collect())
}

/// 添加忽略模式
#[tauri::command]
pub async fn add_ignore_pattern(
    pattern: String,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    let pattern = pattern.trim().to_string();
    if pattern.is_empty() {
        return Err("忽略模式不能为空".to_string());
    }

    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    let claude_dir = PathBuf::from(config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or(""));

    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    let ignore_file = claude_dir.join(format!(".sync-ignore-{}.json", user_id));
    debug!("忽略配置文件路径: {:?}, 文件存在: {}", ignore_file, ignore_file.exists());

    let mut patterns = load_custom_ignore_patterns(&claude_dir, &user_id)?;
    debug!("当前忽略模式: {:?}, 添加新模式: {}", patterns, pattern);

    if !patterns.contains(&pattern) {
        patterns.push(pattern);
        save_custom_ignore_patterns(&claude_dir, &user_id, &patterns)?;
        debug!("保存后的忽略模式: {:?}", patterns);
    } else {
        debug!("模式已存在，无需添加");
    }

    Ok(())
}

/// 删除忽略模式
#[tauri::command]
pub async fn remove_ignore_pattern(
    pattern: String,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    let claude_dir = PathBuf::from(config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or(""));

    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    let mut patterns = load_custom_ignore_patterns(&claude_dir, &user_id)?;
    patterns.retain(|p| p != &pattern);

    save_custom_ignore_patterns(&claude_dir, &user_id, &patterns)?;

    Ok(())
}

/// 从服务器删除文件
#[tauri::command]
pub async fn delete_file_from_server(
    file_path: String,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    // TODO: 实现 gRPC 调用从服务器删除文件
    info!("请求从服务器删除文件: {}", file_path);

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

    // 获取用户 ID
    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    // 加载文件缓存
    let mut cached_states = load_file_cache(&claude_dir, &user_id).unwrap_or_default();

    // 规范化文件路径（移除前导斜杠和尾部斜杠）
    let normalized_path = file_path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_string();

    // 从缓存中删除该文件/目录及其所有子文件
    let mut removed_count = 0;

    // 首先尝试删除精确匹配的条目
    if cached_states.remove(&normalized_path).is_some() {
        removed_count += 1;
        info!("已从缓存中删除: {}", normalized_path);
    }

    // 然后删除所有以该路径为前缀的条目（子文件/子目录）
    let prefix = format!("{}/", normalized_path);
    let keys_to_remove: Vec<String> = cached_states
        .keys()
        .filter(|k| k.starts_with(&prefix))
        .cloned()
        .collect();

    for key in keys_to_remove {
        if cached_states.remove(&key).is_some() {
            removed_count += 1;
            info!("已从缓存中删除子项: {}", key);
        }
    }

    if removed_count > 0 {
        info!("共从缓存中删除了 {} 个条目", removed_count);

        // 保存更新后的缓存
        save_file_cache_direct(&claude_dir, &user_id, &cached_states)?;
    }

    Ok(())
}

/// 直接保存文件缓存（用于部分更新）
fn save_file_cache_direct(
    claude_dir: &Path,
    user_id: &str,
    cached_states: &HashMap<String, String>,
) -> Result<(), String> {
    let cache_file = claude_dir.join(format!(".sync-cache-{}.json", user_id));

    let cache_json = serde_json::to_string_pretty(cached_states)
        .map_err(|e| format!("序列化缓存失败: {}", e))?;

    std::fs::write(&cache_file, cache_json)
        .map_err(|e| format!("写入缓存文件失败: {}", e))?;

    debug!("文件缓存已保存到: {:?}", cache_file);
    Ok(())
}

/// 获取调试信息（用于前端显示忽略模式等调试数据）
#[tauri::command]
pub async fn get_debug_info(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Value, String> {
    // 获取配置
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;

    // 从配置文件中读取类型为 "ignore" 的规则
    let config_rules = config["sync"]["rules"]
        .as_array()
        .unwrap_or(&vec![])
        .clone();

    // 读取完整的 sync 配置段用于调试
    let sync_config = config.get("sync").cloned().unwrap_or(serde_json::json!({}));

    drop(manager);

    // 获取 Claude 目录路径
    let claude_dir = config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let claude_dir = PathBuf::from(claude_dir);

    // 获取用户 ID
    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    // 加载忽略模式（.sync-ignore-{user_id}.json）
    let ignore_patterns = load_custom_ignore_patterns(&claude_dir, &user_id).unwrap_or_default();

    // 提取配置文件中的忽略规则（包括 "ignore" 和 "exclude" 类型）
    let config_ignore_patterns: Vec<String> = config_rules
        .iter()
        .filter(|r| {
            let rule_type = r.get("type").and_then(|t| t.as_str());
            rule_type == Some("ignore") || rule_type == Some("exclude")
        })
        .filter(|r| r.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false))
        .filter_map(|r| r.get("pattern").and_then(|p| p.as_str()).map(|s| s.to_string()))
        .collect();

    // 合并后的所有忽略模式
    let mut all_patterns = ignore_patterns.clone();
    for pattern in &config_ignore_patterns {
        if !all_patterns.contains(pattern) {
            all_patterns.push(pattern.clone());
        }
    }

    // 检查忽略配置文件是否存在
    let ignore_file = claude_dir.join(format!(".sync-ignore-{}.json", user_id));
    let ignore_file_exists = ignore_file.exists();
    let ignore_file_content = if ignore_file_exists {
        std::fs::read_to_string(&ignore_file).unwrap_or_default()
    } else {
        "(文件不存在)".to_string()
    };

    // 读取配置管理器的配置文件路径（用于调试）
    let config_path = {
        let mgr = config_manager.lock().await;
        mgr.config_file_path()
    };

    // 读取配置文件内容
    let config_file_content = std::fs::read_to_string(&config_path).unwrap_or("(无法读取)".to_string());

    Ok(serde_json::json!({
        "user_id": user_id,
        "claude_dir": claude_dir.to_string_lossy().to_string(),
        "ignore_patterns": ignore_patterns,
        "config_ignore_patterns": config_ignore_patterns,
        "all_ignore_patterns": all_patterns,
        "ignore_file_exists": ignore_file_exists,
        "ignore_file_path": ignore_file.to_string_lossy().to_string(),
        "ignore_file_content": ignore_file_content,
        "config_rules": config_rules,
        "sync_config": sync_config,
        "config_file_path": config_path.to_string_lossy().to_string(),
        "config_file_content": config_file_content,
    }))}

/// 服务器文件信息（从缓存中读取）
#[derive(Debug, Clone, serde::Serialize)]
struct ServerFileInfo {
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub last_modified: String,
}

/// 获取服务器文件列表（基于缓存模拟）
#[tauri::command]
pub async fn get_server_file_list(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Vec<ServerFileInfo>, String> {
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    let claude_dir = config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let claude_dir = PathBuf::from(claude_dir);

    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    // 加载服务器文件缓存（实际应该从服务器获取）
    let cached_states = load_file_cache(&claude_dir, &user_id).unwrap_or_default();

    // 构建服务器文件列表（缓存中有但本地没有的文件）
    let mut server_files = Vec::new();
    for (path, hash) in cached_states {
        server_files.push(ServerFileInfo {
            path: path.clone(),
            hash,
            size: 0,
            last_modified: String::new(),
        });
    }

    // 按路径排序
    server_files.sort_by(|a, b| a.path.cmp(&b.path));

    info!("服务器文件列表: 共 {} 个文件", server_files.len());
    Ok(server_files)
}

/// 获取服务器文件树（包含本地没有的文件）
#[tauri::command]
pub async fn get_server_file_tree(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<FileTreeNode, String> {
    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    let claude_dir = config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let claude_dir = PathBuf::from(claude_dir);

    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    // 加载服务器文件缓存
    let cached_states = load_file_cache(&claude_dir, &user_id).unwrap_or_default();

    // 扫描本地文件
    let local_files = scan_files_with_hash(&claude_dir).await.unwrap_or_default();
    let local_file_map: HashMap<_, _> = local_files
        .into_iter()
        .map(|(path, hash, size)| (path, (hash, size)))
        .collect();

    // 构建服务器文件树（合并本地文件和服务器文件）
    let mut all_paths = std::collections::HashSet::new();

    // 添加本地文件路径
    for path in local_file_map.keys() {
        all_paths.insert(path.clone());
    }

    // 添加服务器文件路径
    for path in cached_states.keys() {
        all_paths.insert(path.clone());
    }

    // 构建文件树
    let root = build_server_file_tree(&claude_dir, &all_paths, &local_file_map, &cached_states)?;

    Ok(root)
}

/// 构建服务器文件树
fn build_server_file_tree(
    base_dir: &Path,
    all_paths: &HashSet<String>,
    local_files: &HashMap<String, (String, u64)>,
    server_files: &HashMap<String, String>,
) -> Result<FileTreeNode, String> {
    use std::collections::BTreeMap;

    // 按目录分组
    let mut dir_map: BTreeMap<String, BTreeMap<String, (bool, bool, String, u64)>> = BTreeMap::new();
    // (exists_locally, exists_on_server, hash, size)

    for path in all_paths {
        let parts: Vec<&str> = path.split('/').collect();
        let file_name = parts.last().copied().unwrap_or("");
        let dir_path = if parts.len() > 1 {
            parts[..parts.len()-1].join("/")
        } else {
            String::new()
        };

        let exists_locally = local_files.contains_key(path);
        let exists_on_server = server_files.contains_key(path);

        let (hash, size) = if let Some((h, s)) = local_files.get(path) {
            (h.clone(), *s)
        } else if let Some(h) = server_files.get(path) {
            (h.clone(), 0u64)
        } else {
            (String::new(), 0u64)
        };

        dir_map.entry(dir_path)
            .or_insert_with(BTreeMap::new)
            .insert(file_name.to_string(), (exists_locally, exists_on_server, hash, size));
    }

    // 递归构建树
    fn build_tree_recursive(
        dir_path: &str,
        dir_map: &BTreeMap<String, BTreeMap<String, (bool, bool, String, u64)>>,
        base_dir: &Path,
    ) -> Result<FileTreeNode, String> {
        let dir_name = if dir_path.is_empty() {
            base_dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Root")
                .to_string()
        } else {
            dir_path.split('/').last().unwrap_or("Unknown").to_string()
        };

        let mut children = Vec::new();
        let mut total_size = 0u64;

        // 查找当前目录的直接子文件和子目录
        let mut subdirs = std::collections::BTreeSet::new();
        let mut files = Vec::new();

        for (path, file_map) in dir_map {
            if path == dir_path || path.starts_with(&format!("{}/", dir_path)) {
                let relative = if path == dir_path {
                    String::new()
                } else {
                    path[dir_path.len()..].trim_start_matches('/').to_string()
                };

                if relative.is_empty() {
                    // 当前目录的文件
                    for (name, (local, server, hash, size)) in file_map {
                        let sync_status = match (*local, *server) {
                            (true, true) => SyncStatus::Synced,
                            (true, false) => SyncStatus::NotOnServer,
                            (false, true) => SyncStatus::OnlyOnServer,
                            (false, false) => SyncStatus::NotOnServer,
                        };

                        total_size += size;
                        files.push((name.clone(), FileTreeNode {
                            name: name.clone(),
                            path: if dir_path.is_empty() { name.clone() } else { format!("{}/{}", dir_path, name) },
                            node_type: NodeType::File,
                            sync_status,
                            size: *size,
                            hash: hash.clone(),
                            children: Vec::new(),
                            checked: true,
                            exists_on_server: *server,
                        }));
                    }
                } else if let Some(first_segment) = relative.split('/').next() {
                    if !relative.contains('/') {
                        // 直接子文件
                        let file_map = dir_map.get(path).unwrap();
                        for (name, (local, server, hash, size)) in file_map {
                            let sync_status = match (*local, *server) {
                                (true, true) => SyncStatus::Synced,
                                (true, false) => SyncStatus::NotOnServer,
                                (false, true) => SyncStatus::OnlyOnServer,
                                (false, false) => SyncStatus::NotOnServer,
                            };

                            total_size += size;
                            files.push((name.clone(), FileTreeNode {
                                name: name.clone(),
                                path: if path.is_empty() { name.clone() } else { format!("{}/{}", path, name) },
                                node_type: NodeType::File,
                                sync_status,
                                size: *size,
                                hash: hash.clone(),
                                children: Vec::new(),
                                checked: true,
                                exists_on_server: *server,
                            }));
                        }
                    } else {
                        // 子目录
                        subdirs.insert(if dir_path.is_empty() {
                            first_segment.to_string()
                        } else {
                            format!("{}/{}", dir_path, first_segment)
                        });
                    }
                }
            }
        }

        // 处理子目录
        for subdir in subdirs {
            if let Ok(child) = build_tree_recursive(&subdir, dir_map, base_dir) {
                total_size += child.size;
                children.push(child);
            }
        }

        // 添加文件（排序）
        files.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, file) in files {
            children.push(file);
        }

        // 计算目录状态
        let sync_status = if children.is_empty() {
            SyncStatus::NotOnServer
        } else {
            let has_only_server = children.iter().any(|c| c.sync_status == SyncStatus::OnlyOnServer);
            let has_only_local = children.iter().any(|c| c.sync_status == SyncStatus::NotOnServer);
            let has_pending = children.iter().any(|c| c.sync_status == SyncStatus::Pending);
            let all_synced = children.iter().all(|c| c.sync_status == SyncStatus::Synced);

            if has_only_server || has_only_local || has_pending {
                SyncStatus::Pending
            } else if all_synced {
                SyncStatus::Synced
            } else {
                SyncStatus::NotOnServer
            }
        };

        let exists_on_server = children.iter().any(|c| c.exists_on_server);

        Ok(FileTreeNode {
            name: dir_name,
            path: dir_path.to_string(),
            node_type: NodeType::Directory,
            sync_status,
            size: total_size,
            hash: String::new(),
            children,
            checked: true,
            exists_on_server,
        })
    }

    build_tree_recursive("", &dir_map, base_dir)
}

/// 从服务器下载文件到本地
#[tauri::command]
pub async fn download_file_from_server(
    file_path: String,
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    sync_state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<String, String> {
    info!("请求从服务器下载文件: {}", file_path);

    // TODO: 实际的 gRPC 下载逻辑
    // 目前返回模拟成功消息

    let manager = config_manager.lock().await;
    let config = manager.get_config().await.map_err(|e| e.to_string())?;
    drop(manager);

    let claude_dir = config["sync"]["claude_dir"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let claude_dir = PathBuf::from(claude_dir);

    let user_id = {
        let state = sync_state.lock().await;
        state.get_user_id().map(|s| s.to_string()).unwrap_or_else(|| "guest".to_string())
    };

    // 加载缓存获取文件哈希
    let cached_states = load_file_cache(&claude_dir, &user_id).unwrap_or_default();

    if let Some(_hash) = cached_states.get(&file_path) {
        // 实际实现时，这里会调用 gRPC DownloadFile
        // 目前只是返回成功消息
        info!("文件 {} 已标记为从服务器下载", file_path);
        Ok(format!("文件 {} 已下载到本地", file_path))
    } else {
        Err(format!("服务器上不存在文件: {}", file_path))
    }
}
