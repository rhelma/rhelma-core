#![forbid(unsafe_code)]

//! Attestation evidence verification helpers.
//!
//! Node registration is authenticated by the node's Ed25519 public key.
//! Attestation evidence is additional metadata that may be **format validated**
//! (always) and optionally **verified** (policy-controlled).

use std::process::{Command, Stdio};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

use crate::config::NodeRegistryPolicy;
use crate::error::RegistryError;
use crate::models::NodeAttestationV1;

/// Result of attestation processing.
#[derive(Debug, Clone)]
pub(crate) struct AttestationDecision {
    /// Whether the node should be treated as "attested" for discovery filters.
    pub(crate) attested: bool,
    /// Whether the evidence was verified (cryptographically or via external verifier).
    #[allow(dead_code)]
    pub(crate) verified: bool,
}

/// Verify and decide whether a node is considered "attested".
///
/// This function always performs **basic format validation** and then optionally
/// performs verification depending on policy.
pub(crate) async fn verify_attestation(
    policy: &NodeRegistryPolicy,
    node_id_hex: &str,
    node_pubkey_hex: &str,
    attestation: &NodeAttestationV1,
) -> Result<AttestationDecision, RegistryError> {
    let kind = attestation.kind.trim().to_lowercase();

    if kind == "none" {
        return Ok(AttestationDecision {
            attested: false,
            verified: true,
        });
    }

    // Evidence must be present for non-none kinds when policy requires it.
    let evidence = attestation.evidence.as_deref();
    if policy.require_attestation_evidence && evidence.map(|e| e.trim().is_empty()).unwrap_or(true)
    {
        return Err(RegistryError::bad_request(
            "attestation evidence required by policy",
        ));
    }

    match kind.as_str() {
        "software" => verify_software(policy, node_id_hex, node_pubkey_hex, evidence).await,
        "tpm" | "tpm2" => verify_hardware(policy, "tpm", node_id_hex, evidence).await,
        "sgx" => verify_hardware(policy, "sgx", node_id_hex, evidence).await,
        // Other hardware-ish kinds are accepted via the same verification hook.
        "sev-snp" | "sev_snp" | "snp" => {
            verify_hardware(policy, "sev-snp", node_id_hex, evidence).await
        }
        // Unknown kinds are allowed as long as evidence is not required, or can be verified.
        other => verify_unknown(policy, other, node_id_hex, evidence).await,
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SoftwareEvidenceEnvelope {
    /// SHA-256 of the node binary / image / measurement.
    artifact_hash: String,

    /// Optional signer public key (hex). If absent, the node's key is used.
    #[serde(default)]
    signer_public_key_hex: Option<String>,

    /// Optional Ed25519 signature (hex, 64 bytes).
    #[serde(default)]
    signature_hex: Option<String>,

    /// Optional Ed25519 signature (base64, 64 bytes).
    #[serde(default)]
    signature_b64: Option<String>,
}

async fn verify_software(
    policy: &NodeRegistryPolicy,
    node_id_hex: &str,
    node_pubkey_hex: &str,
    evidence: Option<&str>,
) -> Result<AttestationDecision, RegistryError> {
    let Some(evidence) = evidence else {
        // If evidence is absent and not required, treat as not-attested.
        return Ok(AttestationDecision {
            attested: false,
            verified: false,
        });
    };

    let trimmed = evidence.trim();
    if trimmed.is_empty() {
        return Ok(AttestationDecision {
            attested: false,
            verified: false,
        });
    }

    // Accept either a raw 32-byte hash hex string, or a JSON envelope.
    let (artifact_hash, signer_pub_hex, sig_hex, sig_b64) = if trimmed.starts_with('{') {
        let env: SoftwareEvidenceEnvelope = serde_json::from_str(trimmed).map_err(|e| {
            RegistryError::bad_request(format!("invalid software evidence json: {e}"))
        })?;

        (
            env.artifact_hash,
            env.signer_public_key_hex,
            env.signature_hex,
            env.signature_b64,
        )
    } else {
        (trimmed.to_string(), None, None, None)
    };

    validate_hex_len(&artifact_hash, 64, "software artifact_hash")?;

    // If an external verifier is configured, prefer it.
    let verified = if let Some(cmd) = policy.attestation_verify_cmd.as_deref() {
        verify_with_cmd(cmd, "software", node_id_hex, trimmed, None, None).await?
    } else {
        // Internal verification (best-effort): verify signature over a stable payload.
        if sig_hex.is_none() && sig_b64.is_none() {
            false
        } else {
            let pk_hex = signer_pub_hex.as_deref().unwrap_or(node_pubkey_hex);
            verify_software_signature(pk_hex, node_id_hex, &artifact_hash, sig_hex, sig_b64)?
        }
    };

    if policy.require_attestation_verification && !verified {
        return Err(RegistryError::bad_request(
            "software attestation could not be verified",
        ));
    }

    Ok(AttestationDecision {
        attested: true,
        verified,
    })
}

async fn verify_hardware(
    policy: &NodeRegistryPolicy,
    kind: &str,
    node_id_hex: &str,
    evidence: Option<&str>,
) -> Result<AttestationDecision, RegistryError> {
    let Some(evidence) = evidence else {
        return Ok(AttestationDecision {
            attested: false,
            verified: false,
        });
    };
    let trimmed = evidence.trim();
    if trimmed.is_empty() {
        return Ok(AttestationDecision {
            attested: false,
            verified: false,
        });
    }

    // Format-only validation: allow either hex or base64; require some minimum length.
    // - TPM quotes are typically larger; SGX quotes vary.
    let min_len = if kind == "tpm" { 96 } else { 64 };
    if trimmed.len() < min_len {
        return Err(RegistryError::bad_request(format!(
            "{kind} evidence too short (min {min_len})"
        )));
    }

    // Verification: only via external verifier at this layer.
    let verified = if let Some(cmd) = policy.attestation_verify_cmd.as_deref() {
        verify_with_cmd(
            cmd,
            kind,
            node_id_hex,
            trimmed,
            policy.tpm_pcr_allowlist_json.clone(),
            policy.sgx_mrenclave_allowlist.clone(),
        )
        .await?
    } else {
        false
    };

    if policy.require_attestation_verification && !verified {
        return Err(RegistryError::bad_request(format!(
            "{kind} attestation verifier not configured or verification failed"
        )));
    }

    Ok(AttestationDecision {
        attested: true,
        verified,
    })
}

async fn verify_unknown(
    policy: &NodeRegistryPolicy,
    kind: &str,
    node_id_hex: &str,
    evidence: Option<&str>,
) -> Result<AttestationDecision, RegistryError> {
    let Some(evidence) = evidence else {
        return Ok(AttestationDecision {
            attested: false,
            verified: false,
        });
    };
    let trimmed = evidence.trim();
    if trimmed.is_empty() {
        return Ok(AttestationDecision {
            attested: false,
            verified: false,
        });
    }

    let verified = if let Some(cmd) = policy.attestation_verify_cmd.as_deref() {
        verify_with_cmd(
            cmd,
            kind,
            node_id_hex,
            trimmed,
            policy.tpm_pcr_allowlist_json.clone(),
            policy.sgx_mrenclave_allowlist.clone(),
        )
        .await?
    } else {
        false
    };

    if policy.require_attestation_verification && !verified {
        return Err(RegistryError::bad_request(format!(
            "unknown attestation kind '{kind}' cannot be verified"
        )));
    }

    Ok(AttestationDecision {
        attested: true,
        verified,
    })
}

fn verify_software_signature(
    verifying_pub_hex: &str,
    node_id_hex: &str,
    artifact_hash_hex: &str,
    sig_hex: Option<String>,
    sig_b64: Option<String>,
) -> Result<bool, RegistryError> {
    let pk_bytes = hex::decode(verifying_pub_hex)
        .map_err(|_| RegistryError::bad_request("invalid signer public key hex"))?;
    if pk_bytes.len() != 32 {
        return Err(RegistryError::bad_request(
            "invalid signer public key length",
        ));
    }
    let vk = VerifyingKey::from_bytes(
        pk_bytes
            .as_slice()
            .try_into()
            .map_err(|_| RegistryError::bad_request("invalid signer public key bytes"))?,
    )
    .map_err(|_| RegistryError::bad_request("invalid signer public key"))?;

    let sig_bytes = if let Some(h) = sig_hex {
        hex::decode(h).map_err(|_| RegistryError::bad_request("invalid signature_hex"))?
    } else if let Some(b) = sig_b64 {
        B64.decode(b)
            .map_err(|_| RegistryError::bad_request("invalid signature_b64"))?
    } else {
        return Ok(false);
    };

    if sig_bytes.len() != 64 {
        return Err(RegistryError::bad_request("invalid signature length"));
    }

    let sig = Signature::from_bytes(
        sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| RegistryError::bad_request("invalid signature bytes"))?,
    );

    // Stable payload: bind evidence to node id.
    let payload = format!("rhelma6:software_attestation:v1|{node_id_hex}|{artifact_hash_hex}");
    Ok(vk.verify_strict(payload.as_bytes(), &sig).is_ok())
}

fn validate_hex_len(s: &str, expected_len: usize, label: &str) -> Result<(), RegistryError> {
    if s.len() != expected_len {
        return Err(RegistryError::bad_request(format!(
            "invalid {label} length (expected {expected_len})"
        )));
    }
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(RegistryError::bad_request(format!("invalid {label} hex")));
    }
    Ok(())
}

