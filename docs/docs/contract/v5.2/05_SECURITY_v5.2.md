# Rhelma Zero-Trust Security v5.2

**Release:** January 2027  
**Status:** Final  
**Supersedes:** v5.1 Security

---

## 1. Zero-Trust Principles

**Core Tenet**: *Never Trust, Always Verify*

Every component MUST follow:

1. **Identity is mandatory** (service, workload, user, tenant)
2. **Never trust network location**
3. **Every request must authenticate + authorize**
4. **Least privilege access**
5. **Continuous verification**
6. **Encrypted everywhere**
7. **Secure-by-default workloads**
8. **Audit + tamper-proof logs**
9. **PII obscured and governed**
10. **AI safety integrated at security layer**

---

## 2. Identity Model

### 2.1 Service Identity (SPIFFE/X.509)

All services MUST have cryptographic identity:

**SPIFFE ID Format**:
```
spiffe://rhelma/service/<service-name>
spiffe://rhelma/region/<region>/service/<service-name>
spiffe://rhelma/tenant/<tenant-id>/service/<service-name>

Examples:
spiffe://rhelma/service/api-gateway
spiffe://rhelma/region/eu-central-1/service/ai-orchestrator
spiffe://rhelma/tenant/tenant-123/service/analytics
```

**Certificate Requirements**:
- ✅ Auto-rotation enabled
- ✅ Max lifetime: 7-30 days
- ✅ Issued by trusted CA (Vault, SPIRE, cert-manager)
- ✅ Includes SANs (Subject Alternative Names)

### 2.2 User Identity

**Supported Protocols**:
- OIDC (OpenID Connect)
- OAuth 2.0
- SAML 2.0

**JWT Claims** (Mandatory):

```yaml
JWTClaims:
  # Standard Claims
  sub: string                     # User ID
  iss: string                     # Issuer
  aud: string                     # Audience
  exp: int                        # Expiration (Unix timestamp)
  iat: int                        # Issued at
  nbf: int                        # Not before
  
  # Rhelma Claims
  tenant_id: string
  roles: [string]                 # RBAC roles
  permissions: [string]           # PBAC permissions
  session_id: string
  device_id: string?
  
  # Security
  amr: [string]?                  # Auth methods (password, mfa, biometric)
  acr: string?                    # Auth context class reference
```

**Token Requirements**:
- Access token lifetime: ≤ 15 minutes
- Refresh token lifetime: ≤ 7 days
- Refresh token rotation: On every use

### 2.3 Workload Identity

Kubernetes workloads use ServiceAccount tokens:

```yaml
ServiceAccount:
  name: <service-name>
  namespace: <namespace>
  annotations:
    spiffe.io/identity: spiffe://rhelma/service/<name>
```

---

## 3. Authentication

### 3.1 Service-to-Service (mTLS)

**Requirements**:
- ✅ TLS 1.3 minimum
- ✅ Mutual certificate verification
- ✅ Certificate pinning recommended
- ❌ Self-signed certs forbidden
- ❌ Certificates >30 days forbidden

**Cipher Suites** (Allowed):
```
TLS_AES_256_GCM_SHA384
TLS_AES_128_GCM_SHA256
TLS_CHACHA20_POLY1305_SHA256
```

**Implementation**:
- Istio service mesh (recommended)
- Linkerd
- Consul Connect
- Manual mTLS with cert management

### 3.2 User Authentication

**Multi-Factor Authentication (MFA)**:
- ✅ Required for production access
- ✅ Required for admin operations
- Supported methods: TOTP, WebAuthn, SMS (fallback)

**Session Management**:
- Session timeout: 30 minutes idle, 8 hours absolute
- Concurrent sessions: Limited per user
- Session revocation: Immediate on logout/password change

### 3.3 API Key Authentication

**For service integrations only**:

```yaml
APIKey:
  key_id: uuidv7
  tenant_id: string
  name: string
  permissions: [string]
  created_at: RFC3339
  expires_at: RFC3339
  last_used_at: RFC3339?
  rotation_required: bool
```

**Security Rules**:
- Keys MUST have expiration (max 90 days)
- Keys MUST be prefixed: `rhelma_live_...` or `rhelma_test_...`
- Keys MUST be rotated every 30-60 days
- Unused keys (>30 days) MUST be flagged

---

## 4. Authorization

### 4.1 RBAC (Role-Based Access Control)

**Standard Roles**:

| Role | Permissions | Scope |
|------|-------------|-------|
| `admin` | Full access | Global |
| `developer` | Read/Write code, configs | Per-tenant |
| `operator` | Deploy, scale, monitor | Per-region |
| `analyst` | Read-only metrics, logs | Per-tenant |
| `support` | Read-only, limited PII | Per-tenant |
| `auditor` | Read-only audit logs | Global |

