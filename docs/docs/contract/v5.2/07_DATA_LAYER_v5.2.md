# Rhelma Data Layer v5.2

**Release:** January 2027  
**Status:** Final  
**Supersedes:** v5.1 Vector/Graph/Embeddings

This document defines the unified data layer specification covering vector databases, graph databases, embeddings lifecycle, hybrid retrieval, and multi-modal AI data management.

---

## 1. Purpose

The Rhelma Data Layer enables:

- ✅ RAG (Retrieval-Augmented Generation) at scale
- ✅ Semantic search across modalities
- ✅ Hybrid retrieval (vector + keyword + graph)
- ✅ Multi-modal embeddings (text/image/audio/video)
- ✅ Relationship modeling via graphs
- ✅ Contextual memory & personalization
- ✅ High availability & replication
- ✅ Security & residency compliance

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────┐
│              Application Layer                   │
│         (AI Orchestrator, Services)              │
└──────────────────┬──────────────────────────────┘
                   │
       ┌───────────┼───────────┐
       │           │           │
       ▼           ▼           ▼
┌──────────┐ ┌──────────┐ ┌──────────┐
│  Vector  │ │  Graph   │ │ Keyword  │
│    DB    │ │    DB    │ │  Search  │
│ (Qdrant) │ │ (Neo4j)  │ │(Elastic) │
└──────────┘ └──────────┘ └──────────┘
                   │
                   ▼
         ┌─────────────────┐
         │  Hybrid Query   │
         │    Engine       │
         └─────────────────┘
```

---

## 3. Vector Database (L3)

### 3.1 Approved Implementations

| Database | Status | Use Case |
|----------|--------|----------|
| **Qdrant** | ✅ Recommended | General purpose, open-source |
| **Weaviate** | ✅ Approved | Multi-modal, GraphQL API |
| **Milvus** | ✅ Approved | Large scale, distributed |
| **Pinecone** | ⚠️ SaaS only | Managed service |

### 3.2 Required Features

All compliant vector databases MUST support:

1. ✅ **HNSW or DiskANN** indexing
2. ✅ **Metadata filtering** (tenant_id, tags, etc.)
3. ✅ **Multi-tenant isolation** (namespaces/collections)
4. ✅ **AES-256 encryption** at rest
5. ✅ **Batch operations** (insert, delete, update)
6. ✅ **Snapshots** for backup
7. ✅ **WAL** (Write-Ahead Log) durability
8. ✅ **Multi-region replication** (where allowed)
9. ✅ **Top-K search** with p99 < 100ms
10. ✅ **Graceful index rebuilds** (zero downtime)

### 3.3 Vector Index Configuration

```yaml
VectorIndexConfig:
  # HNSW Parameters
  index_type: HNSW                # or DiskANN
  m: int                          # HNSW: 16-48 connections
  ef_construction: int            # HNSW: 256-512
  ef_search: int                  # Query-time: 128-256
  
  # Quantization (optional)
  quantization:
    enabled: bool
    type: enum                    # scalar | product | binary
    compression_ratio: float      # e.g., 0.25 for 4x compression
  
  # Performance
  max_indexing_threads: int
  on_disk_payload: bool           # Store payloads on disk
```

**Recommended Settings**:

```yaml
# Small scale (< 1M vectors)
m: 16
ef_construction: 200
ef_search: 100

# Medium scale (1M - 10M vectors)
m: 32
ef_construction: 400
ef_search: 150

# Large scale (> 10M vectors)
m: 48
ef_construction: 512
ef_search: 200
quantization: enabled
```

---

## 4. Embeddings v5.2

### 4.1 Embedding Schema

```yaml
EmbeddingV5.2:
  # Identity
  embedding_id: uuidv7
  tenant_id: string
  
  # Modality
  modality: enum                  # text | image | audio | video | multimodal
  
  # Model & Version
  model_version: string           # e.g., "text-embedding-3-large"
  embedding_version: semver       # Semantic version
  
  # Vector Data
  dimension: int                  # 384, 768, 1024, 1536, 3072
  vector: [float]                 # MUST be L2-normalized
  dtype: enum                     # float32 | float16 | int8
  compression: enum               # none | quantized
  similarity: enum                # cosine | dot | euclidean
  
  # Source
  source_hash: string             # sha256 of source data
  source_type: string             # document | chunk | image | audio
  chunk_index: int?               # For chunked documents
  
  # Metadata
  metadata:
    document_id: string?
    tags: [string]
    timestamp: RFC3339
    residency: enum               # GLOBAL | REGIONAL_STRICT
    language: string?             # ISO 639-1
    graph_entities: [string]?     # Linked entities
  
  # Security
  encryption_key_id: string?
  signature: string?              # Integrity check
