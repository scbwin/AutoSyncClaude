use thiserror::Error;

/// 客户端统一错误类型
#[derive(Error, Debug)]
pub enum ClientError {
    /// 配置错误
    #[error("配置错误: {message}")]
    Config { message: String },

    /// 认证错误
    #[error("认证错误: {message}")]
    Auth { message: String },

    /// Token 错误
    #[error("Token 错误: {message}")]
    Token { message: String },

    /// 网络错误
    #[error("网络错误: {message}")]
    Network {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// gRPC 错误
    #[error("gRPC 错误: {code} - {message}")]
    Grpc { code: tonic::Code, message: String },

    /// 文件 I/O 错误
    #[error("文件错误: {path} - {message}")]
    File {
        path: String,
        message: String,
        #[source]
        source: Option<std::io::Error>,
    },

    /// 同步错误
    #[error("同步错误: {path} - {message}")]
    Sync { path: String, message: String },

    /// 冲突错误
    #[error("冲突错误: {path} - {message}")]
    Conflict { path: String, message: String },

    /// 解析错误
    #[error("解析错误: {message}")]
    Parse {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// 验证错误
    #[error("验证错误: {message}")]
    Validation { message: String },

    /// 超时错误
    #[error("超时错误: {operation} 在 {timeout_secs} 秒后超时")]
    Timeout {
        operation: String,
        timeout_secs: u64,
    },

    /// 重试失败
    #[error("重试失败: {operation} 在 {max_retries} 次重试后仍失败")]
    RetryExhausted {
        operation: String,
        max_retries: usize,
        last_error: String,
    },

    /// 内部错误
    #[error("内部错误: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

// 手动实现 Clone,因为包含 Box<dyn Error>
impl Clone for ClientError {
    fn clone(&self) -> Self {
        match self {
            Self::Config { message } => Self::Config {
                message: message.clone(),
            },
            Self::Auth { message } => Self::Auth {
                message: message.clone(),
            },
            Self::Token { message } => Self::Token {
                message: message.clone(),
            },
            Self::Network { message, source: _ } => Self::Network {
                message: message.clone(),
                source: None,
            },
            Self::Grpc { code, message } => Self::Grpc {
                code: *code,
                message: message.clone(),
            },
            Self::File {
                path,
                message,
                source: _,
            } => Self::File {
                path: path.clone(),
                message: message.clone(),
                source: None,
            },
            Self::Sync { path, message } => Self::Sync {
                path: path.clone(),
                message: message.clone(),
            },
            Self::Conflict { path, message } => Self::Conflict {
                path: path.clone(),
                message: message.clone(),
            },
            Self::Parse { message, source: _ } => Self::Parse {
                message: message.clone(),
                source: None,
            },
            Self::Validation { message } => Self::Validation {
                message: message.clone(),
            },
            Self::Timeout {
                operation,
                timeout_secs,
            } => Self::Timeout {
                operation: operation.clone(),
                timeout_secs: *timeout_secs,
            },
            Self::RetryExhausted {
                operation,
                max_retries,
                last_error,
            } => Self::RetryExhausted {
                operation: operation.clone(),
                max_retries: *max_retries,
                last_error: last_error.clone(),
            },
            Self::Internal { message, source: _ } => Self::Internal {
                message: message.clone(),
                source: None,
            },
        }
    }
}

impl ClientError {
    /// 创建配置错误
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// 创建认证错误
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    /// 创建 Token 错误
    pub fn token(message: impl Into<String>) -> Self {
        Self::Token {
            message: message.into(),
        }
    }

    /// 创建网络错误
    pub fn network(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Network {
            message: message.into(),
            source,
        }
    }

    /// 创建 gRPC 错误
    pub fn grpc(code: tonic::Code, message: impl Into<String>) -> Self {
        Self::Grpc {
            code,
            message: message.into(),
        }
    }

    /// 创建文件错误
    pub fn file(
        path: impl Into<String>,
        message: impl Into<String>,
        source: Option<std::io::Error>,
    ) -> Self {
        Self::File {
            path: path.into(),
            message: message.into(),
            source,
        }
    }

    /// 创建同步错误
    pub fn sync(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Sync {
            path: path.into(),
            message: message.into(),
        }
    }

    /// 创建冲突错误
    pub fn conflict(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Conflict {
            path: path.into(),
            message: message.into(),
        }
    }

    /// 创建解析错误
    pub fn parse(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Parse {
            message: message.into(),
            source,
        }
    }

    /// 创建验证错误
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// 创建超时错误
    pub fn timeout(operation: impl Into<String>, timeout_secs: u64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            timeout_secs,
        }
    }

    /// 创建重试失败错误
    pub fn retry_exhausted(
        operation: impl Into<String>,
        max_retries: usize,
        last_error: impl Into<String>,
    ) -> Self {
        Self::RetryExhausted {
            operation: operation.into(),
            max_retries,
            last_error: last_error.into(),
        }
    }

    /// 创建内部错误
    pub fn internal(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            source,
        }
    }

    /// 检查错误是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network { .. }
                | Self::Grpc {
                    code: tonic::Code::Unavailable | tonic::Code::DeadlineExceeded,
                    ..
                }
                | Self::Timeout { .. }
        )
    }

    /// 获取用户友好的错误消息
    pub fn user_message(&self) -> String {
        match self {
            Self::Config { message } => format!("配置问题：{}", message),
            Self::Auth { message } => format!("认证失败：{}", message),
            Self::Token { message } => format!("Token 问题：{}", message),
            Self::Network { message, .. } => format!("网络连接失败：{}", message),
            Self::Grpc { code, message } => {
                format!("服务器错误 ({}): {}", code, message)
            }
            Self::File { path, message, .. } => {
                format!("文件访问失败 ({}): {}", path, message)
            }
            Self::Sync { path, message } => format!("同步失败 ({}): {}", path, message),
            Self::Conflict { path, message } => format!("冲突 ({}): {}", path, message),
            Self::Parse { message, .. } => format!("解析失败：{}", message),
            Self::Validation { message } => format!("验证失败：{}", message),
            Self::Timeout {
                operation,
                timeout_secs,
            } => {
                format!("操作超时：{} 在 {} 秒后未完成", operation, timeout_secs)
            }
            Self::RetryExhausted {
                operation,
                max_retries,
                ..
            } => {
                format!("重试失败：{} 在 {} 次重试后仍失败", operation, max_retries)
            }
            Self::Internal { message, .. } => format!("内部错误：{}", message),
        }
    }

    /// 获取错误代码
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Config { .. } => "CONFIG_ERROR",
            Self::Auth { .. } => "AUTH_ERROR",
            Self::Token { .. } => "TOKEN_ERROR",
            Self::Network { .. } => "NETWORK_ERROR",
            Self::Grpc { .. } => "GRPC_ERROR",
            Self::File { .. } => "FILE_ERROR",
            Self::Sync { .. } => "SYNC_ERROR",
            Self::Conflict { .. } => "CONFLICT_ERROR",
            Self::Parse { .. } => "PARSE_ERROR",
            Self::Validation { .. } => "VALIDATION_ERROR",
            Self::Timeout { .. } => "TIMEOUT_ERROR",
            Self::RetryExhausted { .. } => "RETRY_EXHAUSTED",
            Self::Internal { .. } => "INTERNAL_ERROR",
        }
    }
}

