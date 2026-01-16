use crate::cache::Cache;
use crate::db::DbPool;
use crate::proto::sync::claude_sync::{
    notification_service_server::NotificationService, ChangeNotification, HeartbeatRequest,
    HeartbeatResponse, SubscribeChangesRequest,
};
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

/// NotificationService gRPC 实现
pub struct NotificationGrpcService {
    pool: DbPool,
    cache: Cache,
}

impl NotificationGrpcService {
    /// 创建新的服务实例
    pub fn new(pool: DbPool, cache: Cache) -> Self {
        Self { pool, cache }
    }
}

#[tonic::async_trait]
impl NotificationService for NotificationGrpcService {
    type SubscribeChangesStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<ChangeNotification, Status>> + Send>>;

    async fn subscribe_changes(
        &self,
        _request: Request<SubscribeChangesRequest>,
    ) -> Result<Response<Self::SubscribeChangesStream>, Status> {
        // TODO: 实现变更订阅逻辑
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx
            .send(Ok(ChangeNotification {
                device_id: String::new(),
                changes: vec![],
                timestamp: 0,
            }))
            .await;
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    type HeartbeatStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<HeartbeatResponse, Status>> + Send>>;

    async fn heartbeat(
        &self,
        _request: Request<tonic::Streaming<HeartbeatRequest>>,
    ) -> Result<Response<Self::HeartbeatStream>, Status> {
        // TODO: 实现心跳保活逻辑
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx
            .send(Ok(HeartbeatResponse {
                timestamp: 0,
                pending_changes: vec![],
            }))
            .await;
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_subscribe_changes() {
        // 测试变更订阅
    }
}