```

### 4.2 Vector Normalization

**CRITICAL**: All vectors MUST be L2-normalized for cosine similarity.

```python
import numpy as np

# Normalization
def normalize_vector(v):
    norm = np.linalg.norm(v)
    if norm == 0:
        return v
    return v / norm

# Verification
def is_normalized(v):
    return np.isclose(np.linalg.norm(v), 1.0, atol=1e-6)
```

**Dimension Mismatch**:
- MUST reject vectors with wrong dimension
- MUST return error `VECTOR_DIMENSION_MISMATCH`

### 4.3 Embedding Generation

```yaml
EmbeddingRequest:
  input: string | bytes           # Text or binary data
  modality: enum
  model: string                   # Model identifier
  normalize: bool                 # Default: true
  
  # Advanced
  truncate: bool                  # Truncate long inputs
  pooling: enum?                  # mean | cls | max (for transformers)
```

**Supported Models**:

| Model | Dimension | Modality | Provider |
|-------|-----------|----------|----------|
| text-embedding-3-small | 1536 | text | OpenAI |
| text-embedding-3-large | 3072 | text | OpenAI |
| voyage-2 | 1024 | text | Voyage AI |
| clip-vit-large | 768 | text+image | OpenAI CLIP |
| whisper-embedding | 1024 | audio→text | OpenAI |

---

## 5. Chunking Strategy

### 5.1 Chunking Configuration

```yaml
ChunkingConfigV5.2:
  # Mode
  mode: enum                      # token | semantic | sliding_window
  
  # Token-based
  max_tokens: int                 # Max chunk size
  overlap: int                    # Overlap in tokens
  
  # Semantic
  coherence_threshold: float      # 0.0-1.0
  min_chunk_size: int
  max_chunk_size: int
  
  # Settings
  deterministic: bool             # Reproducible chunking
  preserve_boundaries: bool       # Respect paragraphs/sentences
  
  # Metadata
  include_context: bool           # Add surrounding context
  context_window: int             # Tokens of context
```

### 5.2 Chunking Strategies

#### Token-Based Chunking

```python
def chunk_by_tokens(text: str, max_tokens: int, overlap: int):
    tokens = tokenize(text)
    chunks = []
    
    for i in range(0, len(tokens), max_tokens - overlap):
        chunk = tokens[i:i + max_tokens]
        chunks.append(chunk)
    
    return chunks
```

#### Semantic Chunking

```python
def chunk_semantically(text: str, config: ChunkingConfig):
    sentences = split_sentences(text)
    chunks = []
    current_chunk = []
    
    for sent in sentences:
        current_chunk.append(sent)
        
        if len(current_chunk) >= config.min_chunk_size:
            coherence = calculate_coherence(current_chunk)
            
            if coherence < config.coherence_threshold:
                chunks.append(join(current_chunk))
                current_chunk = [sent]  # Start new chunk
    
    if current_chunk:
        chunks.append(join(current_chunk))
    
    return chunks
```

**Determinism**: Chunking MUST NOT change unless:
- Model version changes
- Chunking config changes
- Document content changes

### 5.3 Chunk Metadata

```yaml
ChunkMetadata:
  chunk_id: uuidv7
  document_id: string
  chunk_index: int
  total_chunks: int
  
  # Content
  tokens: int
  characters: int
  
  # Boundaries
  start_position: int             # Character offset in original
  end_position: int
  
  # Context
  previous_chunk_id: string?
  next_chunk_id: string?
```

---

## 6. Hybrid Retrieval

### 6.1 Retrieval Modes

| Mode | Components | Use Case |
|------|------------|----------|
| **Vector Only** | Cosine similarity | Semantic search |
| **Keyword Only** | BM25 | Exact term matching |
| **Hybrid** | Vector + BM25 | Best of both |
| **Graph-Enhanced** | Vector + BM25 + Graph | Contextual search |

### 6.2 Hybrid Score Formula

```
FinalScore = α × VectorScore + β × KeywordScore + γ × GraphScore

