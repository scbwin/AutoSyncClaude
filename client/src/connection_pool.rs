use crate::error::ClientError;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tonic::transport::Channel;
use tracing::{debug, info};

/// 连接池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 最大连接数
    pub max_connections: usize,

    /// 最小空闲连接数
    pub min_idle_connections: usize,

    /// 连接最大空闲时间（秒）
    pub max_idle_time_secs: u64,

    /// 连接最大生命周期（秒）
    pub max_lifetime_secs: u64,

    /// 连接超时时间（秒）
    pub connection_timeout_secs: u64,

    /// 获取连接超时时间（秒）
    pub acquire_timeout_secs: u64,

    /// 是否启用连接健康检查
    pub enable_health_check: bool,

    /// 健康检查间隔（秒）
    pub health_check_interval_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_idle_connections: 2,
            max_idle_time_secs: 300, // 5 分钟
            max_lifetime_secs: 1800, // 30 分钟
            connection_timeout_secs: 10,
            acquire_timeout_secs: 5,
            enable_health_check: true,
            health_check_interval_secs: 60,
        }
    }
}

/// 连接包装器
struct ConnectionWrapper {
    /// gRPC 通道
    channel: Channel,

    /// 创建时间
    created_at: Instant,

    /// 最后使用时间
    last_used_at: Instant,

    /// 是否在使用中
    in_use: bool,

    /// 使用计数
    use_count: u64,
}

impl ConnectionWrapper {
    fn new(channel: Channel) -> Self {
        let now = Instant::now();
        Self {
            channel,
            created_at: now,
            last_used_at: now,
            in_use: false,
            use_count: 0,
        }
    }

    /// 标记为使用中
    fn mark_in_use(&mut self) {
        self.in_use = true;
        self.last_used_at = Instant::now();
        self.use_count += 1;
    }

    /// 标记为空闲
    fn mark_idle(&mut self) {
        self.in_use = false;
        self.last_used_at = Instant::now();
    }

    /// 检查是否过期
    fn is_expired(&self, max_idle_time: Duration, max_lifetime: Duration) -> bool {
        let idle_time = self.last_used_at.elapsed();
        let lifetime = self.created_at.elapsed();

        idle_time > max_idle_time || lifetime > max_lifetime
    }

    /// 检查是否健康
    async fn is_healthy(&self) -> bool {
        // 简化的健康检查：假设连接是健康的
        // TODO: 实现实际的健康检查逻辑
        true
    }
}

/// 连接池
pub struct ConnectionPool {
    /// 服务器地址
    server_address: String,

    /// 池配置
    config: PoolConfig,

    /// 空闲连接队列（按创建时间排序）
    idle_connections: Arc<Mutex<Vec<ConnectionWrapper>>>,

    /// 活跃连接集合
    active_connections: Arc<RwLock<HashMap<String, ConnectionWrapper>>>,

    /// 信号量（限制最大连接数）
    semaphore: Arc<Semaphore>,

    /// 池是否已关闭
    is_shutdown: Arc<RwLock<bool>>,
}

