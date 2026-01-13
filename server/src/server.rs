use crate::cache::{Cache, RedisPool};
use crate::config::Config;
use crate::db::DbPool;
use crate::grpc::{AuthGrpcService, DeviceGrpcService, FileSyncGrpcService, NotificationGrpcService};
use crate::storage::StorageService;
use anyhow::Result;
use std::net::SocketAddr;
use tokio::signal;
use tonic::transport::Server;
use tracing::{error, info, warn};

/// gRPC 服务器
pub struct GrpcServer {
    config: Config,
    pool: DbPool,
    cache: Cache,
    storage: StorageService,
    redis_pool: cache::RedisPool,
}

impl GrpcServer {
    /// 创建新的服务器实例
    pub async fn new(config: Config) -> Result<Self> {
        // 连接数据库
        let pool = DbPool::from_config(&config).await?;

        // 连接 Redis
        let redis_pool = RedisPool::from_config(&config).await?;
        let cache = Cache::new(redis_pool.inner().clone());

        // 连接 MinIO
        let storage = StorageService::from_config(&config).await?;

        Ok(Self {
            config,
            pool,
            cache,
            storage,
            redis_pool,
        })
    }

    /// 获取数据库连接池
    pub fn get_pool(&self) -> DbPool {
        self.pool.clone()
    }

    /// 获取 Redis 连接池
    pub fn get_redis_pool(&self) -> RedisPool {
        self.redis_pool.clone()
    }

    /// 获取存储服务
    pub fn get_storage(&self) -> StorageService {
        self.storage.clone()
    }

    /// 启动服务器
    pub async fn serve(self) -> Result<()> {
        let addr: SocketAddr = self.config.server_address().parse()?;

        info!("🚀 Starting gRPC server on {}", addr);

        // 创建 gRPC 服务实例
        let auth_service = AuthGrpcService::new(
            self.pool.clone(),
            self.cache.clone(),
            self.config.clone(),
        );

        let device_service = DeviceGrpcService::new(self.pool.clone());

        let sync_service = FileSyncGrpcService::new(
            self.pool.clone(),
            self.cache.clone(),
            self.storage,
        );

        let notification_service = NotificationGrpcService::new(self.pool, self.cache);

        // TODO: 取消注释下面的代码（需要等待 protobuf 生成）
        /*
        let addr = SocketAddr::from(addr);

        let svc = Server::builder()
            .add_service(AuthServiceServer::new(auth_service))
            .add_service(DeviceServiceServer::new(device_service))
            .add_service(FileSyncServiceServer::new(sync_service))
            .add_service(NotificationServiceServer::new(notification_service))
            .serve(addr);

        info!("✓ gRPC server listening on {}", addr);

        // 等待 Ctrl+C
        tokio::select! {
            result = svc => {
                match result {
                    Ok(_) => {
                        info!("Server completed successfully");
                        Ok(())
                    }
                    Err(e) => {
                        error!("Server error: {}", e);
                        Err(e.into())
                    }
                }
            }
            _ = Self::shutdown_signal() => {
                info!("Received shutdown signal");
                Ok(())
            }
        }
        */

        // 临时实现：模拟服务器启动
        info!("⚠ gRPC services initialized but not started");
        info!("   Services ready:");
        info!("   - AuthService");
        info!("   - DeviceService");
        info!("   - FileSyncService");
        info!("   - NotificationService");
        info!("\n💡 To start the actual server:");
        info!("   1. Compile protobuf definitions: cd proto && ./build.sh");
        info!("   2. Uncomment server code in src/server.rs");
        info!("   3. Run: cargo run");

        // 模拟等待信号
        Self::shutdown_signal().await?;
        info!("Shutting down...");

        Ok(())
    }

    /// 等待关闭信号
    async fn shutdown_signal() -> Result<()> {
        #[cfg(unix)]
        {
            // Unix 系统的信号处理
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate())?;
            let mut sigint = signal(SignalKind::interrupt())?;

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT");
                }
            }
        }

        #[cfg(windows)]
        {
            // Windows 的信号处理
            let ctrl_c = tokio::signal::ctrl_c();
            ctrl_c.await?;
            info!("Received Ctrl+C");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_server_creation() {
        let config = Config::from_env().unwrap();
        let server = GrpcServer::new(config).await;
        assert!(server.is_ok());
    }
}
