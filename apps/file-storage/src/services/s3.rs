use crate::config::FileStorageConfig;
use crate::domain::FileRecord;
use crate::error::{FileStorageError, FileStorageResult};
use crate::services::storage_backend::StorageBackend;
use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::Client;
use bytes::Bytes;
use secrecy::ExposeSecret;

#[derive(Clone)]
pub struct S3StorageBackend {
    client: Client,
    bucket: String,
}

impl S3StorageBackend {
    pub async fn new(cfg: &FileStorageConfig) -> FileStorageResult<Self> {
        let endpoint = cfg
            .s3_endpoint
            .clone()
            .ok_or_else(|| FileStorageError::Storage("s3_endpoint missing".into()))?;
        let region = cfg.s3_region.clone().unwrap_or_else(|| "us-east-1".into());
        let bucket = cfg
            .s3_bucket
            .clone()
            .ok_or_else(|| FileStorageError::Storage("s3_bucket missing".into()))?;

        let access = cfg
            .s3_access_key
            .as_ref()
            .ok_or_else(|| FileStorageError::Storage("s3_access_key missing".into()))?;
        let secret = cfg
            .s3_secret_key
            .as_ref()
            .ok_or_else(|| FileStorageError::Storage("s3_secret_key missing".into()))?;

        let creds = Credentials::new(
            access.expose_secret(),
            secret.expose_secret(),
            None,
            None,
            "file-storage-config",
        );

        let region = Region::new(region);
        let config = aws_config::from_env()
            .region(region)
            .credentials_provider(creds)
            .load()
            .await;

        // Override endpoint for S3-compatible providers (e.g. MinIO).
        // aws-sdk-s3 v1 exposes this as `endpoint_url`.
        let mut conf = aws_sdk_s3::config::Builder::from(&config);
        conf = conf.endpoint_url(endpoint);

        let client = Client::from_conf(conf.build());

        Ok(Self { client, bucket })
    }

    fn key_for(&self, rec: &FileRecord) -> String {
        format!("{}/{}/{}", rec.tenant_id, rec.region, rec.storage_path)
    }
}

#[async_trait]
impl StorageBackend for S3StorageBackend {
    async fn put(&self, rec: &FileRecord, data: Bytes) -> FileStorageResult<()> {
        let key = self.key_for(rec);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send()
            .await
            .map_err(|e| FileStorageError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, rec: &FileRecord) -> FileStorageResult<Bytes> {
        let key = self.key_for(rec);
        let out = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| FileStorageError::Storage(e.to_string()))?;

        let data = out
            .body
            .collect()
            .await
            .map_err(|e| FileStorageError::Storage(e.to_string()))?;
        Ok(data.into_bytes())
    }

    async fn delete(&self, rec: &FileRecord) -> FileStorageResult<()> {
        let key = self.key_for(rec);
        let _ = self
            .client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;
        Ok(())
    }
}
