#!/usr/bin/env python3
from __future__ import annotations
import json
import sys
from pathlib import Path

try:
    import yaml  # type: ignore
except Exception as e:
    print(f"[openapi_contract_guard] ERROR: PyYAML is required for deep validation: {e}", file=sys.stderr)
    sys.exit(2)

ROOT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
DOCS = ROOT / "docs" / "openapi"

REQUIRED_SPECS = {
    "api-gateway.yaml": {
        "paths": [
            "/health",
            "/health/ready",
            "/health/region/{region_id}",
            "/admin/region-routing/snapshot",
            "/admin/region-routing/override",
            "/admin/region-routing/simulate-failover",
        ],
        "version": "6.0.0",
        "contract": "v6.0",
    },
    "region-health-aggregator.yaml": {
        "paths": ["/healthz", "/v1/regions/health", "/v1/route"],
        "version": "6.0.0",
        "contract": "v6.0",
    },
}

EXAMPLES = {
    "gateway_region_routing_snapshot.json": ("api-gateway.yaml", "#/components/schemas/RegionRoutingSnapshotResponse"),
    "rha_health_snapshot.json": ("region-health-aggregator.yaml", "#/components/schemas/HealthSnapshot"),
    "rha_route_response.json": ("region-health-aggregator.yaml", "#/components/schemas/RouteResponse"),
}

def die(msg: str) -> None:
    print(f"[openapi_contract_guard] FAIL: {msg}", file=sys.stderr)
    sys.exit(1)

def ok(msg: str) -> None:
    print(f"[openapi_contract_guard] {msg}")

def load_yaml(path: Path):
    try:
        return yaml.safe_load(path.read_text(encoding="utf-8"))
    except Exception as e:
        die(f"cannot parse YAML: {path}: {e}")

def resolve_ref(doc, ref: str):
    if not ref.startswith("#/"):
        die(f"unsupported $ref: {ref}")
    node = doc
    for part in ref[2:].split("/"):
        if part not in node:
            die(f"missing ref component '{part}' in {ref}")
        node = node[part]
    return node

def schema_props(schema: dict, doc: dict) -> set[str]:
    # Resolve $ref if any
    if "$ref" in schema:
        schema = resolve_ref(doc, schema["$ref"])
    props = set(schema.get("properties", {}).keys())
    return props

def validate_example(doc: dict, schema_ref: str, example_path: Path) -> None:
    schema = resolve_ref(doc, schema_ref)
    props = set(schema.get("properties", {}).keys())
    required = set(schema.get("required", []))
    try:
        ex = json.loads(example_path.read_text(encoding="utf-8"))
    except Exception as e:
        die(f"invalid JSON example: {example_path.name}: {e}")

    if not isinstance(ex, dict):
        die(f"example {example_path.name} must be an object")

    missing = sorted(list(required - set(ex.keys())))
    if missing:
        die(f"example {example_path.name} missing required keys: {missing}")

    # allow additionalProperties only if explicitly enabled
    allow_extra = schema.get("additionalProperties", False) is True
    if not allow_extra:
        extra = sorted([k for k in ex.keys() if k not in props])
        if extra:
            die(f"example {example_path.name} has unknown keys not in schema: {extra}")

def main() -> None:
    if not DOCS.exists():
        die("docs/openapi directory is missing")

    for spec, cfg in REQUIRED_SPECS.items():
        p = DOCS / spec
        if not p.exists():
            die(f"missing required spec: {p}")
        doc = load_yaml(p)

        if str(doc.get("openapi", "")).strip()[:3] != "3.0":
            die(f"{spec}: openapi must be 3.0.x")
        info = doc.get("info") or {}
        if info.get("version") != cfg["version"]:
            die(f"{spec}: info.version must be {cfg['version']}")
        if doc.get("x-rhelma-contract-version") != cfg["contract"]:
            die(f"{spec}: x-rhelma-contract-version must be {cfg['contract']}")

        paths = doc.get("paths") or {}
        for needed in cfg["paths"]:
            if needed not in paths:
                die(f"{spec}: missing path: {needed}")

        ok(f"{spec}: basic checks OK")

    # Deep check examples
    ex_dir = DOCS / "examples"
    if not ex_dir.exists():
        die("docs/openapi/examples directory is missing")
    for ex_name, (spec, schema_ref) in EXAMPLES.items():
        ex_path = ex_dir / ex_name
        if not ex_path.exists():
            die(f"missing example: {ex_name}")
        spec_doc = load_yaml(DOCS / spec)
        validate_example(spec_doc, schema_ref, ex_path)
        ok(f"example {ex_name}: shape OK")

    ok("OK")

if __name__ == "__main__":
    main()
