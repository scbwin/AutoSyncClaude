use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===== 用户模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

impl User {
    /// 创建新用户（用于注册）
    pub fn new(username: String, email: String, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_active: true,
        }
    }

    /// 验证密码
    pub fn verify_password(&self, password: &str) -> anyhow::Result<bool> {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        Ok(self.password_hash == hash)
    }
}

// ===== 设备模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_name: String,
    pub device_type: DeviceType,
    pub device_fingerprint: String,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Windows,
    Linux,
    MacOS,
    Other,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Windows => "windows",
            DeviceType::Linux => "linux",
            DeviceType::MacOS => "macos",
            DeviceType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "windows" => DeviceType::Windows,
            "linux" => DeviceType::Linux,
            "macos" => DeviceType::MacOS,
            _ => DeviceType::Other,
        }
    }
}

impl Device {
    /// 创建新设备
    pub fn new(
        user_id: Uuid,
        device_name: String,
        device_type: DeviceType,
        device_fingerprint: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            device_name,
            device_type,
            device_fingerprint,
            last_seen: Utc::now(),
            created_at: Utc::now(),
            is_active: true,
        }
    }

    /// 更新最后在线时间
    pub fn update_last_seen(&mut self) {
        self.last_seen = Utc::now();
    }
}

// ===== Token 模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Option<Uuid>,
    pub token_hash: String,
    pub token_prefix: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_revoked: bool,
}

impl Token {
    /// 生成 Token 前缀（前 8 位用于识别）
    pub fn generate_prefix(token: &str) -> String {
        token.chars().take(8).collect()
    }

    /// 检查 Token 是否过期
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// 检查 Token 是否已撤销
    pub fn is_revoked_or_expired(&self) -> bool {
        self.is_revoked || self.is_expired()
    }
}

// ===== JWT Claims =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    // 标准声明
    pub exp: usize,      // 过期时间
    pub iat: usize,      // 签发时间
    pub iss: String,     // 签发者
    pub sub: String,     // 主题（用户 ID）

    // 自定义声明
    pub user_id: Uuid,
    pub device_id: Option<Uuid>,
    pub token_type: TokenType,
    pub jti: Uuid, // Token ID（用于撤销）
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

// ===== 同步规则模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRule {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Option<Uuid>,
    pub rule_name: String,
    pub rule_type: RuleType,
    pub pattern: String,
    pub file_type: Option<FileType>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleType {
    Include,
    Exclude,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Agent,
    Skill,
    Plugin,
    Command,
    Config,
    Plan,
}

// ===== 文件版本模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub id: Uuid,
    pub user_id: Uuid,
    pub file_path: String,
    pub file_hash: String, // SHA-256
    pub file_size: i64,
    pub storage_path: String,
    pub version_number: i32,
    pub device_id: Uuid,
    pub parent_version_id: Option<Uuid>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

// ===== 同步状态模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub file_path: String,
    pub local_version_id: Option<Uuid>,
    pub remote_version_id: Option<Uuid>,
    pub sync_status: SyncStatus,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Synced,
    PendingUpload,
    PendingDownload,
    Conflict,
    Error,
}

// ===== 冲突模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: Uuid,
    pub user_id: Uuid,
    pub file_path: String,
    pub base_version_id: Uuid,
    pub local_version_id: Uuid,
    pub remote_version_id: Uuid,
    pub conflict_type: ConflictType,
    pub conflict_data: serde_json::Value,
    pub resolution_status: ResolutionStatus,
    pub resolved_version_id: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    ModifyModify,
    ModifyDelete,
    BinaryConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    Unresolved,
    AutoResolved,
    UserResolved,
    Ignored,
}

// ===== 同步会话模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub session_type: SessionType,
    pub status: SessionStatus,
    pub files_processed: i32,
    pub files_succeeded: i32,
    pub files_failed: i32,
    pub files_skipped: i32,
    pub conflicts_detected: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionType {
    Full,
    Incremental,
    Selective,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

// ===== API 请求/响应模型 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub device_name: String,
    pub device_type: DeviceType,
    pub device_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ===== 辅助函数 =====

/// 计算字符串的 SHA-256 哈希
pub fn hash_string(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 生成设备指纹
pub fn generate_device_fingerprint(
    machine_id: &str,
    mac_address: &str,
    hostname: &str,
) -> String {
    let combined = format!("{}|{}|{}", machine_id, mac_address, hostname);
    hash_string(&combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_conversion() {
        assert_eq!(DeviceType::from_str("windows"), DeviceType::Windows);
        assert_eq!(DeviceType::from_str("Linux"), DeviceType::Linux);
        assert_eq!(DeviceType::Windows.as_str(), "windows");
    }

    #[test]
    fn test_token_generate_prefix() {
        let token = "abcdefghijklmnopqrstuvwxyz123456";
        let prefix = Token::generate_prefix(token);
        assert_eq!(prefix, "abcdefgh");
    }

    #[test]
    fn test_hash_string() {
        let hash1 = hash_string("test");
        let hash2 = hash_string("test");
        assert_eq!(hash1, hash2);

        let hash3 = hash_string("different");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hash".to_string(),
        );
        assert_eq!(user.username, "testuser");
        assert!(user.is_active);
    }
}