async fn verify_with_cmd(
    cmd: &str,
    kind: &str,
    node_id_hex: &str,
    evidence: &str,
    tpm_pcr_allowlist_json: Option<String>,
    sgx_mrenclave_allowlist: Option<String>,
) -> Result<bool, RegistryError> {
    let cmd = cmd.to_string();
    let kind = kind.to_string();
    let node_id_hex = node_id_hex.to_string();
    let evidence = evidence.to_string();

    let ok = tokio::task::spawn_blocking(move || {
        let mut command = Command::new(cmd);
        command
            .arg("verify")
            .arg(&kind)
            .arg(&node_id_hex)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if kind == "tpm" {
            if let Some(j) = tpm_pcr_allowlist_json.as_deref() {
                command.env("RHELMA_ATTEST_VERIFIER__TPM_PCR_ALLOWLIST_JSON", j);
            }
        }
        if kind == "sgx" {
            if let Some(v) = sgx_mrenclave_allowlist.as_deref() {
                command.env("RHELMA_ATTEST_VERIFIER__SGX_MRENCLAVE_ALLOWLIST", v);
            }
        }

        let child = command.spawn();

        let Ok(mut child) = child else {
            return false;
        };

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(evidence.as_bytes());
        }

        match child.wait() {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    })
    .await
    .map_err(|e| RegistryError::internal(format!("attestation verify task failed: {e}")))?;

    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;
    use ed25519_dalek::SigningKey;

    fn policy(require: bool) -> NodeRegistryPolicy {
        NodeRegistryPolicy {
            default_min_reputation: 0,
            default_require_attested: false,
            require_manifest_signature: false,
            require_attestation_evidence: true,
            require_attestation_verification: require,
            attestation_verify_cmd: None,
            tpm_pcr_allowlist_json: None,
            sgx_mrenclave_allowlist: None,
            promote_threshold: 5,
            suspend_threshold: -20,
            suspend_seconds: 600,
            delta_ok: 1,
            delta_fail: -5,
            delta_timeout: -3,
            delta_bad_result: -7,
            min_reputation: -100,
            max_reputation: 100,
        }
    }

    #[tokio::test]
    async fn software_attestation_verifies_signature_when_present() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = sk.verifying_key();
        let node_id = hex::encode(pk.to_bytes());
        let artifact_hash = "a".repeat(64);
        let payload = format!("rhelma6:software_attestation:v1|{node_id}|{artifact_hash}");
        let sig = sk.sign(payload.as_bytes());

        let att = NodeAttestationV1 {
            kind: "software".to_string(),
            evidence: Some(
                serde_json::json!({
                    "artifact_hash": artifact_hash,
                    "signature_hex": hex::encode(sig.to_bytes())
                })
                .to_string(),
            ),
            verifier: None,
        };

        let d = verify_attestation(&policy(true), &node_id, &node_id, &att)
            .await
            .expect("verify ok");
        assert!(d.attested);
        assert!(d.verified);
    }

    #[tokio::test]
    async fn software_attestation_rejects_when_verification_required_and_missing_signature() {
        let sk = SigningKey::from_bytes(&[9u8; 32]);
        let pk = sk.verifying_key();
        let node_id = hex::encode(pk.to_bytes());
        let att = NodeAttestationV1 {
            kind: "software".to_string(),
            evidence: Some("a".repeat(64)),
            verifier: None,
        };

        let err = verify_attestation(&policy(true), &node_id, &node_id, &att)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("could not be verified"));
    }
}
