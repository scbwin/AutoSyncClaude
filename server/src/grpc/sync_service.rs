use crate::cache::{Cache, FileChangeNotification, ChangeType};
use crate::db::DbPool;
use crate::storage::StorageService;
use tracing::{debug, error};
use uuid::Uuid;

// TODO: 这些类型应该从 protobuf 生成的代码中导入
// 当前的定义是临时占位符
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_path: String,
    pub file_hash: String,
    pub file_size: u64,
    pub modified_time: i64,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone)]
pub enum FileChangeResult {
    Success,
    Conflict(String),
    NeedsUpload(String),
}

/// FileSyncService gRPC 实现
pub struct FileSyncGrpcService {
    pool: DbPool,
    cache: Cache,
    storage: StorageService,
}

impl FileSyncGrpcService {
    /// 创建新的服务实例
    pub fn new(pool: DbPool, cache: Cache, storage: StorageService) -> Self {
        Self {
            pool,
            cache,
            storage,
        }
    }
}

/*
#[tonic::async_trait]
impl FileSyncService for FileSyncGrpcService {
    /// 上报本地文件变更
    async fn report_changes(
        &self,
        request: Request<ReportChangesRequest>,
    ) -> Result<Response<ReportChangesResponse>, Status> {
        let req = request.into_inner();
        let user_id = extract_user_id_from_request(&request)?;
        let device_id = extract_device_id_from_request(&request)?;

        info!(
            "Received changes report from device {}: {} files",
            device_id,
            req.changes.len()
        );

        let mut conflicts_detected = Vec::new();
        let mut pending_uploads = Vec::new();

        for file_info in req.changes {
            match self
                .process_file_change(&user_id, &device_id, file_info)
                .await
            {
                Ok(FileChangeResult::Success) => {
                    debug!("File change processed: {:?}", file_info.file_path);
                }
                Ok(FileChangeResult::Conflict(file_path)) => {
                    conflicts_detected.push(file_path);
                }
                Ok(FileChangeResult::NeedsUpload(file_path)) => {
                    pending_uploads.push(file_path);
                }
                Err(e) => {
                    error!("Failed to process file change: {}", e);
                }
            }
        }

        // 通知其他在线设备
        self.notify_other_devices(&user_id, &device_id, &req.changes)
            .await;

        Ok(Response::new(ReportChangesResponse {
            success: true,
            message: format!(
                "Processed {} changes",
                req.changes.len() - conflicts_detected.len()
            ),
            conflicts_detected,
            pending_uploads,
        }))
    }

    /// 获取远程变更
    async fn fetch_changes(
        &self,
        request: Request<FetchChangesRequest>,
    ) -> Result<Response<Stream::<FetchChangesResponse>>, Status> {
        let req = request.into_inner();
        let user_id = extract_user_id_from_request(&request)?;

        info!("Fetching changes since: {:?}", req.since_version);

        // 查询数据库获取变更
        let changes = self
            .get_file_changes_since(&user_id, req.since_version, &req.file_patterns)
            .await?;

        // 创建流式响应
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            for chunk in changes.chunks(100) {
                let response = FetchChangesResponse {
                    changes: chunk.to_vec(),
                    has_more: true,
                };

                if tx.send(Ok(response)).await.is_err() {
                    break;
                }
            }

            // 发送结束标记
            let _ = tx
                .send(Ok(FetchChangesResponse {
                    changes: vec![],
                    has_more: false,
                }))
                .await;
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    /// 上传文件（流式）
    async fn upload_file(
        &self,
        request: Request<Stream::<UploadFileRequest>>,
    ) -> Result<Response<UploadFileResponse>, Status> {
        let user_id = extract_user_id_from_request(&request)?;
        let device_id = extract_device_id_from_request(&request)?;
        let mut stream = request.into_inner();

        info!("Starting file upload from device: {}", device_id);

        // 第一条消息必须是元数据
        let first_message = stream
            .message()
            .await
            .ok_or_else(|| Status::invalid_argument("Empty stream"))?
            .ok_or_else(|| Status::invalid_argument("Missing metadata"))?;

        let metadata = match first_message.payload {
            Some(Payload::Metadata(meta)) => meta,
            _ => {
                return Err(Status::invalid_argument("First message must be metadata"));
            }
        };

        debug!(
            "Uploading file: {:?}, size: {}",
            metadata.file_path,
            metadata.file_size
        );

        // 验证文件大小
        if metadata.file_size > MAX_FILE_SIZE {
            return Err(Status::invalid_argument(format!(
                "File too large: {} bytes (max: {})",
                metadata.file_size, MAX_FILE_SIZE
            )));
        }

        // 收集文件数据
        let mut file_data = Vec::new();
        let mut received_bytes = 0;

        while let Some(message) = stream.message().await? {
            match message.payload {
                Some(Payload::Chunk(chunk)) => {
                    file_data.extend_from_slice(&chunk.data);
                    received_bytes += chunk.data.len() as u64;

                    debug!(
                        "Received chunk {}: {} bytes",
                        chunk.chunk_number,
                        chunk.data.len()
                    );
                }
                Some(Payload::Metadata(_)) => {
                    warn!("Received duplicate metadata, ignoring");
                }
                None => {}
            }
        }

        // 验证文件大小
        if received_bytes != metadata.file_size {
            return Err(Status::data_loss(format!(
                "File size mismatch: expected {}, got {}",
                metadata.file_size, received_bytes
            )));
        }

        // 计算文件哈希
        let file_hash = StorageService::hash_file(&file_data);

        // 验证哈希
        if file_hash != metadata.file_hash {
            return Err(Status::data_loss(format!(
                "File hash mismatch: expected {}, got {}",
                metadata.file_hash, file_hash
            )));
        }

        // 上传到存储
        self.storage
            .upload_file(&user_id, &file_hash, file_data, Some("application/octet-stream".to_string()))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // 保存文件版本信息到数据库
        let version_id = self
            .save_file_version(&user_id, &device_id, &metadata, &file_hash, received_bytes)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // 更新同步状态
        self.update_sync_state(&user_id, &device_id, &metadata.file_path, SyncStatus::Synced)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // 通知其他设备
        self.notify_file_change(&user_id, &device_id, &metadata.file_path, ChangeType::Modified)
            .await;

        info!("✓ File uploaded successfully: {:?}", metadata.file_path);

        Ok(Response::new(UploadFileResponse {
            success: true,
            message: "File uploaded successfully".to_string(),
            version_id: version_id.to_string(),
            version_number: metadata.version + 1,
        }))
    }

    /// 下载文件（流式）
    async fn download_file(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<Stream::<DownloadFileResponse>>, Status> {
        let req = request.into_inner();
        let user_id = extract_user_id_from_request(&request)?;

        info!("Downloading file: {:?}", req.file_path);

        // 查询文件版本信息
        let file_version = self
            .get_file_version(&user_id, &req.file_path, req.version_number)
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;

        // 从存储下载文件
        let file_data = self
            .storage
            .download_file(&user_id, &file_version.file_hash)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        info!("Downloaded {} bytes", file_data.len());

        // 创建流式响应
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // 发送元数据
        let metadata = FileInfo {
            file_path: file_version.file_path.clone(),
            file_hash: file_version.file_hash,
            file_size: file_version.file_size,
            modified_at: file_version.created_at.timestamp_millis(),
            version: file_version.version_number,
            device_id: file_version.device_id.to_string(),
            is_deleted: file_version.is_deleted,
            file_type: "binary".to_string(),
        };

        let _ = tx
            .send(Ok(DownloadFileResponse {
                payload: Some(Payload::Metadata(metadata)),
            }))
            .await;

        // 分块发送数据
        let chunk_size = 4 * 1024 * 1024; // 4MB chunks
        for (i, chunk) in file_data.chunks(chunk_size).enumerate() {
            let file_chunk = FileChunk {
                chunk_number: i as i64,
                data: chunk.to_vec(),
                offset: (i * chunk_size) as i64,
            };

            let response = DownloadFileResponse {
                payload: Some(Payload::Chunk(file_chunk)),
            };

            if tx.send(Ok(response)).await.is_err() {
                break;
            }
        }

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    /// 解决冲突
    async fn resolve_conflict(
        &self,
        request: Request<ResolveConflictRequest>,
    ) -> Result<Response<ResolveConflictResponse>, Status> {
        let req = request.into_inner();
        let user_id = extract_user_id_from_request(&request)?;

        info!("Resolving conflict: {}", req.conflict_id);

        // 解析冲突 ID
        let conflict_id = Uuid::parse_str(&req.conflict_id)
            .map_err(|_| Status::invalid_argument("Invalid conflict ID"))?;

        // 查询冲突
        let conflict = self
            .get_conflict(&user_id, &conflict_id)
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;

        match req.resolution.as_str() {
            "keep_local" => {
                self.resolve_conflict_keep_local(&conflict).await?;
            }
            "keep_remote" => {
                self.resolve_conflict_keep_remote(&conflict).await?;
            }
            "keep_merged" => {
                if let Some(merged_content) = req.merged_content {
                    self.resolve_conflict_merge(&conflict, &merged_content)
                        .await?;
                } else {
                    return Err(Status::invalid_argument("Missing merged content"));
                }
            }
            "postpone" => {
                // 暂不解决
                return Ok(Response::new(ResolveConflictResponse {
                    success: true,
                    message: "Conflict postponed".to_string(),
                    new_version_id: String::new(),
                }));
            }
            _ => {
                return Err(Status::invalid_argument("Invalid resolution strategy"));
            }
        }

        info!("✓ Conflict resolved: {}", req.conflict_id);

        Ok(Response::new(ResolveConflictResponse {
            success: true,
            message: "Conflict resolved successfully".to_string(),
            new_version_id: conflict_id.to_string(),
        }))
    }

    /// 获取文件历史
    async fn get_file_history(
        &self,
        request: Request<GetFileHistoryRequest>,
    ) -> Result<Response<GetFileHistoryResponse>, Status> {
        let req = request.into_inner();
        let user_id = extract_user_id_from_request(&request)?;

        info!("Fetching file history: {:?}", req.file_path);

        let versions = self
            .get_file_versions(&user_id, &req.file_path, req.limit)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let version_protos: Vec<FileVersion> = versions
            .iter()
            .map(|v| FileVersion {
                version_id: v.id.to_string(),
                version_number: v.version_number,
                file_path: v.file_path.clone(),
                file_hash: v.file_hash.clone(),
                file_size: v.file_size,
                device_id: v.device_id.to_string(),
                created_at: v.created_at.timestamp(),
            })
            .collect();

        Ok(Response::new(GetFileHistoryResponse { versions: version_protos }))
    }

    /// 恢复文件版本
    async fn restore_file_version(
        &self,
        request: Request<RestoreFileVersionRequest>,
    ) -> Result<Response<RestoreFileVersionResponse>, Status> {
        let req = request.into_inner();
        let user_id = extract_user_id_from_request(&request)?;
        let device_id = extract_device_id_from_request(&request)?;

        info!(
            "Restoring file: {:?} to version {}",
            req.file_path,
            req.version_number
        );

        // 查询目标版本
        let target_version = self
            .get_file_version(&user_id, &req.file_path, req.version_number)
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;

        // 创建新版本（恢复）
        let new_version_id = self
            .create_restored_version(&user_id, &device_id, &target_version)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        info!("✓ File restored: {:?}", req.file_path);

        Ok(Response::new(RestoreFileVersionResponse {
            success: true,
            message: "File restored successfully".to_string(),
            restored_file: Some(FileInfo {
                file_path: req.file_path,
                file_hash: target_version.file_hash,
                file_size: target_version.file_size,
                modified_at: chrono::Utc::now().timestamp_millis(),
                version: target_version.version_number + 1,
                device_id: device_id.to_string(),
                is_deleted: false,
                file_type: "binary".to_string(),
            }),
        }))
    }
}
*/

