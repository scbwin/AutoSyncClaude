use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 文件传输进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    /// 文件路径
    pub file_path: PathBuf,

    /// 总字节数
    pub total_bytes: u64,

    /// 已传输字节数
    pub transferred_bytes: u64,

    /// 开始时间
    pub started_at: DateTime<Utc>,

    /// 完成时间（如果已完成）
    pub completed_at: Option<DateTime<Utc>>,

    /// 是否完成
    pub is_completed: bool,

    /// 是否失败
    pub is_failed: bool,

    /// 错误消息（如果失败）
    pub error_message: Option<String>,
}

impl TransferProgress {
    /// 计算进度百分比
    pub fn progress_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            100.0
        } else {
            (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0
        }
    }

    /// 计算传输速度（字节/秒）
    pub fn transfer_rate(&self) -> f64 {
        let now = if self.is_completed {
            self.completed_at.unwrap_or_else(Utc::now)
        } else {
            Utc::now()
        };

        let elapsed = (now - self.started_at).num_seconds() as f64;

        if elapsed > 0.0 {
            self.transferred_bytes as f64 / elapsed
        } else {
            0.0
        }
    }

    /// 计算剩余时间（秒）
    pub fn estimated_time_remaining(&self) -> Option<f64> {
        if self.is_completed || self.is_failed || self.total_bytes == 0 {
            return None;
        }

        let rate = self.transfer_rate();
        if rate > 0.0 {
            let remaining_bytes = self.total_bytes - self.transferred_bytes;
            Some(remaining_bytes as f64 / rate)
        } else {
            None
        }
    }
}

/// 文件上传请求
#[derive(Debug, Clone)]
pub struct UploadRequest {
    /// 文件路径
    pub file_path: PathBuf,

    /// 用户 ID
    pub user_id: Uuid,

    /// 设备 ID
    pub device_id: Uuid,

    /// 文件哈希（SHA-256）
    pub file_hash: String,

    /// 文件大小
    pub file_size: u64,

    /// 上传 ID（用于断点续传）
    pub upload_id: Option<String>,
}

/// 文件下载请求
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    /// 文件路径
    pub file_path: PathBuf,

    /// 用户 ID
    pub user_id: Uuid,

    /// 版本号（可选，默认最新）
    pub version_number: Option<i64>,
}

/// 文件传输管理器
pub struct TransferManager {
    /// 最大并发上传数
    max_concurrent_uploads: usize,

    /// 最大并发下载数
    max_concurrent_downloads: usize,

    /// 上传信号量
    upload_semaphore: Semaphore,

    /// 下载信号量
    download_semaphore: Semaphore,

    /// 分块大小（字节）
    chunk_size: usize,

    /// 重试次数
    upload_retries: usize,

    /// 下载重试次数
    download_retries: usize,

    /// 重试延迟（秒）
    retry_delay: Duration,
}

impl TransferManager {
    /// 创建新的传输管理器
    pub fn new(
        max_concurrent_uploads: usize,
        max_concurrent_downloads: usize,
        upload_retries: usize,
        download_retries: usize,
        retry_delay: u64,
    ) -> Self {
        Self {
            max_concurrent_uploads,
            max_concurrent_downloads,
            upload_semaphore: Semaphore::new(max_concurrent_uploads),
            download_semaphore: Semaphore::new(max_concurrent_downloads),
            chunk_size: 4 * 1024 * 1024, // 4MB
            upload_retries,
            download_retries,
            retry_delay: Duration::from_secs(retry_delay),
        }
    }

    /// 上传文件（带进度回调）
    pub async fn upload_file<F>(
        &self,
        request: UploadRequest,
        progress_callback: F,
    ) -> Result<TransferProgress>
    where
        F: Fn(TransferProgress) + Send + 'static,
    {
        // 获取上传许可
        let _permit = self.upload_semaphore.acquire().await.unwrap();

        info!("开始上传文件: {:?}", request.file_path);

        let started_at = Utc::now();
        let mut progress = TransferProgress {
            file_path: request.file_path.clone(),
            total_bytes: request.file_size,
            transferred_bytes: 0,
            started_at,
            completed_at: None,
            is_completed: false,
            is_failed: false,
            error_message: None,
        };

        // 读取文件
        let file_content = tokio::fs::read(&request.file_path)
            .await
            .with_context(|| format!("无法读取文件: {:?}", request.file_path))?;

        // 验证文件哈希
        let actual_hash = Self::calculate_hash(&file_content)?;
        if actual_hash != request.file_hash {
            anyhow::bail!(
                "文件哈希不匹配: 期望 {}, 实际 {}",
                request.file_hash,
                actual_hash
            );
        }

        // 分块上传
        let total_chunks = (file_content.len() + self.chunk_size - 1) / self.chunk_size;

        for (i, chunk) in file_content.chunks(self.chunk_size).enumerate() {
            // TODO: 实际上传到服务器的逻辑
            // 这里需要调用 gRPC 客户端的上传方法

            progress.transferred_bytes += chunk.len() as u64;
            progress_callback(progress.clone());

            debug!(
                "上传分块 {}/{}: {} 字节",
                i + 1,
                total_chunks,
                chunk.len()
            );
        }

        progress.is_completed = true;
        progress.completed_at = Some(Utc::now());
        progress_callback(progress.clone());

        info!(
            "文件上传完成: {:?}, 大小: {} 字节",
            request.file_path,
            request.file_size
        );

        Ok(progress)
    }

