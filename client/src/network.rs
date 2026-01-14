use crate::error::ClientError;
use crate::retry::{OfflineQueue, RetryConfig, RetryExecutor};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// 网络状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkStatus {
    /// 在线
    Online,

    /// 离线
    Offline,

    /// 重连中
    Reconnecting,

    /// 未知
    Unknown,
}

/// 网络恢复管理器
pub struct NetworkRecoveryManager {
    /// 当前网络状态
    status: Arc<RwLock<NetworkStatus>>,

    /// gRPC 服务器地址
    server_address: String,

    /// 重试配置
    retry_config: RetryConfig,

    /// 重连间隔（秒）
    reconnect_interval_secs: u64,

    /// 最大重连次数（0 表示无限重试）
    max_reconnect_attempts: usize,

    /// 健康检查间隔（秒）
    health_check_interval_secs: u64,

    /// 离线操作队列
    offline_queue: Arc<OfflineQueue<OfflineOperation>>,
}

/// 离线操作
#[derive(Debug, Clone)]
pub enum OfflineOperation {
    /// 文件上传
    FileUpload {
        path: String,
        hash: String,
        size: u64,
    },

    /// 文件下载
    FileDownload {
        path: String,
        version: Option<i64>,
    },

    /// 变更上报
    ReportChanges {
        changes: Vec<ChangeInfo>,
    },
}

/// 变更信息
#[derive(Debug, Clone)]
pub struct ChangeInfo {
    pub file_path: String,
    pub file_hash: String,
    pub file_size: u64,
}

impl NetworkRecoveryManager {
    /// 创建新的网络恢复管理器
    pub fn new(
        server_address: String,
        retry_config: RetryConfig,
        reconnect_interval_secs: u64,
        max_reconnect_attempts: usize,
    ) -> Self {
        Self {
            status: Arc::new(RwLock::new(NetworkStatus::Unknown)),
            server_address,
            retry_config,
            reconnect_interval_secs,
            max_reconnect_attempts,
            health_check_interval_secs: 30,
            offline_queue: Arc::new(OfflineQueue::new(1000)),
        }
    }

    /// 获取当前网络状态
    pub async fn get_status(&self) -> NetworkStatus {
        *self.status.read().await
    }

    /// 设置网络状态
    async fn set_status(&self, status: NetworkStatus) {
        let mut current = self.status.write().await;
        if *current != status {
            info!("网络状态变更: {:?} -> {:?}", *current, status);
            *current = status;
        }
    }

    /// 检查网络连接
    pub async fn check_connection(&self) -> Result<(), ClientError> {
        // TODO: 实现 actual health check
        // 这里使用占位符实现

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ClientError::network("创建 HTTP 客户端失败", Some(Box::new(e))))?;

        // 尝试连接健康检查端点
        let health_url = format!("{}/health", self.server_address.replace("50051", "3000"));

