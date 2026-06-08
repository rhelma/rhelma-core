#![forbid(unsafe_code)]

//! Rhelma attestation primitives.
//!
//! This crate provides a small, dependency-minimal set of types for representing
//! node attestation evidence and verification results.
//!
//! Hardware-backed attestation backends (TPM/SGX/SEV-SNP) are intentionally
//! represented as **stubs** behind features, so the core workspace can build on
//! all platforms without pulling heavyweight system dependencies.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttestationLevel {
    /// Software-only attestation (key possession + binary hash / config)
    L1,
    /// Hardware-rooted attestation (e.g. TPM quote)
    L2,
    /// Confidential compute enclaves (e.g. SGX quote)
    L3,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttestationEvidence {
    Software(SoftwareEvidence),
    Tpm(TpmEvidence),
    Sgx(SgxEvidence),
    /// Fallback for custom / vendor-specific attestation kinds.
    Unknown {
        kind: String,
        blob: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoftwareEvidence {
    /// Attested binary / container hash (sha256 hex or similar)
    pub artifact_hash: Option<String>,
    /// Optional signature over the artifact hash + nonce
    pub signature: Option<String>,
    /// Optional signer key fingerprint / id
    pub signer: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TpmEvidence {
    /// Base64 (or hex) TPM quote
    pub quote: String,
    /// Base64 (or hex) PCR selection + digest
    pub pcrs: Option<String>,
    /// Optional metadata for the verifier
    pub meta: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SgxEvidence {
    /// Base64 (or hex) SGX quote
    pub quote: String,
    /// Optional expected measurement (MRENCLAVE)
    pub mrenclave: Option<String>,
    /// Optional metadata for the verifier
    pub meta: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttestationReport {
    pub level: AttestationLevel,
    /// Human-readable summary (safe for logs)
    pub summary: String,
    /// Verifier claims (e.g. PCRs, MRENCLAVE, firmware versions)
    pub claims: Option<serde_json::Value>,
}

#[derive(thiserror::Error, Debug)]
pub enum AttestError {
    #[error("unsupported attestation kind: {0}")]
    UnsupportedKind(String),
    #[error("attestation backend not available in this build")]
    NotAvailable,
    #[error("invalid evidence: {0}")]
    InvalidEvidence(String),
    #[error("verification failed: {0}")]
    VerificationFailed(String),
}

/// Trait for hardware-backed evidence generation and verification.
///
/// Note: methods are sync by design to keep this crate minimal. Applications can
/// wrap implementations in async tasks if needed.
pub trait HardwareAttestor: Send + Sync {
    fn level(&self) -> AttestationLevel;

    fn generate_evidence(&self, nonce: &[u8]) -> Result<AttestationEvidence, AttestError>;

    fn verify_evidence(
        &self,
        evidence: &AttestationEvidence,
    ) -> Result<AttestationReport, AttestError>;
}

/// Parse an evidence payload (usually transported as a string) into a typed structure.
///
/// This is a best-effort helper intended for API boundary validation.
pub fn parse_evidence(kind: &str, blob: &str) -> Result<AttestationEvidence, AttestError> {
    let kind = kind.trim().to_ascii_lowercase();
    let blob = blob.trim();

    if kind.is_empty() {
        return Err(AttestError::InvalidEvidence("empty kind".to_string()));
    }

    match kind.as_str() {
        "none" => Err(AttestError::UnsupportedKind("none".to_string())),
        "software" => Ok(AttestationEvidence::Software(SoftwareEvidence {
            artifact_hash: Some(blob.to_string()),
            signature: None,
            signer: None,
        })),
        "tpm" | "tpm2" => Ok(AttestationEvidence::Tpm(TpmEvidence {
            quote: blob.to_string(),
            pcrs: None,
            meta: None,
        })),
        "sgx" => Ok(AttestationEvidence::Sgx(SgxEvidence {
            quote: blob.to_string(),
            mrenclave: None,
            meta: None,
        })),
        other => Ok(AttestationEvidence::Unknown {
            kind: other.to_string(),
            blob: blob.to_string(),
        }),
    }
}

/// Placeholder TPM backend.
#[cfg(feature = "tpm")]
pub mod tpm {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct TpmAttestor;

    impl TpmAttestor {
        pub fn new() -> Result<Self, AttestError> {
            Err(AttestError::NotAvailable)
        }
    }

    impl HardwareAttestor for TpmAttestor {
        fn level(&self) -> AttestationLevel {
            AttestationLevel::L2
        }

        fn generate_evidence(&self, _nonce: &[u8]) -> Result<AttestationEvidence, AttestError> {
            Err(AttestError::NotAvailable)
        }

        fn verify_evidence(
            &self,
            _evidence: &AttestationEvidence,
        ) -> Result<AttestationReport, AttestError> {
            Err(AttestError::NotAvailable)
        }
    }
}

/// Placeholder SGX backend.
#[cfg(feature = "sgx")]
pub mod sgx {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct SgxAttestor;

    impl SgxAttestor {
        pub fn new() -> Result<Self, AttestError> {
            Err(AttestError::NotAvailable)
        }
    }

    impl HardwareAttestor for SgxAttestor {
        fn level(&self) -> AttestationLevel {
            AttestationLevel::L3
        }

        fn generate_evidence(&self, _nonce: &[u8]) -> Result<AttestationEvidence, AttestError> {
            Err(AttestError::NotAvailable)
        }

        fn verify_evidence(
            &self,
            _evidence: &AttestationEvidence,
        ) -> Result<AttestationReport, AttestError> {
            Err(AttestError::NotAvailable)
        }
    }
}
