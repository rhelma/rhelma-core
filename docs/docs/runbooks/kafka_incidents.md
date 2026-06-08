# Kafka Incidents Runbook (Rhelma6)

This runbook covers common Kafka operational failures that impact Rhelma6.

---

## Symptoms → likely causes

| Symptom | Likely causes |
|---|---|
| Consumer lag spikes | broker slow / partition reassign / consumer group rebalance / network |
| Produce timeouts | broker overload / ISR shrink / auth failure / network |
| "Not leader" errors | controller election / partition leadership changes |
| Rebalances every few minutes | unstable members / session timeout too low / pod restarts |

---

## Quick triage

1) **Check broker health**
```bash
kubectl get pods -n kafka
kubectl top pods -n kafka
```

2) **Check consumer lag (example: ai-orchestrator group)**
```bash
kafka-consumer-groups.sh --bootstrap-server $KAFKA --describe --group ai-orchestrator
```

3) **Check recent broker logs**
```bash
kubectl logs -n kafka -l app=kafka --tail=200
```

---

## Runbook: produce timeouts (SEV2)

### Detection
- App logs contain `KafkaError::MessageProduction` or `TimedOut`
- Metrics: increase in `kafka_produce_errors_total` and request latency

### Actions
1) Reduce burst load (if safe)
- temporarily scale down noisy producers
- pause non-essential batch jobs

2) Confirm ISR / under-replicated partitions
```bash
kafka-topics.sh --bootstrap-server $KAFKA --describe | grep -E "UnderReplicated|ISR"
```

3) If ISR is shrinking: check disk and network
```bash
kubectl top nodes
kubectl describe node <node>
```

### Recovery
- If brokers are overloaded, scale Kafka or increase resources
- If one broker is unhealthy, cordon/drain and replace

---

## Runbook: consumer lag runaway (SEV2)

### Detection
- Lag increasing consistently for > 5 minutes

### Actions
1) Check consumer error logs
```bash
kubectl logs -n rhelma6 deploy/ai-orchestrator --tail=200
```

2) If consumer is CPU bound, scale replicas
```bash
kubectl scale -n rhelma6 deploy/ai-orchestrator --replicas=3
```

3) If poison messages suspected
- enable safe mode policy restrictions
- route to a dead-letter topic (if configured)

---

## Post-incident checklist
- [ ] Identify whether the incident was broker, network, or client behavior
- [ ] Record partitions/groups affected
- [ ] Add alert thresholds tuned to observed baselines
- [ ] Add load shedding controls where needed
