#!/usr/bin/env python3
"""OpenAPI drift guard (lightweight, method-aware).

Goal
----
Prevent "contract drift" between implemented Axum routes and docs/openapi/*.yaml by enforcing
that a curated allowlist of (METHOD, PATH) endpoints exists in the OpenAPI spec.

Design constraints
------------------
* Low-noise: we avoid full Rust parsing.
* Method-aware: we validate that the OpenAPI spec contains the expected HTTP method.
* Optional nest support: for a small set of services we expand a configured list of nested
  routers (prefix + file) to derive endpoints.
"""

from __future__ import annotations

import argparse
import os
import re
from typing import Dict, Iterable, List, Mapping, MutableMapping, Optional, Sequence, Set, Tuple


HttpMethod = str
Endpoint = Tuple[HttpMethod, str]  # (method, path)


SERVICE_CONFIG: Dict[str, Dict[str, object]] = {
    "value-ledger": {
        "openapi": "docs/openapi/value-ledger.yaml",
        "code_files": [
            "apps/value-ledger/src/main.rs",
            "apps/value-ledger/src/routes.rs",
        ],
        "required_endpoints": [
            ("GET", "/healthz"),
            ("GET", "/readyz"),
            ("GET", "/metrics"),
            ("GET", "/v1/credits/{subject_id}"),
            ("POST", "/v1/receipts/verify"),
            ("POST", "/v1/credits/earn"),
            ("POST", "/v1/credits/spend"),
            ("POST", "/v1/receipts/issue"),
        ],
    },
    "value-ledger-federation": {
        "openapi": "docs/openapi/value-ledger-federation.yaml",
        "code_files": [
            "apps/value-ledger-federation/src/main.rs",
            "apps/value-ledger-federation/src/gov.rs",
        ],
        "required_endpoints": [
            ("GET", "/healthz"),
            ("GET", "/metrics"),
            ("GET", "/v1/credits/{subject_id}"),
            ("GET", "/v1/federation/snapshot"),
            ("POST", "/v1/federation/push"),
            ("GET", "/v1/policy/hold/{subject_id}"),
            ("GET", "/v1/policy/bridge-drivers"),
            ("GET", "/v1/policy/bridge-drivers/history"),
            ("POST", "/v1/receipts/verify"),
            ("POST", "/v1/admin/tx"),
            ("POST", "/v1/admin/policy/hold"),
            ("POST", "/v1/gov/proposals"),
            ("POST", "/v1/gov/proposals/{id}/sign"),
            ("POST", "/v1/gov/proposals/{id}/commit"),
        ],
    },
    "search-service": {
        "openapi": "docs/openapi/search-service.yaml",
        "code_files": [
            "apps/search-service/src/main.rs",
        ],
        # We expand nested routers explicitly to keep the guard meaningful.
        "nested_routers": [
            ("/search", "apps/search-service/src/routes/search.rs"),
            ("/search/enhanced", "apps/search-service/src/routes/search_enhanced.rs"),
            ("/admin", "apps/search-service/src/routes/admin.rs"),
        ],
        "required_endpoints": [
            ("GET", "/metrics"),
            ("POST", "/search"),
            ("POST", "/search/enhanced"),
            ("GET", "/admin/health"),
            ("GET", "/admin/info"),
        ],
    },
    "file-storage": {
        "openapi": "docs/openapi/file-storage.yaml",
        "code_files": [
            "apps/file-storage/src/main.rs",
        ],
        "required_endpoints": [
            ("GET", "/metrics"),
            ("GET", "/health"),
            ("GET", "/health/deps"),
            ("POST", "/files"),
            ("GET", "/files/{id}"),
            ("GET", "/files/{id}/metadata"),
        ],
    },
}


def read_text(root: str, rel: str) -> str:
    path = os.path.join(root, rel)
    with open(path, "r", encoding="utf-8", errors="ignore") as f:
        return f.read()


