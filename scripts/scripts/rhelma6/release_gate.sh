#!/usr/bin/env bash
set -euo pipefail

# Rhelma6 release gate
# Runs verify + smoke + (optional) OTEL propagation tests + (optional) k6 quick profile
# and emits a single Markdown report.
#
# Output:
#   - benchmarks/out/release_gate_report.md
#   - benchmarks/out/release_gate_manifest.json
#   - benchmarks/out/release_gate_pr_comment.md
#   - benchmarks/out/release_gate_go_no_go_block.md
#   - benchmarks/out/release_gate_*.log

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/benchmarks/out"
REPORT="$OUT_DIR/release_gate_report.md"

mkdir -p "$OUT_DIR"

ts_utc() { date -u "+%Y-%m-%dT%H:%M:%SZ"; }
epoch_s() { date -u "+%s"; }

have_cmd() { command -v "$1" >/dev/null 2>&1; }

md_escape() {
  # Minimal escaping for code blocks: avoid accidental ``` sequences.
  sed 's/```/`\`\`/g'
}

git_line() {
  if have_cmd git && [ -d "$ROOT_DIR/.git" ]; then
    local branch commit describe dirty
    branch="$(git -C "$ROOT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")"
    commit="$(git -C "$ROOT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")"
    describe="$(git -C "$ROOT_DIR" describe --always --dirty --tags 2>/dev/null || echo "$commit")"
    dirty=""
    if ! git -C "$ROOT_DIR" diff --quiet 2>/dev/null || ! git -C "$ROOT_DIR" diff --cached --quiet 2>/dev/null; then
      dirty=" (dirty)"
    fi
    echo "- Git: ${branch} @ ${describe}${dirty}"
  else
    echo "- Git: ⏭️ (repo not a git checkout)"
  fi
}

tool_line() {
  local name="$1"; shift
  if have_cmd "$name"; then
    echo "- $name: $("$name" "$@" 2>/dev/null | head -n 1)"
  else
    echo "- $name: ⏭️ (not found)"
  fi
}





sha256_cmd() {
  if have_cmd sha256sum; then echo "sha256sum"; return 0; fi
  if have_cmd shasum; then echo "shasum -a 256"; return 0; fi
  return 1
}

sha256_file() {
  local f="$1"
  local cmd
  if ! cmd="$(sha256_cmd)"; then
    echo ""
    return 0
  fi
  # shellcheck disable=SC2086
  ( $cmd "$f" | awk '{print $1}' ) 2>/dev/null || true
}

write_manifest() {
  local overall_rc="$1" required_incomplete="$2" verify_state="$3" otel_state="$4" smoke_state="$5" load_state="$6" verify_rc="$7" otel_rc="$8" smoke_rc="$9"
  local manifest="$OUT_DIR/release_gate_manifest.json"

  local sha_report
  sha_report="$(sha256_file "$REPORT")"

  local comment="$OUT_DIR/release_gate_pr_comment.md"
  local block="$OUT_DIR/release_gate_go_no_go_block.md"
  local sha_comment="" sha_block=""
  if [ -f "$comment" ]; then sha_comment="$(sha256_file "$comment")"; fi
  if [ -f "$block" ]; then sha_block="$(sha256_file "$block")"; fi

  local logs=("$OUT_DIR"/release_gate_*.log)

  {
    printf '{
'
    printf '  "generated_at": "%s",
' "$(ts_utc)"
    printf '  "required_incomplete": %s,
' "$required_incomplete"

    if have_cmd git && [ -d "$ROOT_DIR/.git" ]; then
      local branch commit describe
      branch="$(git -C "$ROOT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")"
      commit="$(git -C "$ROOT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")"
      describe="$(git -C "$ROOT_DIR" describe --always --dirty --tags 2>/dev/null || echo "$commit")"
      printf '  "git": { "branch": "%s", "describe": "%s" },
' "$branch" "$describe"
    else
      printf '  "git": null,
'
    fi

    printf '  "results": {
'
    printf '    "overall": %s,
' "$overall_rc"
    printf '    "verify": %s,
' "$verify_rc"
    printf '    "otel_verify": %s,
' "$otel_rc"
    printf '    "smoke_core": %s,
' "$smoke_rc"
    printf '    "load_k6_quick": "%s"
' "$load_state"
    printf '  },
'
    printf '  "states": {
'
    printf '    "verify": "%s",
' "$verify_state"
    printf '    "otel_verify": "%s",
' "$otel_state"
    printf '    "smoke_core": "%s",
' "$smoke_state"
    printf '    "load_k6_quick": "%s"
' "$load_state"
    printf '  },
'

    printf '  "artifacts": [
'
    printf '    { "path": "benchmarks/out/%s", "sha256": "%s" }' "$(basename "$REPORT")" "$sha_report"

    if [ -f "$comment" ]; then
      printf ',
    { "path": "benchmarks/out/%s", "sha256": "%s" }' "$(basename "$comment")" "$sha_comment"
    fi
    if [ -f "$block" ]; then
      printf ',
    { "path": "benchmarks/out/%s", "sha256": "%s" }' "$(basename "$block")" "$sha_block"
    fi

    for f in "${logs[@]}"; do
      [ -f "$f" ] || continue
      local sha
      sha="$(sha256_file "$f")"
      printf ',
    { "path": "benchmarks/out/%s", "sha256": "%s" }' "$(basename "$f")" "$sha"
    done

    printf '
  ]
'
    printf '}
'
  } >"$manifest"
}

