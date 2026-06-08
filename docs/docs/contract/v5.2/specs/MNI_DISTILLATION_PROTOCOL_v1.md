# MNI Distillation Protocol v1

## Overview
Distillation in Rhelma is a controlled process that produces **Model Improvement Artifacts** from
**Approved Training Streams** (ATS). A distillation export is **data**, not a model, and is
intended to feed LoRA/FT pipelines on worker nodes.

## Core Principles
- **Consent-first**: only data with explicit consent (or public commons) can be used.
- **Lineage-first**: every sample must map to a signed lineage record.
- **Policy-first**: only ATS policies may select samples.
- **Rollback-safe**: exports are immutable and versioned; new exports do not overwrite old ones.

## Terms
- **ATS (Approved Training Stream)**: a policy that selects samples for training.
- **Export Bundle**: an immutable directory containing samples + manifest + hashes.
- **Export Manifest**: canonical JSON describing how the export was produced.

## Export Manifest (canonical JSON)
Fields (required unless noted):
- `export_id` (string, ULID/UUID)
- `created_at_unix` (i64)
- `stream_id` (string)
- `stream_version` (u32)
- `dataset_merkle_root_hex` (string)
- `dataset_anchor_head_hash` (string, optional if not anchored)
- `sample_count` (u32)
- `format` (string, e.g. `jsonl.v1`)
- `filters` (object) — exact ATS policy content
- `hashes` (object):
  - `samples_jsonl_sha256`
  - `manifest_sha256`
  - `sha256sums_sha256`
- `signature` (object, optional):
  - `pubkey_b64`
  - `sig_b64`

## Offline Verification
1. Compute SHA-256 of `samples.jsonl` and compare to manifest.
2. Compute SHA-256 of `manifest.json` and compare to `sha256sums.txt`.
3. If signature present, verify `manifest.json` signature.

## Security Notes
- ATS policies must enforce tier rules:
  - `public_commons`: allowed
  - `consent_based`: allowed
  - `private`: forbidden
  - `sensitive`: allowed only if `jury_approved=true` and evidence exists
