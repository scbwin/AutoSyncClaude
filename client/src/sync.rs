use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::ClientConfig;
use crate::conflict::{ConflictResolver, ConflictType};
use crate::rules::RuleEngine;
use crate::transfer::TransferManager;
use crate::watcher::{FileEvent, FileEventType, FileScanner};

/// 同步状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncStatus {
    /// 等待同步
    Pending,
    /// 正在同步
    Syncing,
    /// 已同步
    Synced,
    /// 同步失败
    Failed,
    /// 冲突
    Conflict,
}

/// 文件同步状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSyncState {
    /// 文件路径
    pub path: PathBuf,

    /// 本地哈希
    pub local_hash: Option<String>,

    /// 远程哈希
    pub remote_hash: Option<String>,

    /// 同步状态
    pub status: SyncStatus,

    /// 最后同步时间
    pub last_sync_time: Option<DateTime<Utc>>,

    /// 错误消息（如果同步失败）
    pub error_message: Option<String>,
}

/// 同步模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// 增量同步（基于文件变更事件）
    Incremental,
    /// 全量同步（扫描所有文件）
    Full,
    /// 选择性同步（基于规则）
    Selective,
}

/// 同步引擎
pub struct SyncEngine {
    /// 客户端配置
    config: Arc<ClientConfig>,

    /// 规则引擎
    rule_engine: Arc<RuleEngine>,

    /// 传输管理器
    transfer_manager: Arc<TransferManager>,

    /// 冲突解决器
    conflict_resolver: Arc<ConflictResolver>,

    /// 文件同步状态缓存
    sync_states: Arc<tokio::sync::Mutex<HashMap<PathBuf, FileSyncState>>>,

    /// 用户 ID
    user_id: uuid::Uuid,

    /// 设备 ID
    device_id: uuid::Uuid,
}

impl SyncEngine {
    /// 创建新的同步引擎
    pub fn new(
        config: Arc<ClientConfig>,
        rule_engine: Arc<RuleEngine>,
        transfer_manager: Arc<TransferManager>,
        conflict_resolver: Arc<ConflictResolver>,
        user_id: uuid::Uuid,
        device_id: uuid::Uuid,
    ) -> Self {
        Self {
            config,
            rule_engine,
            transfer_manager,
            conflict_resolver,
            sync_states: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            user_id,
            device_id,
        }
    }

    /// 启动增量同步
    pub async fn start_incremental_sync(
        &self,
        mut event_rx: mpsc::UnboundedReceiver<FileEvent>,
    ) -> Result<()> {
        info!("启动增量同步模式");

        loop {
            match event_rx.recv().await {
                Some(event) => {
                    if let Err(e) = self.handle_file_event(event).await {
                        error!("处理文件事件失败: {}", e);
                    }
                }
                None => {
                    info!("文件事件通道已关闭");
                    break;
                }
            }
        }

        Ok(())
    }

    /// 执行全量同步
    pub async fn run_full_sync(&self) -> Result<SyncSummary> {
        info!("开始全量同步");

        let scanner = FileScanner::new(
            self.config.sync.claude_dir.clone(),
            self.config.get_exclude_paths(),
            self.config.sync.exclude_patterns.clone(),
            self.config.sync.include_types.clone(),
        );

        // 扫描所有文件
        let files = scanner.scan()?;

        info!("全量同步: 找到 {} 个文件", files.len());

        let mut summary = SyncSummary::default();

        // 批量同步文件
        for file_path in files {
            match self.sync_file(&file_path).await {
                Ok(state) => match state.status {
                    SyncStatus::Synced => {
                        summary.synced_count += 1;
                    }
                    SyncStatus::Conflict => {
                        summary.conflict_count += 1;
                        summary.conflicts.push(file_path);
                    }
                    SyncStatus::Failed => {
                        summary.failed_count += 1;
                        summary
                            .errors
                            .push((file_path, state.error_message.unwrap_or_default()));
                    }
                    _ => {}
                },
                Err(e) => {
                    error!("同步文件失败 {:?}: {}", file_path, e);
                    summary.failed_count += 1;
                }
            }
        }

        info!(
            "全量同步完成: {} 成功, {} 失败, {} 冲突",
            summary.synced_count, summary.failed_count, summary.conflict_count
        );

        Ok(summary)
    }

