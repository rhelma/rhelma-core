#![forbid(unsafe_code)]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use std::collections::BTreeMap;

use rhelma_core::governance::crypto::{hs256_sign, GovernanceKeySets, HS256_FPR_PREFIX};
use rhelma_core::governance::policy::{
    compute_bundle_hash, verify_policy_bundle_v1, PolicyBundleV1, PolicyBundleV1Class,
    PolicySignatureV1,
};
use serde_json::json;

fn hs256_fpr(kid: &str) -> String {
    format!("{}{}", HS256_FPR_PREFIX, kid)
}

#[test]
fn critical_bundle_requires_both_councils_and_timelock() {
    // Build key sets.
    let mut policy: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for kid in ["p1", "p2", "p3", "p4", "p5"] {
        policy.insert(hs256_fpr(kid), format!("secret-{kid}").into_bytes());
    }
    let mut security: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for kid in ["s1", "s2", "s3", "s4"] {
        security.insert(hs256_fpr(kid), format!("secret-{kid}").into_bytes());
    }

    let keys = GovernanceKeySets {
        policy_council: policy.clone(),
        security_council: security.clone(),
        ..GovernanceKeySets::default()
    };

    let created_at = Utc::now();
    let activate_not_before = created_at + Duration::seconds(86_400); // 24h

    let mut bundle = PolicyBundleV1 {
        bundle_id: "test-critical".to_string(),
        version: "1".to_string(),
        created_at,
        prev_bundle_hash: None,
        class: PolicyBundleV1Class::Critical,
        summary: "critical change".to_string(),
        policy: json!({"rules": [{"id": "noop"}]}),
        expires_at: None,
        rollback_plan: None,
        activate_not_before: Some(activate_not_before),
        signatures: vec![],
    };

    let bundle_hash = compute_bundle_hash(&bundle).expect("hash");
    let bundle_hash_bytes = URL_SAFE_NO_PAD
        .decode(bundle_hash.as_bytes())
        .expect("decode");

    // Add 4 policy signatures and 3 security signatures.
    for kid in ["p1", "p2", "p3", "p4"] {
        let fpr = hs256_fpr(kid);
        let secret = policy.get(&fpr).unwrap();
        let sig = URL_SAFE_NO_PAD.encode(hs256_sign(&bundle_hash_bytes, secret).unwrap());
        bundle
            .signatures
            .push(PolicySignatureV1 { key_fpr: fpr, sig });
    }
    for kid in ["s1", "s2", "s3"] {
        let fpr = hs256_fpr(kid);
        let secret = security.get(&fpr).unwrap();
        let sig = URL_SAFE_NO_PAD.encode(hs256_sign(&bundle_hash_bytes, secret).unwrap());
        bundle
            .signatures
            .push(PolicySignatureV1 { key_fpr: fpr, sig });
    }

    // Verify after timelock.
    let now = activate_not_before + Duration::seconds(10);
    let verified = verify_policy_bundle_v1(bundle, &keys, None, now).expect("verified");

    assert_eq!(verified.bundle.class, PolicyBundleV1Class::Critical);
    assert_eq!(verified.policy_quorum_required, Some(4));
    assert_eq!(verified.security_quorum_required, Some(3));
    assert!(verified.verified_signers.len() >= 7);
}

#[test]
fn critical_bundle_fails_without_security_quorum() {
    let mut policy: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for kid in ["p1", "p2", "p3", "p4", "p5"] {
        policy.insert(hs256_fpr(kid), format!("secret-{kid}").into_bytes());
    }
    let mut security: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for kid in ["s1", "s2", "s3", "s4"] {
        security.insert(hs256_fpr(kid), format!("secret-{kid}").into_bytes());
    }

    let keys = GovernanceKeySets {
        policy_council: policy.clone(),
        security_council: security.clone(),
        ..GovernanceKeySets::default()
    };

    let created_at = Utc::now();
    let activate_not_before = created_at + Duration::seconds(86_400);

    let mut bundle = PolicyBundleV1 {
        bundle_id: "test-critical-missing-security".to_string(),
        version: "1".to_string(),
        created_at,
        prev_bundle_hash: None,
        class: PolicyBundleV1Class::Critical,
        summary: "critical change".to_string(),
        policy: json!({"rules": []}),
        expires_at: None,
        rollback_plan: None,
        activate_not_before: Some(activate_not_before),
        signatures: vec![],
    };

    let bundle_hash = compute_bundle_hash(&bundle).expect("hash");
    let bundle_hash_bytes = URL_SAFE_NO_PAD
        .decode(bundle_hash.as_bytes())
        .expect("decode");

    for kid in ["p1", "p2", "p3", "p4"] {
        let fpr = hs256_fpr(kid);
        let secret = policy.get(&fpr).unwrap();
        let sig = URL_SAFE_NO_PAD.encode(hs256_sign(&bundle_hash_bytes, secret).unwrap());
        bundle
            .signatures
            .push(PolicySignatureV1 { key_fpr: fpr, sig });
    }

    // Only 1 security signature - should fail.
    {
        let kid = "s1";
        let fpr = hs256_fpr(kid);
        let secret = security.get(&fpr).unwrap();
        let sig = URL_SAFE_NO_PAD.encode(hs256_sign(&bundle_hash_bytes, secret).unwrap());
        bundle
            .signatures
            .push(PolicySignatureV1 { key_fpr: fpr, sig });
    }

    let now = activate_not_before + Duration::seconds(10);
    let err = verify_policy_bundle_v1(bundle, &keys, None, now).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("quorum_not_met"), "unexpected error: {msg}");
}
