# Rhelma AI Orchestration v5.2

**Release:** January 2027  
**Status:** Final  
**Supersedes:** v5.1 AI/LLM Orchestrator

This document defines the complete AI infrastructure standard including the **AI-Assisted Incident Decision Engine** (NEW in v5.2).

---

## 1. Purpose

AI capabilities in Rhelma must be:

- **Safe**: Multi-layer safety checks
- **Explainable**: Auditable decision trails
- **Cost-efficient**: Budget-aware routing
- **Tenant-isolated**: Complete separation
- **Auditable**: Cryptographic audit chain
- **Deterministic**: Reproducible results
- **Multi-provider**: No vendor lock-in
- **Multi-modal**: Text, image, audio, video
- **Intelligent**: Self-healing capabilities (NEW)

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Client Request                        │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                   API Gateway                            │
│              (Auth, Rate Limit, Routing)                 │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                AI Orchestrator                           │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Router v3 (Provider Selection)                  │   │
│  └──────────┬──────────────────────────────────────┘   │
│             ▼                                             │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Prompt Registry                                 │   │
│  └──────────┬──────────────────────────────────────┘   │
│             ▼                                             │
│  ┌─────────────────────────────────────────────────┐   │
│  │  RAG Pipeline (8-stage)                          │   │
│  │  ├─ Ingestion                                    │   │
│  │  ├─ Chunking                                     │   │
│  │  ├─ Embedding                                    │   │
│  │  ├─ Indexing                                     │   │
│  │  ├─ Retrieval                                    │   │
│  │  ├─ Re-ranking                                   │   │
│  │  ├─ Synthesis                                    │   │
│  │  └─ Validation                                   │   │
│  └──────────┬──────────────────────────────────────┘   │
│             ▼                                             │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Safety Engine (PII, Toxicity, Hallucination)   │   │
│  └──────────┬──────────────────────────────────────┘   │
│             ▼                                             │
│  ┌─────────────────────────────────────────────────┐   │
│  │  LLM Provider (OpenAI, Anthropic, Custom)       │   │
│  └──────────┬──────────────────────────────────────┘   │
│             ▼                                             │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Cost Governance                                 │   │
│  └──────────┬──────────────────────────────────────┘   │
│             ▼                                             │
│  ┌─────────────────────────────────────────────────┐   │
│  │  ⚡ Incident Decision Engine (NEW v5.2)         │   │
│  │  - Consume ai.incident.proposed                  │   │
│  │  - Analyze with LLM + RAG                        │   │
│  │  - Sanitize output                               │   │
│  │  - Publish ai.incident.decision                  │   │
│  │  - Optional: ai.command.execute                  │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Response Synthesizer                        │
└─────────────────────────────────────────────────────────┘
```

---

## 3. AI Request Envelope v5.2

```yaml
AIRequestV5.2:
  # Identity (from RequestContext)
  request_id: uuidv7
  correlation_id: uuidv7
  tenant_id: string
  user_id: string?
  region: string
  timestamp: RFC3339
  
  # Model Selection
  model:
    preferred: string?             # e.g., "gpt-4o", "claude-sonnet-4"
    forbidden: [string]?           # Blacklist
    allowed_providers: [string]?   # Whitelist
    fallback_chain: [string]?      # Ordered fallback models
  
  # Input (Multi-modal)
  input:
    text: string?
    messages: [ChatMessage]?       # Conversation history
    image: base64?                 # Image data
    audio: base64?                 # Audio data (wav/mp3)
    video: base64?                 # Video data (mp4/webm)
    documents: [Document]?         # PDFs, docs
  
  # RAG Configuration
  retrieval:
    enabled: bool
    top_k: int                     # Number of results
    filters: map                   # Metadata filters
    hybrid_weights:
      vector: float                # Vector similarity weight
      keyword: float               # BM25 weight
      graph: float                 # Graph traversal weight
    rerank: bool                   # Enable re-ranking
  
  # Safety Configuration
  safety:
    mode: enum                     # STRICT | STANDARD | RELAXED
    forbidden_patterns: [string]?  # Regex patterns
    max_risk_score: int            # 0-100
    pii_detection: bool
    toxicity_check: bool
    hallucination_check: bool
  
  # Tool/Function Calling
  tooling:
    allowed_tools: [string]?
    require_explanation: bool
    max_invocations: int
    parallel_execution: bool
  
  # Constraints
  constraints:
    max_tokens: int
    max_cost_usd: float
    max_latency_ms: int
    max_fallbacks: int
    temperature: float?            # 0.0 - 1.0
    top_p: float?                  # Nucleus sampling
  
  # Metadata
  metadata:
    purpose: string?               # use_case description
    tags: [string]?
    priority: enum?                # LOW | MEDIUM | HIGH | CRITICAL
