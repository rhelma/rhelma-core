#!/usr/bin/env python3
"""Outbound HTTP context guard (Reqwest).

This guard is intentionally heuristic-based: it scans Rust sources for `reqwest`
`.send()` calls and checks the nearby call chain for Rhelma outbound propagation.

Accepted indicators within a small window before `.send()`:

- `.with_rhelma_observability()`
- `client.rhelma_get(...)` / `client.rhelma_post(...)` / `client.rhelma_put(...)` /
  `client.rhelma_patch(...)` / `client.rhelma_delete(...)`

Rationale: in production we want Contract v5.2 headers on every outbound HTTP call.

Allowlist:
- Add regex lines to `scripts/outbound_http_context_guard_allowlist.txt`.

Usage:
  python scripts/outbound_http_context_guard.py .

Exit codes:
- 0: ok
- 2: violations found
"""

from __future__ import annotations

import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Pattern, Tuple


IGNORE_DIRS = {
    ".git",
    "target",
    "node_modules",
    ".next",
    "dist",
    "build",
    "vendor",
}

SEND_RE = re.compile(r"\.send\s*\(\s*\)\s*(?:\.await)?")
INSTRUMENT_RE = re.compile(
    r"with_rhelma_observability\s*\(\s*\)|\.rhelma_(?:get|post|put|patch|delete)\s*\(",
    re.MULTILINE,
)


@dataclass
class Violation:
    file: str
    line: int
    context: str


def load_allowlist(repo_root: Path) -> List[Pattern[str]]:
    p = repo_root / "scripts" / "outbound_http_context_guard_allowlist.txt"
    if not p.exists():
        return []
    out: List[Pattern[str]] = []
    for raw in p.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        try:
            out.append(re.compile(line))
        except re.error as e:
            raise SystemExit(f"Invalid regex in allowlist: {line!r}: {e}")
    return out


def should_ignore_dir(dir_name: str) -> bool:
    return dir_name in IGNORE_DIRS


def iter_rs_files(repo_root: Path) -> Iterable[Path]:
    for root, dirs, files in os.walk(repo_root):
        dirs[:] = [d for d in dirs if not should_ignore_dir(d)]
        for f in files:
            if f.endswith(".rs"):
                yield Path(root) / f


def file_is_allowed(path: Path, allow: List[Pattern[str]]) -> bool:
    s = str(path).replace("\\", "/")
    return any(r.search(s) for r in allow)


def find_violations_in_text(text: str) -> List[Tuple[int, str]]:
    # Fast skip
    if "reqwest" not in text or ".send" not in text:
        return []

    # Find send() occurrences
    violations: List[Tuple[int, str]] = []
    for m in SEND_RE.finditer(text):
        start = max(0, m.start() - 800)  # window before send()
        window = text[start : m.start()]
        if INSTRUMENT_RE.search(window):
            continue

        # determine line number
        line_no = text.count("\n", 0, m.start()) + 1

        # provide context snippet (last 12 lines before send)
        before = text[: m.end()]
        lines = before.splitlines()[-12:]
        ctx = "\n".join(lines)
        violations.append((line_no, ctx))

    return violations


def main(argv: List[str]) -> int:
    repo_root = Path(argv[1] if len(argv) > 1 else ".").resolve()
    allow = load_allowlist(repo_root)

    violations: List[Violation] = []

    for p in iter_rs_files(repo_root):
        # Limit to Rust service code; still scan crates because services can live there.
        rel = p.relative_to(repo_root)
        rel_s = str(rel).replace("\\", "/")
        if not (
            rel_s.startswith("apps/")
            or rel_s.startswith("crates/")
            or rel_s.startswith("observability/")
            or rel_s.startswith("edge-worker/")
        ):
            continue

        # Skip tests: they may mock without propagation.
        if "/tests/" in rel_s or rel_s.endswith("_test.rs"):
            continue

        if file_is_allowed(rel, allow):
            continue

        try:
            text = p.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue

        for line_no, ctx in find_violations_in_text(text):
            violations.append(Violation(rel_s, line_no, ctx))

    if not violations:
        print("outbound_http_context_guard: OK")
        return 0

    print("outbound_http_context_guard: VIOLATIONS FOUND")
    for v in violations[:50]:
        print("-" * 80)
        print(f"{v.file}:{v.line}")
        print(v.context)

    if len(violations) > 50:
        print(f"... and {len(violations) - 50} more")

    print("\nFix: add `.with_rhelma_observability()` before `.send()` (or use client.rhelma_get/...)")
    print("Allowlist: add a regex line to scripts/outbound_http_context_guard_allowlist.txt")
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
