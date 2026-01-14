use crate::error::ClientError;
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: usize,

    /// 初始重试延迟（毫秒）
    pub initial_delay_ms: u64,

    /// 最大重试延迟（毫秒）
    pub max_delay_ms: u64,

    /// 指数退避倍数
    pub multiplier: f64,

    /// 随机化因子（0.0 - 1.0）
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

impl RetryConfig {
    /// 创建新的重试配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 设置初始延迟
    pub fn with_initial_delay_ms(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    /// 设置最大延迟
    pub fn with_max_delay_ms(mut self, max_delay_ms: u64) -> Self {
        self.max_delay_ms = max_delay_ms;
        self
    }

    /// 计算重试延迟（指数退避 + 随机抖动）
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        // 指数退避
        let delay_ms = (self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32))
            .min(self.max_delay_ms as f64) as u64;

        // 添加随机抖动
        let jitter = (delay_ms as f64 * self.jitter_factor * (rand::random::<f64>() - 0.5) * 2.0)
            .abs() as i64;

        let final_delay_ms = (delay_ms as i64 + jitter).max(0) as u64;

        Duration::from_millis(final_delay_ms)
    }
}

/// 重试策略
#[derive(Debug, Clone, Copy)]
pub enum RetryStrategy {
    /// 指数退避
    ExponentialBackoff,

    /// 固定延迟
    FixedDelay,

    /// 立即重试
    Immediate,

    /// 自定义策略
    Custom,
}

/// 重试结果
#[derive(Debug)]
pub enum RetryResult<T> {
    /// 成功
    Success(T),

    /// 重试失败
    Failed {
        last_error: ClientError,
        attempts: usize,
    },
}

/// 重试执行器
pub struct RetryExecutor {
    config: RetryConfig,
    strategy: RetryStrategy,
}

impl RetryExecutor {
    /// 创建新的重试执行器
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            strategy: RetryStrategy::ExponentialBackoff,
        }
    }

    /// 设置重试策略
    pub fn with_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// 执行带重试的操作
    pub async fn execute<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> Result<T, ClientError>
    where
        F: Fn() -> Fut + Clone,
        Fut: std::future::Future<Output = Result<T, ClientError>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("操作 '{}' 在第 {} 次重试后成功", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    last_error = Some(err.clone());

                    // 检查是否可重试
                    if !err.is_retryable() {
                        debug!("操作 '{}' 不可重试: {}", operation_name, err);
                        return Err(err);
                    }

                    // 如果是最后一次尝试，不再重试
                    if attempt == self.config.max_retries {
                        warn!(
                            "操作 '{}' 在 {} 次尝试后失败",
                            operation_name,
                            self.config.max_retries + 1
                        );
                        return Err(err);
                    }

                    // 计算延迟
                    let delay = match self.strategy {
                        RetryStrategy::ExponentialBackoff => self.config.calculate_delay(attempt),
                        RetryStrategy::FixedDelay => {
                            Duration::from_millis(self.config.initial_delay_ms)
                        }
                        RetryStrategy::Immediate => Duration::from_millis(0),
                        RetryStrategy::Custom => self.config.calculate_delay(attempt),
                    };

                    warn!(
                        "操作 '{}' 失败 (尝试 {}/{}): {}. {} 毫秒后重试...",
                        operation_name,
                        attempt + 1,
                        self.config.max_retries + 1,
                        err.user_message(),
                        delay.as_millis()
                    );

                    sleep(delay).await;
                }
            }
        }

        // 理论上不会到达这里
        Err(last_error.unwrap_or_else(|| ClientError::internal("未知错误", None)))
    }

    /// 执行带重试的操作（返回详细结果）
    pub async fn execute_with_result<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> RetryResult<T>
    where
        F: Fn() -> Fut + Clone,
        Fut: std::future::Future<Output = Result<T, ClientError>>,
    {
        match self.execute(operation, operation_name).await {
            Ok(result) => RetryResult::Success(result),
            Err(err) => RetryResult::Failed {
                last_error: err,
                attempts: self.config.max_retries + 1,
            },
        }
    }
}

/// 离线队列（用于网络恢复时处理）
pub struct OfflineQueue<T> {
    queue: std::sync::Arc<tokio::sync::Mutex<Vec<T>>>,
    max_size: usize,
}

impl<T> OfflineQueue<T> {
    /// 创建新的离线队列
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            max_size,
        }
    }

    /// 添加项目到队列
    pub async fn push(&self, item: T) -> Result<(), ClientError> {
        let mut queue = self.queue.lock().await;

        if queue.len() >= self.max_size {
            return Err(ClientError::internal(
                format!("离线队列已满 (最大: {})", self.max_size),
                None,
            ));
        }

        queue.push(item);
        Ok(())
    }

    /// 从队列中取出所有项目
    pub async fn drain(&self) -> Vec<T> {
        let mut queue = self.queue.lock().await;
        std::mem::take(&mut *queue)
    }

    /// 获取队列大小
    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// 检查队列是否为空
    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }

    /// 清空队列
    pub async fn clear(&self) {
        self.queue.lock().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_calculate_delay() {
        let config = RetryConfig::new()
            .with_initial_delay_ms(1000)
            .with_max_retries(5);

        let delay1 = config.calculate_delay(0);
        let delay2 = config.calculate_delay(1);
        let delay3 = config.calculate_delay(2);

        // 延迟应该指数增长
        assert!(delay2.as_millis() >= delay1.as_millis());
        assert!(delay3.as_millis() >= delay2.as_millis());
    }

    #[tokio::test]
    async fn test_retry_executor_success() {
        let config = RetryConfig::new().with_max_retries(3);
        let executor = RetryExecutor::new(config);

        let result = executor
            .execute(
                || async { Ok::<_, ClientError>("success") },
                "test_operation",
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_retry_executor_retry() {
        use std::sync::atomic::{AtomicI32, Ordering};
        use std::sync::Arc;

        let config = RetryConfig::new()
            .with_max_retries(3)
            .with_initial_delay_ms(10);
        let executor = RetryExecutor::new(config);

        let attempt_count = Arc::new(AtomicI32::new(0));

        let result = executor
            .execute(
                || {
                    let counter = attempt_count.clone();
                    async move {
                        let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
                        if count < 3 {
                            Err(ClientError::network("临时错误", None))
                        } else {
                            Ok::<_, ClientError>("success")
                        }
                    }
                },
                "test_operation",
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_executor_exhausted() {
        let config = RetryConfig::new()
            .with_max_retries(2)
            .with_initial_delay_ms(10);
        let executor = RetryExecutor::new(config);

        let result = executor
            .execute(
                || async { Err::<(), _>(ClientError::network("持续错误", None)) },
                "test_operation",
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_offline_queue() {
        let queue = OfflineQueue::new(10);

        // 添加项目
        for i in 1..=5 {
            queue.push(i).await.unwrap();
        }

        assert_eq!(queue.len().await, 5);
        assert!(!queue.is_empty().await);

        // 取出所有项目
        let items = queue.drain().await;
        assert_eq!(items.len(), 5);
        assert!(queue.is_empty().await);

        // 测试队列满
        for i in 1..=11 {
            let result = queue.push(i).await;
            if i == 11 {
                assert!(result.is_err());
            } else {
                assert!(result.is_ok());
            }
        }
    }
}
