#![forbid(unsafe_code)]

use std::time::Duration;

use axum::{extract::Extension, http::StatusCode, response::IntoResponse, Json};

use serde::Serialize;

use crate::config::{FileStorageConfig, StorageProviderKind};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Field `status`.
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct HealthDepsResponse {
    /// Field `status`.
    pub status: &'static str,
    /// Field `db`.
    pub db: DepStatus,
    /// Field `storage`.
    pub storage: DepStatus,
}

#[derive(Debug, Serialize)]
pub struct DepStatus {
    /// Field `ok`.
    pub ok: bool,
    /// Field `detail`.
    pub detail: Option<String>,
}

/// Liveness probe.
///
/// Always returns 200 if the process is running.
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}

/// Dependency/readiness probe.
///
/// Returns 200 only if DB and the configured storage backend are reachable.
///
/// Notes:
/// - Uses short timeouts to avoid tying up worker threads.
/// - Error details are sanitized (no credentials / URLs).
pub async fn health_deps(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Extension(cfg): Extension<std::sync::Arc<FileStorageConfig>>,
) -> impl IntoResponse {
    let db = check_db(&state).await;
    let storage = check_storage(&cfg).await;

    let ok = db.ok && storage.ok;

    let status = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let body = HealthDepsResponse {
        status: if ok { "ok" } else { "degraded" },
        db,
        storage,
    };

    (status, Json(body))
}

async fn check_db(state: &AppState) -> DepStatus {
    let fut = async {
        sqlx::query("SELECT 1")
            .execute(&state.pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    };

    match tokio::time::timeout(Duration::from_millis(250), fut).await {
        Ok(Ok(())) => DepStatus {
            ok: true,
            detail: None,
        },
        Ok(Err(e)) => DepStatus {
            ok: false,
            detail: Some(format!("db error: {e}")),
        },
        Err(_) => DepStatus {
            ok: false,
            detail: Some("db timeout".into()),
        },
    }
}

async fn check_storage(cfg: &FileStorageConfig) -> DepStatus {
    match cfg.default_provider {
        StorageProviderKind::LocalFs => check_local_fs(cfg).await,
        StorageProviderKind::S3 => check_s3(cfg).await,
    }
}

async fn check_local_fs(cfg: &FileStorageConfig) -> DepStatus {
    use tokio::fs;
    use tokio::io::AsyncWriteExt;

    let root = cfg.local_root.clone();

    let fut = async {
        fs::create_dir_all(&root)
            .await
            .map_err(|e| format!("localfs create_dir_all failed: {e}"))?;

        let name = format!(
            "{}/.rhelma_healthcheck_{}",
            root.trim_end_matches('/'),
            uuid::Uuid::now_v7()
        );
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&name)
            .await
            .map_err(|e| format!("localfs create failed: {e}"))?;

        f.write_all(b"ok")
            .await
            .map_err(|e| format!("localfs write failed: {e}"))?;

        let _ = fs::remove_file(&name).await;
        Ok::<(), String>(())
    };

    match tokio::time::timeout(Duration::from_millis(300), fut).await {
        Ok(Ok(())) => DepStatus {
            ok: true,
            detail: None,
        },
        Ok(Err(e)) => DepStatus {
            ok: false,
            detail: Some(e),
        },
        Err(_) => DepStatus {
            ok: false,
            detail: Some("localfs timeout".into()),
        },
    }
}

async fn check_s3(cfg: &FileStorageConfig) -> DepStatus {
    use aws_credential_types::Credentials;
    use aws_sdk_s3::config::Region;
    use aws_sdk_s3::Client;
    use secrecy::ExposeSecret;

    let fut = async {
        let endpoint = cfg
            .s3_endpoint
            .clone()
            .ok_or_else(|| "s3_endpoint missing".to_string())?;
        let bucket = cfg
            .s3_bucket
            .clone()
            .ok_or_else(|| "s3_bucket missing".to_string())?;
        let region = cfg.s3_region.clone().unwrap_or_else(|| "us-east-1".into());

        let access = cfg
            .s3_access_key
            .as_ref()
            .ok_or_else(|| "s3_access_key missing".to_string())?;
        let secret = cfg
            .s3_secret_key
            .as_ref()
            .ok_or_else(|| "s3_secret_key missing".to_string())?;

        let creds = Credentials::new(
            access.expose_secret(),
            secret.expose_secret(),
            None,
            None,
            "file-storage-health",
        );

        let region = Region::new(region);
        let config = aws_config::from_env()
            .region(region)
            .credentials_provider(creds)
            .load()
            .await;

        let mut conf = aws_sdk_s3::config::Builder::from(&config);
        conf = conf.endpoint_url(endpoint);
        let client = Client::from_conf(conf.build());

        // Prefer a cheap call.
        client
            .head_bucket()
            .bucket(bucket)
            .send()
            .await
            .map_err(|e| format!("s3 head_bucket failed: {e}"))?;

        Ok::<(), String>(())
    };

    match tokio::time::timeout(Duration::from_millis(700), fut).await {
        Ok(Ok(())) => DepStatus {
            ok: true,
            detail: None,
        },
        Ok(Err(e)) => DepStatus {
            ok: false,
            detail: Some(e),
        },
        Err(_) => DepStatus {
            ok: false,
            detail: Some("s3 timeout".into()),
        },
    }
}
