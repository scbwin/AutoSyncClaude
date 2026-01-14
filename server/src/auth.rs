use crate::cache::Cache;
use crate::config::Config;
use crate::db::{DbPool, TokenRepository, UserRepository};
use crate::models::{Claims, TokenType};
use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use tracing::{info, warn};
use uuid::Uuid;

/// JWT 认证服务
pub struct AuthService {
    pool: DbPool,
    cache: Cache,
    config: Config,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    /// 创建新的认证服务实例
    pub fn new(pool: DbPool, cache: Cache, config: Config) -> Self {
        let secret = config.jwt.secret.clone();
        Self {
            pool,
            cache,
            config,
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    /// 用户注册
    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<(Uuid, String)> {
        info!("Registering new user: {}", email);

        // 检查邮箱是否已存在
        if let Some(_) = UserRepository::find_by_email(self.pool.inner(), &email).await? {
            return Err(anyhow::anyhow!("Email already registered"));
        }

        // 检查用户名是否已存在
        if let Some(_) = UserRepository::find_by_username(self.pool.inner(), &username).await? {
            return Err(anyhow::anyhow!("Username already taken"));
        }

        // 验证密码强度
        Self::validate_password(&password)?;

        // 哈希密码
        let password_hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)?;

        // 创建用户
        let user = UserRepository::create(
            self.pool.inner(),
            &username,
            &email,
            &password_hash,
        )
        .await?;

        info!("User registered successfully: {}", user.id);

        Ok((user.id, user.email))
    }

    /// 用户登录
    pub async fn login(
        &self,
        email: String,
        password: String,
        device_name: String,
        device_type: &str,
        device_fingerprint: String,
    ) -> Result<LoginResult> {
        info!("Login attempt for: {}", email);

        // 查找用户
        let user_row = UserRepository::find_by_email(self.pool.inner(), &email).await?
            .ok_or_else(|| anyhow::anyhow!("Invalid email or password"))?;

        if !user_row.is_active {
            return Err(anyhow::anyhow!("User account is inactive"));
        }

        // 验证密码
        if !bcrypt::verify(&password, &user_row.password_hash)? {
            warn!("Failed password verification for: {}", email);
            return Err(anyhow::anyhow!("Invalid email or password"));
        }

        // 查找或创建设备
        use crate::db::DeviceRepository;
        let device = match DeviceRepository::find_by_fingerprint(self.pool.inner(), &device_fingerprint).await? {
            Some(dev) => {
                // 更新最后在线时间
                DeviceRepository::update_last_seen(self.pool.inner(), &dev.id).await?;
                dev
            }
            None => {
                // 注册新设备
                DeviceRepository::create(
                    self.pool.inner(),
                    &user_row.id,
                    &device_name,
                    device_type,
                    &device_fingerprint,
                )
                .await?
            }
        };

        // 生成 Token
        let (access_token, refresh_token) = self.generate_tokens(
            user_row.id,
            Some(device.id),
        )?;

        // 保存 Refresh Token 到数据库
        let token_hash = Self::hash_token(&refresh_token);
        let token_prefix = Self::generate_token_prefix(&refresh_token);
        let expires_at = Utc::now() + Duration::seconds(self.config.jwt.refresh_token_expiration as i64);

        TokenRepository::save(
            self.pool.inner(),
            &user_row.id,
            Some(&device.id),
            &token_hash,
            &token_prefix,
            &expires_at,
        )
        .await?;

        // 更新用户最后登录时间
        UserRepository::update_last_login(self.pool.inner(), &user_row.id).await?;

        // 设备上线
        self.cache.device_online(&device.id, &user_row.id).await?;

        info!("User logged in successfully: {}", user_row.id);

        Ok(LoginResult {
            user_id: user_row.id,
            device_id: device.id,
            access_token,
            refresh_token,
            expires_at: expires_at.clone(),
        })
    }

    /// 刷新 Access Token
    pub async fn refresh_token(&self, refresh_token: String) -> Result<TokenResponse> {
        // 验证 Refresh Token
        let claims = self.verify_token(&refresh_token, TokenType::Refresh)?;

        // 检查 Token 是否被撤销
        let token_hash = Self::hash_token(&refresh_token);
        if let Some(token_record) = TokenRepository::find_by_hash(self.pool.inner(), &token_hash).await? {
            if token_record.is_revoked || token_record.expires_at < Utc::now() {
                return Err(anyhow::anyhow!("Refresh token has been revoked or expired"));
            }
        } else {
            return Err(anyhow::anyhow!("Invalid refresh token"));
        }

        // 生成新的 Access Token
        let access_token = self.generate_token(
            claims.user_id,
            claims.device_id,
            TokenType::Access,
        )?;

        let expires_at = Utc::now() + Duration::seconds(self.config.jwt.access_token_expiration as i64);

        Ok(TokenResponse {
            access_token,
            expires_at,
        })
    }

    /// 登出（撤销 Refresh Token）
    pub async fn logout(&self, refresh_token: String) -> Result<()> {
        info!("Processing logout");

        // 验证 Token
        let claims = self.verify_token(&refresh_token, TokenType::Refresh)?;

        // 获取 Token 记录
        let token_hash = Self::hash_token(&refresh_token);
        if let Some(token_record) = TokenRepository::find_by_hash(self.pool.inner(), &token_hash).await? {
            // 撤销 Token
            TokenRepository::revoke(self.pool.inner(), &token_record.id).await?;

            // 设备离线
            if let Some(device_id) = claims.device_id {
                let user = UserRepository::find_by_id(self.pool.inner(), &claims.user_id).await?;
                if let Some(user) = user {
                    self.cache.device_offline(&device_id, &user.id).await?;
                }
            }

            info!("User logged out: {}", claims.user_id);
        }

        Ok(())
    }

    /// 验证 Token
    pub async fn verify_access_token(&self, token: &str) -> Result<Claims> {
        let claims = self.verify_token(token, TokenType::Access)?;

        // 检查 Token 是否在黑名单中
        if self.cache.is_token_revoked(&claims.jti).await? {
            return Err(anyhow::anyhow!("Token has been revoked"));
        }

        Ok(claims)
    }

    /// 撤销 Token（加入黑名单）
    pub async fn revoke_token(&self, jti: Uuid, expires_at: i64) -> Result<()> {
        self.cache.revoke_token(&jti, expires_at).await?;
        Ok(())
    }

    /// ===== 内部辅助方法 =====

    /// 生成 Access Token 和 Refresh Token
    fn generate_tokens(&self, user_id: Uuid, device_id: Option<Uuid>) -> Result<(String, String)> {
        let access_token = self.generate_token(user_id, device_id, TokenType::Access)?;
        let refresh_token = self.generate_token(user_id, device_id, TokenType::Refresh)?;
        Ok((access_token, refresh_token))
    }

    /// 生成 Token
    fn generate_token(
        &self,
        user_id: Uuid,
        device_id: Option<Uuid>,
        token_type: TokenType,
    ) -> Result<String> {
        let now = Utc::now();
        let expiration = match token_type {
            TokenType::Access => self.config.access_token_expiration(),
            TokenType::Refresh => self.config.refresh_token_expiration(),
        };

        let claims = Claims {
            exp: (now + expiration).timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: self.config.jwt.issuer.clone(),
            sub: user_id.to_string(),
            user_id,
            device_id,
            token_type,
            jti: Uuid::new_v4(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;

        Ok(token)
    }

    /// 验证 Token
    fn verify_token(&self, token: &str, expected_type: TokenType) -> Result<Claims> {
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;

        // 检查 Token 类型
        if token_data.claims.token_type != expected_type {
            return Err(anyhow::anyhow!("Invalid token type"));
        }

        Ok(token_data.claims)
    }

    /// 哈希 Token
    fn hash_token(token: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 生成 Token 前缀
    fn generate_token_prefix(token: &str) -> String {
        token.chars().take(8).collect()
    }

    /// 验证密码强度
    fn validate_password(password: &str) -> Result<()> {
        if password.len() < 8 {
            return Err(anyhow::anyhow!("Password must be at least 8 characters long"));
        }

        // 可以添加更多密码强度验证
        Ok(())
    }
}

// ===== 返回类型 =====

/// 登录结果
#[derive(Debug, Clone)]
pub struct LoginResult {
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Token 响应
#[derive(Debug, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_token() {
        let token = "test_token_12345";
        let hash1 = AuthService::hash_token(token);
        let hash2 = AuthService::hash_token(token);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_token_prefix() {
        let token = "abcdefghijklmnopqrstuvwxyz";
        let prefix = AuthService::generate_token_prefix(token);
        assert_eq!(prefix, "abcdefgh");
    }

    #[test]
    fn test_validate_password() {
        assert!(AuthService::validate_password("short").is_err());
        assert!(AuthService::validate_password("longenoughpassword").is_ok());
    }
}
