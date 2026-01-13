use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// 服务器配置
    pub server: ServerConfig,
    /// 认证配置
    pub auth: AuthConfig,
    /// 同步配置
    pub sync: SyncConfig,
    /// 冲突解决配置
    pub conflict: ConflictConfig,
    /// 性能配置
    pub performance: PerformanceConfig,
    /// 日志配置
    pub logging: LoggingConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// gRPC 服务器地址
    #[serde(default = "default_server_address")]
    pub address: String,

    /// HTTP 健康检查地址
    #[serde(default = "default_health_check_address")]
    pub health_check_address: String,

    /// 连接超时（秒）
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout: u64,

    /// 请求超时（秒）
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,

    /// 启用 TLS
    #[serde(default = "default_tls_enabled")]
    pub tls_enabled: bool,

    /// TLS 证书路径（可选）
    pub tls_cert_path: Option<String>,
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Token 存储路径
    #[serde(default = "default_token_dir")]
    pub token_dir: PathBuf,

    /// Token 加密密钥（可选，未设置则不加密）
    pub encryption_key: Option<String>,

    /// 自动刷新 Token
    #[serde(default = "default_auto_refresh")]
    pub auto_refresh: bool,

    /// Token 过期前多久刷新（秒）
    #[serde(default = "default_refresh_before")]
    pub refresh_before: u64,
}

/// 同步配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Claude 配置目录
    #[serde(default = "default_claude_dir")]
    pub claude_dir: PathBuf,

    /// 同步间隔（秒，0 表示实时同步）
    #[serde(default = "default_sync_interval")]
    pub sync_interval: u64,

    /// 批处理窗口（秒）
    #[serde(default = "default_batch_window")]
    pub batch_window: u64,

    /// 排除目录
    #[serde(default = "default_exclude_dirs")]
    pub exclude_dirs: Vec<String>,

    /// 排除文件模式
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,

    /// 包含的文件类型
    #[serde(default = "default_include_types")]
    pub include_types: Vec<String>,

    /// 同步规则（本地配置，优先级低于服务器规则）
    #[serde(default)]
    pub rules: Vec<SyncRule>,
}

/// 冲突解决配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictConfig {
    /// 默认解决策略
    #[serde(default = "default_conflict_strategy")]
    pub default_strategy: String,

    /// 自动合并文本文件
    #[serde(default = "default_auto_merge_text")]
    pub auto_merge_text: bool,

    /// 自动合并 JSON/YAML
    #[serde(default = "default_auto_merge_structured")]
    pub auto_merge_structured: bool,

    /// 冲突文件存储目录
    #[serde(default = "default_conflict_dir")]
    pub conflict_dir: PathBuf,

    /// 保留冲突副本
    #[serde(default = "default_keep_conflict_copy")]
    pub keep_conflict_copy: bool,
}

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 防抖延迟（毫秒）
    #[serde(default = "default_debounce_delay")]
    pub debounce_delay: u64,

    /// 大文件阈值（字节，默认 10MB）
    #[serde(default = "default_large_file_threshold")]
    pub large_file_threshold: u64,

    /// 最大并发上传数
    #[serde(default = "default_max_concurrent_uploads")]
    pub max_concurrent_uploads: usize,

    /// 最大并发下载数
    #[serde(default = "default_max_concurrent_downloads")]
    pub max_concurrent_downloads: usize,

    /// 上传重试次数
    #[serde(default = "default_upload_retries")]
    pub upload_retries: usize,

    /// 下载重试次数
    #[serde(default = "default_download_retries")]
    pub download_retries: usize,

    /// 重试延迟（秒）
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    #[serde(default = "default_log_level")]
    pub level: String,

    /// 日志文件路径（可选）
    pub log_file: Option<PathBuf>,

    /// 日志格式（json, text）
    #[serde(default = "default_log_format")]
    pub format: String,
}

/// 同步规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRule {
    /// 规则名称
    pub name: String,

    /// 规则类型（include/exclude）
    pub rule_type: String,

    /// 文件模式（Glob 或正则表达式）
    pub pattern: String,

    /// 文件类型（可选）
    pub file_type: Option<String>,

    /// 优先级（数字越大优先级越高）
    #[serde(default)]
    pub priority: i32,

    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

// ===== 默认值函数 =====

fn default_server_address() -> String {
    "http://localhost:50051".to_string()
}

fn default_health_check_address() -> String {
    "http://localhost:3000".to_string()
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_request_timeout() -> u64 {
    300
}

fn default_tls_enabled() -> bool {
    false
}

fn default_token_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法找到用户主目录")
        .join(".claude-sync")
        .join("tokens")
}

