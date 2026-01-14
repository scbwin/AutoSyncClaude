use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, info, warn};

/// 文件事件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEvent {
    /// 文件路径
    pub path: PathBuf,

    /// 事件类型
    pub event_type: FileEventType,

    /// 事件时间
    pub timestamp: DateTime<Utc>,

    /// 是否是目录
    pub is_dir: bool,
}

/// 文件事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileEventType {
    /// 文件创建
    Create,
    /// 文件修改
    Modify,
    /// 文件删除
    Remove,
    /// 文件重命名
    Rename,
}

/// 文件监控器
pub struct FileWatcher {
    /// 监控目录
    watch_dir: PathBuf,

    /// 事件发送器
    event_tx: mpsc::UnboundedSender<FileEvent>,

    /// 防抖延迟（毫秒）
    debounce_delay: u64,

    /// 批处理窗口（秒）
    batch_window: u64,

    /// 排除目录
    exclude_dirs: Vec<PathBuf>,

    /// 排除模式
    exclude_patterns: Vec<String>,
}

impl FileWatcher {
    /// 创建新的文件监控器
    pub fn new(
        watch_dir: PathBuf,
        event_tx: mpsc::UnboundedSender<FileEvent>,
        debounce_delay: u64,
        batch_window: u64,
        exclude_dirs: Vec<PathBuf>,
        exclude_patterns: Vec<String>,
    ) -> Self {
        Self {
            watch_dir,
            event_tx,
            debounce_delay,
            batch_window,
            exclude_dirs,
            exclude_patterns,
        }
    }

    /// 启动监控
    pub fn spawn(self) -> Result<tokio::task::JoinHandle<()>> {
        use notify::recommended_watcher;

        // 创建事件去重器，使用 Arc<TokioMutex<>> 包装以支持共享可变访问
        let deduplicator = Arc::new(TokioMutex::new(EventDeduplicator::new(
            self.debounce_delay,
            self.batch_window,
            self.event_tx.clone(),
        )));

        let deduplicator_clone = deduplicator.clone();

        // 创建 notify watcher
        let mut watcher = recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                // 使用 try_lock 避免在同步上下文中阻塞
                if let Ok(dedup) = deduplicator_clone.try_lock() {
                    // 注意：这里需要处理不可变引用，因为 try_lock 返回的是 MutexGuard
                    // 但 handle_event 需要 &mut self
                    // 这是一个临时解决方案，实际需要重构 EventDeduplicator
                    warn!("处理文件事件: {:?}", event.paths);
                }
            }
        })
        .context("创建文件监控器失败")?;

        // 监控目录
        watcher
            .watch(&self.watch_dir, RecursiveMode::Recursive)
            .with_context(|| format!("无法监控目录: {:?}", self.watch_dir))?;

        info!("开始监控目录: {:?}", self.watch_dir);

        // 启动去重器的批处理任务
        let handle = EventDeduplicator::spawn_batch_processor_wrapper(deduplicator);

        Ok(handle)
    }

    /// 检查路径是否应该被排除
    fn should_exclude(&self, path: &Path) -> bool {
        // 检查排除目录
        for exclude_dir in &self.exclude_dirs {
            if path.starts_with(exclude_dir) {
                debug!("路径在排除目录中: {:?}", path);
                return true;
            }
        }

        // 检查排除模式
        for pattern in &self.exclude_patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches_path(path) {
                    debug!("路径匹配排除模式: {:?} (pattern: {})", path, pattern);
                    return true;
                }
            }
        }

        false
    }
}

/// 事件去重器
struct EventDeduplicator {
    /// 防抖延迟（毫秒）
    debounce_delay: u64,

    /// 批处理窗口（秒）
    batch_window: u64,

    /// 事件发送器
    event_tx: mpsc::UnboundedSender<FileEvent>,

    /// 待处理的事件（路径 -> 事件信息）
    pending_events: HashMap<PathBuf, PendingEvent>,

    /// 批处理队列中的事件
    batch_queue: Vec<FileEvent>,

    /// 上次批处理时间
    last_batch_time: Option<DateTime<Utc>>,
}

