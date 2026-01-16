use crate::auth::AuthService as LocalAuthService;
use crate::cache::Cache;
use crate::config::Config;
use crate::db::DbPool;
use crate::proto::claude_sync::{
    auth_service_server::AuthService as AuthServiceTrait, LoginRequest as ProtoLoginRequest,
    LoginResponse as ProtoLoginResponse, LogoutRequest, LogoutResponse, RefreshTokenRequest,
    RefreshTokenResponse, RegisterRequest as ProtoRegisterRequest,
    RegisterResponse as ProtoRegisterResponse, RevokeTokenRequest, RevokeTokenResponse,
};
use std::str::FromStr;
use tonic::{Request, Response, Status};

/// AuthService gRPC 实现
pub struct AuthGrpcService {
    auth_service: LocalAuthService,
}

impl AuthGrpcService {
    /// 创建新的 gRPC 服务实例
    pub fn new(pool: DbPool, cache: Cache, config: Config) -> Self {
        let auth_service = LocalAuthService::new(pool, cache, config);
        Self { auth_service }
    }
}

#[tonic::async_trait]
impl AuthServiceTrait for AuthGrpcService {
    async fn register(
        &self,
        request: Request<ProtoRegisterRequest>,
    ) -> Result<Response<ProtoRegisterResponse>, Status> {
        let req = request.into_inner();

        // 调用认证服务
        match self
            .auth_service
            .register(req.username, req.email, req.password)
            .await
        {
            Ok((user_id, _email)) => Ok(Response::new(ProtoRegisterResponse {
                success: true,
                message: "Registration successful".to_string(),
                user_id: user_id.to_string(),
            })),
            Err(e) => {
                tracing::error!("Registration failed: {}", e);
                Err(Status::already_exists(e.to_string()))
            }
        }
    }

    async fn login(
        &self,
        request: Request<ProtoLoginRequest>,
    ) -> Result<Response<ProtoLoginResponse>, Status> {
        let req = request.into_inner();

        match self
            .auth_service
            .login(
                req.email,
                req.password,
                req.device_name,
                &req.device_type,
                req.device_fingerprint,
            )
            .await
        {
            Ok(result) => Ok(Response::new(ProtoLoginResponse {
                success: true,
                message: "Login successful".to_string(),
                access_token: result.access_token,
                refresh_token: result.refresh_token,
                expires_at: result.expires_at.timestamp(),
                user_id: result.user_id.to_string(),
                device_id: result.device_id.to_string(),
            })),
            Err(e) => {
                tracing::error!("Login failed: {}", e);
                Err(Status::unauthenticated(e.to_string()))
            }
        }
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let req = request.into_inner();

        match self.auth_service.refresh_token(req.refresh_token).await {
            Ok(response) => Ok(Response::new(RefreshTokenResponse {
                success: true,
                message: "Token refreshed successfully".to_string(),
                access_token: response.access_token,
                expires_at: response.expires_at.timestamp(),
            })),
            Err(e) => {
                tracing::error!("Token refresh failed: {}", e);
                Err(Status::unauthenticated(e.to_string()))
            }
        }
    }

    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        let req = request.into_inner();

        match self.auth_service.logout(req.refresh_token).await {
            Ok(_) => Ok(Response::new(LogoutResponse {
                success: true,
                message: "Logout successful".to_string(),
            })),
            Err(e) => {
                tracing::error!("Logout failed: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }

    async fn revoke_token(
        &self,
        request: Request<RevokeTokenRequest>,
    ) -> Result<Response<RevokeTokenResponse>, Status> {
        let req = request.into_inner();

        // 解析 Token ID
        let token_id = uuid::Uuid::from_str(&req.token_id)
            .map_err(|_| Status::invalid_argument("Invalid token ID"))?;

        // 计算过期时间（假设 30 天后过期）
        let expires_at = chrono::Utc::now().timestamp() + (30 * 24 * 60 * 60);

        match self.auth_service.revoke_token(token_id, expires_at).await {
            Ok(_) => Ok(Response::new(RevokeTokenResponse {
                success: true,
                message: "Token revoked successfully".to_string(),
            })),
            Err(e) => {
                tracing::error!("Token revocation failed: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_register() {
        // 测试注册功能
    }

    #[tokio::test]
    #[ignore]
    async fn test_login() {
        // 测试登录功能
    }
}