fn default_auto_refresh() -> bool {
    true
}

fn default_refresh_before() -> u64 {
    300 // 5 分钟
}

fn default_claude_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法找到用户主目录")
        .join(".claude")
}

fn default_sync_interval() -> u64 {
    0 // 0 表示实时同步
}

fn default_batch_window() -> u64 {
    2 // 2 秒
}

fn default_exclude_dirs() -> Vec<String> {
    vec![
        "cache".to_string(),
        "downloads".to_string(),
        "image-cache".to_string(),
        "file-history".to_string(),
        "shell-snapshots".to_string(),
        "statsig".to_string(),
    ]
}

fn default_exclude_patterns() -> Vec<String> {
    vec![]
}

fn default_include_types() -> Vec<String> {
    vec![
        "text".to_string(),
        "json".to_string(),
        "yaml".to_string(),
        "toml".to_string(),
        "md".to_string(),
    ]
}

fn default_conflict_strategy() -> String {
    "manual".to_string() // manual, keep_local, keep_remote, keep_newer
}

fn default_auto_merge_text() -> bool {
    true
}

fn default_auto_merge_structured() -> bool {
    true
}

fn default_conflict_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法找到用户主目录")
        .join(".claude-sync")
        .join("conflicts")
}

fn default_keep_conflict_copy() -> bool {
    true
}

fn default_debounce_delay() -> u64 {
    500 // 500 毫秒
}

fn default_large_file_threshold() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_max_concurrent_uploads() -> usize {
    5
}

fn default_max_concurrent_downloads() -> usize {
    10
}

fn default_upload_retries() -> usize {
    3
}

fn default_download_retries() -> usize {
    3
}

fn default_retry_delay() -> u64 {
    5
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "text".to_string()
}

fn default_enabled() -> bool {
    true
}

