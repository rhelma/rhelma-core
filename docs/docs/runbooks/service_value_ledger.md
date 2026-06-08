# Service Runbook: Value Ledger (Rhelma6)

## What this service does

Value Ledger stores and serves credit/value state. The federation component coordinates snapshots and replication.

## Primary signals

- Health/readiness endpoints for ledger and federation
- Snapshot lag metrics and error rate
- Symptoms: balance mismatches, failed transactions, snapshot drift

## Quick triage

```bash
kubectl get pods -n rhelma6 -l app=value-ledger -o wide
kubectl get pods -n rhelma6 -l app=value-ledger-federation -o wide
kubectl logs -n rhelma6 -l app=value-ledger --tail=200
kubectl logs -n rhelma6 -l app=value-ledger-federation --tail=200
```

## Fast recovery actions

- restart federation to force a fresh sync:
  ```bash
  kubectl rollout restart deploy/value-ledger-federation -n rhelma6
  kubectl rollout status deploy/value-ledger-federation -n rhelma6 --timeout=180s
  ```

- restart ledger if it is crashlooping:
  ```bash
  kubectl rollout restart deploy/value-ledger -n rhelma6
  kubectl rollout status deploy/value-ledger -n rhelma6 --timeout=180s
  ```

## Consistency validation

For a small set of subjects, compare balances across peers (service endpoints may differ by deployment):

```bash
for peer in vlf-0 vlf-1 vlf-2; do
  echo "=== $peer ==="
  curl -s "http://${peer}:8098/v1/credits/subject_123" | jq .balance
done
```

If inconsistencies persist, reduce write traffic until the issue is understood.

## Evidence collection

- capture the subject_id(s) affected
- snapshot federation status output and recent ledger logs
- capture any governance policy changes near the incident window
