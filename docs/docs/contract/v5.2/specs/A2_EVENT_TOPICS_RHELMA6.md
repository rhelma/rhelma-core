# Rhelma6 — Annex A2: Event Topics for the Node Swarm

**Status:** Draft

This annex defines canonical topics for node lifecycle and swarm operations.

Topics should follow the general Rhelma taxonomy style:

- `realm.node.*`
- `realm.swarm.*`
- `realm.gov.*`
- `realm.value.*`

## Node lifecycle

- `realm.node.registered`
- `realm.node.verified`
- `realm.node.heartbeat`
- `realm.node.suspended`
- `realm.node.retired`

## Swarm routing

- `realm.swarm.route.request`
- `realm.swarm.route.decision`
- `realm.swarm.task.assigned`
- `realm.swarm.task.result`

## Governance

- `realm.gov.policy.published`
- `realm.gov.key.rotated`
- `realm.gov.log.appended`

## Value / incentives

- `realm.value.reputation.updated`
- `realm.value.credit.issued`
- `realm.value.credit.spent`

## Security signals

- `realm.security.attestation.failed`
- `realm.security.sybil.suspected`
- `realm.security.policy.violation`
