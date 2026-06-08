# MNI Secure Runner Contract v1

This contract defines the minimal request/response between an MNI controller (e.g. `mni-rag`)
and a sandboxed execution environment (e.g. `rhelma-sandbox-runner`).

## RunRequestV1 (JSON)

Fields:
- `run_id` (string, UUID or ULID)
- `export_id` (string)
- `export_path` (string, local or mounted path)
- `profile` (string, allowlist profile id)
- `command` (array of strings) — executable + args
- `env` (map string→string) — **allowlisted keys only**
- `inputs_sha256` (string) — hash of `manifest.json` (or of the bundle root)
- `created_at_unix` (int64)
- `requested_by` (string) — subject id (node_id/user_id/service)

## RunResultV1 (JSON)

- `run_id`
- `status` (one of: prepared, executed, verified, committed, rolled_back, failed)
- `started_at_unix`, `finished_at_unix`
- `stdout_path`, `stderr_path`
- `artifacts_dir`
- `checksums_path`
- `receipt_v1` (object) — signed receipt

## ReceiptV1

- `run_id`
- `inputs_sha256`
- `artifacts_root_sha256`
- `policy_profile`
- `runner_id`
- `issued_at_unix`
- `signature_b64` (Ed25519 over canonical JSON)

## Security Requirements

- No network by default.
- Only allowlisted command prefixes and paths.
- Workspace must be isolated and wiped on rollback.