        match client.get(&health_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    self.set_status(NetworkStatus::Online).await;
                    Ok(())
                } else {
                    self.set_status(NetworkStatus::Offline).await;
                    Err(ClientError::network(
                        format!("健康检查失败: {}", response.status()),
                        None,
                    ))
                }
            }
            Err(e) => {
                self.set_status(NetworkStatus::Offline).await;
                Err(ClientError::network(
                    format!("无法连接到服务器: {}", e),
                    Some(Box::new(e)),
                ))
            }
        }
    }

    /// 执行带网络恢复的操作
    pub async fn execute_with_recovery<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> Result<T, ClientError>
    where
        F: Fn() -> Fut + Clone,
        Fut: std::future::Future<Output = Result<T, ClientError>>,
    {
        // 首先检查网络连接
        self.ensure_online().await?;

        // 执行操作
        let executor = RetryExecutor::new(self.retry_config.clone());
        executor.execute(operation, operation_name).await
    }

    /// 确保网络在线
    async fn ensure_online(&self) -> Result<(), ClientError> {
        loop {
            let status = self.get_status().await;

            match status {
                NetworkStatus::Online => return Ok(()),
                NetworkStatus::Offline | NetworkStatus::Unknown => {
                    warn!("网络离线，尝试重连...");
                    self.reconnect().await?;
                }
                NetworkStatus::Reconnecting => {
                    // 正在重连，等待
                    debug!("等待重连完成...");
                    sleep(Duration::from_secs(1)).await;
                    // 继续循环
                }
            }
        }
    }

    /// 重新连接
    async fn reconnect(&self) -> Result<(), ClientError> {
        self.set_status(NetworkStatus::Reconnecting).await;

        let mut attempts = 0;
        let max_attempts = if self.max_reconnect_attempts == 0 {
            usize::MAX
        } else {
            self.max_reconnect_attempts
        };

        loop {
            attempts += 1;

            debug!("重连尝试 {}/{}", attempts, max_attempts);

            match self.check_connection().await {
                Ok(_) => {
                    info!("重连成功");

                    // 处理离线队列中的操作
                    self.process_offline_queue().await?;

                    return Ok(());
                }
                Err(e) => {
                    warn!("重连失败 (尝试 {}/{}): {}", attempts, max_attempts, e.user_message());

                    if attempts >= max_attempts {
                        self.set_status(NetworkStatus::Offline).await;
                        return Err(ClientError::network(
                            format!("重连失败，已达到最大尝试次数 ({})", max_attempts),
                            None,
                        ));
                    }

                    // 等待后重试
                    sleep(Duration::from_secs(self.reconnect_interval_secs)).await;
                }
            }
        }
    }

    /// 添加离线操作到队列
    pub async fn queue_offline_operation(&self, operation: OfflineOperation) -> anyhow::Result<()> {
        self.offline_queue.push(operation).await.map_err(|e| {
            ClientError::internal(
                format!("无法添加离线操作: {}", e.user_message()),
                None,
            )
        })?;

        debug!(
            "操作已添加到离线队列，当前队列大小: {}",
            self.offline_queue.len().await
        );

        Ok(())
    }

    /// 处理离线队列
    async fn process_offline_queue(&self) -> Result<(), ClientError> {
        let operations = self.offline_queue.drain().await;

        if operations.is_empty() {
            return Ok(());
        }

        info!("处理离线队列中的 {} 个操作", operations.len());

        for operation in operations {
            if let Err(e) = self.process_operation(operation.clone()).await {
                warn!("处理离线操作失败: {}", e.user_message());

                // 将失败的操作重新放回队列
                self.offline_queue.push(operation).await?;
            }
        }

        Ok(())
    }

    /// 处理单个离线操作
    async fn process_operation(&self, operation: OfflineOperation) -> Result<(), ClientError> {
        match operation {
            OfflineOperation::FileUpload { path, hash: _, size: _ } => {
                info!("处理离线上传: {}", path);
                // TODO: 实现实际的上传逻辑
                Ok(())
            }
            OfflineOperation::FileDownload { path, version } => {
                info!("处理离线下载: {} (版本: {:?})", path, version);
                // TODO: 实现实际的下载逻辑
                Ok(())
            }
            OfflineOperation::ReportChanges { changes } => {
                info!("处理离线变更上报: {} 个文件", changes.len());
                // TODO: 实现实际的变更上报逻辑
                Ok(())
            }
        }
    }

    /// 启动健康检查任务
    pub fn spawn_health_check_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                self.health_check_interval_secs,
            ));

            loop {
                interval.tick().await;

                debug!("执行定期健康检查");

                match self.check_connection().await {
                    Ok(_) => {
                        debug!("健康检查通过");
                    }
                    Err(e) => {
                        warn!("健康检查失败: {}", e.user_message());

                        // 尝试重连
                        if let Err(reconnect_err) = self.reconnect().await {
                            warn!("自动重连失败: {}", reconnect_err.user_message());
                        }
                    }
                }
            }
        })
    }

    /// 启动网络监控任务
    pub fn spawn_network_monitor(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                let status = self.get_status().await;
                debug!("网络状态: {:?}", status);

                // 如果离线，尝试重连
                if status == NetworkStatus::Offline {
                    info!("检测到网络离线，尝试重连...");
                    if let Err(e) = self.reconnect().await {
                        warn!("重连失败: {}", e.user_message());
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_status() {
        let manager = NetworkRecoveryManager::new(
            "http://localhost:50051".to_string(),
            RetryConfig::default(),
            5,
            3,
        );

        assert_eq!(manager.get_status().await, NetworkStatus::Unknown);

        manager.set_status(NetworkStatus::Online).await;
        assert_eq!(manager.get_status().await, NetworkStatus::Online);
    }

    #[tokio::test]
    async fn test_offline_queue() {
        let manager = NetworkRecoveryManager::new(
            "http://localhost:50051".to_string(),
            RetryConfig::default(),
            5,
            3,
        );

        let operation = OfflineOperation::FileUpload {
            path: "/test/path".to_string(),
            hash: "abc123".to_string(),
            size: 1024,
        };

        assert!(manager.queue_offline_operation(operation.clone()).await.is_ok());
        assert_eq!(manager.offline_queue.len().await, 1);
    }
}
