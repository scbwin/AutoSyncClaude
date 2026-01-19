use crate::proto::claude_sync::auth_service_client::AuthServiceClient;
use crate::proto::claude_sync::{LoginRequest, RegisterRequest};
use tonic::transport::Channel;

/// 注册结果
#[derive(Debug, Clone)]
pub struct RegisterResult {
    pub success: bool,
    pub message: String,
    pub user_id: String,
}

/// 登录结果
#[derive(Debug, Clone)]
pub struct LoginResult {
    pub success: bool,
    pub message: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub user_id: String,
    pub device_id: String,
}

/// gRPC 认证客户端
pub struct AuthClient {
    client: Option<AuthServiceClient<Channel>>,
    server_url: String,
}

impl AuthClient {
    /// 创建新的认证客户端（不立即连接）
    pub fn new(server_url: String) -> Self {
        Self {
            client: None,
            server_url,
        }
    }

    /// 连接到服务器
    async fn ensure_connected(&mut self) -> Result<(), String> {
        if self.client.is_none() {
            let client = AuthServiceClient::connect(self.server_url.clone())
                .await
                .map_err(|e| format!("连接服务器失败: {}", e))?;
            self.client = Some(client);
        }
        Ok(())
    }

    /// 用户注册
    pub async fn register(
        &mut self,
        username: String,
        email: String,
        password: String,
    ) -> Result<RegisterResult, String> {
        self.ensure_connected().await?;

        let request = RegisterRequest { username, email, password };
        let client = self.client.as_mut().unwrap();

        let response = client
            .register(request)
            .await
            .map_err(|e| format!("注册请求失败: {}", e))?;

        let response = response.into_inner();

        Ok(RegisterResult {
            success: response.success,
            message: response.message,
            user_id: response.user_id,
        })
    }

    /// 用户登录
    pub async fn login(
        &mut self,
        email: String,
        password: String,
        device_name: String,
        device_type: String,
        device_fingerprint: String,
    ) -> Result<LoginResult, String> {
        self.ensure_connected().await?;

        let request = LoginRequest {
            email,
            password,
            device_name,
            device_type,
            device_fingerprint,
        };

        let client = self.client.as_mut().unwrap();

        let response = client
            .login(request)
            .await
            .map_err(|e| format!("登录请求失败: {}", e))?;

        let response = response.into_inner();

        Ok(LoginResult {
            success: response.success,
            message: response.message,
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_at: response.expires_at,
            user_id: response.user_id,
            device_id: response.device_id,
        })
    }

    /// 刷新 Token
    pub async fn refresh_token(&mut self, refresh_token: String) -> Result<LoginResult, String> {
        self.ensure_connected().await?;

        let request = crate::proto::claude_sync::RefreshTokenRequest { refresh_token };
        let client = self.client.as_mut().unwrap();

        let response = client
            .refresh_token(request)
            .await
            .map_err(|e| format!("刷新 Token 失败: {}", e))?;

        let response = response.into_inner();

        Ok(LoginResult {
            success: response.success,
            message: response.message,
            access_token: response.access_token,
            refresh_token: String::new(), // 刷新后不返回新的 refresh_token
            expires_at: response.expires_at,
            user_id: String::new(),
            device_id: String::new(),
        })
    }

    /// 登出
    pub async fn logout(&mut self, refresh_token: String) -> Result<(), String> {
        self.ensure_connected().await?;

        let request = crate::proto::claude_sync::LogoutRequest { refresh_token };
        let client = self.client.as_mut().unwrap();

        client
            .logout(request)
            .await
            .map_err(|e| format!("登出请求失败: {}", e))?;

        Ok(())
    }
}