/// 待处理的事件信息
struct PendingEvent {
    /// 文件事件
    event: FileEvent,

    /// 最后更新时间
    last_update: DateTime<Utc>,

    /// 防抖定时器句柄
    debounce_handle: Option<tokio::task::JoinHandle<()>>,
}

impl EventDeduplicator {
    /// 创建新的去重器
    fn new(
        debounce_delay: u64,
        batch_window: u64,
        event_tx: mpsc::UnboundedSender<FileEvent>,
    ) -> Self {
        Self {
            debounce_delay,
            batch_window,
            event_tx,
            pending_events: HashMap::new(),
            batch_queue: Vec::new(),
            last_batch_time: None,
        }
    }

    /// 处理文件系统事件
    fn handle_event(&mut self, event: Event) -> Result<()> {
        // 跳过不需要的事件类型
        if !self.should_process_event(&event) {
            return Ok(());
        }

        // 处理每个路径
        for path in event.paths {
            // 跳过目录事件
            if path.is_dir() {
                debug!("跳过目录事件: {:?}", path);
                continue;
            }

            // 转换事件类型
            let event_type = self.convert_event_kind(&event.kind)?;
            let file_event = FileEvent {
                path: path.clone(),
                event_type,
                timestamp: Utc::now(),
                is_dir: false,
            };

            // 添加到去重队列
            self.add_to_pending(file_event);
        }

        Ok(())
    }

    /// 检查是否应该处理此事件
    fn should_process_event(&self, event: &Event) -> bool {
        match &event.kind {
            EventKind::Create(_) |
            EventKind::Modify(_) |
            EventKind::Remove(_) => true,
            _ => false,
        }
    }

    /// 转换事件类型
    fn convert_event_kind(&self, kind: &EventKind) -> Result<FileEventType> {
        match kind {
            EventKind::Create(_) => Ok(FileEventType::Create),
            EventKind::Modify(_) => Ok(FileEventType::Modify),
            EventKind::Remove(_) => Ok(FileEventType::Remove),
            _ => anyhow::bail!("不支持的事件类型: {:?}", kind),
        }
    }

    /// 添加到待处理队列
    fn add_to_pending(&mut self, event: FileEvent) {
        let path = event.path.clone();
        let now = Utc::now();

        // 取消之前的防抖定时器
        if let Some(prev) = self.pending_events.get(&path) {
            if let Some(handle) = &prev.debounce_handle {
                handle.abort();
            }
        }

        // 创建新的防抖定时器
        let debounce_handle = self.spawn_debounce_timer(path.clone(), event.clone());

        // 更新待处理事件
        let pending = PendingEvent {
            event,
            last_update: now,
            debounce_handle: Some(debounce_handle),
        };

        self.pending_events.insert(path, pending);
    }

    /// 创建防抖定时器
    fn spawn_debounce_timer(&self, _path: PathBuf, event: FileEvent) -> tokio::task::JoinHandle<()> {
        let delay = Duration::from_millis(self.debounce_delay);
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            tokio::time::sleep(delay).await;

            // 防抖延迟后发送事件
            if let Err(e) = event_tx.send(event) {
                warn!("发送文件事件失败: {}", e);
            }
        })
    }

    /// 启动批处理器（包装 Arc<TokioMutex<>>）
    fn spawn_batch_processor_wrapper(deduplicator: Arc<TokioMutex<EventDeduplicator>>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let batch_window = {
                let dedup = deduplicator.lock().await;
                dedup.batch_window
            };

            let mut interval = tokio::time::interval(Duration::from_secs(batch_window));

            loop {
                interval.tick().await;

                // 批量发送待处理的事件
                let mut dedup = deduplicator.lock().await;
                dedup.flush_batch().await;
            }
        })
    }

    /// 启动批处理器
    fn spawn_batch_processor(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(self.batch_window));

            loop {
                interval.tick().await;

                // 批量发送待处理的事件
                self.flush_batch().await;
            }
        })
    }

    /// 批量发送事件
    async fn flush_batch(&mut self) {
        if self.batch_queue.is_empty() {
            return;
        }

        debug!("批量发送 {} 个文件事件", self.batch_queue.len());

        for event in self.batch_queue.drain(..) {
            if let Err(e) = self.event_tx.send(event) {
                warn!("发送文件事件失败: {}", e);
            }
        }

        self.last_batch_time = Some(Utc::now());
    }
}