Where:
- α, β, γ are weights (sum to 1.0)
- VectorScore ∈ [0, 1] (cosine similarity)
- KeywordScore ∈ [0, 1] (normalized BM25)
- GraphScore ∈ [0, 1] (PageRank or centrality)
```

**Default Weights**:
- α = 0.60 (Vector)
- β = 0.25 (Keyword)
- γ = 0.15 (Graph)

**Weight Tuning** (per tenant):

```yaml
RetrievalWeights:
  tenant_id: string
  use_case: string
  weights:
    vector: 0.60
    keyword: 0.25
    graph: 0.15
  
  # Dynamic adjustment
  auto_tune: bool
  performance_metric: enum        # ndcg | precision | recall
```

### 6.3 Re-ranking

After initial retrieval, optionally re-rank results:

```yaml
ReRankingConfig:
  enabled: bool
  model: string                   # cross-encoder model
  top_k_to_rerank: int            # Re-rank top N results
  final_top_k: int                # Return final N
```

**Re-ranker Models**:
- `ms-marco-MiniLM-L-12-v2` (fast)
- `cross-encoder/ms-marco-electra-base` (accurate)

---

## 7. Multi-Modal Embeddings

### 7.1 Modality Support

#### Text
```yaml
TextEmbedding:
  input: string
  model: text-embedding-3-large
  dimension: 3072
```

#### Image
```yaml
ImageEmbedding:
  input: base64                   # JPEG/PNG
  model: clip-vit-large-patch14
  dimension: 768
```

#### Audio
```yaml
AudioEmbedding:
  input: base64                   # WAV/MP3
  model: whisper-large
  transcription: string           # Intermediate text
  dimension: 1024
```

#### Video
```yaml
VideoEmbedding:
  input: base64                   # MP4/WebM
  frame_rate: int                 # Frames per second to extract
  model: clip-vit-large-patch14
  
  # Output: Array of frame embeddings
  frame_embeddings: [
    {
      timestamp_ms: int
      embedding: [float]
    }
  ]
```

### 7.2 Multi-Modal Search

**Query Across Modalities**:

```yaml
MultiModalQuery:
  query_text: string?
  query_image: base64?
  query_audio: base64?
  
  # Target modalities
  search_modalities: [text, image, audio, video]
  
  # Weights per modality
  modality_weights:
    text: 0.4
    image: 0.4
    audio: 0.1
    video: 0.1
  
  top_k: int
```

---

## 8. Graph Database (Optional)

### 8.1 Supported Backends

| Database | Status | Query Language |
|----------|--------|----------------|
| **Neo4j** | ✅ Recommended | Cypher |
| **Memgraph** | ✅ Approved | Cypher |
| **TigerGraph** | ⚠️ Advanced | GSQL |

### 8.2 Graph Schema

```yaml
# Nodes
GraphNode:
  id: string
  type: enum                      # document | entity | topic | chunk
  properties: map<string, any>
  
  # Examples:
  - type: document
    properties:
      title: string
      created_at: timestamp
  
  - type: entity
    properties:
      name: string
      category: string

# Edges
GraphEdge:
  from_node: string
  to_node: string
  type: enum                      # REFERS_TO | MENTIONS | SUPPORTS | 
                                  # SUMMARIZES | NEXT_CHUNK | EMBEDS
  weight: float                   # Relationship strength
  properties: map<string, any>
  timestamp: RFC3339
