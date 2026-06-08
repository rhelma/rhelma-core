# MNI RAG API v1

## POST /v1/dataset/ingest
Headers:
- `x-admin-token`: required unless `tier=public_commons`

Body:
```json
{
  "doc_id": "optional-stable-id",
  "source": "lexicon|forum|code|security-report|...",
  "tier": "public_commons|consent_based|sensitive_jury_gated|private_no_train",
  "consent": true,
  "content": "text",
  "tags": ["optional", "tags"]
}
```

Response:
```json
{ "doc_id": "...", "content_sha256_hex": "..." }
```

## POST /v1/query
Body:
```json
{ "query": "string", "top_k": 5 }
```

Response:
```json
{
  "query_id": "...",
  "hits": [
    {
      "doc_id": "...",
      "source": "...",
      "score": 3,
      "snippet": "...",
      "content_sha256_hex": "..."
    }
  ]
}
```
