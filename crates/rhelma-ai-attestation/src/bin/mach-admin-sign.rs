//! rhelma-admin-sign — helper CLI to generate HS256 admin action attestations.
//!
//! Designed for signing `lexicon.promote` / `lexicon.discard` admin actions in ai-orchestrator.
//!
//! Examples:
//!   # Single signature (uses primary key from env keyring)
//!   cargo run -p rhelma-ai-attestation --bin rhelma-admin-sign -- lexicon.promote --format json
//!
//!   # Two signatures (2-of-3 quorum)
//!   cargo run -p rhelma-ai-attestation --bin rhelma-admin-sign -- lexicon.promote --kids admin-a,admin-b --format headers
//!
//! Env:
//!   RHELMA_AI_ATTESTATION__HMAC_KEYS="admin-a:secretA,admin-b:secretB,admin-c:secretC"
//!   RHELMA_AI_ATTESTATION__PRIMARY_KID="admin-a"   (optional)

#![forbid(unsafe_code)]

use std::collections::HashSet;

use chrono::Utc;
use rhelma_ai_attestation::{
    load_hs256_keyring_from_env, sign_hs256, sign_hs256_with_keyring, AttestationV1, Hs256KeyRing,
};

fn usage_and_exit() -> ! {
    eprintln!(
        r#"rhelma-admin-sign

USAGE:
  rhelma-admin-sign <action> [--ts RFC3339] [--request-id ID] [--staging-digest HEX] [--active-before-digest HEX]
                 [--kids kid1,kid2] [--format headers|json|curl] [--url URL]

ACTIONS:
  lexicon.promote
  lexicon.discard

FORMATS:
  headers  Prints x-rhelma-admin-action-ts and x-rhelma-admin-action-attestation(s) headers (single-line JSON)
  json     Prints JSON object: {{ payload, attestations }}
  curl     Prints a curl example (requires --url)

ENV:
  RHELMA_AI_ATTESTATION__HMAC_KEYS (preferred) or RHELMA_AI_ATTESTATION__HMAC_SECRET (legacy)

NOTES:
  - Server enforces timestamp skew, so --ts should be close to current time.
  - For quorum > 1, supply --kids with multiple distinct kid values.
"#
    );
    std::process::exit(2);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutFmt {
    Headers,
    Json,
    Curl,
}

fn parse_fmt(s: &str) -> Option<OutFmt> {
    match s {
        "headers" => Some(OutFmt::Headers),
        "json" => Some(OutFmt::Json),
        "curl" => Some(OutFmt::Curl),
        _ => None,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);

    let action = match args.next() {
        Some(a) if a == "lexicon.promote" || a == "lexicon-promote" => {
            "lexicon.promote".to_string()
        }
        Some(a) if a == "lexicon.discard" || a == "lexicon-discard" => {
            "lexicon.discard".to_string()
        }
        _ => usage_and_exit(),
    };

    let mut ts: Option<String> = None;
    let mut request_id: Option<String> = None;
    let mut staging_digest: Option<String> = None;
    let mut active_before_digest: Option<String> = None;
    let mut kids_csv: Option<String> = None;
    let mut fmt = OutFmt::Headers;
    let mut url: Option<String> = None;

    while let Some(a) = args.next() {
        match a.as_str() {
            "--ts" => ts = args.next(),
            "--request-id" => request_id = args.next(),
            "--staging-digest" => staging_digest = args.next(),
            "--active-before-digest" => active_before_digest = args.next(),
            "--kids" => kids_csv = args.next(),
            "--format" => {
                let v = args.next().unwrap_or_default();
                fmt = parse_fmt(&v).unwrap_or_else(|| usage_and_exit());
            }
            "--url" => url = args.next(),
            "-h" | "--help" => usage_and_exit(),
            _ => usage_and_exit(),
        }
    }

    let ts = ts.unwrap_or_else(|| Utc::now().to_rfc3339());
    let payload = serde_json::json!({
        "kind": "rhelma.lexicon.admin_action.v1",
        "action": action,
        "ts": ts,
        "request_id": request_id,
        "staging_digest_sha256_hex": staging_digest,
        "active_before_digest_sha256_hex": active_before_digest,
    });

    let kr = load_hs256_keyring_from_env();
    if kr.is_empty() {
        eprintln!("ERROR: no HS256 keys found in env (set RHELMA_AI_ATTESTATION__HMAC_KEYS or RHELMA_AI_ATTESTATION__HMAC_SECRET)");
        std::process::exit(1);
    }

    let atts = if let Some(kcsv) = kids_csv {
        sign_with_kids(&payload, &kr, &kcsv)
    } else {
        vec![sign_hs256_with_keyring(&payload, &kr).unwrap_or_else(|e| {
            eprintln!("ERROR: failed to sign with keyring: {e}");
            std::process::exit(1);
        })]
    };

    match fmt {
        OutFmt::Json => {
            let out = serde_json::json!({
                "payload": payload,
                "attestations": atts,
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        OutFmt::Headers => {
            print_headers(&payload, &atts);
        }
        OutFmt::Curl => {
            let url = url.unwrap_or_else(|| {
                eprintln!("ERROR: --url is required for --format curl");
                std::process::exit(2);
            });
            print_curl(&url, &payload, &atts);
        }
    }
}

fn sign_with_kids(
    payload: &serde_json::Value,
    kr: &Hs256KeyRing,
    kids_csv: &str,
) -> Vec<AttestationV1> {
    let mut out: Vec<AttestationV1> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for kid in kids_csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if !seen.insert(kid.to_string()) {
            continue;
        }
        let key = kr
            .keys
            .iter()
            .find(|k| k.kid.as_deref() == Some(kid))
            .unwrap_or_else(|| {
                eprintln!(
                    "ERROR: kid '{kid}' not found in keyring (RHELMA_AI_ATTESTATION__HMAC_KEYS)"
                );
                std::process::exit(1);
            });

        let att = sign_hs256(payload, &key.secret, Some(kid.to_string())).unwrap_or_else(|e| {
            eprintln!("ERROR: failed to sign payload with kid '{kid}': {e}");
            std::process::exit(1);
        });
        out.push(att);
    }

    if out.is_empty() {
        eprintln!("ERROR: --kids provided but no valid kids parsed");
        std::process::exit(2);
    }

    out
}

fn print_headers(payload: &serde_json::Value, atts: &[AttestationV1]) {
    let ts = payload.get("ts").and_then(|v| v.as_str()).unwrap_or("");
    println!("x-rhelma-admin-action-ts: {}", ts);

    if atts.len() == 1 {
        let raw = serde_json::to_string(atts.first().unwrap()).unwrap();
        println!("x-rhelma-admin-action-attestation: {}", raw);
    } else {
        let raw = serde_json::to_string(atts).unwrap();
        println!("x-rhelma-admin-action-attestations: {}", raw);
    }
}

fn print_curl(url: &str, payload: &serde_json::Value, atts: &[AttestationV1]) {
    let ts = payload.get("ts").and_then(|v| v.as_str()).unwrap_or("");
    let hdr_att = if atts.len() == 1 {
        format!(
            r#"-H 'x-rhelma-admin-action-attestation: {}'"#,
            serde_json::to_string(atts.first().unwrap()).unwrap()
        )
    } else {
        format!(
            r#"-H 'x-rhelma-admin-action-attestations: {}'"#,
            serde_json::to_string(atts).unwrap()
        )
    };

    println!(
        r#"curl -sS -X POST '{url}' \
  -H 'content-type: application/json' \
  -H 'x-rhelma-admin-action-ts: {ts}' \
  {hdr_att} \
  -d '{{}}'"#
    );
}