```

---

## 4. Router v3 Architecture

Router v3 provides intelligent model selection based on cost, latency, capabilities, and reliability.

### Routing Pipeline

```
1. Provider Filtering (region, residency, policy)
2. Capability Matching (task requirements)
3. Provider Scoring (cost, latency, reliability)
4. Budget Verification (tenant limits)
5. SLA Enforcement (latency targets)
6. Safety Pre-check (prompt validation)
7. Final Decision
```

### Provider Score Formula

```
Score = (W₁ × CostEfficiency) +
        (W₂ × LatencyConfidence) +
        (W₃ × SafetyRating) +
        (W₄ × Reliability) +
        (W₅ × CapabilityMatch)
```

**Default Weights**:
- W₁ = 0.30 (Cost)
- W₂ = 0.25 (Latency)
- W₃ = 0.20 (Safety)
- W₄ = 0.15 (Reliability)
- W₅ = 0.10 (Capability)

### Routing Decision Log

```yaml
AIRoutingDecisionV5.2:
  provider: string                 # Selected provider
  model: string                    # Selected model
  score: float                     # Decision score
  fallback_chain: [string]         # Ordered fallbacks
  reason: string                   # Selection rationale
  alternatives: [object]           # Other considered options
  decision_time_ms: int
```

---

## 5. Prompt Registry v5.2

### Prompt Schema

```yaml
PromptV5.2:
  # Identity
  prompt_id: uuidv7
  version: int                     # Incremental version
  name: string                     # Human-readable name
  
  # Template
  template: string                 # Prompt template with variables
  variables:
    required: [string]
    optional: [string]
    defaults: map<string, any>
  
  # Safety
  safety:
    risk_level: enum               # LOW | MEDIUM | HIGH | CRITICAL
    safety_prefix: string?         # Prepended safety instructions
    forbidden_patterns: [regex]
  
  # Model Requirements
  model_suitability: [string]      # Compatible models
  min_context_length: int
  expected_tokens_out: int
  
  # Cost & Performance
  cost_class: enum                 # LOW | MEDIUM | HIGH
  avg_latency_ms: int
  
  # Metadata
  tags: [string]
  use_case: string
  created_at: RFC3339
  created_by: string
  approved: bool
```

### Registry API

```
GET  /prompts
GET  /prompts/{id}/v/{version}
POST /prompts
PUT  /prompts/{id}/v/{version}
```

---

## 6. RAG Pipeline v5.2 (8-Stage)

### Stage Overview

```
┌──────────────┐
│ 1. Ingestion │  → Load documents, validate format
└──────┬───────┘
       ▼
┌──────────────┐
│ 2. Chunking  │  → Split into semantic chunks
└──────┬───────┘
       ▼
┌──────────────┐
│ 3. Embedding │  → Generate vector embeddings
└──────┬───────┘
       ▼
┌──────────────┐
│ 4. Indexing  │  → Store in vector + keyword + graph DB
└──────┬───────┘
       ▼
┌──────────────┐
│ 5. Retrieval │  → Hybrid search (vector + keyword + graph)
└──────┬───────┘
       ▼
┌──────────────┐
│ 6. Re-ranking│  → Score and reorder results
└──────┬───────┘
       ▼
┌──────────────┐
│ 7. Synthesis │  → Generate response with context
└──────┬───────┘
       ▼
┌──────────────┐
│ 8. Validation│  → Check hallucination, PII, safety
└──────────────┘
```

### Chunking Configuration

```yaml
ChunkingConfigV5.2:
  mode: enum                       # token | semantic | sliding_window
  max_tokens: int                  # Maximum chunk size
  overlap: int                     # Overlap between chunks
  coherence_threshold: float       # Semantic coherence score
  deterministic: bool              # Reproducible chunking
  preserve_boundaries: bool        # Respect paragraphs/sections
