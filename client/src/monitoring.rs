use crate::error::ClientError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, span, warn, Level};

/// 性能指标
#[derive(Debug, Clone, Serialize)]
pub struct Metric {
    /// 指标名称
    pub name: String,

    /// 指标值
    pub value: f64,

    /// 指标类型
    pub metric_type: MetricType,

    /// 时间戳
    pub timestamp: DateTime<Utc>,

    /// 标签
    pub tags: Vec<(String, String)>,
}

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MetricType {
    /// 计数器
    Counter,

    /// 测量值
    Gauge,

    /// 直方图
    Histogram,

    /// 摘要
    Summary,
}

/// 性能统计
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceStats {
    /// 同步总次数
    pub sync_total_count: u64,

    /// 同步成功次数
    pub sync_success_count: u64,

    /// 同步失败次数
    pub sync_failure_count: u64,

    /// 文件上传总数
    pub upload_total_count: u64,

    /// 文件下载总数
    pub download_total_count: u64,

    /// 上传字节总数
    pub upload_total_bytes: u64,

    /// 下载字节总数
    pub download_total_bytes: u64,

    /// 平均同步持续时间（毫秒）
    pub avg_sync_duration_ms: f64,

    /// 平均上传速度（字节/秒）
    pub avg_upload_speed: f64,

    /// 平均下载速度（字节/秒）
    pub avg_download_speed: f64,

    /// 当前网络状态
    pub network_status: String,

    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 监控管理器
pub struct MonitoringManager {
    /// 性能指标
    metrics: Arc<RwLock<Vec<Metric>>>,

    /// 最大指标数量
    max_metrics: usize,

    /// 性能统计
    stats: Arc<RwLock<PerformanceStats>>,

    /// 是否启用监控
    enabled: Arc<RwLock<bool>>,

    /// 慢操作阈值（毫秒）
    slow_operation_threshold_ms: u64,
}

impl MonitoringManager {
    /// 创建新的监控管理器
    pub fn new(max_metrics: usize, slow_operation_threshold_ms: u64) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::with_capacity(max_metrics))),
            max_metrics,
            stats: Arc::new(RwLock::new(PerformanceStats {
                sync_total_count: 0,
                sync_success_count: 0,
                sync_failure_count: 0,
                upload_total_count: 0,
                download_total_count: 0,
                upload_total_bytes: 0,
                download_total_bytes: 0,
                avg_sync_duration_ms: 0.0,
                avg_upload_speed: 0.0,
                avg_download_speed: 0.0,
                network_status: "Unknown".to_string(),
                last_updated: Utc::now(),
            })),
            enabled: Arc::new(RwLock::new(true)),
            slow_operation_threshold_ms,
        }
    }

    /// 记录指标
    pub async fn record_metric(&self, metric: Metric) {
        if !*self.enabled.read().await {
            return;
        }

        let mut metrics = self.metrics.write().await;

        // 如果达到最大容量，移除最旧的指标
        if metrics.len() >= self.max_metrics {
            metrics.remove(0);
        }

        metrics.push(metric);
    }

    /// 记录计数器
    pub async fn record_counter(
        &self,
        name: impl Into<String>,
        value: f64,
        tags: Vec<(String, String)>,
    ) {
        let metric = Metric {
            name: name.into(),
            value,
            metric_type: MetricType::Counter,
            timestamp: Utc::now(),
            tags,
        };

        self.record_metric(metric).await;
    }

    /// 记录测量值
    pub async fn record_gauge(
        &self,
        name: impl Into<String>,
        value: f64,
        tags: Vec<(String, String)>,
    ) {
        let metric = Metric {
            name: name.into(),
            value,
            metric_type: MetricType::Gauge,
            timestamp: Utc::now(),
            tags,
        };

        self.record_metric(metric).await;
    }

    /// 记录直方图
    pub async fn record_histogram(
        &self,
        name: impl Into<String>,
        value: f64,
        tags: Vec<(String, String)>,
    ) {
        let metric = Metric {
            name: name.into(),
            value,
            metric_type: MetricType::Histogram,
            timestamp: Utc::now(),
            tags,
        };

        self.record_metric(metric).await;
    }

    /// 获取所有指标
    pub async fn get_metrics(&self) -> Vec<Metric> {
        self.metrics.read().await.clone()
    }

    /// 获取指定名称的指标
    pub async fn get_metrics_by_name(&self, name: &str) -> Vec<Metric> {
        let metrics = self.metrics.read().await;
        metrics
            .iter()
            .filter(|m| m.name == name)
            .cloned()
            .collect()
    }

    /// 清空指标
    pub async fn clear_metrics(&self) {
        self.metrics.write().await.clear();
        info!("性能指标已清空");
    }

    /// 更新网络状态
    pub async fn update_network_status(&self, status: impl Into<String>) {
        let mut stats = self.stats.write().await;
        stats.network_status = status.into();
        stats.last_updated = Utc::now();
        debug!("网络状态已更新: {}", stats.network_status);
    }

    /// 记录同步开始
    pub async fn record_sync_start(&self) -> SyncTimer {
        SyncTimer {
            manager: self.clone(),
            start: Instant::now(),
            file_count: 0,
            upload_bytes: 0,
            download_bytes: 0,
        }
    }

    /// 更新同步统计
    pub async fn update_sync_stats(
        &self,
        success: bool,
        duration_ms: u64,
        upload_bytes: u64,
        download_bytes: u64,
    ) {
        let mut stats = self.stats.write().await;

        stats.sync_total_count += 1;
        if success {
            stats.sync_success_count += 1;
        } else {
            stats.sync_failure_count += 1;
        }

        // 更新平均同步持续时间
        let total_duration = stats.avg_sync_duration_ms * (stats.sync_total_count - 1) as f64;
        stats.avg_sync_duration_ms =
            (total_duration + duration_ms as f64) / stats.sync_total_count as f64;

        // 更新上传/下载统计
        if upload_bytes > 0 {
            stats.upload_total_count += 1;
            stats.upload_total_bytes += upload_bytes;
        }

        if download_bytes > 0 {
            stats.download_total_count += 1;
            stats.download_total_bytes += download_bytes;
        }

        stats.last_updated = Utc::now();

        // 记录指标
        self.record_histogram(
            "sync_duration_ms",
            duration_ms as f64,
            vec![("success".to_string(), success.to_string())],
        )
        .await;

        if upload_bytes > 0 {
            self.record_counter("upload_bytes", upload_bytes as f64, vec![]).await;
        }

        if download_bytes > 0 {
            self
                .record_counter("download_bytes", download_bytes as f64, vec![])
                .await;
        }
    }

    /// 获取性能统计
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        self.stats.read().await.clone()
    }

    /// 导出指标为 JSON
    pub async fn export_metrics_json(&self) -> Result<String, ClientError> {
        let metrics = self.get_metrics().await;

        serde_json::to_string_pretty(&metrics)
            .map_err(|e| ClientError::internal("无法序列化指标", Some(Box::new(e))))
    }

    /// 导出指标为 Prometheus 格式
    pub async fn export_metrics_prometheus(&self) -> String {
        let metrics = self.get_metrics().await;
        let mut output = String::new();

        // 按指标名称分组
        let mut grouped_metrics: std::collections::HashMap<String, Vec<&Metric>> =
            std::collections::HashMap::new();

        for metric in &metrics {
            grouped_metrics
                .entry(metric.name.clone())
                .or_insert_with(Vec::new)
                .push(metric);
        }

        // 生成 Prometheus 格式
        for (name, group) in grouped_metrics {
            // 输出 HELP 和 TYPE
            output.push_str(&format!("# HELP {} {}\n", name, name));
            output.push_str(&format!(
                "# TYPE {} {}\n",
                name,
                match group.first().unwrap().metric_type {
                    MetricType::Counter => "counter",
                    MetricType::Gauge => "gauge",
                    MetricType::Histogram => "histogram",
                    MetricType::Summary => "summary",
                }
            ));

            // 输出指标值（使用最新值）
            if let Some(latest) = group.last() {
                let tags_str = if latest.tags.is_empty() {
                    String::new()
                } else {
                    let tags: Vec<String> = latest
                        .tags
                        .iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect();
                    format!("{{{}}}", tags.join(","))
                };
                output.push_str(&format!("{}{} {}\n", name, tags_str, latest.value));
            }

            output.push('\n');
        }

        output
    }

    /// 启用监控
    pub async fn enable(&self) {
        *self.enabled.write().await = true;
        info!("监控已启用");
    }

    /// 禁用监控
    pub async fn disable(&self) {
        *self.enabled.write().await = false;
        info!("监控已禁用");
    }

    /// 检查是否启用
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// 记录慢操作
    pub async fn record_slow_operation(
        &self,
        operation: impl Into<String>,
        duration: Duration,
    ) {
        let duration_ms = duration.as_millis() as u64;

        if duration_ms >= self.slow_operation_threshold_ms {
            let operation_name = operation.into();
            warn!(
                "检测到慢操作: {} 耗时 {} ms",
                operation_name, duration_ms
            );

            self.record_histogram(
                format!("slow_operation_{}", operation_name),
                duration_ms as f64,
                vec![("threshold_ms".to_string(), self.slow_operation_threshold_ms.to_string())],
            )
            .await;
        }
    }

    /// 打印性能摘要
    pub async fn print_performance_summary(&self) {
        let stats = self.get_performance_stats().await;

        info!("========== 性能统计摘要 ==========");
        info!("同步总次数: {}", stats.sync_total_count);
        info!("同步成功次数: {}", stats.sync_success_count);
        info!("同步失败次数: {}", stats.sync_failure_count);

        if stats.sync_total_count > 0 {
            let success_rate =
                (stats.sync_success_count as f64 / stats.sync_total_count as f64) * 100.0;
            info!("同步成功率: {:.1}%", success_rate);
        }

        info!("文件上传总数: {}", stats.upload_total_count);
        info!("文件下载总数: {}", stats.download_total_count);
        info!("上传字节总数: {} bytes", stats.upload_total_bytes);
        info!("下载字节总数: {} bytes", stats.download_total_bytes);
        info!(
            "平均同步持续时间: {:.2} ms",
            stats.avg_sync_duration_ms
        );
        info!("平均上传速度: {:.2} bytes/s", stats.avg_upload_speed);
        info!("平均下载速度: {:.2} bytes/s", stats.avg_download_speed);
        info!("网络状态: {}", stats.network_status);
        info!("最后更新: {}", stats.last_updated);
        info!("==================================");
    }
}

