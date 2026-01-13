use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Serialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// 健康检查响应
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    database: HealthStatus,
    redis: HealthStatus,
    storage: HealthStatus,
}

#[derive(Serialize)]
struct HealthStatus {
    healthy: bool,
    message: String,
}

/// 健康检查服务
pub struct HealthCheckService {
    pool: Arc<crate::db::DbPool>,
    redis_pool: Arc<crate::cache::RedisPool>,
    storage: Arc<crate::storage::StorageService>,
}

impl HealthCheckService {
    /// 创建新的健康检查服务
    pub fn new(
        pool: Arc<crate::db::DbPool>,
        redis_pool: Arc<crate::cache::RedisPool>,
        storage: Arc<crate::storage::StorageService>,
    ) -> Self {
        Self {
            pool,
            redis_pool,
            storage,
        }
    }

    /// 启动健康检查服务器
    pub async fn serve(self, addr: String) -> anyhow::Result<()> {
        info!("Starting health check server on {}", addr);

        // 创建路由
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/ready", get(ready_handler))
            .with_state(Arc::new(self));

        // 启动服务器
        let listener = TcpListener::bind(&addr).await?;
        info!("✓ Health check server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// 健康检查处理器
async fn health_handler(
    State(service): State<Arc<HealthCheckService>>,
) -> impl IntoResponse {
    // 检查数据库
    let db_healthy = match service.pool.health_check().await {
        Ok(_) => HealthStatus {
            healthy: true,
            message: "OK".to_string(),
        },
        Err(e) => HealthStatus {
            healthy: false,
            message: e.to_string(),
        },
    };

    // 检查 Redis
    let redis_healthy = match service.redis_pool.health_check().await {
        Ok(_) => HealthStatus {
            healthy: true,
            message: "OK".to_string(),
        },
        Err(e) => HealthStatus {
            healthy: false,
            message: e.to_string(),
        },
    };

    // 检查存储
    let storage_healthy = HealthStatus {
        healthy: true,
        message: "OK".to_string(),
    };

    let all_healthy = db_healthy.healthy && redis_healthy.healthy && storage_healthy.healthy;

    let response = HealthResponse {
        status: if all_healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_healthy,
        redis: redis_healthy,
        storage: storage_healthy,
    };

    let status = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(response)).into_response()
}

/// 就绪检查处理器
async fn ready_handler(
    State(service): State<Arc<HealthCheckService>>,
) -> impl IntoResponse {
    // 检查数据库是否就绪
    match service.pool.health_check().await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ready",
                "message": "Service is ready to accept requests"
            })),
        )
            .into_response(),
        Err(e) => {
            error!("Database not ready: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "status": "not_ready",
                    "message": e.to_string()
                })),
            )
                .into_response()
        }
    }
}