```

### Embedding Object

```yaml
EmbeddingV5.2:
  embedding_id: uuidv7
  tenant_id: string
  modality: enum                   # text | image | audio | video | multimodal
  embedding_version: semver        # Version tracking
  model_version: string            # e.g., text-embedding-3-large
  dimension: int                   # Vector dimensions
  vector: [float]                  # MUST be normalized
  dtype: enum                      # float32 | float16 | int8
  compression: enum                # none | quantized
  similarity: enum                 # cosine | dot | l2
  
  metadata:
    document_id: string?
    source_type: string
    chunk_index: int
    tags: [string]
    timestamp: RFC3339
    residency: enum
  
  source_hash: string              # sha256 of source
  signature: string?               # Optional integrity check
```

### Hybrid Retrieval Score

```
FinalScore = α × VectorScore +
             β × KeywordScore +
             γ × GraphScore

Default: α=0.60, β=0.25, γ=0.15
```

---

## 7. Safety Engine v5.2

### Safety Pipeline

```
Input → Prompt Safety → LLM Call → Output Safety → Validation
         ↓                            ↓
    PII Detection              PII Detection
    Toxicity Check             Toxicity Check
    Pattern Match              Hallucination Check
    Injection Detection        Policy Validation
```

### Safety Result Schema

```yaml
AISafetyResultV5.2:
  status: enum                     # ALLOWED | BLOCKED | MODIFIED
  risk_score: int                  # 0-100
  violations: [object]
    - type: enum                   # PII | TOXICITY | INJECTION | HALLUCINATION
      details: string
      severity: enum               # LOW | MEDIUM | HIGH | CRITICAL
      location: string             # input | output
  
  modifications: [object]?
    - field: string
      original: string
      sanitized: string
  
  sanitized_output: string?
  processing_time_ms: int
```

### Safety Actions

- **ALLOWED**: Pass through unchanged
- **BLOCKED**: Return error `AI_SAFETY_BLOCK`
- **MODIFIED**: Sanitize and continue with warning

---

## 8. Cost Governance v5.2

### Budget Enforcement

```yaml
CostLimits:
  per_request_usd: float
  per_user_daily_usd: float
  per_tenant_monthly_usd: float
  embedding_budget_usd: float
  retrieval_budget_usd: float
```

### Billing Record

```yaml
AIBillingRecordV5.2:
  request_id: uuidv7
  tenant_id: string
  user_id: string?
  
  model: string
  provider: string
  
  tokens_in: int
  tokens_out: int
  embedding_tokens: int
  
  costs:
    llm_usd: float
    embedding_usd: float
    retrieval_usd: float
    total_usd: float
  
  timestamp: RFC3339
  region: string
```

### Cost Violation Handling

- Return HTTP 429 `AI_COST_EXCEEDED`
- Emit event `ai.cost.violation`
- Log CRITICAL with full context
- Notify tenant (email/webhook)

---

## 9. Multi-Modal AI Support

### Supported Modalities

| Modality | Input Format | Use Cases |
|----------|-------------|-----------|
| Text | UTF-8 string | Chat, completion, analysis |
| Image | base64 JPEG/PNG | Vision, OCR, classification |
| Audio | base64 WAV/MP3 | Transcription, analysis |
| Video | base64 MP4/WebM | Frame extraction, analysis |

### Multi-Modal Embeddings

```yaml
embedding_type: text | image | audio | video | multimodal

For multi-modal RAG:
- Image → CLIP embedding
- Audio → Whisper → text embedding
- Video → frame extraction → CLIP embeddings
- Store modality in metadata
```

---

## 10. Function/Tool Calling

### Tool Schema

```yaml
AIToolV5.2:
  name: string                     # Tool identifier
  description: string              # Natural language description
  input_schema: JSONSchema         # Input validation
  output_schema: JSONSchema        # Output validation
  handler: enum                    # URL | internal | lambda
  timeout_ms: int
  max_retries: int
