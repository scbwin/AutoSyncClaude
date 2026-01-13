use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub minio: MinioConfig,
    pub jwt: JwtConfig,
    pub sync: SyncConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub health_check_port: u16,
    pub max_connections: usize,
    pub timeout: u64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: u64, // seconds
    pub idle_timeout: u64, // seconds
    pub max_lifetime: u64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u64, // seconds
    pub command_timeout: u64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinioConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: String,
    pub timeout: u64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiration: u64, // seconds
    pub refresh_token_expiration: u64, // seconds
    pub issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub max_file_size: u64, // bytes
    pub chunk_size: u64, // bytes
    pub compression_enabled: bool,
    pub version_retention_days: u32,
    pub max_versions_per_file: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // "json" or "pretty"
    pub file: Option<String>,
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self, anyhow::Error> {
        // 加载 .env 文件（如果存在）
        dotenv::dotenv().ok();

        Ok(Self {
            server: ServerConfig {
                host: Self::get_env("SERVER_HOST", "0.0.0.0".to_string()),
                port: Self::get_env("SERVER_PORT", "50051").parse()?,
                health_check_port: Self::get_env("HEALTH_CHECK_PORT", "8080").parse()?,
                max_connections: Self::get_env("MAX_CONNECTIONS", "10000").parse()?,
                timeout: Self::get_env("SERVER_TIMEOUT", "30").parse()?,
            },
            database: DatabaseConfig {
                url: Self::get_env(
                    "DATABASE_URL",
                    "postgresql://claude_sync:password@localhost/claude_sync".to_string(),
                ),
                max_connections: Self::get_env("DB_MAX_CONNECTIONS", "20").parse()?,
                min_connections: Self::get_env("DB_MIN_CONNECTIONS", "5").parse()?,
                acquire_timeout: Self::get_env("DB_ACQUIRE_TIMEOUT", "30").parse()?,
                idle_timeout: Self::get_env("DB_IDLE_TIMEOUT", "600").parse()?,
                max_lifetime: Self::get_env("DB_MAX_LIFETIME", "1800").parse()?,
            },
            redis: RedisConfig {
                url: Self::get_env("REDIS_URL", "redis://127.0.0.1:6379".to_string()),
                max_connections: Self::get_env("REDIS_MAX_CONNECTIONS", "20").parse()?,
                connection_timeout: Self::get_env("REDIS_CONNECTION_TIMEOUT", "5").parse()?,
                command_timeout: Self::get_env("REDIS_COMMAND_TIMEOUT", "5").parse()?,
            },
            minio: MinioConfig {
                endpoint: Self::get_env("MINIO_ENDPOINT", "localhost:9000".to_string()),
                access_key: Self::get_env("MINIO_ACCESS_KEY", "minioadmin".to_string()),
                secret_key: Self::get_env("MINIO_SECRET_KEY", "minioadmin".to_string()),
                bucket: Self::get_env("MINIO_BUCKET", "claude-sync".to_string()),
                region: Self::get_env("MINIO_REGION", "us-east-1".to_string()),
                timeout: Self::get_env("MINIO_TIMEOUT", "30").parse()?,
            },
            jwt: JwtConfig {
                secret: Self::get_env("JWT_SECRET", "your-secret-key-change-it".to_string()),
                access_token_expiration: Self::get_env(
                    "JWT_ACCESS_TOKEN_EXPIRATION",
                    "3600",
                )
                .parse()?,
                refresh_token_expiration: Self::get_env(
                    "JWT_REFRESH_TOKEN_EXPIRATION",
                    "2592000",
                )
                .parse()?,
                issuer: Self::get_env("JWT_ISSUER", "claude-sync".to_string()),
            },
            sync: SyncConfig {
                max_file_size: Self::get_env("MAX_FILE_SIZE", "104857600").parse()?, // 100MB
                chunk_size: Self::get_env("CHUNK_SIZE", "4194304").parse()?   // 4MB
                compression_enabled: Self::get_env("COMPRESSION_ENABLED", "true").parse()?,
                version_retention_days: Self::get_env("VERSION_RETENTION_DAYS", "90").parse()?,
                max_versions_per_file: Self::get_env("MAX_VERSIONS_PER_FILE", "100").parse()?,
            },
            logging: LoggingConfig {
                level: Self::get_env("RUST_LOG", "info".to_string()),
                format: Self::get_env("LOG_FORMAT", "json".to_string()),
                file: std::env::var("LOG_FILE").ok(),
            },
        })
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        // 验证 JWT 密钥长度（至少 32 字节）
        if self.jwt.secret.len() < 32 {
            return Err(anyhow::anyhow!(
                "JWT_SECRET must be at least 32 characters long"
            ));
        }

        // 验证端口范围
        if self.server.port == 0 || self.server.port > 65535 {
            return Err(anyhow::anyhow!("Invalid server port: {}", self.server.port));
        }

        // 验证数据库连接
        if self.database.url.is_empty() {
            return Err(anyhow::anyhow!("DATABASE_URL cannot be empty"));
        }

        // 验证文件大小
        if self.sync.max_file_size == 0 {
            return Err(anyhow::anyhow!("MAX_FILE_SIZE must be greater than 0"));
        }

        // 验证分块大小
        if self.sync.chunk_size == 0 || self.sync.chunk_size > self.sync.max_file_size {
            return Err(anyhow::anyhow!("Invalid CHUNK_SIZE"));
        }

        Ok(())
    }

    /// 获取环境变量或默认值
    fn get_env(key: &str, default: String) -> String {
        std::env::var(key).unwrap_or(default)
    }

    /// 服务器地址
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// 健康检查地址
    pub fn health_check_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.health_check_port)
    }

    /// Access Token 过期时间
    pub fn access_token_expiration(&self) -> Duration {
        Duration::from_secs(self.jwt.access_token_expiration)
    }

    /// Refresh Token 过期时间
    pub fn refresh_token_expiration(&self) -> Duration {
        Duration::from_secs(self.jwt.refresh_token_expiration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        let config = Config::from_env().unwrap();
        config.validate().unwrap();
    }

    #[test]
    fn test_server_address() {
        let config = Config::from_env().unwrap();
        let addr = config.server_address();
        assert!(addr.contains(':'));
    }

    #[test]
    fn test_jwt_secret_validation() {
        let mut config = Config::from_env().unwrap();
        config.jwt.secret = "short".to_string();
        assert!(config.validate().is_err());
    }
}