**Custom Roles**:
- Defined per tenant
- Max 50 custom roles per tenant
- Must inherit from base role

### 4.2 PBAC (Permission-Based Access Control)

**Permission Format**:
```
<resource>:<action>

Examples:
invoice:create
invoice:read
invoice:delete
user:impersonate
config:update
ai:execute_command
```

**Policy Evaluation**:
- OPA (Open Policy Agent) recommended
- Rego policy language
- Centralized policy repository

**Example Policy**:

```rego
package rhelma.authz

# Allow invoice creation if user has permission
allow {
  input.user.permissions[_] == "invoice:create"
  input.tenant.status == "active"
}

# Deny cross-tenant access
deny {
  input.resource.tenant_id != input.user.tenant_id
}

# Deny if residency violated
deny {
  input.tenant.residency == "STRICT"
  input.resource.region != input.tenant.region_primary
}
```

### 4.3 Multi-Tenant Isolation

**Mandatory Rules**:
1. No request may access another tenant's resources
2. Cross-tenant requests → HTTP 403 `FORBIDDEN`
3. All queries MUST filter by `tenant_id`
4. Vector DB MUST isolate by tenant namespace
5. Event streams MUST partition by tenant

**Enforcement Points**:
- API Gateway (first line)
- Service layer (defense in depth)
- Database layer (row-level security)
- Vector DB layer (namespace isolation)

---

## 5. Network Security

### 5.1 Network Segmentation

```
┌─────────────────────────────────────────┐
│         Public Internet                  │
└────────────┬────────────────────────────┘
             │ TLS 1.3
             ▼
┌─────────────────────────────────────────┐
│      API Gateway (DMZ)                   │
│  - WAF (Web Application Firewall)       │
│  - DDoS protection                       │
│  - Rate limiting                         │
└────────────┬────────────────────────────┘
             │ mTLS
             ▼
┌─────────────────────────────────────────┐
│    Internal Services Network             │
│  - Service mesh                          │
│  - Zero-trust networking                 │
└────────────┬────────────────────────────┘
             │ mTLS + encrypted
             ▼
┌─────────────────────────────────────────┐
│      Data Layer (L2/L3/L4)              │
│  - Encrypted at rest                     │
│  - Network ACLs                          │
└─────────────────────────────────────────┘
```

**Rules**:
- ❌ NO direct external access to internal services
- ❌ NO public IPs on internal services
- ✅ ALL traffic encrypted (TLS 1.3+)
- ✅ Service mesh for service-to-service

### 5.2 Egress Control

**Allowed Egress**:
- AI Providers (OpenAI, Anthropic, etc.)
- Payment processors (Stripe, PayPal)
- Email gateways (SendGrid, SES)
- Monitoring (DataDog, New Relic)

**Forbidden**:
- Wildcard outbound (0.0.0.0/0)
- Unknown IPs/domains
- Tor exit nodes
- Known malicious IPs

**Violation Event**: `sec.egress.violation`

```yaml
Payload:
  service: string
  destination_ip: string
  destination_port: int
  blocked: bool
  reason: string
  timestamp: RFC3339
```

---

## 6. Encryption

### 6.1 Data in Transit

**Requirements**:
- Protocol: TLS 1.3 (minimum)
- Cipher: AES-256-GCM or ChaCha20-Poly1305
- Perfect Forward Secrecy (PFS): Required
- Certificate validation: Strict

### 6.2 Data at Rest

**Encryption Targets**:
- ✅ Database (L3)
- ✅ Object storage (L4)
- ✅ Vector embeddings
- ✅ Cache (L2) - if sensitive data
- ✅ Backups
- ✅ Logs (if PII present)

**Algorithm**: AES-256

**Key Management**:
- AWS KMS
- Google Cloud KMS
- Azure Key Vault
- HashiCorp Vault

**Key Rotation**:
- Encryption keys: Every 90 days
- Automatic rotation with dual-key period

### 6.3 Field-Level Encryption

For highly sensitive fields (e.g., credit cards):

```yaml
EncryptedField:
  ciphertext: string              # Base64-encoded encrypted data
  algorithm: string               # AES-256-GCM
  key_id: string                  # KMS key reference
  iv: string                      # Initialization vector
```

---

## 7. Secrets Management

### 7.1 Secret Storage

**Approved Backends**:
- HashiCorp Vault (recommended)
- AWS Secrets Manager
- Google Secret Manager
- Azure Key Vault
- Kubernetes Secrets (with encryption at rest)

**Forbidden**:
- ❌ `.env` files with secrets
- ❌ Hardcoded in code
- ❌ Config files in Git
- ❌ Plain text anywhere

### 7.2 Secret Types & Rotation

