#!/usr/bin/env python3
"""Check chaos_summary.json against a small baseline.

Usage:
  python3 scripts/rhelma6/chaos/check_chaos_summary.py <out_md> <summary_json> <baseline_json>

Baseline format:
  {"baseline": {"max_failed": 0, "max_duration_sec": 900}}

Exit codes:
  0 = ok
  3 = violation
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    out_md = Path(sys.argv[1])
    summary_path = Path(sys.argv[2])
    baseline_path = Path(sys.argv[3])

    summary = read_json(summary_path)
    baseline = read_json(baseline_path)

    base = baseline.get("baseline")
    if not isinstance(base, dict):
        print("baseline.baseline must be an object", file=sys.stderr)
        return 2

    max_failed = int(base.get("max_failed", 0))
    max_duration = int(base.get("max_duration_sec", 0))

    failed = int(summary.get("failed", 0))
    duration = int(summary.get("duration_sec", 0))
    overall = str(summary.get("overall", "unknown"))

    failed_ok = failed <= max_failed
    duration_ok = (max_duration <= 0) or (duration <= max_duration)
    overall_ok = overall == "ok"

    ok = failed_ok and duration_ok and overall_ok

    lines = []
    lines.append(f"# Chaos Baseline Check — {summary_path.name}")
    lines.append("")
    lines.append("| Check | Baseline | Actual | Status |")
    lines.append("|---|---:|---:|:---:|")
    lines.append(f"| failed tests | <= {max_failed} | {failed} | {'✅' if failed_ok else '❌'} |")
    if max_duration > 0:
        lines.append(f"| duration (sec) | <= {max_duration} | {duration} | {'✅' if duration_ok else '❌'} |")
    else:
        lines.append(f"| duration (sec) | - | {duration} | ✅ |")
    lines.append(f"| overall | ok | {overall} | {'✅' if overall_ok else '❌'} |")
    lines.append("")
    lines.append("> Within baseline limits." if ok else "> Baseline violation detected.")

    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_md.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")

    return 0 if ok else 3


if __name__ == "__main__":
    raise SystemExit(main())
