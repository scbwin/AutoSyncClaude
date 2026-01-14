use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::info;

use crate::config::Config;

/// 数据库连接池
#[derive(Clone)]
pub struct DbPool {
    pool: sqlx::PgPool,
}

impl DbPool {
    /// 从配置创建数据库连接池
    pub async fn from_config(config: &Config) -> Result<Self> {
        info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(config.database.max_connections as u32)
            .min_connections(config.database.min_connections as u32)
            .acquire_timeout(Duration::from_secs(config.database.acquire_timeout))
            .idle_timeout(Duration::from_secs(config.database.idle_timeout))
            .max_lifetime(Duration::from_secs(config.database.max_lifetime))
            .connect(&config.database.url)
            .await?;

        info!("Database connected successfully");

        // 运行迁移（可选）
        // Self::run_migrations(&pool).await?;

        Ok(Self { pool })
    }

    /// 获取 SQLx 连接池
    pub fn inner(&self) -> &sqlx::PgPool {
        &self.pool
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }

    /// 运行数据库迁移
    #[allow(dead_code)]
    async fn run_migrations(pool: &sqlx::PgPool) -> Result<()> {
        info!("Running database migrations...");
        sqlx::migrate!("./migrations").run(pool).await?;
        info!("Migrations completed successfully");
        Ok(())
    }
}

/// 数据库事务辅助宏
#[macro_export]
macro_rules! transaction {
    ($pool:expr, $body:expr) => {{
        use anyhow::Result;
        let mut tx = $pool.begin().await?;
        let result = async move {
            $body
        };
        let result = result.await;
        match result {
            Ok(r) => {
                tx.commit().await?;
                Ok(r)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e)
            }
        }
    }};
}

/// 用户查询辅助结构
pub struct UserRepository;