write_pr_comment() {
  local overall_rc="$1" required_incomplete="$2" verify_state="$3" otel_state="$4" smoke_state="$5" load_state="$6" verify_rc="$7" otel_rc="$8" smoke_rc="$9"
  local comment="$OUT_DIR/release_gate_pr_comment.md"
  local block="$OUT_DIR/release_gate_go_no_go_block.md"

  local overall_word="NO-GO" overall_icon="❌"
  if [ "$overall_rc" -eq 0 ] && [ "$required_incomplete" -eq 0 ]; then
    overall_word="GO"; overall_icon="✅"
  elif [ "$overall_rc" -eq 0 ] && [ "$required_incomplete" -ne 0 ]; then
    overall_word="INCOMPLETE"; overall_icon="⚠️"
  fi

  local v_mark="[ ]" o_mark="[ ]" s_mark="[ ]" l_mark="[ ]"
  [ "$verify_state" = "PASS" ] && v_mark="[x]"
  [ "$otel_state" = "PASS" ] && o_mark="[x]"
  [ "$smoke_state" = "PASS" ] && s_mark="[x]"
  [ "$load_state" = "PASS" ] && l_mark="[x]"

  cat >"$block" <<EOF
## GO/NO-GO (paste into release ticket)

- $v_mark Verify PASS
- $o_mark OTEL verify PASS (optional)
- $s_mark Smoke (core) PASS
- $l_mark Load (k6 quick) PASS (optional)
- [ ] Change approved
- [ ] Rollback plan reviewed
- [ ] Decision: GO / NO-GO
- Approver: ________  Time: ________
EOF

  cat >"$comment" <<EOF
<!-- Rhelma6 Release Gate (autogenerated) -->
## Rhelma6 Release Gate — $overall_icon **$overall_word**

**Gates**
- Verify: $( [ "$verify_rc" -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL" )
- OTEL verify: $( [ "$otel_state" = "PASS" ] && echo "✅ PASS" || ( [ "$otel_state" = "FAIL" ] && echo "❌ FAIL" || echo "⏭️ SKIP" ) )
- Smoke (core): $( [ "$smoke_state" = "PASS" ] && echo "✅ PASS" || ( [ "$smoke_state" = "FAIL" ] && echo "❌ FAIL" || echo "⏭️ SKIP" ) )
- Load (k6 quick): $( [ "$load_state" = "PASS" ] && echo "✅ PASS" || ( [ "$load_state" = "FAIL" ] && echo "❌ FAIL" || echo "⏭️ SKIP" ) )

**Artifacts (repo paths)**
- `benchmarks/out/release_gate_report.md`
- `benchmarks/out/release_gate_manifest.json`
- `benchmarks/out/release_gate_pr_comment.md` (this file)
- `benchmarks/out/release_gate_go_no_go_block.md`

**Go/No-Go checklist**
- $v_mark Verify PASS
- $o_mark OTEL verify PASS (optional)
- $s_mark Smoke (core) PASS
- $l_mark Load (k6 quick) PASS (optional)
- [ ] Change approved
- [ ] Rollback plan reviewed
- [ ] Decision: GO / NO-GO
- Approver: ________  Time: ________

<details><summary>Release notes / next actions</summary>

- If **NO-GO**: follow `docs/runbooks/rollout_canary_rollback.md` and `docs/runbooks/incident_response.md`.
- If this is a multi-region change: review `docs/runbooks/regional_failover.md`.

</details>
EOF
}
step() {
  local name="$1"; shift
  local cmd=("$@")
  local log="$OUT_DIR/release_gate_${name// /_}.log"
  local start end rc
  local start_s end_s dur_s

  start="$(ts_utc)"
  start_s="$(epoch_s)"

  printf "
## %s

" "$name" >>"$REPORT"
  printf "- Start: %s
" "$start" >>"$REPORT"
  printf "- Command: `%q`" "${cmd[0]}" >>"$REPORT"
  for i in "${cmd[@]:1}"; do printf " `%q`" "$i" >>"$REPORT"; done
  printf "
" >>"$REPORT"

  set +e
  (cd "$ROOT_DIR" && "${cmd[@]}") >"$log" 2>&1
  rc=$?
  set -e

  end="$(ts_utc)"
  end_s="$(epoch_s)"
  dur_s=$(( end_s - start_s ))

  printf "- End: %s
" "$end" >>"$REPORT"
  printf "- Duration: %ss
" "$dur_s" >>"$REPORT"
  if [ "$rc" -eq 0 ]; then
    printf "- Result: ✅ PASS

" >>"$REPORT"
  else
    printf "- Result: ❌ FAIL (exit %s)

" "$rc" >>"$REPORT"
  fi

  printf "<details><summary>Output (tail)</summary>

```
" >>"$REPORT"
  tail -n 200 "$log" | md_escape >>"$REPORT"
  printf "
```

</details>
" >>"$REPORT"

  return "$rc"
}

main() {
  local overall_rc=0
  local verify_rc=0 otel_rc=0 smoke_rc=0 load_rc=0
  local verify_state="PASS" otel_state="SKIP" smoke_state="PASS" load_state="PASS"
  local skip_smoke="${RHELMA_RELEASE_GATE_SKIP_SMOKE:-0}"
  local skip_load="${RHELMA_RELEASE_GATE_SKIP_LOAD:-0}"
  local otel_enabled="${RHELMA_RELEASE_GATE_OTEL_VERIFY:-0}"
  local required_incomplete=0

  : >"$REPORT"
  {
    echo "# Rhelma6 Release Gate Report"
    echo
    echo "## Decision block"
    echo
    echo "> **GO / NO-GO** (fill when you promote)"
    echo "> - Verify: ⬜ PASS / ⬜ FAIL"
    echo "> - Smoke: ⬜ PASS / ⬜ FAIL"
    echo "> - Load: ⬜ PASS / ⬜ FAIL / ⬜ SKIP"
    echo "> - OTEL verify: ⬜ PASS / ⬜ FAIL / ⬜ SKIP (optional)"
    echo "> - Change approved: ⬜"
    echo "> - Rollback plan reviewed: ⬜"
    echo "> - Decision: ⬜ GO / ⬜ NO-GO"
    echo "> - Approver: ________  Time: ________"
    echo
    echo "## Context"
    echo
    echo "- Generated: $(ts_utc)"
    echo "- Host: $(uname -a)"
    git_line
    echo
    echo "## Tooling"
    echo
    tool_line rustc --version
    tool_line cargo --version
    tool_line k6 version
    tool_line docker --version
    tool_line kubectl version --client --short
    echo
    echo "## What this gate checks"
    echo
    echo "Required:"
    echo "- `scripts/verify.*` (format/lint/tests + repo guards)"
    echo "- `scripts/rhelma6/smoke_core.*` (fast health checks for critical services)"
    echo
    echo "Optional:"
    echo "- Quick k6 load signal (only when `k6` is available)"
    echo "- OTEL propagation regression tests (when RHELMA_RELEASE_GATE_OTEL_VERIFY=1)"
    echo "- You can force-skip load with `RHELMA_RELEASE_GATE_SKIP_LOAD=1`"
    echo
    echo "CI helpers:"
    echo "- Skip smoke (required) with `RHELMA_RELEASE_GATE_SKIP_SMOKE=1` (report becomes INCOMPLETE)"
    echo
    echo "Helpful next steps:"
    echo "- Rollout/Canary/Rollback runbook: `docs/runbooks/rollout_canary_rollback.md`"
    echo "- Incident response: `docs/runbooks/incident_response.md`"
    echo "- Regional failover: `docs/runbooks/regional_failover.md`"
  } >>"$REPORT"

  echo "Running release gate…"

  if ! step "Verify" env RHELMA_VERIFY_OTEL=0 ./scripts/verify.sh; then verify_rc=1; verify_state="FAIL"; overall_rc=1; fi
  if [ "$otel_enabled" = "1" ]; then
    if [ -x "$ROOT_DIR/scripts/verify_otel.sh" ]; then
      if ! step "OTEL verify" env RHELMA_VERIFY_OTEL=1 ./scripts/verify_otel.sh; then otel_rc=1; otel_state="FAIL"; overall_rc=1; else otel_state="PASS"; fi
    else
      otel_state="SKIP"
      {
        echo
        echo "## OTEL verify"
        echo
        echo "- Result: ⏭️ SKIP (scripts/verify_otel.sh not found)"
      } >>"$REPORT"
    fi
  else
    otel_state="SKIP"
    {
      echo
      echo "## OTEL verify"
      echo
      echo "- Result: ⏭️ SKIP (disabled; set RHELMA_RELEASE_GATE_OTEL_VERIFY=1 to enable)"
    } >>"$REPORT"
  fi

  if [ "$skip_smoke" = "1" ]; then
    smoke_state="SKIP"
    required_incomplete=1
    {
      echo
      echo "## Smoke (core)"
      echo
      echo "- Result: ⏭️ SKIP (disabled by RHELMA_RELEASE_GATE_SKIP_SMOKE=1)"
      echo
      echo "Set RHELMA_RELEASE_GATE_SKIP_SMOKE=0 to run the required smoke gate."
    } >>"$REPORT"
  else
    if ! step "Smoke (core)" ./scripts/rhelma6/smoke_core.sh; then smoke_rc=1; smoke_state="FAIL"; overall_rc=1; fi
  fi

  if [ "$skip_load" = "1" ]; then
    load_state="SKIP"
    {
      echo
      echo "## Load (k6 quick)"
      echo
      echo "- Result: ⏭️ SKIP (disabled by RHELMA_RELEASE_GATE_SKIP_LOAD=1)"
      echo
      echo "Set RHELMA_RELEASE_GATE_SKIP_LOAD=0 (and install k6) to include a load signal."
    } >>"$REPORT"
  elif have_cmd k6; then
    if ! step "Load (k6 quick)" ./scripts/rhelma6/load/run_k6_profiles.sh quick both; then load_rc=1; load_state="FAIL"; overall_rc=1; fi
  else
    load_state="SKIP"
    {
      echo
      echo "## Load (k6 quick)"
      echo
      echo "- Result: ⏭️ SKIP (k6 not found in PATH)"
      echo
      echo "Install k6 and re-run to include a load signal."
    } >>"$REPORT"
  fi

  {
    echo
    echo "# Summary"
    echo
    echo "| Gate | Required | Result |"
    echo "|---|---:|---|"
    echo "| Verify | ✅ | $([ "$verify_state" = "PASS" ] && echo "✅ PASS" || echo "❌ FAIL") |"
    echo "| OTEL verify | Optional | $([ "$otel_state" = "PASS" ] && echo "✅ PASS" || ([ "$otel_state" = "FAIL" ] && echo "❌ FAIL" || echo "⏭️ SKIP")) |"
    echo "| Smoke (core) | ✅ | $([ "$smoke_state" = "PASS" ] && echo "✅ PASS" || ([ "$smoke_state" = "FAIL" ] && echo "❌ FAIL" || echo "⏭️ SKIP")) |"
    echo "| Load (k6 quick) | Optional | $([ "$load_state" = "PASS" ] && echo "✅ PASS" || ([ "$load_state" = "FAIL" ] && echo "❌ FAIL" || echo "⏭️ SKIP")) |"
    echo
    if [ "$overall_rc" -eq 0 ] && [ "$required_incomplete" -eq 0 ]; then
      echo "✅ **Recommendation:** GO (all required gates passed)."
    elif [ "$overall_rc" -eq 0 ] && [ "$required_incomplete" -ne 0 ]; then
      echo "⚠️ **Recommendation:** INCOMPLETE (a required gate was skipped by configuration)."
    else
      echo "❌ **Recommendation:** NO-GO (one or more required gates failed)."
    fi
    echo
    echo "Artifacts:"
    echo "- Report: $REPORT"
    echo "- Manifest: $OUT_DIR/release_gate_manifest.json"
    echo "- PR comment snippet: $OUT_DIR/release_gate_pr_comment.md"
    echo "- GO/NO-GO block: $OUT_DIR/release_gate_go_no_go_block.md"
    echo "- Logs: $OUT_DIR/release_gate_*.log"
  } >>"$REPORT"

  echo "Report written to: $REPORT"
  

  # Emit machine-readable manifest + PR-ready snippets
  write_pr_comment "$overall_rc" "$required_incomplete" "$verify_state" "$otel_state" "$smoke_state" "$load_state" "$verify_rc" "$otel_rc" "$smoke_rc"
  write_manifest "$overall_rc" "$required_incomplete" "$verify_state" "$otel_state" "$smoke_state" "$load_state" "$verify_rc" "$otel_rc" "$smoke_rc"
exit "$overall_rc"
}

main "$@"
