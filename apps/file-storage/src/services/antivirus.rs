#[cfg(feature = "antivirus")]
use crate::error::FileStorageError;
use crate::error::FileStorageResult;
use bytes::Bytes;

/// Antivirus scanning.
///
/// - When feature `antivirus` is enabled, this function MUST perform a real scan.
/// - When the feature is disabled, scanning is a no-op and always succeeds.
/// - If scanning fails (IO/timeout/provider error) the service should fail-closed
///   by returning an error.
#[cfg(not(feature = "antivirus"))]
pub async fn scan_bytes(_data: &Bytes) -> FileStorageResult<()> {
    Ok(())
}

#[cfg(feature = "antivirus")]
pub async fn scan_bytes(_data: &Bytes) -> FileStorageResult<()> {
    // NOTE(v5.2): antivirus scanning is **fail-closed** when enabled.
    //
    // Future(v2027): integrate a real scanner (e.g. ClamAV daemon, ICAP, or a managed provider)
    // behind this feature.
    Err(FileStorageError::Storage(
        "antivirus scanning enabled but not configured".to_string(),
    ))
}