    /// 下载文件（带进度回调）
    pub async fn download_file<F>(
        &self,
        request: DownloadRequest,
        progress_callback: F,
    ) -> Result<TransferProgress>
    where
        F: Fn(TransferProgress) + Send + 'static,
    {
        // 获取下载许可
        let _permit = self.download_semaphore.acquire().await.unwrap();

        info!("开始下载文件: {:?}", request.file_path);

        let started_at = Utc::now();
        let mut progress = TransferProgress {
            file_path: request.file_path.clone(),
            total_bytes: 0, // 未知大小
            transferred_bytes: 0,
            started_at,
            completed_at: None,
            is_completed: false,
            is_failed: false,
            error_message: None,
        };

        // TODO: 实际从服务器下载的逻辑
        // 这里需要调用 gRPC 客户端的下载方法

        // 确保父目录存在
        if let Some(parent) = request.file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("无法创建目录: {:?}", parent))?;
        }

        // 模拟下载（实际应该从 gRPC 流接收数据）
        progress.total_bytes = 1024 * 1024; // 1MB 示例
        progress.transferred_bytes = progress.total_bytes;
        progress.is_completed = true;
        progress.completed_at = Some(Utc::now());
        progress_callback(progress.clone());

        info!("文件下载完成: {:?}", request.file_path);

        Ok(progress)
    }

    /// 批量上传文件
    pub async fn batch_upload<F>(
        &self,
        requests: Vec<UploadRequest>,
        progress_callback: F,
    ) -> Vec<Result<TransferProgress>>
    where
        F: Fn(TransferProgress) + Clone + Send + 'static,
    {
        let mut handles = Vec::new();

        for request in requests {
            let manager = self.clone_manager();
            let callback = progress_callback.clone();

            let handle = tokio::spawn(async move {
                manager.upload_file(request, callback).await
            });

            handles.push(handle);
        }

        // 等待所有上传完成
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap_or_else(|e| Err(e.into())));
        }

        results
    }

    /// 批量下载文件
    pub async fn batch_download<F>(
        &self,
        requests: Vec<DownloadRequest>,
        progress_callback: F,
    ) -> Vec<Result<TransferProgress>>
    where
        F: Fn(TransferProgress) + Clone + Send + 'static,
    {
        let mut handles = Vec::new();

        for request in requests {
            let manager = self.clone_manager();
            let callback = progress_callback.clone();

            let handle = tokio::spawn(async move {
                manager.download_file(request, callback).await
            });

            handles.push(handle);
        }

        // 等待所有下载完成
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap_or_else(|e| Err(e.into())));
        }

        results
    }

    /// 计算文件哈希
    pub fn calculate_hash(content: &[u8]) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// 计算文件哈希（异步）
    pub async fn calculate_file_hash(path: &Path) -> Result<String> {
        let content = tokio::fs::read(path)
            .await
            .with_context(|| format!("无法读取文件: {:?}", path))?;

        Self::calculate_hash(&content)
    }

    /// 克隆管理器（用于并发）
    fn clone_manager(&self) -> Self {
        Self {
            max_concurrent_uploads: self.max_concurrent_uploads,
            max_concurrent_downloads: self.max_concurrent_downloads,
            upload_semaphore: Semaphore::new(self.max_concurrent_uploads),
            download_semaphore: Semaphore::new(self.max_concurrent_downloads),
            chunk_size: self.chunk_size,
            upload_retries: self.upload_retries,
            download_retries: self.download_retries,
            retry_delay: self.retry_delay,
        }
    }
}

/// 断点续传管理器
pub struct ResumableTransfer {
    /// 传输状态文件路径
    state_file: PathBuf,
}

impl ResumableTransfer {
    /// 创建新的断点续传管理器
    pub fn new(state_dir: PathBuf) -> Self {
        let state_file = state_dir.join("transfer_state.json");
        Self { state_file }
    }

    /// 保存传输状态
    pub async fn save_state(&self, state: &TransferProgress) -> Result<()> {
        // 确保状态目录存在
        if let Some(parent) = self.state_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(state)
            .context("无法序列化传输状态")?;

        tokio::fs::write(&self.state_file, content)
            .await
            .with_context(|| format!("无法写入传输状态: {:?}", self.state_file))?;

        Ok(())
    }

    /// 加载传输状态
    pub async fn load_state(&self) -> Result<Option<TransferProgress>> {
        if !self.state_file.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&self.state_file)
            .await
            .with_context(|| format!("无法读取传输状态: {:?}", self.state_file))?;

        let state = serde_json::from_str(&content)
            .context("无法解析传输状态")?;

        Ok(Some(state))
    }

    /// 删除传输状态
    pub async fn delete_state(&self) -> Result<()> {
        if self.state_file.exists() {
            tokio::fs::remove_file(&self.state_file)
                .await
                .with_context(|| format!("无法删除传输状态: {:?}", self.state_file))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calculate_hash() {
        let content = b"Hello, World!";
        let hash = TransferManager::calculate_hash(content).unwrap();

        assert_eq!(hash.len(), 64); // SHA-256 哈希长度
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_transfer_progress() {
        let progress = TransferProgress {
            file_path: PathBuf::from("test.txt"),
            total_bytes: 1000,
            transferred_bytes: 500,
            started_at: Utc::now(),
            completed_at: None,
            is_completed: false,
            is_failed: false,
            error_message: None,
        };

        assert_eq!(progress.progress_percent(), 50.0);

        // 测试完成状态
        let completed = TransferProgress {
            transferred_bytes: 1000,
            is_completed: true,
            completed_at: Some(Utc::now()),
            ..progress
        };

        assert_eq!(completed.progress_percent(), 100.0);
    }
}