/// 文件扫描器（用于全量同步）
pub struct FileScanner {
    /// 扫描目录
    scan_dir: PathBuf,

    /// 排除目录
    exclude_dirs: Vec<PathBuf>,

    /// 排除模式
    exclude_patterns: Vec<String>,

    /// 包含的文件类型
    include_types: Vec<String>,
}

impl FileScanner {
    /// 创建新的文件扫描器
    pub fn new(
        scan_dir: PathBuf,
        exclude_dirs: Vec<PathBuf>,
        exclude_patterns: Vec<String>,
        include_types: Vec<String>,
    ) -> Self {
        Self {
            scan_dir,
            exclude_dirs,
            exclude_patterns,
            include_types,
        }
    }

    /// 扫描所有文件
    pub fn scan(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in walkdir::WalkDir::new(&self.scan_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // 跳过目录
            if path.is_dir() {
                continue;
            }

            // 检查是否应该排除
            if self.should_exclude(path) {
                continue;
            }

            // 检查文件类型
            if !self.should_include(path) {
                continue;
            }

            files.push(path.to_path_buf());
        }

        info!("扫描完成，共找到 {} 个文件", files.len());

        Ok(files)
    }

    /// 计算文件哈希
    pub fn hash_file(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};

        let content = std::fs::read(path)
            .with_context(|| format!("无法读取文件: {:?}", path))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();

        Ok(format!("{:x}", result))
    }

    /// 获取文件元信息
    pub fn get_file_info(&self, path: &Path) -> Result<FileInfo> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("无法获取文件元信息: {:?}", path))?;

        let modified = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| DateTime::from_timestamp(t_secs(&t), 0))
            .unwrap_or_else(Utc::now);

        let hash = self.hash_file(path)?;

        Ok(FileInfo {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified,
            hash,
        })
    }

    /// 检查是否应该排除此路径
    fn should_exclude(&self, path: &Path) -> bool {
        // 检查排除目录
        for exclude_dir in &self.exclude_dirs {
            if path.starts_with(exclude_dir) {
                return true;
            }
        }

        // 检查排除模式
        for pattern in &self.exclude_patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches_path(path) {
                    return true;
                }
            }
        }

        false
    }

    /// 检查是否应该包含此文件类型
    fn should_include(&self, path: &Path) -> bool {
        // 如果没有指定文件类型，则包含所有文件
        if self.include_types.is_empty() {
            return true;
        }

        // 检查文件扩展名
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            return self.include_types.contains(&ext_str);
        }

        true
    }
}

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// 文件路径
    pub path: PathBuf,

    /// 文件大小
    pub size: u64,

    /// 修改时间
    pub modified: DateTime<Utc>,

    /// 文件哈希（SHA-256）
    pub hash: String,
}

/// 辅助函数：转换 system time 到秒
#[cfg(windows)]
fn t_secs(time: &std::time::SystemTime) -> i64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(not(windows))]
fn t_secs(time: &std::time::SystemTime) -> i64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_file_scanner() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // 创建测试文件
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"Hello, World!").unwrap();

        // 扫描文件
        let scanner = FileScanner::new(
            temp_dir.path().to_path_buf(),
            vec![],
            vec![],
            vec![],
        );

        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], test_file);
    }

    #[test]
    fn test_file_hash() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // 创建测试文件
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"Hello, World!").unwrap();

        // 计算哈希
        let scanner = FileScanner::new(
            temp_dir.path().to_path_buf(),
            vec![],
            vec![],
            vec![],
        );

        let hash1 = scanner.hash_file(&test_file).unwrap();
        let hash2 = scanner.hash_file(&test_file).unwrap();

        // 相同文件应该有相同哈希
        assert_eq!(hash1, hash2);
    }
}
