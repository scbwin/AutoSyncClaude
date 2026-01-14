use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Token 存储结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStorage {
    /// Access Token
    pub access_token: String,

    /// Refresh Token
    pub refresh_token: String,

    /// 设备 ID
    pub device_id: String,

    /// 用户 ID
    pub user_id: String,

    /// Access Token 过期时间（时间戳）
    pub access_expires_at: i64,

    /// Refresh Token 过期时间（时间戳）
    pub refresh_expires_at: i64,
}

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 过期时间
    pub exp: i64,

    /// 签发时间
    pub iat: i64,

    /// 签发者
    pub iss: String,

    /// 主题（用户 ID）
    pub sub: String,

    /// 用户 ID
    pub user_id: Uuid,

    /// 设备 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<Uuid>,

    /// Token 类型（access/refresh）
    pub token_type: String,

    /// JWT ID（用于撤销）
    pub jti: Uuid,
}

/// Token 管理器
pub struct TokenManager {
    /// Token 存储目录
    token_dir: PathBuf,

    /// 加密密钥（可选）
    encryption_key: Option<String>,

    /// JWT 密钥（用于解码，仅用于验证）
    #[allow(dead_code)]
    jwt_secret: String,
}

impl TokenManager {
    /// 创建新的 Token 管理器
    pub fn new(token_dir: PathBuf, encryption_key: Option<String>, jwt_secret: String) -> Self {
        Self {
            token_dir,
            encryption_key,
            jwt_secret,
        }
    }

    /// 保存 Token
    pub fn save_tokens(&self, tokens: TokenStorage) -> Result<()> {
        // 确保目录存在
        fs::create_dir_all(&self.token_dir)
            .with_context(|| format!("无法创建 token 目录: {:?}", self.token_dir))?;

        let token_file = self.token_file()?;

        // 序列化 Token
        let content = serde_json::to_string_pretty(&tokens).context("无法序列化 Token")?;

        // 如果设置了加密密钥，则加密 Token
        let content_to_write = if let Some(ref key) = self.encryption_key {
            self.encrypt(&content, key)?
        } else {
            content
        };

        // 写入文件
        fs::write(&token_file, content_to_write)
            .with_context(|| format!("无法写入 token 文件: {:?}", token_file))?;

        info!("Token 已保存到: {:?}", token_file);

        Ok(())
    }

    /// 加载 Token
    pub fn load_tokens(&self) -> Result<TokenStorage> {
        let token_file = self.token_file()?;

        if !token_file.exists() {
            anyhow::bail!("Token 文件不存在: {:?}", token_file);
        }

        // 读取文件
        let content = fs::read_to_string(&token_file)
            .with_context(|| format!("无法读取 token 文件: {:?}", token_file))?;

        // 如果设置了加密密钥，则解密 Token
        let content = if let Some(ref key) = self.encryption_key {
            self.decrypt(&content, key)?
        } else {
            content
        };

        // 反序列化
        let tokens: TokenStorage = serde_json::from_str(&content).context("无法解析 Token")?;

        debug!("Token 加载成功");

        Ok(tokens)
    }

    /// 删除 Token
    pub fn delete_tokens(&self) -> Result<()> {
        let token_file = self.token_file()?;

        if token_file.exists() {
            fs::remove_file(&token_file)
                .with_context(|| format!("无法删除 token 文件: {:?}", token_file))?;
            info!("Token 已删除: {:?}", token_file);
        }

        Ok(())
    }

    /// 检查 Token 是否存在
    pub fn has_tokens(&self) -> bool {
        self.token_file().map(|p| p.exists()).unwrap_or(false)
    }

    /// 检查 Access Token 是否需要刷新
    pub fn needs_refresh(&self, refresh_before: i64) -> Result<bool> {
        let tokens = self.load_tokens()?;

        let now = Utc::now().timestamp();
        let needs_refresh = tokens.access_expires_at - now < refresh_before;

        if needs_refresh {
            info!("Access Token 即将过期，需要刷新");
        }

        Ok(needs_refresh)
    }

    /// 检查 Access Token 是否已过期
    pub fn is_access_expired(&self) -> Result<bool> {
        let tokens = self.load_tokens()?;

        let now = Utc::now().timestamp();
        let expired = now >= tokens.access_expires_at;

        if expired {
            warn!("Access Token 已过期");
        }

        Ok(expired)
    }

    /// 检查 Refresh Token 是否已过期
    pub fn is_refresh_expired(&self) -> Result<bool> {
        let tokens = self.load_tokens()?;

        let now = Utc::now().timestamp();
        let expired = now >= tokens.refresh_expires_at;

        if expired {
            warn!("Refresh Token 已过期，需要重新登录");
        }

        Ok(expired)
    }

    /// 解码 Token（不验证签名，仅用于查看信息）
    pub fn decode_token(&self, token: &str) -> Result<Claims> {
        // 不验证签名，仅解码
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(b"dummy_secret"),
            &Validation::default(),
        )
        .context("无法解码 Token")?;

