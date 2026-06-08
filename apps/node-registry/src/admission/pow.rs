//! PoW admission helper (registration throttle).
//!
//! Phase 46 wires this into the registry admission flow.
//!
//! Design goals (MVP):
//! - Make registry spam expensive (anti-sybil / anti-abuse).
//! - Keep verification cheap server-side.
//! - Allow tuning via difficulty bits.
//!
//! Rule:
//!   hash = SHA256(nonce || solution_bytes)
//!   accept iff hash has at least `difficulty_bits` leading zero bits.

use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn generate_nonce() -> [u8; 32] {
    let a = Uuid::new_v4().into_bytes();
    let b = Uuid::new_v4().into_bytes();
    let mut out = [0u8; 32];
    out[..16].copy_from_slice(&a);
    out[16..].copy_from_slice(&b);
    out
}

pub fn verify_pow(nonce: &[u8; 32], difficulty_bits: u8, solution: &[u8]) -> bool {
    if difficulty_bits == 0 {
        return true;
    }
    let mut hasher = Sha256::new();
    hasher.update(nonce);
    hasher.update(solution);
    let out = hasher.finalize();

    let mut zeros: u32 = 0;
    for b in out {
        if b == 0 {
            zeros += 8;
        } else {
            zeros += b.leading_zeros();
            break;
        }
        if zeros >= difficulty_bits as u32 {
            return true;
        }
    }
    zeros >= difficulty_bits as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow_verify_accepts_easy() {
        let nonce = [0u8; 32];
        // difficulty 0 always ok
        assert!(verify_pow(&nonce, 0, b"anything"));
    }

    #[test]
    fn pow_verify_rejects_invalid_high_difficulty() {
        let nonce = [0u8; 32];
        // extremely high should reject for random small solutions
        assert!(!verify_pow(&nonce, 64, b"tiny"));
    }
}