def extract_openapi_endpoints(yaml_text: str) -> Mapping[str, Set[str]]:
    """Return mapping path -> set(methods) from an OpenAPI YAML text.

    Naive parser that only understands indentation:
      paths:
        /path:
          get:
          post:
    """
    endpoints: MutableMapping[str, Set[str]] = {}
    in_paths = False
    current_path: Optional[str] = None

    for line in yaml_text.splitlines():
        if re.match(r"^paths:\s*$", line):
            in_paths = True
            continue

        if not in_paths:
            continue

        # Exit paths section
        if re.match(r"^components:\s*$", line):
            break

        m_path = re.match(r"^\s{2}(/[^:]+):\s*$", line)
        if m_path:
            current_path = m_path.group(1).strip()
            endpoints.setdefault(current_path, set())
            continue

        if current_path:
            m_method = re.match(r"^\s{4}(get|post|put|delete|patch|head|options):\s*$", line)
            if m_method:
                endpoints[current_path].add(m_method.group(1).upper())

    return endpoints


def normalize_code_path(p: str) -> str:
    # Axum uses :param; OpenAPI uses {param}
    p = re.sub(r":([A-Za-z_][A-Za-z0-9_]*)", r"{\1}", p)
    # Normalize trailing slash (OpenAPI specs should not rely on it)
    if len(p) > 1 and p.endswith("/"):
        p = p[:-1]
    return p


def join_paths(prefix: str, suffix: str) -> str:
    prefix = prefix.rstrip("/")
    suffix = suffix.strip()
    if suffix in ("", "/"):
        combined = prefix or "/"
    else:
        combined = f"{prefix}/{suffix.lstrip('/')}"
    return normalize_code_path(combined)


_ROUTE_RE = re.compile(
    r"\.route\(\s*\"([^\"]+)\"\s*,\s*(?:[A-Za-z0-9_:]*::)?(get|post|put|delete|patch|head|options)\s*\(",
    re.MULTILINE,
)


def extract_code_endpoints(code: str) -> List[Endpoint]:
    eps: List[Endpoint] = []
    for m in _ROUTE_RE.finditer(code):
        path = normalize_code_path(m.group(1))
        method = m.group(2).upper()
        eps.append((method, path))
    return eps


def extract_nested_endpoints(root: str, nested: Sequence[Tuple[str, str]]) -> List[Endpoint]:
    eps: List[Endpoint] = []
    for prefix, file_rel in nested:
        p = os.path.join(root, file_rel)
        if not os.path.exists(p):
            continue
        code = read_text(root, file_rel)
        for method, route_path in extract_code_endpoints(code):
            eps.append((method, join_paths(prefix, route_path)))
    return eps


def check_service(root: str, name: str) -> Tuple[bool, List[str]]:
    cfg = SERVICE_CONFIG[name]
    openapi_text = read_text(root, str(cfg["openapi"]))
    spec = extract_openapi_endpoints(openapi_text)

    # Gather endpoints seen in code (best-effort; used for extra signal only).
    code_eps: Set[Endpoint] = set()
    for rel in cfg.get("code_files", []):
        rel = str(rel)
        if os.path.exists(os.path.join(root, rel)):
            code_eps.update(extract_code_endpoints(read_text(root, rel)))
    if "nested_routers" in cfg:
        code_eps.update(extract_nested_endpoints(root, cfg["nested_routers"]))  # type: ignore[arg-type]

    missing: List[str] = []
    required: Sequence[Endpoint] = cfg.get("required_endpoints", [])  # type: ignore[assignment]
    for method, path in required:
        methods = spec.get(path)
        if not methods or method.upper() not in methods:
            missing.append(f"[{name}] missing in OpenAPI: {method.upper()} {path}")

    # Optional: warn (not fail) if a required endpoint isn't detected in code.
    # This keeps the guard low-noise while still helping catch obvious regressions.
    # We emit these as FAIL only if the OpenAPI is already missing something.
    if missing:
        # Add extra hints.
        for method, path in required:
            if (method.upper(), path) not in code_eps:
                missing.append(f"[{name}] hint: endpoint not detected in code scan: {method.upper()} {path}")

    ok = len([m for m in missing if "missing in OpenAPI" in m]) == 0
    return ok, missing


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("root", nargs="?", default=".", help="repo root")
    ap.add_argument("--service", default="all", help="service name or 'all'")
    args = ap.parse_args()

    root = args.root
    services = list(SERVICE_CONFIG.keys()) if args.service == "all" else [args.service]

    errors: List[str] = []
    for s in services:
        if s not in SERVICE_CONFIG:
            errors.append(f"Unknown service: {s}")
            continue
        ok, missing = check_service(root, s)
        if not ok:
            errors.extend(missing)

    if errors:
        print("openapi_drift_guard: FAIL")
        for e in errors:
            print(" -", e)
        return 1
    print("openapi_drift_guard: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
