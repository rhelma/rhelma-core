#!/usr/bin/env python3
"""Generate a small Markdown report from one or more k6 --summary-export JSON files.

Usage:
  python3 scripts/rhelma6/load/k6_summary_to_md.py <out_md> <summary1.json> [summary2.json ...]

The report is intentionally simple so it can be uploaded as a CI artifact.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def get_metric(data: dict, name: str) -> dict | None:
    metrics = data.get("metrics") or {}
    m = metrics.get(name)
    return m if isinstance(m, dict) else None


def stat(m: dict | None, key: str) -> float | None:
    if not m:
        return None
    values = m.get("values")
    if not isinstance(values, dict):
        return None
    v = values.get(key)
    return float(v) if isinstance(v, (int, float)) else None


def fmt_ms(v: float | None) -> str:
    if v is None:
        return "-"
    # k6 uses ms for http_req_duration, but we keep it generic.
    return f"{v:.2f} ms"


def fmt_pct(v: float | None) -> str:
    if v is None:
        return "-"
    return f"{(v * 100):.2f}%"


def render_one(path: Path, data: dict) -> str:
    http = get_metric(data, "http_req_duration")
    failed = get_metric(data, "http_req_failed")
    reqs = get_metric(data, "http_reqs")
    vus = get_metric(data, "vus")
    vus_max = get_metric(data, "vus_max")

    lines: list[str] = []
    lines.append(f"## {path.name}")

    # Core stats
    lines.append("")
    lines.append("| Metric | Value |")
    lines.append("|---|---:|")
    lines.append(f"| http_req_duration p(95) | {fmt_ms(stat(http, 'p(95)'))} |")
    lines.append(f"| http_req_duration p(99) | {fmt_ms(stat(http, 'p(99)'))} |")
    lines.append(f"| http_req_duration avg | {fmt_ms(stat(http, 'avg'))} |")
    lines.append(f"| http_req_failed rate | {fmt_pct(stat(failed, 'rate'))} |")
    if reqs:
        lines.append(f"| http_reqs count | {stat(reqs, 'count') or 0:.0f} |")
        lines.append(f"| http_reqs rate | {stat(reqs, 'rate') or 0:.2f} req/s |")
    if vus:
        lines.append(f"| vus | {stat(vus, 'value') or 0:.0f} |")
    if vus_max:
        lines.append(f"| vus_max | {stat(vus_max, 'value') or 0:.0f} |")

    return "\n".join(lines)


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    out_md = Path(sys.argv[1])
    inputs = [Path(p) for p in sys.argv[2:]]

    sections: list[str] = []
    sections.append("# k6 Load Test Report")
    sections.append("")
    for p in inputs:
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
        except Exception as e:
            sections.append(f"## {p.name}\n\n> Failed to read/parse: {e}")
            continue
        sections.append(render_one(p, data))
        sections.append("")

    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_md.write_text("\n".join(sections).rstrip() + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