// ===== 内部辅助方法 =====

impl FileSyncGrpcService {
    /// 处理文件变更
    async fn process_file_change(
        &self,
        _user_id: &Uuid,
        _device_id: &Uuid,
        file_info: FileInfo,
    ) -> Result<FileChangeResult> {
        // TODO: 实现文件变更处理逻辑
        debug!("Processing file change: {:?}", file_info.file_path);
        Ok(FileChangeResult::Success)
    }

    /// 通知其他设备
    async fn notify_other_devices(
        &self,
        user_id: &Uuid,
        current_device_id: &Uuid,
        changes: &[FileInfo],
    ) {
        // 获取用户的所有在线设备
        match self.cache.get_online_devices(user_id).await {
            Ok(devices) => {
                for device_id in devices {
                    if device_id != *current_device_id {
                        for change in changes {
                            let notification = FileChangeNotification {
                                file_path: change.file_path.clone(),
                                device_id: *current_device_id,
                                change_type: ChangeType::Modified,
                                timestamp: chrono::Utc::now().timestamp(),
                            };

                            if let Err(e) =
                                self.cache.push_file_change(user_id, &notification).await
                            {
                                error!("Failed to push file change: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to get online devices: {}", e);
            }
        }
    }

    /// 通知单个文件变更
    async fn notify_file_change(
        &self,
        user_id: &Uuid,
        device_id: &Uuid,
        file_path: &str,
        change_type: ChangeType,
    ) {
        let notification = FileChangeNotification {
            file_path: file_path.to_string(),
            device_id: *device_id,
            change_type,
            timestamp: chrono::Utc::now().timestamp(),
        };

        if let Err(e) = self.cache.push_file_change(user_id, &notification).await {
            error!("Failed to push file change: {}", e);
        }
    }

    // TODO: 添加更多辅助方法
    // - get_file_changes_since
    // - save_file_version
    // - update_sync_state
    // - get_file_version
    // - get_conflict
    // - resolve_conflict_* (各种解决策略)
    // - get_file_versions
    // - create_restored_version
}

// ===== 常量 =====

const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_report_changes() {
        // 测试变更上报
    }

    #[tokio::test]
    #[ignore]
    async fn test_upload_file() {
        // 测试文件上传
    }
}
