# MNI Dataset Anchoring Protocol v1

This protocol defines how an MNI dataset checkpoint becomes **anchored** and **verifiable** across the network.

## 1) Dataset Checkpoint (Publisher side)
Publisher (mni-rag) emits a checkpoint with:
- dataset_id
- merkle_root_hex
- ts_unix
- publisher_node_id
- publisher_pubkey_b64
- publisher_sig_b64

Signature: Ed25519 over canonical payload:
`mni.dataset.checkpoint.v1|{dataset_id}|{merkle_root_hex}|{ts_unix}|{publisher_node_id}`

## 2) Anchoring (Governance side)
Anchoring is performed by the federated value ledger (VLF) as a policy artifact update.

- Operation kinds:
  - `policy.mni_datasets.set`
  - `policy.mni_datasets.rollback`

- The policy artifact is append-only. "Active head" is the latest valid record,
  unless rolled back within rollback window.

## 3) Verification (Consumer side)
A consumer verifies:
1) publisher signature is valid (Ed25519)
2) the merkle root matches the dataset head / inclusion proof (Phase 32)
3) the root is present in VLF active head (policy artifact)
4) optional: dataset_id is allowlisted and not denylisted

## 4) Propagation
Checkpoint payloads can be propagated via gossip-discovery checkpoints (Phase 29–30).
Anchors propagate via VLF federation snapshot replication.

## 5) Privacy
No raw documents are included. Only Merkle roots and minimal metadata.