```

### Tool Constraints

- `max_tool_invocations` enforced per request
- Infinite loops prevented via depth limit
- All tool calls audited in `ops.audit@v2`

---

## 11. ⚡ AI-Assisted Incident Decision Engine (NEW v5.2)

### Purpose

Automatically analyze incidents proposed by Observability-Agent and decide on remediation actions.

### Architecture

```
┌─────────────────────────────────────────┐
│     Observability-Agent                  │
│  (Detects anomaly, proposes incident)    │
└──────────────┬──────────────────────────┘
               │ ai.incident.proposed@v1
               ▼
┌─────────────────────────────────────────┐
│   AI Orchestrator - Incident Engine     │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │ 1. Event Consumer                  │ │
│  │    - Consume ai.incident.proposed  │ │
│  │    - Validate payload              │ │
│  └────────┬───────────────────────────┘ │
│           ▼                              │
│  ┌────────────────────────────────────┐ │
│  │ 2. Context Builder                 │ │
│  │    - Fetch service metadata        │ │
│  │    - Retrieve historical incidents │ │
│  │    - Build RAG context             │ │
│  └────────┬───────────────────────────┘ │
│           ▼                              │
│  ┌────────────────────────────────────┐ │
│  │ 3. LLM Analysis                    │ │
│  │    - Load prompt template          │ │
│  │    - Call LLM with context         │ │
│  │    - Parse structured output       │ │
│  └────────┬───────────────────────────┘ │
│           ▼                              │
│  ┌────────────────────────────────────┐ │
│  │ 4. Safety & Sanitization           │ │
│  │    - PII detection                 │ │
│  │    - Redact sensitive data         │ │
│  │    - Validate reasoning            │ │
│  └────────┬───────────────────────────┘ │
│           ▼                              │
│  ┌────────────────────────────────────┐ │
│  │ 5. Decision Publisher              │ │
│  │    - Publish ai.incident.decision  │ │
│  │    - Optional: ai.command.execute  │ │
│  │    - Audit via ops.audit@v2        │ │
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│     Observability-Agent                  │
│  (Receives decision, may execute cmd)    │
└─────────────────────────────────────────┘
```

### Workflow Steps

#### Step 1: Consume Incident Proposal

```rust
// Pseudo-code
async fn handle_incident_proposed(event: IncidentProposedEvent) {
    // Validate event
    validate_event_schema(&event)?;
    
    // Check idempotency
    if already_processed(&event.incident_id) {
        return Ok(());
    }
    
    // Store in IncidentStore
    store_incident(&event).await?;
    
    // Trigger analysis
    analyze_incident(&event).await?;
}
```

#### Step 2: Build Analysis Context

```yaml
AnalysisContext:
  incident: IncidentProposedEvent
  
  service_metadata:
    name: string
    version: string
    dependencies: [string]
    recent_deployments: [object]
  
  historical_incidents:
    similar_incidents: [object]   # RAG from vector DB
    resolution_patterns: [string]
  
  system_state:
    cpu_usage: float
    memory_usage: float
    error_rate: float
    latency_p99: float
  
  tenant_policy:
    auto_remediation_enabled: bool
    allowed_actions: [string]
    escalation_threshold: float
```

#### Step 3: LLM Prompt Template

```yaml
PromptTemplate: incident_analysis_v1

System:
You are an expert SRE analyzing a production incident.
Analyze the incident and recommend actions based on historical patterns.

IMPORTANT:
- Never include PII, passwords, or tokens
- Provide confidence score (0.0-1.0)
- Recommend only safe, reversible actions
- If confidence < 0.8, escalate to human

Context:
Service: {{service.name}}
Region: {{region}}
Severity: {{incident.severity}}
Metrics: {{metrics}}
Similar past incidents: {{historical_incidents}}

Question:
What is the root cause and recommended action?

