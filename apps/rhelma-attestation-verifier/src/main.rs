#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::io::{Read, Write};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
struct SoftwareEvidenceEnvelope {
    /// SHA-256 of the node binary / image.
    artifact_hash: String,
    #[serde(default)]
    signer_public_key_hex: Option<String>,
    #[serde(default)]
    signature_hex: Option<String>,
    #[serde(default)]
    signature_b64: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TpmEvidenceEnvelope {
    /// Optional quote (base64).
    #[serde(default)]
    quote_b64: Option<String>,

    /// Optional quote (hex).
    #[serde(default)]
    quote_hex: Option<String>,

    /// PCR selection values (already hashed), map: PCR index -> sha256 hex.
    #[serde(default)]
    pcrs_sha256_hex: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct SgxEvidenceEnvelope {
    /// MRENCLAVE (hex).
    #[serde(default)]
    mrenclave_hex: Option<String>,

    /// Optional quote (base64).
    #[serde(default)]
    quote_b64: Option<String>,

    /// Optional quote (hex).
    #[serde(default)]
    quote_hex: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 || args[1] != "verify" {
        eprintln!(
            "usage: {} verify <kind> <node_id_hex>",
            args.first()
                .map(String::as_str)
                .unwrap_or("rhelma-attestation-verifier")
        );
        std::process::exit(2);
    }

    let kind = args[2].trim().to_lowercase();
    let node_id_hex = args[3].trim().to_string();

    let mut evidence = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut evidence) {
        eprintln!("failed to read evidence: {e}");
        std::process::exit(2);
    }

    let evidence = evidence.trim();
    if evidence.is_empty() {
        eprintln!("empty evidence");
        std::process::exit(2);
    }

    let strict = env_bool("RHELMA_ATTEST_VERIFIER__STRICT", false);

    let ok = match kind.as_str() {
        "software" => verify_software(&node_id_hex, evidence),
        "tpm" | "tpm2" => verify_tpm(evidence, strict),
        "sgx" => verify_sgx(evidence, strict),
        other => verify_hardware_min(other, evidence),
    };

    if ok {
        let mut h = Sha256::new();
        h.update(evidence.as_bytes());
        let digest = hex::encode(h.finalize());
        let out = serde_json::json!({
            "kind": kind,
            "node_id_hex": node_id_hex,
            "evidence_sha256_hex": digest,
            "verified": true
        });
        let _ = writeln!(std::io::stdout(), "{}", out);
        std::process::exit(0);
    }

    eprintln!("verification failed");
    std::process::exit(2);
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| {
            let v = v.trim();
            !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false"))
        })
        .unwrap_or(default)
}

fn verify_hardware_min(kind: &str, evidence: &str) -> bool {
    // MVP: perform basic sanity checks only.
    // Real deployments should replace this binary with an environment-specific verifier.
    let min_len = if kind == "tpm" { 96 } else { 64 };
    evidence.len() >= min_len
}

fn verify_tpm(evidence: &str, strict: bool) -> bool {
    // Optional allowlist enforcement.
    let allowlist = match std::env::var("RHELMA_ATTEST_VERIFIER__TPM_PCR_ALLOWLIST_JSON") {
        Ok(v) if !v.trim().is_empty() => {
            match serde_json::from_str::<HashMap<String, String>>(&v) {
                Ok(m) => m,
                Err(_) => {
                    if strict {
                        return false;
                    }
                    HashMap::new()
                }
            }
        }
        _ => HashMap::new(),
    };

    if !evidence.starts_with('{') {
        // Non-JSON evidence: only allow if we are not enforcing allowlist.
        if !allowlist.is_empty() {
            return false;
        }
        return verify_hardware_min("tpm", evidence);
    }

    let env: TpmEvidenceEnvelope = match serde_json::from_str(evidence) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Basic sanity: quote present OR evidence length (json) can still be validated via PCRs.
    if let Some(q) = env.quote_hex.as_deref() {
        if !is_hex_min(q, 96) {
            return false;
        }
    }
    if let Some(q) = env.quote_b64.as_deref() {
        if B64.decode(q).ok().map(|b| b.len() < 48).unwrap_or(true) {
            return false;
        }
    }

    if allowlist.is_empty() {
        return true;
    }

    let Some(pcrs) = env.pcrs_sha256_hex.as_ref() else {
        return false;
    };

    for (pcr_idx, expected_hex) in allowlist {
        let Some(actual) = pcrs.get(&pcr_idx) else {
            return false;
        };
        if !eq_hex(actual, &expected_hex) {
            return false;
        }
    }

    true
}