| Secret Type | Max Age | Rotation Method |
|-------------|---------|-----------------|
| DB password | 30 days | Automated |
| API keys | 30-60 days | Automated |
| JWT signing keys | 90 days | Automated with overlap |
| TLS certificates | Auto-rotation | cert-manager |
| AI provider keys | 30-60 days | Manual with notification |
| Encryption keys | 90 days | KMS auto-rotation |

### 7.3 Secret Injection

**Kubernetes**:
```yaml
# Using External Secrets Operator
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: ai-orchestrator-secrets
spec:
  secretStoreRef:
    name: vault-backend
  target:
    name: ai-orchestrator-secrets
  data:
    - secretKey: openai-api-key
      remoteRef:
        key: ai/openai
        property: api-key
```

**Environment Variables** (Discouraged):
- Only for non-sensitive config
- Secrets MUST come from secret store

---

## 8. Runtime Security

### 8.1 Container Security

**Requirements**:
- ✅ Read-only root filesystem
- ✅ Non-root user (UID > 1000)
- ✅ No privilege escalation
- ✅ Security context constraints
- ✅ Resource limits (CPU, memory)
- ❌ Privileged containers forbidden
- ❌ Host network mode forbidden

**Example Pod Security**:

```yaml
apiVersion: v1
kind: Pod
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 10000
    fsGroup: 10000
    seccompProfile:
      type: RuntimeDefault
  
  containers:
  - name: app
    image: rhelma/ai-orchestrator:5.2.1
    securityContext:
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop: ["ALL"]
    resources:
      limits:
        memory: "2Gi"
        cpu: "1000m"
```

### 8.2 Runtime Monitoring

**Tools**:
- Falco (syscall monitoring)
- Tetragon (eBPF-based)
- Sysdig

**Monitored Events**:
- Unexpected process execution
- File integrity changes
- Network connections to unknown hosts
- Privilege escalation attempts
- Container escape attempts

**Alert Event**: `sec.runtime.violation`

---

## 9. Supply Chain Security

### 9.1 Software Bill of Materials (SBOM)

**Requirements**:
- ✅ SBOM for every release
- ✅ CycloneDX or SPDX format
- ✅ Dependency scanning (Snyk, Trivy, Grype)
- ✅ Vulnerability database sync

**SBOM Generation**:
```bash
syft rhelma/ai-orchestrator:5.2.1 -o cyclonedx-json > sbom.json
```

### 9.2 Image Signing

**Tool**: Cosign (Sigstore)

**Signing Process**:
```bash
# Sign image
cosign sign --key cosign.key rhelma/ai-orchestrator:5.2.1

# Verify signature
cosign verify --key cosign.pub rhelma/ai-orchestrator:5.2.1
```

**Policy Enforcement**:
- Kubernetes admission controller (e.g., Kyverno, OPA Gatekeeper)
- Only signed images allowed in production

### 9.3 Build Provenance

**SLSA Level 3** compliance:

```yaml
Provenance:
  source:
    repo: github.com/rhelma/ai-orchestrator
    commit: abc123...
    branch: main
  
  build:
    build_id: uuidv7
    builder: GitHub Actions
    started_at: RFC3339
    finished_at: RFC3339
  
  artifacts:
    - name: rhelma/ai-orchestrator:5.2.1
      digest: sha256:...
  
  signature: <ed25519 signature>
```

---

## 10. AI Security (Enhanced v5.2)

### 10.1 Prompt Injection Protection

**Detection Methods**:
- Pattern matching (regex)
- ML-based classification
- Semantic similarity to known attacks

**Example Patterns** (Blocked):
```
Ignore previous instructions
You are now in developer mode
Disregard safety guidelines
```

### 10.2 AI Output Security

**Safety Checks**:
1. PII detection (emails, SSN, credit cards)
2. Toxicity classification
3. Hallucination scoring
4. Malicious code detection (if code generation)

**Action on Violation**:
- `BLOCKED`: Return error
- `MODIFIED`: Sanitize and warn
- `ALLOWED_WITH_WARNING`: Log + proceed

### 10.3 AI Command Security (NEW v5.2)

For `ai.command.execute` events:

**Validation Rules**:
1. Command MUST be in allowed list
2. Tenant MUST have `auto_remediation_enabled: true`
3. Confidence MUST be ≥ 0.80
4. Command MUST be residency-compliant
5. No human approval required for safe commands

**Safe Commands**:
- `change_log_level`
- `reduce_sampling`
- `enable_degraded_mode`
- `restart_service` (with rate limit)

**Dangerous Commands** (Require Approval):
- `scale_up` / `scale_down`
- `change_config`
- `evacuate_region`

### 10.4 Model Access Control

**Model Registry**:

