#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use rdkafka::config::ClientConfig;
use rdkafka::message::{Header, OwnedHeaders};
use rdkafka::producer::{FutureProducer, FutureRecord};

use crate::config::KafkaConfig;
use crate::metrics;
use crate::reason::DlqReason;
use crate::topic::resolve_topic;

/// A DLQ record. Designed for investigation + replay tooling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqRecord {
    /// Machine-friendly classification (e.g. "decode_error", "handler_error").
    pub reason: DlqReason,

    /// Whether the underlying error was considered retryable.
    ///
    /// For example: transport errors are usually retryable, decode errors are not.
    pub retryable: Option<bool>,

    /// Handler attempts that were made (only present for handler failures).
    pub attempts: Option<u32>,

    /// Field `error`.
    pub error: String,

    /// Field `original_topic`.
    pub original_topic: Option<String>,
    /// Field `partition`.
    pub partition: Option<i32>,
    /// Field `offset`.
    pub offset: Option<i64>,

    /// Original message timestamp if available (milliseconds since epoch).
    pub original_timestamp_ms: Option<i64>,

    // ---------------------------------------------------------------------
    // Event identity / context enrichment (queryable without decoding payload)
    // ---------------------------------------------------------------------
    /// Field `event_id`.
    pub event_id: Option<String>,
    /// Field `schema_ref`.
    pub schema_ref: Option<String>,

    /// W3C trace context.
    pub traceparent: Option<String>,
    /// Field `trace_id`.
    pub trace_id: Option<String>,
    /// Field `span_id`.
    pub span_id: Option<String>,

    /// Request/correlation context.
    pub request_id: Option<String>,
    /// Field `correlation_id`.
    pub correlation_id: Option<String>,

    /// Tenant/region hints.
    pub tenant_id: Option<String>,
    /// Field `region`.
    pub region: Option<String>,

    /// RFC3339 timestamp when DLQ was produced.
    pub dlq_at: DateTime<Utc>,

    /// Base64 of original payload (possibly truncated).
    pub payload_b64: String,

    /// Whether payload was truncated.
    pub truncated: bool,
}

fn dlq_headers_from_record(r: &DlqRecord) -> OwnedHeaders {
    let mut h = OwnedHeaders::new();

    // Stable classification
    h = h.insert(Header {
        key: "x-rhelma-dlq-reason",
        value: Some(r.reason.as_str().as_bytes()),
    });

    // Identity
    if let Some(v) = r.event_id.as_deref() {
        h = h.insert(Header {
            key: "x-rhelma-event-id",
            value: Some(v.as_bytes()),
        });
    }
    if let Some(v) = r.schema_ref.as_deref() {
        h = h.insert(Header {
            key: "x-rhelma-schema-ref",
            value: Some(v.as_bytes()),
        });
    }

    // Trace
    if let Some(v) = r.traceparent.as_deref() {
        h = h.insert(Header {
            key: "traceparent",
            value: Some(v.as_bytes()),
        });
    }
    if let Some(v) = r.trace_id.as_deref() {
        h = h.insert(Header {
            key: "x-trace-id",
            value: Some(v.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-rhelma-trace-id",
            value: Some(v.as_bytes()),
        });
    }
    if let Some(v) = r.span_id.as_deref() {
        h = h.insert(Header {
            key: "x-span-id",
            value: Some(v.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-rhelma-span-id",
            value: Some(v.as_bytes()),
        });
    }

    // Request/correlation
    if let Some(v) = r.request_id.as_deref() {
        h = h.insert(Header {
            key: "x-rhelma-request-id",
            value: Some(v.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-request-id",
            value: Some(v.as_bytes()),
        });
    }
    if let Some(v) = r.correlation_id.as_deref() {
        h = h.insert(Header {
            key: "x-rhelma-correlation-id",
            value: Some(v.as_bytes()),
        });
        h = h.insert(Header {
            key: "x-correlation-id",
            value: Some(v.as_bytes()),
        });
    }

    // Tenant/region
    if let Some(v) = r.tenant_id.as_deref() {
        h = h.insert(Header {
            key: "x-tenant-id",
            value: Some(v.as_bytes()),
        });
    }
    if let Some(v) = r.region.as_deref() {
        h = h.insert(Header {
            key: "x-region",
            value: Some(v.as_bytes()),
        });
    }

    h
}

pub struct DlqPublisher {
    producer: FutureProducer,
    cfg: KafkaConfig,
}

impl DlqPublisher {
    pub fn new(cfg: KafkaConfig) -> Result<Self, String> {
        cfg.validate_for_dlq_publisher()
            .map_err(|e| format!("invalid dlq config: {e}"))?;
        let mut cc = ClientConfig::new();
        cc.set("bootstrap.servers", &cfg.brokers);
        cc.set("message.timeout.ms", "5000");
        let producer: FutureProducer = cc
            .create()
            .map_err(|e| format!("failed to create dlq producer: {e}"))?;
        Ok(Self { producer, cfg })
    }

    pub async fn publish(&self, record: &DlqRecord, key: &str) -> Result<(), String> {
        let topic = resolve_topic(self.cfg.topic_prefix.as_str(), &self.cfg.dlq_topic)
            .map_err(|e| e.to_string())?;
        let payload = serde_json::to_vec(record).map_err(|e| e.to_string())?;

        let headers = dlq_headers_from_record(record);

        let fr = FutureRecord {
            topic: &topic,
            payload: Some(&payload),
            key: Some(key),
            partition: None,
            timestamp: Some(record.dlq_at.timestamp_millis()),
            headers: Some(headers),
        };

        self.producer
            .send(fr, std::time::Duration::from_secs(0))
            .await
            .map_err(|(e, _)| {
                metrics::inc_dlq_publish_error(record.reason);
                format!("dlq send failed: {e}")
            })?;

        metrics::inc_dlq_publish_success(record.reason);

        Ok(())
    }

    pub fn max_payload_bytes(&self) -> usize {
        self.cfg.dlq_max_payload_bytes
    }
}