```

### 8.3 Common Patterns

#### Document Chunking Graph

```cypher
CREATE (d:Document {id: 'doc-123', title: 'Rhelma Architecture'})
CREATE (c1:Chunk {id: 'chunk-1', content: '...'})
CREATE (c2:Chunk {id: 'chunk-2', content: '...'})
CREATE (d)-[:HAS_CHUNK {index: 0}]->(c1)
CREATE (d)-[:HAS_CHUNK {index: 1}]->(c2)
CREATE (c1)-[:NEXT_CHUNK]->(c2)
```

#### Entity Relationships

```cypher
CREATE (d:Document {id: 'doc-123'})
CREATE (e1:Entity {name: 'Kubernetes', type: 'Technology'})
CREATE (e2:Entity {name: 'Docker', type: 'Technology'})
CREATE (d)-[:MENTIONS]->(e1)
CREATE (d)-[:MENTIONS]->(e2)
CREATE (e1)-[:USES {weight: 0.9}]->(e2)
```

### 8.4 Graph-Enhanced Retrieval

```cypher
// Find documents related to "Kubernetes" via entities
MATCH (d:Document)-[:MENTIONS]->(e1:Entity {name: 'Kubernetes'})
MATCH (e1)-[:RELATED_TO]-(e2:Entity)
MATCH (d2:Document)-[:MENTIONS]->(e2)
WHERE d.id != d2.id
RETURN d2.id, d2.title, count(e2) as relevance
ORDER BY relevance DESC
LIMIT 10
```

---

## 9. Embedding Regeneration

### 9.1 Regeneration Triggers

Regeneration MUST occur when:

| Trigger | Version Bump |
|---------|--------------|
| Document content changes | PATCH |
| Chunking config changes | MINOR |
| Model changes | MINOR |
| Dimension changes | MAJOR |
| Source hash mismatch | PATCH |

### 9.2 Regeneration Event

```yaml
Topic: vector.regenerated@v1

Payload:
  embedding_id: uuidv7
  tenant_id: string
  old_version: string             # e.g., 1.2.3
  new_version: string             # e.g., 1.3.0
  reason: enum                    # model_upgrade | doc_changed | config_changed
  regenerated_at: RFC3339
  
  # Metrics
  old_dimension: int
  new_dimension: int
  cost_usd: float?
```

### 9.3 Regeneration Strategy

**Batch Regeneration**:

```yaml
RegenerationJob:
  job_id: uuidv7
  tenant_id: string?              # Null = all tenants
  reason: string
  
  filters:
    model_version: string?        # Regenerate specific model
    created_before: RFC3339?      # Regenerate old embeddings
  
  # Progress
  total_embeddings: int
  processed: int
  failed: int
  
  # Status
  status: enum                    # PENDING | RUNNING | COMPLETED | FAILED
  started_at: RFC3339
  estimated_completion: RFC3339
```

**Incremental Regeneration** (Zero Downtime):

```
1. Create new index (index_v2)
2. Generate new embeddings → index_v2
3. Validate index_v2
4. Atomic pointer swap (index → index_v2)
5. Delete old index (index_v1)
```

---

## 10. High Availability & Replication

### 10.1 Intra-Region Replication

**Requirements**:
- Minimum 3 replicas
- Quorum-based writes (2 of 3)
- Async reads allowed (with staleness warning)

**Qdrant Configuration**:

```yaml
replication:
  factor: 3
  write_consistency: quorum       # or all
  
sharding:
  number: 4                       # Number of shards
  method: auto                    # or custom
```

### 10.2 Cross-Region Replication

**Allowed For**:
- ✅ GLOBAL tenants
- ❌ REGIONAL_STRICT tenants

**Configuration**:

```yaml
CrossRegionReplication:
  primary_region: eu-central-1
  replica_regions:
    - us-east-1
    - ap-southeast-1
  
  replication:
    mode: async                   # or sync (higher latency)
    max_lag_ms: 5000
  
  encryption:
    in_transit: true
    algorithm: TLS 1.3
```

### 10.3 Failover

**Automatic Failover**:
- Detection time: < 10 seconds
- Promotion time: < 5 seconds
- Total RTO: < 15 seconds

**Failover Event**:

```yaml
Topic: vector.failover@v1

Payload:
  cluster_id: string
  failed_region: string
  promoted_region: string
  started_at: RFC3339
  completed_at: RFC3339
  data_loss: bool
  affected_tenants: [string]
```

---

## 11. Security & Residency

### 11.1 Encryption

**At Rest**:
- Algorithm: AES-256-GCM
- Key management: KMS (AWS/GCP/Azure)
- Key rotation: Every 90 days

**In Transit**:
- TLS 1.3 for all connections
- mTLS for service-to-service

### 11.2 Tenant Isolation

**Vector DB Namespaces**:

```yaml
# Qdrant Collections
collection: embeddings_tenant_123
collection: embeddings_tenant_456