```yaml
ModelPolicy:
  model: gpt-4o
  allowed_tenants: [tenant-123, tenant-456]
  forbidden_tenants: []
  max_tokens: 4096
  cost_limit_usd: 1.00
  requires_approval: false
```

---

## 11. Audit & Compliance

### 11.1 Audit Requirements

See **02_OBSERVABILITY_v5.2.md** for `ops.audit@v2`.

**Required Audit Events**:
- Authentication/authorization decisions
- Configuration changes
- Data access (sensitive resources)
- AI command executions (NEW)
- Incident decisions (NEW)
- Permission changes
- Security violations

### 11.2 Compliance Standards

**Supported Frameworks**:
- SOC 2 Type II
- ISO 27001
- GDPR (EU)
- HIPAA (Healthcare)
- PCI DSS (Payment)

**Compliance Controls**:
- Access logs (180 days retention)
- Encryption at rest/transit
- MFA enforcement
- Audit trail immutability
- Incident response procedures

---

## 12. PII Governance Matrix

| Data Type | Allowed? | Encryption | Logging | Residency |
|-----------|----------|------------|---------|-----------|
| Password | ❌ Never | N/A | ❌ Never | N/A |
| JWT token | ❌ Never | N/A | Hash only | N/A |
| Email | ⚠️ Hashed | Yes | ❌ Redacted | Regional |
| Name | ⚠️ | Yes | Redacted | Regional |
| IP address | ✅ | Yes | ✅ Allowed | Regional |
| Credit card | ⚠️ Tokenized | Yes | ❌ Never | Strict |
| SSN/Gov ID | ⚠️ Tokenized | Yes | ❌ Never | Strict |
| AI prompts | ✅ | Yes | ❌ Redacted | Regional |
| AI outputs | ✅ Sanitized | Yes | ✅ Sanitized | Regional |
| Embeddings | ✅ | AES-256 | ❌ Never | Strict |

---

## 13. Incident Response

### 13.1 Security Incident Types

| Type | Severity | Response Time |
|------|----------|---------------|
| Data breach | CRITICAL | < 30 minutes |
| Authentication bypass | CRITICAL | < 1 hour |
| Privilege escalation | HIGH | < 2 hours |
| DDoS attack | MEDIUM | < 30 minutes |
| Malware detection | HIGH | < 1 hour |
| Insider threat | HIGH | < 2 hours |
| AI safety violation | MEDIUM | < 1 hour |

### 13.2 Incident Response Workflow

```
1. Detection (Automated alerts)
2. Containment (Isolate affected systems)
3. Investigation (Forensics, log analysis)
4. Remediation (Patch, rotate secrets)
5. Recovery (Restore normal operations)
6. Post-mortem (RCA, improvements)
```

### 13.3 Forensics Artifact Collection

**Automatically Collect**:
- Logs (7 days before/after)
- Audit trail
- Network traces
- Container images
- Memory dumps (if safe)
- Database snapshots

---

## 14. Security Testing

### 14.1 Required Tests

| Test Type | Frequency | Tool |
|-----------|-----------|------|
| SAST (Static) | Every commit | Semgrep, SonarQube |
| DAST (Dynamic) | Weekly | OWASP ZAP, Burp |
| Dependency scan | Daily | Snyk, Trivy |
| Container scan | Every build | Trivy, Grype |
| Penetration test | Quarterly | External firm |
| Red team exercise | Annually | Internal/external |

### 14.2 Vulnerability Management

**SLA by Severity**:

| Severity | Remediation SLA |
|----------|-----------------|
| Critical | 7 days |
| High | 30 days |
| Medium | 90 days |
| Low | Best effort |

---

## 15. Security Metrics

```
# Authentication failures
auth_failure_total{reason, tenant_id}

# Authorization denials
authz_denied_total{resource, permission}

# Security violations
security_violation_total{type, severity}

# Certificate expiration
cert_expiry_days{service, cert_type}

# Secret age
secret_age_days{secret_type}

# Vulnerability count
vulnerability_count{severity, component}

# NEW: AI safety blocks
ai_safety_block_total{category, severity}

# NEW: Command security violations
ai_command_security_violation_total{command, reason}
```

---

## 16. Compliance Checklist

A system is **Security v5.2 Compliant** if:

✅ Uses mTLS for service-to-service  
✅ Enforces MFA for production access  
✅ Implements RBAC + PBAC  
✅ Encrypts all data (transit & rest)  
✅ Manages secrets in approved backends  
✅ Rotates secrets on schedule  
✅ Uses non-root containers  
✅ Implements runtime security monitoring  
✅ Signs container images  
✅ Produces SBOMs  
✅ Enforces AI safety checks  
✅ Validates AI commands (NEW)  
✅ Maintains cryptographic audit chain  
✅ Conducts regular security testing  
✅ Meets incident response SLAs  

---

**End of Security v5.2**