impl UserRepository {
    /// 根据邮箱查找用户
    pub async fn find_by_email(pool: &sqlx::PgPool, email: &str) -> Result<Option<UserRow>> {
        let user = sqlx::query_as!(
            UserRow,
            r#"
            SELECT id, username, email, password_hash, created_at, updated_at, is_active
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    /// 根据用户名查找用户
    pub async fn find_by_username(pool: &sqlx::PgPool, username: &str) -> Result<Option<UserRow>> {
        let user = sqlx::query_as!(
            UserRow,
            r#"
            SELECT id, username, email, password_hash, created_at, updated_at, is_active
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    /// 根据 ID 查找用户
    pub async fn find_by_id(pool: &sqlx::PgPool, id: &uuid::Uuid) -> Result<Option<UserRow>> {
        let user = sqlx::query_as!(
            UserRow,
            r#"
            SELECT id, username, email, password_hash, created_at, updated_at, is_active
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    /// 创建新用户
    pub async fn create(
        pool: &sqlx::PgPool,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<UserRow> {
        let user = sqlx::query_as!(
            UserRow,
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, username, email, password_hash, created_at, updated_at, is_active
            "#,
            username,
            email,
            password_hash
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    /// 更新用户最后登录时间
    pub async fn update_last_login(pool: &sqlx::PgPool, user_id: &uuid::Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE users
            SET updated_at = NOW()
            WHERE id = $1
            "#,
            user_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// 设备查询辅助结构
pub struct DeviceRepository;

impl DeviceRepository {
    /// 根据用户 ID 查找所有设备
    pub async fn find_by_user(
        pool: &sqlx::PgPool,
        user_id: &uuid::Uuid,
    ) -> Result<Vec<DeviceRow>> {
        let devices = sqlx::query_as!(
            DeviceRow,
            r#"
            SELECT id, user_id, device_name, device_type, device_fingerprint,
                   last_seen, created_at, is_active
            FROM devices
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(pool)
        .await?;

        Ok(devices)
    }

    /// 根据指纹查找设备
    pub async fn find_by_fingerprint(
        pool: &sqlx::PgPool,
        fingerprint: &str,
    ) -> Result<Option<DeviceRow>> {
        let device = sqlx::query_as!(
            DeviceRow,
            r#"
            SELECT id, user_id, device_name, device_type, device_fingerprint,
                   last_seen, created_at, is_active
            FROM devices
            WHERE device_fingerprint = $1
            "#,
            fingerprint
        )
        .fetch_optional(pool)
        .await?;

        Ok(device)
    }

    /// 创建新设备
    pub async fn create(
        pool: &sqlx::PgPool,
        user_id: &uuid::Uuid,
        device_name: &str,
        device_type: &str,
        device_fingerprint: &str,
    ) -> Result<DeviceRow> {
        let device = sqlx::query_as!(
            DeviceRow,
            r#"
            INSERT INTO devices (user_id, device_name, device_type, device_fingerprint)
            VALUES ($1, $2, $3, $4)
            RETURNING id, user_id, device_name, device_type, device_fingerprint,
                      last_seen, created_at, is_active
            "#,
            user_id,
            device_name,
            device_type,
            device_fingerprint
        )
        .fetch_one(pool)
        .await?;

        Ok(device)
    }

    /// 更新设备最后在线时间
    pub async fn update_last_seen(pool: &sqlx::PgPool, device_id: &uuid::Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE devices
            SET last_seen = NOW()
            WHERE id = $1
            "#,
            device_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 删除设备
    pub async fn delete(pool: &sqlx::PgPool, device_id: &uuid::Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE devices
            SET is_active = false
            WHERE id = $1
            "#,
            device_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Token 查询辅助结构
pub struct TokenRepository;

impl TokenRepository {
    /// 保存 Token
    pub async fn save(
        pool: &sqlx::PgPool,
        user_id: &uuid::Uuid,
        device_id: Option<&uuid::Uuid>,
        token_hash: &str,
        token_prefix: &str,
        expires_at: &chrono::DateTime<chrono::Utc>,
    ) -> Result<TokenRow> {
        let token = sqlx::query_as!(
            TokenRow,
            r#"
            INSERT INTO access_tokens (user_id, device_id, token_hash, token_prefix, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, device_id, token_hash, token_prefix,
                      expires_at, created_at, last_used, is_revoked
            "#,
            user_id,
            device_id,
            token_hash,
            token_prefix,
            expires_at
        )
        .fetch_one(pool)
        .await?;

        Ok(token)
    }

    /// 根据 Token 哈希查找
    pub async fn find_by_hash(pool: &sqlx::PgPool, token_hash: &str) -> Result<Option<TokenRow>> {
        let token = sqlx::query_as!(
            TokenRow,
            r#"
            SELECT id, user_id, device_id, token_hash, token_prefix,
                   expires_at, created_at, last_used, is_revoked
            FROM access_tokens
            WHERE token_hash = $1
            "#,
            token_hash
        )
        .fetch_optional(pool)
        .await?;

        Ok(token)
    }

    /// 撤销 Token
    pub async fn revoke(pool: &sqlx::PgPool, token_id: &uuid::Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE access_tokens
            SET is_revoked = true
            WHERE id = $1
            "#,
            token_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 更新 Token 最后使用时间
    pub async fn update_last_used(pool: &sqlx::PgPool, token_id: &uuid::Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE access_tokens
            SET last_used = NOW()
            WHERE id = $1
            "#,
            token_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 清理过期 Token
    pub async fn cleanup_expired(pool: &sqlx::PgPool) -> Result<u64> {
        let result: sqlx::postgres::PgQueryResult = sqlx::query!(
            r#"
            DELETE FROM access_tokens
            WHERE expires_at < NOW() - INTERVAL '7 days'
            "#
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}

// ===== 数据行结构 =====

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct DeviceRow {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub device_name: String,
    pub device_type: String,
    pub device_fingerprint: String,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct TokenRow {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub device_id: Option<uuid::Uuid>,
    pub token_hash: String,
    pub token_prefix: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub is_revoked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要数据库连接
    async fn test_user_crud() {
        // 测试用户 CRUD 操作
    }
}
