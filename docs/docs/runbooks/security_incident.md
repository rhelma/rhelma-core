# Security Incident Runbook (Rhelma6)

This runbook provides the first 60 minutes of actions for suspected security incidents.

---

## Severity levels

- **SEV1**: confirmed data exfiltration, credential compromise, malicious code execution
- **SEV2**: suspected compromise, repeated auth bypass attempts, unexpected privilege escalation
- **SEV3**: suspicious behavior, scanner activity, policy misconfiguration

---

## First 15 minutes (containment)

1) **Declare incident** and start a timeline.
2) **Freeze changes**:
   - pause deploys (CI/CD) and disable auto-sync (e.g., ArgoCD) if present
3) **Enable Safe Mode** (if applicable):
   - set `RHELMA_GOVERNANCE_SAFE_MODE=1`
4) **Rotate and revoke**:
   - admin tokens
   - any exposed governance HS256 secrets
   - OAuth/JWT signing keys if risk indicates
5) **Preserve evidence**:
   - snapshot logs and audit streams
   - export relevant Kafka topics offsets and consumer states

---

## Investigation checklist

### Authn/Authz
- Check for unusual spikes in:
  - `401`, `403`, `429`
  - admin endpoints access
- Validate JWT issuer/audience
- Review RBAC decision logs (if enabled)

### Governance
- Verify active policy bundle hash and signers
- Confirm no unexpected high-impact policy activation
- Review `/v1/governance/history`

### Infrastructure
- Identify pods/nodes with:
  - unexpected restarts
  - unexpected outbound traffic
  - privilege escalation (hostPath, privileged containers)

---

## Recovery

- Remove malicious changes
- Patch vulnerabilities / rotate credentials
- Re-enable deploys **only** after approval
- Post-incident: root cause + prevention work items
