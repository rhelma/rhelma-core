# Rhelma6 Runbooks

This directory contains **production-facing runbooks** for operating the Rhelma6 platform.

## Index

- [Incident Response](./incident_response.md)
- [Disaster Recovery](./disaster_recovery.md)
- [Regional Failover](./regional_failover.md)
- [Multi-Region Failover Drill](./multi_region_failover_drill.md)
- [Kafka Incidents](./kafka_incidents.md)
- [Security Incident Response](./security_incident.md)
- [Governance Policy Activation](./governance_policy_activation.md)
- [Capacity Planning](./capacity_planning.md)
- [Launch Readiness Checklist](./launch_readiness_checklist.md)
- [Rollout, Canary, and Rollback](./rollout_canary_rollback.md)
- [Release Gate](./release_gate.md)

## Service runbooks

- [Admin Web](./service_admin_web.md)
- [Agent Handoff](./service_agent_handoff.md)
- [AI Companion](./service_ai_companion.md)
- [AI Orchestrator](./service_ai_orchestrator.md)
- [API Gateway](./service_api_gateway.md)
- [Bridge Adapter](./service_bridge_adapter.md)
- [Digital Family Vault](./service_digital_family_vault.md)
- [Edge Worker](./service_edge_worker.md)
- [File Storage](./service_file_storage.md)
- [Gossip Discovery](./service_gossip_discovery.md)
- [Guardian Agent](./service_guardian_agent.md)
- [Rhelma Attestation Verifier](./service_rhelma_attestation_verifier.md)
- [Rhelma Bridge Drivers](./service_rhelma_bridge_drivers.md)
- [Rhelma Governance Signer](./service_rhelma_governance_signer.md)
- [Rhelma Node](./service_rhelma_node.md)
- [Rhelma Sandbox Runner](./service_rhelma_sandbox_runner.md)
- [MNI Rag](./service_mni_rag.md)
- [Multi Frontend](./service_multi_frontend.md)
- [Node Registry](./service_node_registry.md)
- [Patch Applier](./service_patch_applier.md)
- [Realm Hub](./service_realm_hub.md)
- [Realtime Service](./service_realtime_service.md)
- [Region Health Aggregator](./service_region_health_aggregator.md)
- [Sandbox Runner](./service_sandbox_runner.md)
- [Search Service](./service_search_service.md)
- [Security Governance](./service_security_governance.md)
- [Value Ledger](./service_value_ledger.md)
- [Value Ledger Federation](./service_value_ledger_federation.md)
- [Web](./service_web.md)

## Conventions

- **SEV levels** follow the Incident Response playbook.
- All commands assume a Kubernetes namespace called `rhelma6`.
- Prefer **observability first**: dashboards + logs + recent events, then take action.
- When in doubt: **stop the bleeding**, then collect evidence, then recover.
