use crate::db::DbPool;

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

/*
#[tonic::async_trait]
impl DeviceService for DeviceGrpcService {
    async fn register_device(
        &self,
        request: Request<RegisterDeviceRequest>,
    ) -> Result<Response<RegisterDeviceResponse>, Status> {
        let req = request.into_inner();

        // 从 Token 中获取 user_id
        // let user_id = extract_user_id_from_request(&request)?;

        match DeviceRepository::create(
            &self.pool,
            &user_id,
            &req.device_name,
            &req.device_type,
            &req.device_fingerprint,
        )
        .await
        {
            Ok(device) => Ok(Response::new(RegisterDeviceResponse {
                success: true,
                message: "Device registered successfully".to_string(),
                device_id: device.id.to_string(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn list_devices(
        &self,
        request: Request<ListDevicesRequest>,
    ) -> Result<Response<ListDevicesResponse>, Status> {
        // let user_id = extract_user_id_from_request(&request)?;

        match DeviceRepository::find_by_user(&self.pool, &user_id).await {
            Ok(devices) => {
                let device_protos: Vec<Device> = devices
                    .iter()
                    .map(|d| Device {
                        device_id: d.id.to_string(),
                        device_name: d.device_name.clone(),
                        device_type: d.device_type.clone(),
                        device_fingerprint: d.device_fingerprint.clone(),
                        last_seen: d.last_seen.timestamp(),
                        created_at: d.created_at.timestamp(),
                        is_active: d.is_active,
                    })
                    .collect();

                Ok(Response::new(ListDevicesResponse { devices: device_protos }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn update_device(
        &self,
        request: Request<UpdateDeviceRequest>,
    ) -> Result<Response<UpdateDeviceResponse>, Status> {
        let req = request.into_inner();

        // 验证设备属于当前用户
        // 更新设备名称

        Ok(Response::new(UpdateDeviceResponse {
            success: true,
            message: "Device updated successfully".to_string(),
        }))
    }

    async fn remove_device(
        &self,
        request: Request<RemoveDeviceRequest>,
    ) -> Result<Response<RemoveDeviceResponse>, Status> {
        let req = request.into_inner();
        let device_id = uuid::Uuid::parse_str(&req.device_id)
            .map_err(|_| Status::invalid_argument("Invalid device ID"))?;

        match DeviceRepository::delete(&self.pool, &device_id).await {
            Ok(_) => Ok(Response::new(RemoveDeviceResponse {
                success: true,
                message: "Device removed successfully".to_string(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_register_device() {
        // 测试设备注册
    }
}
