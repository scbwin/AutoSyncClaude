use crate::config::Config;
use anyhow::Result;
use rusoto_core::Region;
use rusoto_s3::{
    DeleteObjectRequest, GetObjectRequest, PutObjectRequest, S3Client, S3Ext,
};
use sha2::{Digest, Sha256};
use std::io::Read;
use tracing::{debug, error, info};
use uuid::Uuid;

/// MinIO/S3 对象存储服务
#[derive(Clone)]
pub struct StorageService {
    client: S3Client,
    bucket: String,
}

impl StorageService {
    /// 从配置创建存储服务
    pub async fn from_config(config: &Config) -> Result<Self> {
        info!("Connecting to MinIO/S3 storage...");

        // 配置 S3 客户端
        let region = Region::Custom {
            name: config.minio.region.clone(),
            endpoint: config.minio.endpoint.clone(),
        };

        // 创建客户端（使用默认凭据链）
        // 注意：生产环境应该使用环境变量或 AWS 凭据文件
        let client = S3Client::new(region.clone());

        let storage = Self {
            client,
            bucket: config.minio.bucket.clone(),
        };

        // 确保存储桶存在
        storage.ensure_bucket().await?;

        info!("✓ Storage connected successfully");

        Ok(storage)
    }

    /// 确保存储桶存在
    async fn ensure_bucket(&self) -> Result<()> {
        // 检查存储桶是否存在
        match self.client.list_buckets().await {
            Ok(response) => {
                let exists = response.buckets.iter().any(|b| b.name == self.bucket);
                if !exists {
                    // 创建存储桶
                    info!("Creating bucket: {}", self.bucket);
                    self.client.create_bucket().await?;
                    info!("✓ Bucket created: {}", self.bucket);
                } else {
                    debug!("Bucket exists: {}", self.bucket);
                }
            }
            Err(e) => {
                error!("Failed to list buckets: {}", e);
                return Err(anyhow::anyhow!("Failed to connect to storage: {}", e));
            }
        }
        Ok(())
    }

    /// ===== 文件操作 =====

    /// 上传文件
    pub async fn upload_file(
        &self,
        user_id: &Uuid,
        file_hash: &str,
        data: Vec<u8>,
        content_type: Option<String>,
    ) -> Result<StoragePath> {
        let storage_path = self.generate_storage_path(user_id, file_hash);

        debug!(
            "Uploading file: user_id={}, hash={}, size={}",
            user_id,
            file_hash,
            data.len()
        );

        let put_request = PutObjectRequest {
            bucket: self.bucket.clone(),
            key: storage_path.full_path(),
            body: Some(data.into()),
            content_type,
            ..Default::default()
        };

        self.client
            .put_object(put_request)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upload file: {}", e))?;

        debug!("✓ File uploaded successfully");

        Ok(storage_path)
    }

    /// 下载文件
    pub async fn download_file(&self, user_id: &Uuid, file_hash: &str) -> Result<Vec<u8>> {
        let storage_path = self.generate_storage_path(user_id, file_hash);

        debug!(
            "Downloading file: user_id={}, hash={}",
            user_id, file_hash
        );

        let get_request = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: storage_path.full_path(),
            ..Default::default()
        };

        let result = self
            .client
            .get_object(get_request)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download file: {}", e))?;

        let mut data = Vec::new();
        let mut reader = result.body.into_async_read();
        reader
            .read_to_end(&mut data)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file data: {}", e))?;

        debug!("✓ File downloaded successfully: {} bytes", data.len());