fn verify_sgx(evidence: &str, strict: bool) -> bool {
    // Optional allowlist enforcement.
    let allow = std::env::var("RHELMA_ATTEST_VERIFIER__SGX_MRENCLAVE_ALLOWLIST")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let allowlist = match allow {
        None => Vec::<String>::new(),
        Some(v) => {
            // Try JSON array first.
            if v.starts_with('[') {
                match serde_json::from_str::<Vec<String>>(&v) {
                    Ok(arr) => arr,
                    Err(_) => {
                        if strict {
                            return false;
                        }
                        Vec::new()
                    }
                }
            } else {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
        }
    };

    if !evidence.starts_with('{') {
        if !allowlist.is_empty() {
            return false;
        }
        return verify_hardware_min("sgx", evidence);
    }

    let env: SgxEvidenceEnvelope = match serde_json::from_str(evidence) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(q) = env.quote_hex.as_deref() {
        if !is_hex_min(q, 64) {
            return false;
        }
    }
    if let Some(q) = env.quote_b64.as_deref() {
        if B64.decode(q).ok().map(|b| b.len() < 32).unwrap_or(true) {
            return false;
        }
    }

    if allowlist.is_empty() {
        return true;
    }

    let Some(mrenclave) = env.mrenclave_hex.as_deref() else {
        return false;
    };
    if !is_hex_len(mrenclave, 64) {
        return false;
    }

    allowlist.iter().any(|a| eq_hex(a, mrenclave))
}

fn verify_software(node_id_hex: &str, evidence: &str) -> bool {
    // Accept raw 32-byte hash hex, or a JSON envelope.
    if evidence.starts_with('{') {
        let env: SoftwareEvidenceEnvelope = match serde_json::from_str(evidence) {
            Ok(v) => v,
            Err(_) => return false,
        };

        if !is_hex_len(&env.artifact_hash, 64) {
            return false;
        }

        // If signature is present, verify it over a stable payload.
        let Some(sig_bytes) =
            decode_sig(env.signature_hex.as_deref(), env.signature_b64.as_deref())
        else {
            return true;
        };

        let signer_pk_hex = env.signer_public_key_hex.as_deref().unwrap_or(node_id_hex);
        let Ok(pk_bytes) = hex::decode(signer_pk_hex) else {
            return false;
        };
        if pk_bytes.len() != 32 {
            return false;
        }

        let Ok(pk_arr) = <[u8; 32]>::try_from(pk_bytes.as_slice()) else {
            return false;
        };

        let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) else {
            return false;
        };

        if sig_bytes.len() != 64 {
            return false;
        }
        let Ok(sig_arr) = <[u8; 64]>::try_from(sig_bytes.as_slice()) else {
            return false;
        };
        let sig = Signature::from_bytes(&sig_arr);

        let payload = format!(
            "rhelma6:software_attestation:v1|{node_id_hex}|{}",
            env.artifact_hash
        );
        vk.verify_strict(payload.as_bytes(), &sig).is_ok()
    } else {
        is_hex_len(evidence, 64)
    }
}

fn is_hex_len(s: &str, expected: usize) -> bool {
    s.len() == expected && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_hex_min(s: &str, min_len: usize) -> bool {
    s.len() >= min_len && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn eq_hex(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn decode_sig(sig_hex: Option<&str>, sig_b64: Option<&str>) -> Option<Vec<u8>> {
    if let Some(h) = sig_hex {
        hex::decode(h).ok()
    } else if let Some(b) = sig_b64 {
        B64.decode(b).ok()
    } else {
        None
    }
}
