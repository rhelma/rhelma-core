use crate::domain::FileRecord;
use crate::error::{FileStorageError, FileStorageResult};
use crate::services::storage_backend::StorageBackend;
use async_trait::async_trait;
use bytes::Bytes;
use std::fs;
use std::path::PathBuf;
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
pub struct LocalFsStorageBackend {
    root: String,
}

impl LocalFsStorageBackend {
    pub fn new(root: &str) -> Self {
        fs::create_dir_all(root).ok();
        Self {
            root: root.to_string(),
        }
    }

    fn full_path(&self, rec: &FileRecord) -> PathBuf {
        let mut p = PathBuf::from(&self.root);
        p.push(&rec.tenant_id);
        p.push(&rec.region);
        p.push(&rec.storage_path);
        p
    }
}

#[async_trait]
impl StorageBackend for LocalFsStorageBackend {
    async fn put(&self, rec: &FileRecord, data: Bytes) -> FileStorageResult<()> {
        let path = self.full_path(rec);
        if let Some(parent) = path.parent() {
            tokio_fs::create_dir_all(parent)
                .await
                .map_err(|e| FileStorageError::Storage(e.to_string()))?;
        }
        let mut file = tokio_fs::File::create(&path)
            .await
            .map_err(|e| FileStorageError::Storage(e.to_string()))?;
        file.write_all(&data)
            .await
            .map_err(|e| FileStorageError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, rec: &FileRecord) -> FileStorageResult<Bytes> {
        let path = self.full_path(rec);
        let data = tokio_fs::read(&path)
            .await
            .map_err(|e| FileStorageError::Storage(e.to_string()))?;
        Ok(Bytes::from(data))
    }

    async fn delete(&self, rec: &FileRecord) -> FileStorageResult<()> {
        let path = self.full_path(rec);
        let _ = tokio_fs::remove_file(&path).await; // ignore missing file
        Ok(())
    }
}
