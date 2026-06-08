#!/usr/bin/env python3
"""Compare a k6 --summary-export JSON against a baseline.

Supported baseline formats:
  - A full k6 --summary-export JSON.
  - A small custom JSON file:
      {"baseline": {"http_req_duration_p95_ms": 500.0, "http_req_failed_rate": 0.01}}

Exit codes:
  0 = within limits
  3 = regression detected
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def get_metric(data: dict[str, Any], name: str) -> dict[str, Any] | None:
    metrics = data.get("metrics")
    if not isinstance(metrics, dict):
        return None
    m = metrics.get(name)
    return m if isinstance(m, dict) else None


def stat(m: dict[str, Any] | None, key: str) -> float | None:
    if not m:
        return None
    values = m.get("values")
    if not isinstance(values, dict):
        return None
    v = values.get(key)
    if isinstance(v, (int, float)):
        return float(v)
    return None


def extract_k6(summary: dict[str, Any]) -> tuple[float | None, float | None]:
    http = get_metric(summary, "http_req_duration")
    failed = get_metric(summary, "http_req_failed")
    p95 = stat(http, "p(95)")
    fail_rate = stat(failed, "rate")
    return p95, fail_rate


def extract_baseline(baseline: dict[str, Any]) -> tuple[float | None, float | None]:
    # Custom compact baseline
    b = baseline.get("baseline")
    if isinstance(b, dict):
        p95 = b.get("http_req_duration_p95_ms")
        fr = b.get("http_req_failed_rate")
        p95_v = float(p95) if isinstance(p95, (int, float)) else None
        fr_v = float(fr) if isinstance(fr, (int, float)) else None
        return p95_v, fr_v
    # Otherwise assume it's a k6 summary-export
    return extract_k6(baseline)


def fmt_ms(v: float | None) -> str:
    return "-" if v is None else f"{v:.2f} ms"


def fmt_pct(v: float | None) -> str:
    return "-" if v is None else f"{v * 100:.2f}%"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("out_md", type=Path)
    ap.add_argument("current", type=Path)
    ap.add_argument("baseline", type=Path)
    ap.add_argument("--max-p95-regression-pct", type=float, default=20.0)
    ap.add_argument("--max-failed-rate", type=float, default=0.01)
    ap.add_argument("--name", type=str, default="")
    args = ap.parse_args()

    current_data = json.loads(args.current.read_text(encoding="utf-8"))
    base_data = json.loads(args.baseline.read_text(encoding="utf-8"))

    cur_p95, cur_fail = extract_k6(current_data)
    base_p95, base_fail = extract_baseline(base_data)

    allowed_p95 = None
    if base_p95 is not None and cur_p95 is not None:
        allowed_p95 = base_p95 * (1.0 + (args.max_p95_regression_pct / 100.0))

    # For failed rate we enforce a hard cap. If baseline has a non-zero rate,
    # we also enforce a relative regression cap.
    allowed_fail = args.max_failed_rate
    if base_fail is not None and base_fail > 0:
        allowed_fail = min(allowed_fail, base_fail * (1.0 + (args.max_p95_regression_pct / 100.0)))

    p95_ok = True
    fail_ok = True

    if allowed_p95 is not None and cur_p95 is not None:
        p95_ok = cur_p95 <= allowed_p95

    if cur_fail is not None:
        fail_ok = cur_fail <= allowed_fail

    title = args.name.strip() or args.current.name

    lines: list[str] = []
    lines.append(f"# k6 Baseline Comparison — {title}")
    lines.append("")
    lines.append("| Metric | Baseline | Current | Allowed | Status |")
    lines.append("|---|---:|---:|---:|:---:|")
    lines.append(
        f"| http_req_duration p(95) | {fmt_ms(base_p95)} | {fmt_ms(cur_p95)} | {fmt_ms(allowed_p95)} | {'✅' if p95_ok else '❌'} |"
    )
    lines.append(
        f"| http_req_failed rate | {fmt_pct(base_fail)} | {fmt_pct(cur_fail)} | {fmt_pct(allowed_fail)} | {'✅' if fail_ok else '❌'} |"
    )
    lines.append("")
    if not p95_ok or not fail_ok:
        lines.append("> Regression detected. See table above.")
    else:
        lines.append("> Within baseline limits.")

    args.out_md.parent.mkdir(parents=True, exist_ok=True)
    args.out_md.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")

    return 0 if (p95_ok and fail_ok) else 3


if __name__ == "__main__":
    raise SystemExit(main())
