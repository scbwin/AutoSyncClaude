use crate::config::Config;
use anyhow::Result;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use std::time::Duration;
use tracing::{error, info};

/// Redis 连接池
#[derive(Clone)]
pub struct RedisPool {
    pool: Pool,
}

impl RedisPool {
    /// 从配置创建 Redis 连接池
    pub async fn from_config(config: &Config) -> Result<Self> {
        info!("Connecting to Redis...");

        let cfg = Config::from_url(config.redis.url.clone());
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| anyhow::anyhow!("Failed to create Redis pool: {}", e))?;

        // 测试连接
        {
            let mut conn = pool
                .get()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get Redis connection: {}", e))?;

            let _: String = redis::cmd("PING")
                .query_async(&mut conn)
                .await
                .map_err(|e| anyhow::anyhow!("Redis PING failed: {}", e))?;
        }

        info!("Redis connected successfully");

        Ok(Self { pool })
    }

    /// 获取连接池
    pub fn inner(&self) -> &Pool {
        &self.pool
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }
}

/// Redis 缓存操作
pub struct Cache {
    pool: Pool,
}

impl Cache {
    /// 创建新的 Cache 实例
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// ===== Token 黑名单操作 =====

    /// 将 Token 加入黑名单
    pub async fn revoke_token(&self, jti: &uuid::Uuid, expires_at: i64) -> Result<()> {
        let key = format!("token:blacklist:{}", jti);
        let ttl = expires_at - chrono::Utc::now().timestamp();
        let ttl = ttl.max(0) as usize;

        let mut conn = self.pool.get().await?;
        conn.set_ex(key, "1", ttl).await?;

        Ok(())
    }

    /// 检查 Token 是否在黑名单中
    pub async fn is_token_revoked(&self, jti: &uuid::Uuid) -> Result<bool> {
        let key = format!("token:blacklist:{}", jti);
        let mut conn = self.pool.get().await?;
        let exists: bool = conn.exists(&key).await?;
        Ok(exists)
    }

    /// ===== 在线设备管理 =====

    /// 设备上线
    pub async fn device_online(&self, device_id: &uuid::Uuid, user_id: &uuid::Uuid) -> Result<()> {
        let key = format!("device:online:{}", user_id);
        let mut conn = self.pool.get().await?;

        // 添加到在线设备集合
        conn.sadd(&key, device_id.to_string()).await?;

        // 设置过期时间（30 分钟）
        conn.expire(&key, 1800).await?;

        Ok(())
    }

    /// 设备离线
    pub async fn device_offline(&self, device_id: &uuid::Uuid, user_id: &uuid::Uuid) -> Result<()> {
        let key = format!("device:online:{}", user_id);
        let mut conn = self.pool.get().await?;

        conn.srem(&key, device_id.to_string()).await?;

        Ok(())
    }

    /// 获取用户所有在线设备
    pub async fn get_online_devices(&self, user_id: &uuid::Uuid) -> Result<Vec<uuid::Uuid>> {
        let key = format!("device:online:{}", user_id);
        let mut conn = self.pool.get().await?;

        let devices: Vec<String> = conn.smembers(&key).await?;

        let devices = devices
            .iter()
            .filter_map(|s| uuid::Uuid::parse_str(s).ok())
            .collect();

        Ok(devices)
    }

    /// 检查设备是否在线
    pub async fn is_device_online(&self, device_id: &uuid::Uuid, user_id: &uuid::Uuid) -> Result<bool> {
        let key = format!("device:online:{}", user_id);
        let mut conn = self.pool.get().await?;

        let is_member: bool = conn.sismember(&key, device_id.to_string()).await?;

        Ok(is_member)
    }

    /// ===== 变更通知队列 =====

    /// 添加文件变更到队列
    pub async fn push_file_change(
        &self,
        user_id: &uuid::Uuid,
        change: &FileChangeNotification,
    ) -> Result<()> {
        let key = format!("changes:{}", user_id);
        let mut conn = self.pool.get().await?;

        let value = serde_json::to_string(change)?;
        conn.rpush(&key, value).await?;

        // 限制队列长度（最多保留 1000 条）
        conn.ltrim(&key, -1000, -1).await?;

        Ok(())
    }

    /// 获取文件变更列表
    pub async fn get_file_changes(
        &self,
        user_id: &uuid::Uuid,
        count: usize,
    ) -> Result<Vec<FileChangeNotification>> {
        let key = format!("changes:{}", user_id);
        let mut conn = self.pool.get().await?;

        let changes: Vec<String> = conn.lpop(&key, count).await?;

        let changes = changes
            .iter()
            .filter_map(|s| serde_json::from_str(s).ok())
            .collect();

        Ok(changes)
    }

    /// ===== 通用缓存操作 =====

    /// 设置缓存
    pub async fn set(
        &self,
        key: &str,
        value: &str,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let mut conn = self.pool.get().await?;

        if let Some(ttl) = ttl {
            conn.set_ex(key, value, ttl.as_secs() as usize).await?;
        } else {
            conn.set(key, value).await?;
        }

        Ok(())
    }

    /// 获取缓存
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.pool.get().await?;
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    /// 删除缓存
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;
        conn.del(key).await?;
        Ok(())
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.pool.get().await?;
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    /// 设置过期时间
    pub async fn expire(&self, key: &str, ttl: Duration) -> Result<()> {
        let mut conn = self.pool.get().await?;
        conn.expire(key, ttl.as_secs() as usize).await?;
        Ok(())
    }

    /// 批量删除
    pub async fn delete_multiple(&self, keys: &[String]) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }

        let mut conn = self.pool.get().await?;
        conn.del(keys).await?;
        Ok(())
    }

    /// ===== 计数器操作 =====

    /// 增加计数器
    pub async fn incr(&self, key: &str) -> Result<i64> {
        let mut conn = self.pool.get().await?;
        let value: i64 = conn.incr(key, 1).await?;
        Ok(value)
    }

    /// 减少计数器
    pub async fn decr(&self, key: &str) -> Result<i64> {
        let mut conn = self.pool.get().await?;
        let value: i64 = conn.decr(key, 1).await?;
        Ok(value)
    }

    /// 获取计数器值
    pub async fn get_counter(&self, key: &str) -> Result<i64> {
        let mut conn = self.pool.get().await?;
        let value: i64 = conn.get(key).await?;
        Ok(value)
    }
}

/// 文件变更通知
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileChangeNotification {
    pub file_path: String,
    pub device_id: uuid::Uuid,
    pub change_type: ChangeType,
    pub timestamp: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要 Redis 连接
    async fn test_cache_operations() {
        // 测试基本缓存操作
    }

    #[tokio::test]
    #[ignore]
    async fn test_token_revocation() {
        // 测试 Token 撤销
    }

    #[tokio::test]
    #[ignore]
    async fn test_online_devices() {
        // 测试在线设备管理
    }
}