// 从标准错误类型转换
impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        Self::File {
            path: "unknown".to_string(),
            message: err.to_string(),
            source: Some(err),
        }
    }
}

impl From<toml::de::Error> for ClientError {
    fn from(err: toml::de::Error) -> Self {
        Self::Parse {
            message: "TOML 解析失败".to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse {
            message: "JSON 解析失败".to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<tonic::transport::Error> for ClientError {
    fn from(err: tonic::transport::Error) -> Self {
        Self::Network {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<tonic::Status> for ClientError {
    fn from(status: tonic::Status) -> Self {
        Self::Grpc {
            code: status.code(),
            message: status.message().to_string(),
        }
    }
}

/// 客户端 Result 类型别名
pub type Result<T> = std::result::Result<T, ClientError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = ClientError::config("测试消息");
        assert_eq!(err.error_code(), "CONFIG_ERROR");
        assert!(err.user_message().contains("配置问题"));
    }

    #[test]
    fn test_retryable_check() {
        let network_err = ClientError::network("连接失败", None);
        assert!(network_err.is_retryable());

        let config_err = ClientError::config("无效配置");
        assert!(!config_err.is_retryable());
    }

    #[test]
    fn test_grpc_error() {
        let status = tonic::Status::cancelled("已取消");
        let err: ClientError = status.into();

        assert_eq!(err.error_code(), "GRPC_ERROR");
        assert!(err.user_message().contains("服务器错误"));
    }
}