# OR with metadata filtering
collection: embeddings_global
filter: {tenant_id: "tenant-123"}
```

**Mandatory**:
- No cross-tenant queries
- Separate indexes per tenant (or strict filtering)
- Audit all access

### 11.3 Residency Enforcement

```yaml
ResidencyCheck:
  operation: search | insert | delete
  tenant_id: string
  tenant_residency: GLOBAL | REGIONAL_STRICT
  
  data_region: string
  request_region: string
  
  # Validation
  allowed: bool
  reason: string?
```

**Violation Handling**:
- Return HTTP 451 `RESIDENCY_VIOLATION`
- Emit `residency.violation` event
- Log CRITICAL entry

---

## 12. Cost Attribution

### 12.1 Cost Tracking

```yaml
VectorCostRecord:
  tenant_id: string
  operation: enum                 # search | insert | delete | regenerate
  
  # Metrics
  vectors_processed: int
  compute_time_ms: int
  storage_mb: float
  
  # Costs
  compute_cost_usd: float
  storage_cost_usd: float
  total_cost_usd: float
  
  timestamp: RFC3339
  region: string
```

### 12.2 Cost Formula

```
TotalCost = (ComputeTime × RegionRate) + (StorageUsed × StorageRate)

Example Rates:
- Compute: $0.0001 per second
- Storage: $0.10 per GB/month
```

---

## 13. Observability

### 13.1 Metrics

```
# Vector operations
vector_lookup_total{index, tenant_id, outcome}
vector_lookup_duration_seconds{index, tenant_id}
vector_insert_total{index}
vector_insert_duration_seconds{index}
vector_regeneration_total{reason}
vector_regeneration_duration_seconds

# Index health
vector_index_size_bytes{index}
vector_count_total{index, tenant_id}
vector_compaction_duration_seconds{index}

# Replication
vector_replication_lag_seconds{region}
vector_failover_total{region}

# Cost
vector_cost_total_usd{tenant_id, operation}
```

### 13.2 Traces

```
vector.search
  ├─ vector.query.build
  ├─ vector.index.lookup
  ├─ vector.rerank (optional)
  └─ vector.results.format

vector.ingest
  ├─ embedding.generate
  ├─ vector.normalize
  ├─ vector.index.insert
  └─ metadata.store
```

### 13.3 Logs

```yaml
VectorLogV5.2:
  index: string
  tenant_id: string
  operation: enum
  latency_ms: int
  top_k: int?
  result_count: int?
  error: string?
  
  # NEW in v5.2
  modality: string?
  model_version: string?
```

---

## 14. Query API

### 14.1 Vector Search

```yaml
VectorSearchRequest:
  # Query
  vector: [float]                 # Query embedding
  top_k: int                      # Number of results
  
  # Filters
  filter: object                  # Metadata filters
    tenant_id: string             # REQUIRED
    tags: [string]?
    date_range:
      from: RFC3339
      to: RFC3339
  
  # Scoring
  threshold: float?               # Minimum similarity score
  rescore: bool?                  # Enable re-ranking
  
  # Performance
  ef_search: int?                 # HNSW parameter override
  timeout_ms: int?
```

**Response**:

```yaml
VectorSearchResponse:
  results: [
    {
      id: string
      score: float                # Similarity score
      metadata: object
      payload: object?            # Optional full payload
    }
  ]
  
  # Metadata
  search_time_ms: int
  total_candidates: int
  reranked: bool
```

### 14.2 Hybrid Search

```yaml
HybridSearchRequest:
  # Query components
  text_query: string?
  vector_query: [float]?
  
  # Weights
  weights:
    vector: float
    keyword: float
    graph: float
  
  # Config
  top_k: int
  filter: object
```

---

## 15. Compliance Checklist

A system is **Data Layer v5.2 Compliant** if:

✅ Uses approved vector database  
✅ Normalizes all vectors  
✅ Versions embeddings (semver)  
✅ Supports hybrid retrieval  
✅ Implements multi-modal embeddings  
✅ Enforces residency  
✅ Encrypts at rest (AES-256)  
✅ Supports HA & replication  
✅ Uses atomic index rebuilds  
✅ Exposes required metrics  
✅ Implements cost tracking  
✅ Handles regeneration correctly  
✅ Isolates tenants  

---

**End of Data Layer v5.2**