impl ClientConfig {
    /// 加载配置文件
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            info!("配置文件不存在，将创建默认配置: {:?}", config_path);
            let default_config = Self::default();
            default_config.save(&config_path)?;
            return Ok(default_config);
        }

        info!("加载配置文件: {:?}", config_path);
        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("无法读取配置文件: {:?}", config_path))?;

        let config: ClientConfig = toml::from_str(&content)
            .with_context(|| format!("无法解析配置文件: {:?}", config_path))?;

        debug!("配置加载成功: {:#?}", config);

        Ok(config)
    }

    /// 保存配置文件
    pub fn save(&self, path: &Path) -> Result<()> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建配置目录: {:?}", parent))?;
        }

        let content = toml::to_string_pretty(self)
            .context("无法序列化配置")?;

        std::fs::write(path, content)
            .with_context(|| format!("无法写入配置文件: {:?}", path))?;

        info!("配置已保存: {:?}", path);

        Ok(())
    }

    /// 获取配置文件路径
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::home_dir()
            .expect("无法找到用户主目录")
            .join(".claude-sync");

        Ok(config_dir.join("config.toml"))
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        // 验证服务器地址
        if self.server.address.is_empty() {
            anyhow::bail!("服务器地址不能为空");
        }

        // 验证 Claude 目录
        if !self.sync.claude_dir.exists() {
            anyhow::bail!(
                "Claude 配置目录不存在: {:?}",
                self.sync.claude_dir
            );
        }

        // 验证冲突解决策略
        match self.conflict.default_strategy.as_str() {
            "manual" | "keep_local" | "keep_remote" | "keep_newer" => {}
            _ => {
                anyhow::bail!(
                    "无效的冲突解决策略: {}",
                    self.conflict.default_strategy
                );
            }
        }

        // 验证日志级别
        match self.logging.level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => {
                anyhow::bail!("无效的日志级别: {}", self.logging.level);
            }
        }

        debug!("配置验证通过");

        Ok(())
    }

    /// 初始化配置（创建必要的目录）
    pub fn initialize(&self) -> Result<()> {
        // 创建 token 目录
        if !self.auth.token_dir.exists() {
            std::fs::create_dir_all(&self.auth.token_dir)
                .with_context(|| format!("无法创建 token 目录: {:?}", self.auth.token_dir))?;
            info!("创建 token 目录: {:?}", self.auth.token_dir);
        }

        // 创建冲突目录
        if !self.conflict.conflict_dir.exists() {
            std::fs::create_dir_all(&self.conflict.conflict_dir)
                .with_context(|| format!("无法创建冲突目录: {:?}", self.conflict.conflict_dir))?;
            info!("创建冲突目录: {:?}", self.conflict.conflict_dir);
        }

        info!("配置初始化完成");

        Ok(())
    }

    /// 获取排除目录的完整路径
    pub fn get_exclude_paths(&self) -> Vec<PathBuf> {
        self.sync
            .exclude_dirs
            .iter()
            .map(|dir| self.sync.claude_dir.join(dir))
            .collect()
    }

    /// 检查路径是否应该被排除
    pub fn should_exclude(&self, path: &Path) -> bool {
        // 检查排除目录
        for exclude_dir in &self.sync.exclude_dirs {
            if path.starts_with(self.sync.claude_dir.join(exclude_dir)) {
                debug!("路径在排除目录中: {:?}", path);
                return true;
            }
        }

        // 检查排除模式
        for pattern in &self.sync.exclude_patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches_path(path) {
                    debug!("路径匹配排除模式: {:?} (pattern: {})", path, pattern);
                    return true;
                }
            }
        }

        false
    }

    /// 应用同步规则
    pub fn apply_rules(&self, path: &Path, file_type: &str) -> bool {
        let mut should_sync = true;
        let mut highest_priority = i32::MIN;

        for rule in &self.sync.rules {
            if !rule.enabled {
                continue;
            }

            // 检查文件类型
            if let Some(ref rule_file_type) = rule.file_type {
                if rule_file_type != file_type {
                    continue;
                }
            }

            // 检查模式匹配
            if let Ok(glob_pattern) = glob::Pattern::new(&rule.pattern) {
                if glob_pattern.matches_path(path) {
                    // 优先级更高的规则会覆盖之前的规则
                    if rule.priority > highest_priority {
                        should_sync = rule.rule_type == "include";
                        highest_priority = rule.priority;
                    }
                }
            }
        }

        should_sync
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                address: default_server_address(),
                health_check_address: default_health_check_address(),
                connection_timeout: default_connection_timeout(),
                request_timeout: default_request_timeout(),
                tls_enabled: default_tls_enabled(),
                tls_cert_path: None,
            },
            auth: AuthConfig {
                token_dir: default_token_dir(),
                encryption_key: None,
                auto_refresh: default_auto_refresh(),
                refresh_before: default_refresh_before(),
            },
            sync: SyncConfig {
                claude_dir: default_claude_dir(),
                sync_interval: default_sync_interval(),
                batch_window: default_batch_window(),
                exclude_dirs: default_exclude_dirs(),
                exclude_patterns: default_exclude_patterns(),
                include_types: default_include_types(),
                rules: vec![],
            },
            conflict: ConflictConfig {
                default_strategy: default_conflict_strategy(),
                auto_merge_text: default_auto_merge_text(),
                auto_merge_structured: default_auto_merge_structured(),
                conflict_dir: default_conflict_dir(),
                keep_conflict_copy: default_keep_conflict_copy(),
            },
            performance: PerformanceConfig {
                debounce_delay: default_debounce_delay(),
                large_file_threshold: default_large_file_threshold(),
                max_concurrent_uploads: default_max_concurrent_uploads(),
                max_concurrent_downloads: default_max_concurrent_downloads(),
                upload_retries: default_upload_retries(),
                download_retries: default_download_retries(),
                retry_delay: default_retry_delay(),
            },
            logging: LoggingConfig {
                level: default_log_level(),
                log_file: None,
                format: default_log_format(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert_eq!(config.server.address, "http://localhost:50051");
        assert_eq!(config.performance.debounce_delay, 500);
    }

    #[test]
    fn test_should_exclude() {
        let config = ClientConfig::default();
        let claude_dir = config.sync.claude_dir.clone();

        // 测试排除目录
        let cache_path = claude_dir.join("cache").join("test.txt");
        assert!(config.should_exclude(&cache_path));

        // 测试非排除目录
        let agents_path = claude_dir.join("agents").join("test.md");
        assert!(!config.should_exclude(&agents_path));
    }

    #[test]
    fn test_apply_rules() {
        let mut config = ClientConfig::default();

        // 添加包含规则
        config.sync.rules.push(SyncRule {
            name: "include-md".to_string(),
            rule_type: "include".to_string(),
            pattern: "*.md".to_string(),
            file_type: Some("text".to_string()),
            priority: 0,
            enabled: true,
        });

        // 添加排除规则
        config.sync.rules.push(SyncRule {
            name: "exclude-temp".to_string(),
            rule_type: "exclude".to_string(),
            pattern: "*-temp.md".to_string(),
            file_type: Some("text".to_string()),
            priority: 10,
            enabled: true,
        });

        let test_path = PathBuf::from("test-temp.md");

        // 排除规则优先级更高
        assert!(!config.apply_rules(&test_path, "text"));

        let normal_path = PathBuf::from("test.md");
        assert!(config.apply_rules(&normal_path, "text"));
    }
}