impl ConnectionPool {
    /// 创建新的连接池
    pub fn new(server_address: String, config: PoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));

        Self {
            server_address,
            config,
            idle_connections: Arc::new(Mutex::new(Vec::new())),
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            semaphore,
            is_shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// 获取连接
    pub async fn acquire(&self) -> Result<PooledConnection, ClientError> {
        // 检查池是否已关闭
        if *self.is_shutdown.read().await {
            return Err(ClientError::internal("连接池已关闭", None));
        }

        // 等待信号量（限制并发连接数）
        let _permit = tokio::time::timeout(
            Duration::from_secs(self.config.acquire_timeout_secs),
            self.semaphore.acquire(),
        )
        .await
        .map_err(|_| ClientError::timeout("获取连接", self.config.acquire_timeout_secs))?
        .map_err(|_| ClientError::internal("信号量关闭", None))?;

        // 尝试从空闲连接中获取
        {
            let mut idle = self.idle_connections.lock().await;

            // 清理过期连接
            let max_idle_time = Duration::from_secs(self.config.max_idle_time_secs);
            let max_lifetime = Duration::from_secs(self.config.max_lifetime_secs);

            idle.retain(|conn| !conn.is_expired(max_idle_time, max_lifetime));

            // 检查健康连接
            if self.config.enable_health_check {
                let mut healthy_connections = Vec::new();

                for conn in idle.drain(..) {
                    if conn.is_healthy().await {
                        healthy_connections.push(conn);
                    } else {
                        debug!("移除不健康的连接");
                    }
                }

                *idle = healthy_connections;
            }

            // 获取第一个空闲连接
            if let Some(mut conn) = idle.pop() {
                conn.mark_in_use();

                let conn_id = format!("conn_{}", conn.use_count);
                self.active_connections
                    .write()
                    .await
                    .insert(conn_id.clone(), conn);

                debug!("从池中获取连接: {}", conn_id);

                return Ok(PooledConnection {
                    pool: self.clone(),
                    conn_id: Some(conn_id),
                });
            }
        }

        // 没有可用连接，创建新连接
        debug!("创建新连接");

        let channel = tokio::time::timeout(
            Duration::from_secs(self.config.connection_timeout_secs),
            self.create_connection(),
        )
        .await
        .map_err(|_| ClientError::timeout("创建连接", self.config.connection_timeout_secs))??;

        let mut wrapper = ConnectionWrapper::new(channel);
        wrapper.mark_in_use();

        let conn_id = format!("conn_{}", wrapper.use_count);
        self.active_connections
            .write()
            .await
            .insert(conn_id.clone(), wrapper);

        info!("创建新连接: {}", conn_id);

        Ok(PooledConnection {
            pool: self.clone(),
            conn_id: Some(conn_id),
        })
    }

    /// 归还连接
    async fn release(&self, conn_id: String) {
        // 从活跃连接中移除
        let mut active = self.active_connections.write().await;
        if let Some(mut conn) = active.remove(&conn_id) {
            conn.mark_idle();

            // 检查连接是否仍然有效
            let max_idle_time = Duration::from_secs(self.config.max_idle_time_secs);
            let max_lifetime = Duration::from_secs(self.config.max_lifetime_secs);

            if conn.is_expired(max_idle_time, max_lifetime) {
                debug!("连接已过期，关闭: {}", conn_id);
            } else {
                // 将连接放回空闲队列
                let idle = self.idle_connections.lock().await;

                // 如果空闲连接过多，关闭这个连接
                if idle.len() >= self.config.max_connections {
                    debug!("空闲连接过多，关闭: {}", conn_id);
                } else {
                    drop(idle);
                    let mut idle = self.idle_connections.lock().await;
                    idle.push(conn);
                    debug!("归还连接到池: {}", conn_id);
                }
            }
        }
    }

    /// 创建新连接
    async fn create_connection(&self) -> Result<Channel, ClientError> {
        Channel::from_shared(self.server_address.clone())
            .map_err(|e| ClientError::network("无效的服务器地址", Some(Box::new(e))))?
            .timeout(Duration::from_secs(self.config.connection_timeout_secs))
            .connect()
            .await
            .map_err(|e| ClientError::network("连接服务器失败", Some(Box::new(e))))
    }

    /// 关闭连接池
    pub async fn shutdown(&self) {
        info!("关闭连接池...");

        *self.is_shutdown.write().await = true;

        // 清空所有连接
        self.idle_connections.lock().await.clear();
        self.active_connections.write().await.clear();

        info!("连接池已关闭");
    }

    /// 获取池统计信息
    pub async fn stats(&self) -> PoolStats {
        let idle_count = self.idle_connections.lock().await.len();
        let active_count = self.active_connections.read().await.len();

        PoolStats {
            total_connections: idle_count + active_count,
            idle_connections: idle_count,
            active_connections: active_count,
            max_connections: self.config.max_connections,
            waiting_for_connection: self.semaphore.available_permits(),
        }
    }

    /// 启动健康检查任务
    pub fn spawn_health_check(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(self.config.health_check_interval_secs));

            loop {
                interval.tick().await;

                if *self.is_shutdown.read().await {
                    break;
                }

                debug!("执行连接池健康检查");

                // 清理过期和不健康的连接
                let max_idle_time = Duration::from_secs(self.config.max_idle_time_secs);
                let max_lifetime = Duration::from_secs(self.config.max_lifetime_secs);

                let mut idle = self.idle_connections.lock().await;
                let before_count = idle.len();

                idle.retain(|conn| !conn.is_expired(max_idle_time, max_lifetime));

                let after_count = idle.len();

                if before_count != after_count {
                    debug!("清理了 {} 个过期连接", before_count - after_count);
                }
            }
        })
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            server_address: self.server_address.clone(),
            config: self.config.clone(),
            idle_connections: Arc::clone(&self.idle_connections),
            active_connections: Arc::clone(&self.active_connections),
            semaphore: Arc::clone(&self.semaphore),
            is_shutdown: Arc::clone(&self.is_shutdown),
        }
    }
}

/// 池化的连接
pub struct PooledConnection {
    /// 连接池
    pool: ConnectionPool,

    /// 连接 ID
    conn_id: Option<String>,
}