        Ok(data)
    }

    /// 删除文件
    pub async fn delete_file(&self, user_id: &Uuid, file_hash: &str) -> Result<()> {
        let storage_path = self.generate_storage_path(user_id, file_hash);

        debug!(
            "Deleting file: user_id={}, hash={}",
            user_id, file_hash
        );

        let delete_request = DeleteObjectRequest {
            bucket: self.bucket.clone(),
            key: storage_path.full_path(),
            ..Default::default()
        };

        self.client
            .delete_object(delete_request)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete file: {}", e))?;

        debug!("✓ File deleted successfully");

        Ok(())
    }

    /// 检查文件是否存在
    pub async fn file_exists(&self, user_id: &Uuid, file_hash: &str) -> Result<bool> {
        let storage_path = self.generate_storage_path(user_id, file_hash);

        match self
            .client
            .head_object(rusoto_s3::HeadObjectRequest {
                bucket: self.bucket.clone(),
                key: storage_path.full_path(),
                ..Default::default()
            })
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(anyhow::anyhow!("Failed to check file existence: {}", e))
                }
            }
        }
    }

    /// ===== 辅助方法 =====

    /// 生成存储路径
    fn generate_storage_path(&self, user_id: &Uuid, file_hash: &str) -> StoragePath {
        StoragePath::new(user_id, file_hash)
    }

    /// 计算文件哈希（SHA-256）
    pub fn hash_file(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// 验证文件哈希
    pub fn verify_hash(data: &[u8], expected_hash: &str) -> bool {
        let actual_hash = Self::hash_file(data);
        actual_hash == expected_hash
    }
}

/// 存储路径
#[derive(Debug, Clone)]
pub struct StoragePath {
    pub user_id: Uuid,
    pub file_hash: String,
}

impl StoragePath {
    /// 创建新的存储路径
    pub fn new(user_id: &Uuid, file_hash: &str) -> Self {
        Self {
            user_id: *user_id,
            file_hash: file_hash.to_string(),
        }
    }

    /// 完整路径
    pub fn full_path(&self) -> String {
        format!("users/{}/files/{}.data", self.user_id, self.file_hash)
    }

    /// 版本元数据路径
    pub fn version_metadata_path(&self, version_id: &Uuid) -> String {
        format!(
            "users/{}/versions/{}.meta",
            self.user_id, version_id
        )
    }

    /// 冲突备份路径
    pub fn conflict_backup_path(&self, conflict_id: &Uuid, suffix: &str) -> String {
        format!(
            "users/{}/conflicts/{}/{}.data",
            self.user_id, conflict_id, suffix
        )
    }
}

/// 分块上传管理器
pub struct ChunkedUpload {
    storage: StorageService,
    upload_id: String,
    user_id: Uuid,
    file_hash: String,
    parts: Vec<UploadPart>,
}

#[derive(Debug, Clone)]
pub struct UploadPart {
    pub part_number: i32,
    pub etag: String,
}

impl ChunkedUpload {
    /// 开始分块上传
    pub async fn begin(
        storage: StorageService,
        user_id: &Uuid,
        file_hash: &str,
        _file_size: i64,
    ) -> Result<Self> {
        // TODO: 实现分块上传初始化
        Ok(Self {
            storage,
            upload_id: Uuid::new_v4().to_string(),
            user_id: *user_id,
            file_hash: file_hash.to_string(),
            parts: Vec::new(),
        })
    }

    /// 上传分块
    pub async fn upload_part(&mut self, part_number: i32, data: Vec<u8>) -> Result<()> {
        // TODO: 实现分块上传
        debug!("Uploading part {}: {} bytes", part_number, data.len());
        Ok(())
    }

    /// 完成分块上传
    pub async fn complete(self) -> Result<()> {
        // TODO: 实现分块上传完成
        debug!("Completing chunked upload");
        Ok(())
    }

    /// 取消分块上传
    pub async fn abort(self) -> Result<()> {
        // TODO: 实现分块上传取消
        debug!("Aborting chunked upload");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_file() {
        let data = b"test data";
        let hash1 = StorageService::hash_file(data);
        let hash2 = StorageService::hash_file(data);
        assert_eq!(hash1, hash2);

        let different_data = b"different data";
        let hash3 = StorageService::hash_file(different_data);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_verify_hash() {
        let data = b"test data";
        let hash = StorageService::hash_file(data);
        assert!(StorageService::verify_hash(data, &hash));
        assert!(!StorageService::verify_hash(data, "wrong_hash"));
    }

    #[test]
    fn test_storage_path() {
        let user_id = Uuid::new_v4();
        let file_hash = "abc123";
        let path = StoragePath::new(&user_id, file_hash);

        assert!(path.full_path().contains(&user_id.to_string()));
        assert!(path.full_path().contains(file_hash));
    }
}