Output JSON:
{
  "final_severity": "info|warning|critical",
  "root_cause": "brief description",
  "recommended_action": "specific action or null",
  "reasoning": "explanation",
  "confidence": 0.0-1.0,
  "escalate": true|false
}
```

#### Step 4: Sanitize LLM Output

```rust
async fn sanitize_llm_response(response: String) -> Result<SanitizedResponse> {
    // Parse JSON
    let parsed: LLMDecision = serde_json::from_str(&response)?;
    
    // PII detection
    if contains_pii(&parsed.reasoning) {
        return Err(Error::PIIDetected);
    }
    
    // Redact sensitive patterns
    let sanitized_reasoning = redact_patterns(&parsed.reasoning);
    
    // Validate confidence
    if parsed.confidence < 0.0 || parsed.confidence > 1.0 {
        return Err(Error::InvalidConfidence);
    }
    
    Ok(SanitizedResponse {
        final_severity: parsed.final_severity,
        recommended_action: parsed.recommended_action,
        reasoning: sanitized_reasoning,
        confidence: parsed.confidence,
    })
}
```

#### Step 5: Publish Decision

```rust
async fn publish_decision(incident_id: String, decision: SanitizedResponse) {
    // Build decision event
    let decision_event = IncidentDecisionEvent {
        incident_id,
        final_severity: decision.final_severity,
        recommended_action: decision.recommended_action,
        reasoning: decision.reasoning,
        confidence: decision.confidence,
        generated_at: Utc::now(),
        model_used: "gpt-4o",
        processing_time_ms: 1234,
    };
    
    // Publish to event bus
    event_bus.publish("ai.incident.decision", &decision_event).await?;
    
    // If action recommended and confidence high
    if let Some(action) = decision.recommended_action {
        if decision.confidence >= 0.8 {
            publish_command(&incident_id, &action).await?;
        }
    }
    
    // Audit trail
    audit_decision(&decision_event).await?;
}
```

### Incident Store

```yaml
IncidentStoreV5.2:
  incident_id: uuidv7               # Primary key
  service: string
  region: string
  severity: enum
  status: enum                      # proposed | analyzed | resolved | escalated
  
  proposed_at: RFC3339
  analyzed_at: RFC3339?
  resolved_at: RFC3339?
  
  llm_confidence: float?
  recommended_action: string?
  reasoning: string?
  
  commands_executed: [string]
  
  metadata: object
```

### Decision Confidence Thresholds

| Confidence | Action |
|------------|--------|
| ≥ 0.80 | Auto-remediate (if safe) |
| 0.50 - 0.79 | Suggest action, await approval |
| < 0.50 | Escalate to human |

### Safety Constraints

**Never auto-execute** if:
- Confidence < 0.80
- Tenant has `auto_remediation_enabled: false`
- Action is in forbidden list
- Incident severity is CRITICAL (require human approval)
- Similar incident recently failed remediation

### Metrics

```
ai_incident_analyzed_total{outcome}
ai_incident_analysis_duration_seconds
ai_incident_confidence_score{bucket}
ai_incident_auto_remediated_total
ai_incident_escalated_total
ai_incident_decision_errors_total
```

---

## 12. Observability Requirements

### Spans

- `ai.router.decision`
- `ai.prompt.load`
- `ai.rag.retrieval`
- `ai.embedding.generate`
- `ai.llm.call`
- `ai.safety.check`
- `ai.incident.analyze` (NEW)
- `ai.incident.decision` (NEW)

### Metrics

```
ai_request_total{model, provider, status}
ai_request_duration_seconds{model, provider}
ai_tokens_input_total{model, provider, tenant_id}
ai_tokens_output_total{model, provider, tenant_id}
ai_cost_total_usd{model, provider, tenant_id}
ai_fallback_total{reason}
ai_safety_block_total{category}
ai_tool_call_total{tool_name, model}
```

---

## 13. SLA Requirements

| Metric | Target | Reference |
|--------|--------|-----------|
| LLM call p99 | < 2500ms | A1_SLA_MATRIX |
| RAG pipeline p99 | < 500ms | A1_SLA_MATRIX |
| Vector search p99 | < 100ms | A1_SLA_MATRIX |
| Embedding gen p95 | < 150ms | A1_SLA_MATRIX |
| Safety check | < 120ms | A1_SLA_MATRIX |
| Failure rate | < 0.5% | A1_SLA_MATRIX |
| Incident analysis | < 10s | NEW |

---

## 14. Compliance Checklist

A system is **AI Orchestration v5.2 Compliant** if:

✅ Implements Router v3  
✅ Uses Prompt Registry  
✅ Implements 8-stage RAG pipeline  
✅ Enforces Safety Engine  
✅ Tracks cost governance  
✅ Supports multi-modal AI  
✅ Implements Incident Decision Engine (NEW)  
✅ Signs audit events (ops.audit@v2)  
✅ Meets SLA targets  
✅ Passes security audit  

---

**End of AI Orchestration v5.2**