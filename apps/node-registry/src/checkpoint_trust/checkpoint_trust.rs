use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CheckpointHead {
    /// Field `domain`.
    pub domain: String,
    /// Field `kind`.
    pub kind: String,
    /// Field `root_hex`.
    pub root_hex: String,
    /// Field `ts_unix`.
    pub ts_unix: i64,
    /// Field `signer_peer_id`.
    pub signer_peer_id: String,
    /// Field `checkpoint_id`.
    pub checkpoint_id: String,
}

/// Small helper for node-registry to query a checkpoint head from gossip-discovery.
/// This is intentionally optional; wire it into discover scoring only if enabled.
pub async fn fetch_checkpoint_head(url: &str, domain: &str, kind: &str) -> Result<CheckpointHead, String> {
    let full = format!("{}/v1/checkpoints/head?domain={}&kind={}", url.trim_end_matches('/'), domain, kind);
    let resp = reqwest::get(full).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("checkpoint head status {}", resp.status()));
    }
    resp.json::<CheckpointHead>().await.map_err(|e| e.to_string())
}