impl PooledConnection {
    /// 获取底层通道
    pub fn channel(&self) -> &Channel {
        // 注意：这里需要从活跃连接中获取
        // 由于借用检查器的限制，实际实现可能需要调整
        // 这里简化为返回一个引用
        // 在实际使用中，应该通过 conn_id 获取对应的通道
        unimplemented!("需要重新设计以支持此功能")
    }

    /// 直接获取 gRPC 客户端
    pub async fn get_client<T>(&self) -> Result<T, ClientError>
    where
        T: From<Channel>,
    {
        if let Some(conn_id) = &self.conn_id {
            let active = self.pool.active_connections.read().await;
            if let Some(wrapper) = active.get(conn_id) {
                return Ok(T::from(wrapper.channel.clone()));
            }
        }

        Err(ClientError::internal("连接不存在", None))
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(conn_id) = self.conn_id.take() {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                pool.release(conn_id).await;
            });
        }
    }
}

/// 池统计信息
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// 总连接数
    pub total_connections: usize,

    /// 空闲连接数
    pub idle_connections: usize,

    /// 活跃连接数
    pub active_connections: usize,

    /// 最大连接数
    pub max_connections: usize,

    /// 等待连接的请求数
    pub waiting_for_connection: usize,
}

/// 连接池管理器（单例模式）
pub struct ConnectionPoolManager {
    pools: Arc<RwLock<HashMap<String, Arc<ConnectionPool>>>>,
}

impl ConnectionPoolManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取或创建连接池
    pub async fn get_or_create_pool(
        &self,
        server_address: String,
        config: PoolConfig,
    ) -> Result<Arc<ConnectionPool>, ClientError> {
        // 先尝试读取
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&server_address) {
                return Ok(Arc::clone(pool));
            }
        }

        // 需要创建新池
        let mut pools = self.pools.write().await;

        // 双重检查
        if let Some(pool) = pools.get(&server_address) {
            return Ok(Arc::clone(pool));
        }

        // 创建新池
        let pool = Arc::new(ConnectionPool::new(server_address.clone(), config));

        // 启动健康检查
        pool.clone().spawn_health_check();

        pools.insert(server_address.clone(), Arc::clone(&pool));

        info!("为 {} 创建了新的连接池", server_address);

        Ok(pool)
    }

    /// 关闭所有连接池
    pub async fn shutdown_all(&self) {
        let pools = self.pools.read().await;

        for pool in pools.values() {
            pool.shutdown().await;
        }

        self.pools.write().await.clear();
    }

    /// 获取所有池的统计信息
    pub async fn get_all_stats(&self) -> HashMap<String, PoolStats> {
        let pools = self.pools.read().await;
        let mut stats = HashMap::new();

        for (address, pool) in pools.iter() {
            let pool_stats = pool.stats().await;
            stats.insert(address.clone(), pool_stats);
        }

        stats
    }
}

impl Default for ConnectionPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 性能优化工具
pub struct PerformanceOptimizer {
    /// 连接池管理器
    pool_manager: Arc<ConnectionPoolManager>,

    /// 是否启用批处理
    enable_batching: bool,

    /// 批处理大小
    batch_size: usize,

    /// 是否启用压缩
    enable_compression: bool,
}

impl PerformanceOptimizer {
    /// 创建新的性能优化器
    pub fn new(pool_manager: Arc<ConnectionPoolManager>) -> Self {
        Self {
            pool_manager,
            enable_batching: true,
            batch_size: 10,
            enable_compression: true,
        }
    }

    /// 设置批处理配置
    pub fn with_batching(mut self, enable: bool, batch_size: usize) -> Self {
        self.enable_batching = enable;
        self.batch_size = batch_size;
        self
    }

    /// 设置压缩配置
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    /// 获取连接池管理器
    pub fn pool_manager(&self) -> &ConnectionPoolManager {
        &self.pool_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_idle_connections, 2);
    }

    #[tokio::test]
    async fn test_connection_wrapper_expiry() {
        let wrapper =
            ConnectionWrapper::new(Channel::from_static("http://localhost:50051").connect_lazy());

        let max_idle_time = Duration::from_secs(0); // 立即过期
        let max_lifetime = Duration::from_secs(3600);

        // 连接应该因为空闲时间过期
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(wrapper.is_expired(max_idle_time, max_lifetime));
    }

    #[tokio::test]
    async fn test_pool_manager() {
        let manager = ConnectionPoolManager::new();

        let config = PoolConfig::default();

        // 获取或创建池
        let pool = manager
            .get_or_create_pool("http://localhost:50051".to_string(), config)
            .await;

        assert!(pool.is_ok());

        // 获取现有池
        let pool2 = manager
            .get_or_create_pool("http://localhost:50051".to_string(), PoolConfig::default())
            .await;

        assert!(pool2.is_ok());
    }
}
