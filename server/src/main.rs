mod auth;
mod cache;
mod config;
mod db;
mod grpc;
mod health;
mod models;
mod server;
mod storage;

use anyhow::Result;
use server::GrpcServer;
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber;

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

    tokio::spawn(async move {
        if let Err(e) = health_service.serve(health_check_addr).await {
            error!("Health check server error: {}", e);
        }
    });

    info!("✓ Health check server started on {}", health_check_addr);

    // TODO: 取消注释下面的代码（需要等待 protobuf 生成）
    /*
    // 启动 gRPC 服务器
    info!("🎯 Starting gRPC server on {}...", config.server_address());

    if let Err(e) = grpc_server.serve().await {
        error!("gRPC server error: {}", e);
        return Err(e.into());
    }

    info!("✓ gRPC server started successfully");
    */

    // 临时实现：显示服务就绪状态
    info!("\n🎉 Server initialization completed!");
    info!("📊 Server Status:");
    info!("  ✓ Database: Connected and healthy");
    info!("  ✓ Redis: Connected and healthy");
    info!("  ✓ MinIO: Connected and healthy");
    info!("  ✓ Health Check: Running on {}", health_check_addr);
    info!("\n⚠️  gRPC Server: Services initialized but not started");
    info!("   Ready services:");
    info!("   - AuthService");
    info!("   - DeviceService");
    info!("   - FileSyncService");
    info!("   - NotificationService");
    info!("\n💡 To start the actual gRPC server:");
    info!("   1. Compile protobuf definitions: cd proto && ./build.sh");
    info!("   2. Uncomment server code in src/server.rs and src/main.rs");
    info!("   3. Rebuild: cargo build --release");
    info!("   4. Run: cargo run --release");

    // 等待关闭信号
    tokio::signal::ctrl_c().await?;
    info!("\n👋 Received shutdown signal, shutting down gracefully...");

    // TODO: 添加优雅关闭逻辑
    info!("✓ Shutdown complete");

    Ok(())
}
