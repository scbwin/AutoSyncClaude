use crate::cache::Cache;
use crate::db::DbPool;

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

/*
#[tonic::async_trait]
impl NotificationService for NotificationGrpcService {
    /// 订阅文件变更通知
    async fn subscribe_changes(
        &self,
        request: Request<SubscribeChangesRequest>,
    ) -> Result<Response<Stream::<ChangeNotification>>, Status> {
        let req = request.into_inner();
        let user_id = extract_user_id_from_request(&request)?;
        let device_id = extract_device_id_from_request(&request)?;

        info!(
            "Device {} subscribing to changes for user {}",
            device_id,
            user_id
        );

        // 创建消息通道
        let (tx, rx) = mpsc::channel(100);

        // 启动变更推送任务
        let cache = self.cache.clone();
        let user_id_clone = user_id.clone();
        let device_id_clone = device_id.clone();

        tokio::spawn(async move {
            // 设置设备在线
            if let Err(e) = cache.device_online(&device_id_clone, &user_id_clone).await {
                error!("Failed to set device online: {}", e);
            }

            // 持续监听变更
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                // 检查连接是否还活着
                if tx.is_closed() {
                    info!("Device {} disconnected", device_id_clone);
                    // 设置设备离线
                    let _ = cache.device_offline(&device_id_clone, &user_id_clone).await;
                    break;
                }

                // 获取待处理的变更
                match cache
                    .get_file_changes(&user_id_clone, 10)
                    .await
                {
                    Ok(changes) => {
                        if !changes.is_empty() {
                            for change in changes {
                                // 应用过滤模式
                                if !matches_patterns(&change.file_path, &req.file_patterns) {
                                    continue;
                                }

                                let notification = ChangeNotification {
                                    device_id: change.device_id.to_string(),
                                    changes: vec![FileInfo {
                                        file_path: change.file_path.clone(),
                                        file_hash: String::new(),
                                        file_size: 0,
                                        modified_at: change.timestamp,
                                        version: 0,
                                        device_id: change.device_id.to_string(),
                                        is_deleted: false,
                                        file_type: "text".to_string(),
                                    }],
                                    timestamp: change.timestamp,
                                };

                                if tx.send(Ok(notification)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get file changes: {}", e);
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    /// 心跳保活
    async fn heartbeat(
        &self,
        request: Request<Stream::<HeartbeatRequest>>,
    ) -> Result<Response<Stream::<HeartbeatResponse>>, Status> {
        let user_id = extract_user_id_from_request(&request)?;
        let device_id = extract_device_id_from_request(&request)?;
        let mut stream = request.into_inner();

        info!("Starting heartbeat for device {}", device_id);

        let (tx, rx) = mpsc::channel(100);

        let cache = self.cache.clone();

        tokio::spawn(async move {
            // 设置设备在线
            let _ = cache.device_online(&device_id, &user_id).await;

            let mut last_activity = std::time::Instant::now();

            loop {
                match stream.message().await {
                    Ok(Some(heartbeat)) => {
                        last_activity = std::time::Instant::now();
                        debug!("Received heartbeat from device {}", device_id);

                        // 检查是否有待处理的变更
                        match cache
                            .get_file_changes(&user_id, 10)
                            .await
                        {
                            Ok(changes) => {
                                if !changes.is_empty() {
                                    let file_infos: Vec<FileInfo> = changes
                                        .iter()
                                        .map(|c| FileInfo {
                                            file_path: c.file_path.clone(),
                                            file_hash: String::new(),
                                            file_size: 0,
                                            modified_at: c.timestamp,
                                            version: 0,
                                            device_id: c.device_id.to_string(),
                                            is_deleted: false,
                                            file_type: "text".to_string(),
                                        })
                                        .collect();

                                    let response = HeartbeatResponse {
                                        timestamp: heartbeat.timestamp,
                                        pending_changes: file_infos,
                                    };

                                    if tx.send(Ok(response)).await.is_err() {
                                        break;
                                    }
                                } else {
                                    // 没有待处理的变更，发送简单确认
                                    let response = HeartbeatResponse {
                                        timestamp: chrono::Utc::now().timestamp(),
                                        pending_changes: vec![],
                                    };

                                    if tx.send(Ok(response)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to get file changes: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        info!("Device {} closed heartbeat stream", device_id);
                        break;
                    }
                    Err(e) => {
                        error!("Heartbeat stream error: {}", e);
                        break;
                    }
                }

                // 检查超时（30 秒无心跳则断开）
                if last_activity.elapsed() > Duration::from_secs(30) {
                    warn!(
                        "Device {} heartbeat timeout, disconnecting",
                        device_id
                    );
                    break;
                }
            }

            // 设置设备离线
            let _ = cache.device_offline(&device_id, &user_id).await;
            info!("Heartbeat ended for device {}", device_id);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
*/

// ===== 辅助函数 =====

/// 检查文件路径是否匹配过滤模式
fn matches_patterns(file_path: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return true; // 没有过滤模式，接受所有
    }

    for pattern in patterns {
        if glob_match(pattern, file_path) {
            return true;
        }
    }

    false
}

/// 简单的 glob 模式匹配
fn glob_match(pattern: &str, text: &str) -> bool {
    // 简化实现，实际应该使用 glob crate
    if pattern == "**" || pattern == "*" {
        return true;
    }

    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return text.starts_with(parts[0]) && text.ends_with(parts[1]);
        }
    }

    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_patterns() {
        assert!(matches_patterns("test.txt", &[]));
        assert!(matches_patterns("test.txt", &["*.txt".to_string()]));
        assert!(!matches_patterns("test.md", &["*.txt".to_string()]));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("**", "anything"));
        assert!(glob_match("*", "test.txt"));
        assert!(glob_match("*.txt", "test.txt"));
        assert!(!glob_match("*.txt", "test.md"));
    }
}
