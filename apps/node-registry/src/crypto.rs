#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::error::RegistryError;

pub fn verify_signature_hex(
    message: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<bool, RegistryError> {
    let sig_bytes = hex::decode(signature_hex.trim())
        .map_err(|e| RegistryError::bad_request(format!("invalid signature_hex: {e}")))?;
    if sig_bytes.len() != 64 {
        return Err(RegistryError::bad_request(
            "invalid signature length (expected 64 bytes)",
        ));
    }

    let pk_bytes = hex::decode(public_key_hex.trim())
        .map_err(|e| RegistryError::bad_request(format!("invalid public_key_hex: {e}")))?;
    if pk_bytes.len() != 32 {
        return Err(RegistryError::bad_request(
            "invalid public key length (expected 32 bytes)",
        ));
    }

    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&pk_bytes);
    let vk = VerifyingKey::from_bytes(&pk_arr)
        .map_err(|e| RegistryError::bad_request(format!("invalid verifying key: {e}")))?;

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let sig = Signature::from_bytes(&sig_arr);

    Ok(vk.verify(message, &sig).is_ok())
}