    /// 处理文件事件
    async fn handle_file_event(&self, event: FileEvent) -> Result<()> {
        debug!(
            "处理文件事件: {:?}, 类型: {:?}",
            event.path, event.event_type
        );

        // 检查是否应该排除
        if self.config.should_exclude(&event.path) {
            debug!("文件被排除，跳过: {:?}", event.path);
            return Ok(());
        }

        // 检查规则
        let file_type = crate::rules::detect_file_type(&event.path);
        if !self.config.apply_rules(&event.path, &file_type) {
            debug!("文件不匹配同步规则，跳过: {:?}", event.path);
            return Ok(());
        }

        match event.event_type {
            FileEventType::Create | FileEventType::Modify => {
                self.sync_file(&event.path).await?;
            }
            FileEventType::Remove => {
                self.handle_file_removal(&event.path).await?;
            }
            FileEventType::Rename => {
                // TODO: 处理重命名
                debug!("文件重命名，暂未实现: {:?}", event.path);
            }
        }

        Ok(())
    }

    /// 同步单个文件
    pub async fn sync_file(&self, file_path: &Path) -> Result<FileSyncState> {
        info!("同步文件: {:?}", file_path);

        // 计算本地哈希
        let scanner = FileScanner::new(
            self.config.sync.claude_dir.clone(),
            self.config.get_exclude_paths(),
            self.config.sync.exclude_patterns.clone(),
            self.config.sync.include_types.clone(),
        );

        let local_hash = scanner.hash_file(file_path)?;

        // 检查远程状态
        // TODO: 调用 gRPC 客户端查询远程文件状态
        let remote_hash: Option<String> = None;

        // 判断同步方向
        let sync_action = if let Some(remote) = &remote_hash {
            if &local_hash == remote {
                // 哈希相同，无需同步
                return Ok(FileSyncState {
                    path: file_path.to_path_buf(),
                    local_hash: Some(local_hash),
                    remote_hash: remote_hash.clone(),
                    status: SyncStatus::Synced,
                    last_sync_time: Some(Utc::now()),
                    error_message: None,
                });
            } else {
                // 哈希不同，需要检测冲突
                SyncAction::NeedSync
            }
        } else {
            // 远程不存在，需要上传
            SyncAction::Upload
        };

        match sync_action {
            SyncAction::Upload => self.upload_file(file_path, &local_hash).await,
            SyncAction::Download => self.download_file(file_path).await,
            SyncAction::NeedSync => {
                self.resolve_and_sync(file_path, &local_hash, remote_hash.as_ref().unwrap())
                    .await
            }
            SyncAction::NoAction => Ok(FileSyncState {
                path: file_path.to_path_buf(),
                local_hash: Some(local_hash),
                remote_hash,
                status: SyncStatus::Synced,
                last_sync_time: Some(Utc::now()),
                error_message: None,
            }),
        }
    }

    /// 上传文件
    async fn upload_file(&self, file_path: &Path, local_hash: &str) -> Result<FileSyncState> {
        info!("上传文件: {:?}", file_path);

        // TODO: 调用传输管理器上传文件
        // TODO: 调用 gRPC 客户端上报文件变更

        let state = FileSyncState {
            path: file_path.to_path_buf(),
            local_hash: Some(local_hash.to_string()),
            remote_hash: Some(local_hash.to_string()),
            status: SyncStatus::Synced,
            last_sync_time: Some(Utc::now()),
            error_message: None,
        };

        // 更新状态缓存
        self.update_sync_state(file_path, state.clone()).await;

        Ok(state)
    }

    /// 下载文件
    async fn download_file(&self, file_path: &Path) -> Result<FileSyncState> {
        info!("下载文件: {:?}", file_path);

        // TODO: 调用传输管理器下载文件
        // TODO: 重新计算本地哈希

        let state = FileSyncState {
            path: file_path.to_path_buf(),
            local_hash: None,
            remote_hash: None,
            status: SyncStatus::Synced,
            last_sync_time: Some(Utc::now()),
            error_message: None,
        };

        // 更新状态缓存
        self.update_sync_state(file_path, state.clone()).await;

        Ok(state)
    }