impl Clone for MonitoringManager {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            max_metrics: self.max_metrics,
            stats: Arc::clone(&self.stats),
            enabled: Arc::clone(&self.enabled),
            slow_operation_threshold_ms: self.slow_operation_threshold_ms,
        }
    }
}

/// 同步计时器
pub struct SyncTimer {
    manager: MonitoringManager,
    start: Instant,
    file_count: usize,
    upload_bytes: u64,
    download_bytes: u64,
}

impl SyncTimer {
    /// 添加上传字节数
    pub fn add_upload_bytes(&mut self, bytes: u64) {
        self.upload_bytes += bytes;
    }

    /// 添加下载字节数
    pub fn add_download_bytes(&mut self, bytes: u64) {
        self.download_bytes += bytes;
    }

    /// 增加文件计数
    pub fn increment_file_count(&mut self) {
        self.file_count += 1;
    }

    /// 完成同步
    pub async fn complete(self, success: bool) {
        let duration = self.start.elapsed();
        let duration_ms = duration.as_millis() as u64;

        self.manager
            .update_sync_stats(success, duration_ms, self.upload_bytes, self.download_bytes)
            .await;

        // 记录到 tracing
        let span = span!(Level::INFO, "sync_complete", success = success, duration_ms = duration_ms);
        let _enter = span.enter();

        info!(
            "同步完成: {} 个文件, {} ms 上传, {} ms 下载, 耗时 {} ms",
            self.file_count,
            self.upload_bytes,
            self.download_bytes,
            duration_ms
        );

        // 检查是否为慢操作
        self.manager
            .record_slow_operation("sync", duration)
            .await;
    }
}

