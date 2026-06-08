# Rhelma 6 Load / Smoke Scripts

- `k6_smoke.js` is a template.
- Set `RHELMA6_BASE` to point at your gateway or specific service.

Example:
```bash
RHELMA6_BASE=http://127.0.0.1:8090 k6 run scripts/rhelma6/load/k6_smoke.js
```

## Profile runner

If you have `k6` installed locally, you can run the repo's standard profiles:

```bash
# assumes api-gateway on :8080 and node-registry on :9010
./scripts/rhelma6/load/run_k6_profiles.sh quick both
./scripts/rhelma6/load/run_k6_profiles.sh standard gateway
```

## Baseline comparison

After running a profile (or CI), compare a k6 summary against the repo baseline:

```bash
python3 scripts/rhelma6/load/k6_compare_to_baseline.py \
  benchmarks/out/k6_gateway_compare.md \
  benchmarks/out/k6_gateway_summary.json \
  benchmarks/baselines/k6_gateway_baseline.json \
  --name "api-gateway" \
  --max-p95-regression-pct 20 \
  --max-failed-rate 0.01
```

Update baseline templates under `benchmarks/baselines/` after your first stable run.
