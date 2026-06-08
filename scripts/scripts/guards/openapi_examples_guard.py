#!/usr/bin/env python3
"""OpenAPI examples guard (lightweight).

Validates that JSON examples exist, parse, and contain expected minimal keys.
This is intentionally shallow to avoid schema engine dependencies.
"""
from __future__ import annotations
import argparse
import json
import os
import sys
from typing import Dict, List

EXAMPLES: Dict[str, List[str]] = {
    # value-ledger
    "docs/openapi/examples/value_ledger_balance_response.json": ["subject_id", "balance", "updated_at"],
    "docs/openapi/examples/value_ledger_earn_request.json": ["subject_id", "amount", "reason"],
    "docs/openapi/examples/value_ledger_spend_request.json": ["subject_id", "amount", "reason"],
    "docs/openapi/examples/value_ledger_receipt_issue_request.json": ["subject_id", "amount", "purpose"],
    "docs/openapi/examples/value_ledger_receipt_verify_request.json": ["receipt"],
    "docs/openapi/examples/value_ledger_receipt_verify_response.json": ["valid"],
    # value-ledger-federation
    "docs/openapi/examples/vlf_credits_response.json": ["subject_id", "balance", "tx_count"],
    "docs/openapi/examples/vlf_snapshot_response.json": ["cursor", "txs"],
    "docs/openapi/examples/vlf_push_result.json": ["accepted", "rejected"],
    "docs/openapi/examples/vlf_admin_issue_tx_request.json": ["subject_id", "delta", "reason"],
    "docs/openapi/examples/vlf_policy_hold_state_response.json": ["subject_id", "held", "active_hold_ids", "entries"],
    "docs/openapi/examples/vlf_gov_create_proposal_request.json": ["kind", "payload"],
    "docs/openapi/examples/vlf_gov_proposal_view_response.json": ["id", "kind", "payload", "required_quorum", "valid_signatures", "committed"],
    "docs/openapi/examples/vlf_verify_receipt_request.json": ["receipt"],
    "docs/openapi/examples/vlf_verify_receipt_response.json": ["ok", "required_quorum", "valid_sigs"],

    # search-service
    "docs/openapi/examples/search_service_search_request.json": ["query"],
    "docs/openapi/examples/search_service_search_response.json": ["total", "hits"],
    "docs/openapi/examples/search_service_admin_health.json": ["service", "region", "overall"],

    # file-storage
    "docs/openapi/examples/file_storage_upload_response.json": [
        "file_id",
        "download_url",
        "size_bytes",
        "content_type",
        "checksum",
    ],
    "docs/openapi/examples/file_storage_metadata_response.json": [
        "id",
        "tenant_id",
        "region",
        "original_name",
        "content_type",
        "size_bytes",
        "checksum",
        "storage_backend",
        "storage_path",
        "status",
        "created_at",
    ],
    "docs/openapi/examples/file_storage_health_deps_ok.json": ["status", "db", "storage"],
}

def load_json(path: str) -> object:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("root", nargs="?", default=".", help="repo root")
    args = ap.parse_args()
    root = args.root

    errors: List[str] = []
    for rel, keys in EXAMPLES.items():
        p = os.path.join(root, rel)
        if not os.path.exists(p):
            errors.append(f"missing example file: {rel}")
            continue
        try:
            obj = load_json(p)
        except Exception as e:
            errors.append(f"invalid JSON in {rel}: {e}")
            continue
        if not isinstance(obj, dict):
            errors.append(f"example {rel} must be a JSON object")
            continue
        for k in keys:
            if k not in obj:
                errors.append(f"example {rel} missing key: {k}")
    if errors:
        print("openapi_examples_guard: FAIL")
        for e in errors:
            print(" -", e)
        return 1
    print("openapi_examples_guard: OK")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
