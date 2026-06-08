use crate::config::FileStorageConfig;
use crate::error::{FileStorageError, FileStorageResult};
use bytes::Bytes;

/// Simple storage policy engine mapping request → validation rules.
pub struct StoragePolicy<'a> {
    cfg: &'a FileStorageConfig,
}

impl<'a> StoragePolicy<'a> {
    pub fn new(cfg: &'a FileStorageConfig) -> Self {
        Self { cfg }
    }

    pub fn validate_size(&self, size: u64) -> FileStorageResult<()> {
        if size > self.cfg.max_file_size_bytes {
            return Err(FileStorageError::FileTooLarge(size));
        }
        Ok(())
    }

    pub fn validate_mime(&self, mime: &str) -> FileStorageResult<()> {
        if self.cfg.allowed_mime_prefixes.is_empty() {
            return Ok(());
        }
        if self
            .cfg
            .allowed_mime_prefixes
            .iter()
            .any(|p| mime.starts_with(p))
        {
            Ok(())
        } else {
            Err(FileStorageError::MimeNotAllowed(mime.to_string()))
        }
    }

    pub fn should_scan(&self, _body: &Bytes) -> bool {
        self.cfg.antivirus_enabled
    }
}