    /// 解决冲突并同步
    async fn resolve_and_sync(
        &self,
        file_path: &Path,
        local_hash: &str,
        remote_hash: &str,
    ) -> Result<FileSyncState> {
        warn!("检测到冲突: {:?}", file_path);

        // 读取本地和远程内容
        let local_content = tokio::fs::read_to_string(file_path).await?;
        let remote_content = String::new(); // TODO: 从远程下载

        // 尝试自动合并
        let merge_result = self.conflict_resolver.resolve(
            file_path,
            &local_content,
            &remote_content,
            None,
            ConflictType::ModifyModify,
        )?;

        match merge_result {
            crate::conflict::MergeResult::Merged(merged_content) => {
                // 写入合并后的内容
                tokio::fs::write(file_path, merged_content).await?;

                // 重新上传
                self.upload_file(file_path, local_hash).await
            }
            crate::conflict::MergeResult::Conflict(conflict_content) => {
                // 写入冲突标记
                let conflict_path = file_path.with_extension("conflict");
                tokio::fs::write(&conflict_path, conflict_content).await?;

                let state = FileSyncState {
                    path: file_path.to_path_buf(),
                    local_hash: Some(local_hash.to_string()),
                    remote_hash: Some(remote_hash.to_string()),
                    status: SyncStatus::Conflict,
                    last_sync_time: Some(Utc::now()),
                    error_message: Some("存在未解决的冲突".to_string()),
                };

                // 更新状态缓存
                self.update_sync_state(file_path, state.clone()).await;

                Ok(state)
            }
            _ => {
                // 其他结果，使用默认策略
                let default_result = self
                    .conflict_resolver
                    .apply_default_strategy(&local_content, &remote_content);

                match default_result {
                    crate::conflict::MergeResult::Merged(content) => {
                        tokio::fs::write(file_path, content).await?;
                        self.upload_file(file_path, local_hash).await
                    }
                    _ => Ok(FileSyncState {
                        path: file_path.to_path_buf(),
                        local_hash: Some(local_hash.to_string()),
                        remote_hash: Some(remote_hash.to_string()),
                        status: SyncStatus::Conflict,
                        last_sync_time: Some(Utc::now()),
                        error_message: Some("使用默认策略后仍存在冲突".to_string()),
                    }),
                }
            }
        }
    }

    /// 处理文件删除
    async fn handle_file_removal(&self, file_path: &Path) -> Result<()> {
        info!("处理文件删除: {:?}", file_path);

        // TODO: 调用 gRPC 客户端通知服务器文件已删除

        // 从状态缓存中移除
        let mut states = self.sync_states.lock().await;
        states.remove(file_path);

        Ok(())
    }

    /// 更新同步状态
    async fn update_sync_state(&self, file_path: &Path, state: FileSyncState) {
        let mut states = self.sync_states.lock().await;
        states.insert(file_path.to_path_buf(), state);
    }

    /// 获取同步状态
    pub async fn get_sync_state(&self, file_path: &Path) -> Option<FileSyncState> {
        let states = self.sync_states.lock().await;
        states.get(file_path).cloned()
    }

    /// 获取所有同步状态
    pub async fn get_all_sync_states(&self) -> Vec<FileSyncState> {
        let states = self.sync_states.lock().await;
        states.values().cloned().collect()
    }
}

/// 同步操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncAction {
    /// 上传
    Upload,
    /// 下载
    Download,
    /// 需要同步（可能冲突）
    NeedSync,
    /// 无需操作
    NoAction,
}

/// 同步摘要
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncSummary {
    /// 成功同步的文件数
    pub synced_count: usize,

    /// 失败的文件数
    pub failed_count: usize,

    /// 冲突的文件数
    pub conflict_count: usize,

    /// 冲突文件列表
    pub conflicts: Vec<PathBuf>,

    /// 错误列表
    pub errors: Vec<(PathBuf, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status() {
        let status1 = SyncStatus::Syncing;
        let status2 = SyncStatus::Syncing;
        assert_eq!(status1, status2);

        let status3 = SyncStatus::Synced;
        assert_ne!(status1, status3);
    }
}
