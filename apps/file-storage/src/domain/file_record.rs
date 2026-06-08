use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FileId(pub Uuid);

impl FileId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for FileId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileStatus {
    /// Variant `Active`.
    Active,
    /// Variant `Deleted`.
    Deleted,
    /// Variant `Archived`.
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackendKind {
    /// Variant `LocalFs`.
    LocalFs,
    /// Variant `S3`.
    S3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// Field `id`.
    pub id: FileId,
    /// Field `tenant_id`.
    pub tenant_id: String,
    /// Field `region`.
    pub region: String,
    /// Field `original_name`.
    pub original_name: String,
    /// Field `content_type`.
    pub content_type: String,
    /// Field `size_bytes`.
    pub size_bytes: i64,
    /// Field `checksum`.
    pub checksum: String,
    /// Field `storage_backend`.
    pub storage_backend: StorageBackendKind,
    /// Field `storage_path`.
    pub storage_path: String,
    /// Field `status`.
    pub status: FileStatus,
    /// Field `created_at`.
    pub created_at: DateTime<Utc>,
    /// Field `created_by`.
    pub created_by: Option<String>,
    /// Field `deleted_at`.
    pub deleted_at: Option<DateTime<Utc>>,
}