        Ok(token_data.claims)
    }

    /// 获取当前 Access Token
    pub fn get_access_token(&self) -> Result<String> {
        let tokens = self.load_tokens()?;
        Ok(tokens.access_token)
    }

    /// 获取当前 Refresh Token
    pub fn get_refresh_token(&self) -> Result<String> {
        let tokens = self.load_tokens()?;
        Ok(tokens.refresh_token)
    }

    /// 获取设备 ID
    pub fn get_device_id(&self) -> Result<String> {
        let tokens = self.load_tokens()?;
        Ok(tokens.device_id)
    }

    /// 获取用户 ID
    pub fn get_user_id(&self) -> Result<String> {
        let tokens = self.load_tokens()?;
        Ok(tokens.user_id)
    }

    /// 更新 Access Token
    pub fn update_access_token(&self, new_access_token: String, expires_at: i64) -> Result<()> {
        let mut tokens = self.load_tokens()?;
        tokens.access_token = new_access_token;
        tokens.access_expires_at = expires_at;

        self.save_tokens(tokens)?;

        info!("Access Token 已更新");

        Ok(())
    }

    /// 加密内容
    fn encrypt(&self, plaintext: &str, key: &str) -> Result<String> {
        use aes_gcm::{
            aead::{Aead, AeadCore, KeyInit, OsRng},
            Aes256Gcm,
        };

        // 从密钥派生 32 字节密钥
        let key_bytes = Self::derive_key(key)?;

        // 生成随机 nonce
        let cipher = Aes256Gcm::new(&key_bytes.into());
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // 加密
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("加密失败: {}", e))?;

        // 组合 nonce 和密文
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);

        // Base64 编码
        Ok(base64_helper::encode(&result))
    }

    /// 解密内容
    fn decrypt(&self, ciphertext: &str, key: &str) -> Result<String> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        // Base64 解码
        let data = base64_helper::decode(ciphertext).context("Base64 解码失败")?;

        if data.len() < 12 {
            anyhow::bail!("密文太短");
        }

        // 分离 nonce 和密文
        let (nonce_bytes, ciphertext_bytes) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // 从密钥派生 32 字节密钥
        let key_bytes = Self::derive_key(key)?;

        // 解密
        let cipher = Aes256Gcm::new(&key_bytes.into());
        let plaintext = cipher
            .decrypt(nonce, ciphertext_bytes)
            .map_err(|e| anyhow::anyhow!("解密失败: {}", e))?;

        String::from_utf8(plaintext).context("UTF-8 解码失败")
    }

    /// 从字符串派生密钥
    fn derive_key(key: &str) -> Result<[u8; 32]> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&result);

        Ok(key_bytes)
    }

    /// 获取 token 文件路径
    fn token_file(&self) -> Result<PathBuf> {
        Ok(self.token_dir.join("tokens.json"))
    }

    /// 从服务器响应创建 Token 存储
    pub fn from_login_response(
        access_token: String,
        refresh_token: String,
        device_id: String,
        user_id: String,
    ) -> Result<TokenStorage> {
        // 解码 Access Token 获取过期时间
        // 注意：这里需要 JWT 密钥，实际使用时应该从配置中获取
        // 这里我们假设服务器会返回过期时间信息

        let now = Utc::now();
        let access_expires_at = (now + Duration::hours(1)).timestamp(); // Access Token 1 小时
        let refresh_expires_at = (now + Duration::days(30)).timestamp(); // Refresh Token 30 天

        Ok(TokenStorage {
            access_token,
            refresh_token,
            device_id,
            user_id,
            access_expires_at,
            refresh_expires_at,
        })
    }

    /// 验证 Token 格式（不验证签名）
    pub fn validate_token_format(token: &str) -> Result<()> {
        if token.is_empty() {
            anyhow::bail!("Token 为空");
        }

        // 检查 JWT 格式（header.payload.signature）
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            anyhow::bail!("Token 格式无效");
        }

        // JWT 使用 base64url 编码，需要特殊处理
        use base64::prelude::*;

        // 尝试解码 header 和 payload
        BASE64_URL_SAFE_NO_PAD
            .decode(parts[0])
            .context("Token header 解码失败")?;
        BASE64_URL_SAFE_NO_PAD
            .decode(parts[1])
            .context("Token payload 解码失败")?;

        Ok(())
    }
}

// ===== 辅助函数 =====

/// Base64 编码/解码辅助函数
mod base64_helper {
    use anyhow::{Context, Result};

    pub fn encode(data: &[u8]) -> String {
        use base64::prelude::*;
        BASE64_STANDARD.encode(data)
    }

    pub fn decode(data: &str) -> Result<Vec<u8>> {
        use base64::prelude::*;
        BASE64_STANDARD.decode(data).context("Base64 解码失败")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_token_format() {
        // 有效 Token
        let valid_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert!(TokenManager::validate_token_format(valid_token).is_ok());

        // 无效 Token
        let invalid_token = "invalid.token";
        assert!(TokenManager::validate_token_format(invalid_token).is_err());

        // 空 Token
        assert!(TokenManager::validate_token_format("").is_err());
    }

    #[test]
    fn test_derive_key() {
        let key1 = TokenManager::derive_key("test_password").unwrap();
        let key2 = TokenManager::derive_key("test_password").unwrap();
        let key3 = TokenManager::derive_key("different_password").unwrap();

        // 相同密码应该生成相同密钥
        assert_eq!(key1, key2);

        // 不同密码应该生成不同密钥
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let manager = TokenManager::new(
            PathBuf::from("/tmp/test"),
            Some("test_encryption_key".to_string()),
            "jwt_secret".to_string(),
        );

        let plaintext = "This is a secret message";
        let encrypted = manager.encrypt(plaintext, "test_key").unwrap();
        let decrypted = manager.decrypt(&encrypted, "test_key").unwrap();

        assert_eq!(plaintext, decrypted);

        // 错误的密钥应该解密失败
        assert!(manager.decrypt(&encrypted, "wrong_key").is_err());
    }
}