/// 操作计时器
pub struct OperationTimer {
    manager: MonitoringManager,
    operation_name: String,
    start: Instant,
}

impl OperationTimer {
    /// 创建新的操作计时器
    pub fn new(manager: MonitoringManager, operation_name: impl Into<String>) -> Self {
        Self {
            manager,
            operation_name: operation_name.into(),
            start: Instant::now(),
        }
    }

    /// 完成操作
    pub async fn complete(self) {
        let duration = self.start.elapsed();

        self.manager
            .record_histogram(
                format!("operation_{}", self.operation_name),
                duration.as_millis() as f64,
                vec![],
            )
            .await;

        // 检查是否为慢操作
        self.manager
            .record_slow_operation(&self.operation_name, duration)
            .await;

        debug!("操作 '{}' 耗时 {} ms", self.operation_name, duration.as_millis());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_manager() {
        let manager = MonitoringManager::new(100, 1000);

        // 记录指标
        manager
            .record_counter("test_counter", 1.0, vec![("label".to_string(), "value".to_string())])
            .await;

        manager
            .record_gauge("test_gauge", 42.0, vec![])
            .await;

        // 获取指标
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.len(), 2);

        let counter_metrics = manager.get_metrics_by_name("test_counter").await;
        assert_eq!(counter_metrics.len(), 1);
    }

    #[tokio::test]
    async fn test_sync_stats() {
        let manager = MonitoringManager::new(100, 1000);

        // 记录同步
        let mut timer = manager.record_sync_start().await;
        timer.add_upload_bytes(1024);
        timer.add_download_bytes(2048);
        timer.complete(true).await;

        // 检查统计
        let stats = manager.get_performance_stats().await;
        assert_eq!(stats.sync_total_count, 1);
        assert_eq!(stats.sync_success_count, 1);
        assert_eq!(stats.upload_total_bytes, 1024);
        assert_eq!(stats.download_total_bytes, 2048);
    }

    #[tokio::test]
    async fn test_metrics_export() {
        let manager = MonitoringManager::new(100, 1000);

        manager
            .record_counter("export_test", 42.0, vec![])
            .await;

        // JSON 导出
        let json = manager.export_metrics_json().await.unwrap();
        assert!(json.contains("export_test"));

        // Prometheus 导出
        let prometheus = manager.export_metrics_prometheus().await;
        assert!(prometheus.contains("export_test"));
    }

    #[tokio::test]
    async fn test_slow_operation() {
        let manager = MonitoringManager::new(100, 10); // 10ms 阈值

        let timer = OperationTimer::new(manager.clone(), "test_operation");
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        timer.complete().await;

        let metrics = manager.get_metrics_by_name("slow_operation_test_operation").await;
        assert!(!metrics.is_empty());
    }
}
