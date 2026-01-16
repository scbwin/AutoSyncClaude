use crate::db::DbPool;
use crate::proto::sync::claude_sync::{
    device_service_server::DeviceService, ListDevicesRequest, ListDevicesResponse,
    RegisterDeviceRequest, RegisterDeviceResponse, RemoveDeviceRequest, RemoveDeviceResponse,
    UpdateDeviceRequest, UpdateDeviceResponse,
};
use tonic::{Request, Response, Status};

/// DeviceService gRPC 实现
pub struct DeviceGrpcService {
    pool: DbPool,
}

impl DeviceGrpcService {
    /// 创建新的 gRPC 服务实例
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[tonic::async_trait]
impl DeviceService for DeviceGrpcService {
    async fn register_device(
        &self,
        _request: Request<RegisterDeviceRequest>,
    ) -> Result<Response<RegisterDeviceResponse>, Status> {
        // TODO: 实现设备注册逻辑
        Ok(Response::new(RegisterDeviceResponse {
            success: true,
            message: "Device registration not yet implemented".to_string(),
            device_id: String::new(),
        }))
    }

    async fn list_devices(
        &self,
        _request: Request<ListDevicesRequest>,
    ) -> Result<Response<ListDevicesResponse>, Status> {
        // TODO: 实现设备列表查询逻辑
        Ok(Response::new(ListDevicesResponse { devices: vec![] }))
    }

    async fn update_device(
        &self,
        _request: Request<UpdateDeviceRequest>,
    ) -> Result<Response<UpdateDeviceResponse>, Status> {
        // TODO: 实现设备更新逻辑
        Ok(Response::new(UpdateDeviceResponse {
            success: true,
            message: "Device update not yet implemented".to_string(),
        }))
    }

    async fn remove_device(
        &self,
        _request: Request<RemoveDeviceRequest>,
    ) -> Result<Response<RemoveDeviceResponse>, Status> {
        // TODO: 实现设备删除逻辑
        Ok(Response::new(RemoveDeviceResponse {
            success: true,
            message: "Device removal not yet implemented".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_register_device() {
        // 测试设备注册
    }
}
