mod auth;
mod cache;
mod config;
mod db;
mod grpc;
mod health;
mod models;
// proto 模块由 build.rs 在构建时生成，直接引用生成的文件
// 路径相对于 src/ 目录（main.rs 所在目录）
#[path = "proto/sync.rs"]
mod sync;
mod server;
mod storage;

use anyhow::Result;
use server::GrpcServer;
use std::sync::Arc;
use tracing::{error, info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Claude Sync Server v0.1.0");
    info!("Starting server initialization...");

    // 加载配置
    let config = config::Config::from_env()?;
    config.validate()?;
    info!("✓ Configuration loaded and validated");

    // 创建 gRPC 服务器实例（会自动连接所有服务）
    let grpc_server = GrpcServer::new(config.clone()).await?;
    info!("✓ All services initialized successfully");
    info!("  - Database: Connected");
    info!("  - Redis: Connected");
    info!("  - MinIO: Connected");

    // 提取服务组件用于健康检查
    let pool = grpc_server.get_pool();
    let redis_pool = grpc_server.get_redis_pool();
    let storage = grpc_server.get_storage();

    // 启动健康检查服务器
    let health_check_addr = config.health_check_address();
    let health_service = health::HealthCheckService::new(
        Arc::new(pool.clone()),
        Arc::new(redis_pool.clone()),
        Arc::new(storage.clone()),
    );

    let health_addr_for_log = health_check_addr.clone();
    tokio::spawn(async move {
        if let Err(e) = health_service.serve(health_check_addr).await {
            error!("Health check server error: {}", e);
        }
    });

    info!("✓ Health check server started on {}", health_addr_for_log);

    // 启动 gRPC 服务器
    info!("🎯 Starting gRPC server on {}...", config.server_address());

    if let Err(e) = grpc_server.serve().await {
        error!("gRPC server error: {}", e);
        return Err(e.into());
    }

    info!("✓ gRPC server started successfully");

    Ok(())
}
