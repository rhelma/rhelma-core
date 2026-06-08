#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;

use crate::config::{FileStorageConfig, StorageProviderKind};
use crate::domain::{FileRecord, StorageBackendKind};
use crate::error::FileStorageResult;
use crate::services::{local_fs::LocalFsStorageBackend, s3::S3StorageBackend};

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put(&self, rec: &FileRecord, data: Bytes) -> FileStorageResult<()>;
    async fn get(&self, rec: &FileRecord) -> FileStorageResult<Bytes>;
    async fn delete(&self, rec: &FileRecord) -> FileStorageResult<()>;
}

#[derive(Clone)]
pub struct StorageBackendManager {
    default: Arc<dyn StorageBackend>,
}

impl StorageBackendManager {
    /// Build backend manager from config.
    ///
    /// Notes:
    /// - Uses `async_trait`, so `Arc<dyn StorageBackend>` is object-safe.
    /// - `backend_for` currently returns the default backend (single-provider).
    pub async fn from_config(cfg: &FileStorageConfig) -> FileStorageResult<Self> {
        let backend: Arc<dyn StorageBackend> = match cfg.default_provider {
            StorageProviderKind::LocalFs => Arc::new(LocalFsStorageBackend::new(&cfg.local_root)),
            StorageProviderKind::S3 => Arc::new(S3StorageBackend::new(cfg).await?),
        };

        Ok(Self { default: backend })
    }

    /// Select backend for a given record.
    /// Today this is a single-provider deployment, so we always return `default`.
    pub fn backend_for(&self, _backend: StorageBackendKind) -> Arc<dyn StorageBackend> {
        self.default.clone()
    }
}
