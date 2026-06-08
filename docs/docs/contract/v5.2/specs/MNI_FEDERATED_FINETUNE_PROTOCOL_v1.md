# MNI Federated Fine-tune Protocol v1 (Permissioned)

## Overview
This protocol defines how nodes submit fine-tuning updates to the Rhelma network while preserving:
- safety against model poisoning/backdoors,
- lineage and reproducibility,
- governance control over activation/rollback.

## Object: UpdateProposalV1
Fields (canonical JSON for signing):
- `proposal_id` (uuid or 32-byte hex)
- `node_id` (hex; derived from pubkey)
- `pubkey_b64`
- `submitted_at_unix`
- `distill_export_id`
- `distill_export_sha256_hex`
- `trainer_config_sha256_hex`
- `delta_artifact_sha256_hex`
- `delta_artifact_size_bytes`
- `track` (e.g., `mni.core`, `mni.security`)
- `attestation_digest_hex` (optional but recommended)
- `reputation_snapshot` (integer 0..1000)
- `policy_head_hash_hex` (policy artifact head observed when creating)
- `signature_b64` (ed25519 over canonical bytes excluding signature)

## Object: AcceptanceCertificateV1
- `proposal_id`
- `accepted_at_unix`
- `accepted_by` (coordinator id)
- `accepted_policy_head_hash_hex`
- `activation_head_prev` (optional)
- `activation_head_new`
- `certificate_sig_b64` (ed25519)

## Policy Gates
- Allowed tracks
- Allowed trainer configs (hash allow-list)
- Min reputation threshold
- Require attested (bool)

## Anti-poisoning Rules
- Reject oversized deltas by policy (max bytes)
- Reject unknown trainer configs
- Require reproducible bundle hashes (export + config + delta)
- Require challenge set evaluation for certain tracks (Phase 40)

## Events (optional)
- `mni.federated.update.proposed@v1`
- `mni.federated.update.accepted@v1`
- `mni.federated.update.rejected@v1`
