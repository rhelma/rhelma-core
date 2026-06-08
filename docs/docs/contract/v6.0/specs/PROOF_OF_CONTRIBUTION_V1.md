# Proof of Contribution v1

## Objective
Reward nodes for real value, not identity count.

## Signals
- Uptime (validated by heartbeats + peer observation)
- Successful job completions
- Low error rate
- Low latency (bounded by region)
- Challenge task pass rate

## Reward policy
- Credits multiplier depends on lane:
  - Lane A: 1.0x
  - Lane B: 1.5x
  - Lane C: 2.0x

## Penalties
- False reporting: reputation drop + credit clawback
- Repeated failures: dampening → quarantine (Phase 11)
