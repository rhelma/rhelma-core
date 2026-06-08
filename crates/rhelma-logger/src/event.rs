//! rhelma-logger: LogEvent schema v5.2-aligned output
//!
//! این ماژول event اصلی لاگ را تعریف می‌کند که:
//! - با Rhelma Contract v5.1 سازگار است
//! - برای جستجو/ایندکس شدن در سیستم‌های لاگ (ELK / Loki / …) مناسب است
//! - با RequestContext و RhelmaError از rhelma-core هم‌راستا است.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;

/// سطح لاگ (با نام‌های lowercase برای JSON)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Variant `Trace`.
    Trace,
    /// Variant `Debug`.
    Debug,
    /// Variant `Info`.
    Info,
    /// Variant `Warn`.
    Warn,
    /// Variant `Error`.
    Error,
    /// High-severity incidents that require immediate attention.
    Critical,
}

/// رویداد لاگ ساختار‌یافته.
/// این struct باید **پایدار** باشد (schema-versioned) و تا حد امکان backward-compatible بماند.
#[derive(Debug, Clone, Deserialize)]
pub struct LogEvent {
    /// نسخه‌ی اسکیمای لاگ (v5.2 = 3).
    pub schema_version: u8,

    /// زمان ایجاد لاگ (UTC, RFC3339 میلی‌ثانیه‌ای).
    #[serde(with = "crate::event_time_ms")]
    pub timestamp: OffsetDateTime,

    /// سطح لاگ.
    pub level: LogLevel,

    /// پیام اصلی لاگ (human readable).
    pub message: String,

    // -------- Service / Environment Context --------
    /// نام سرویس منطقی (api-gateway, search-service, ...).
    pub service_name: String,

    /// نسخه‌ی سرویس (git SHA / SemVer).
    pub service_version: String,

    /// شناسه‌ی instance سرویس (pod/container/host).
    /// از LoggerConfig پر می‌شود (RHELMA_INSTANCE_ID/HOSTNAME).
    pub service_instance_id: String,

    /// محیط اجرا (development / staging / production).
    pub environment: String,

    /// منطقه‌ی استقرار (eu-west-1, us-east-1, local, ...).
    pub region: String,

    // -------- Identity / Request / Trace Context --------
    /// شناسه‌ی درخواست (request_id).
    pub request_id: Option<String>,

    /// شناسه‌ی correlation برای ردیابی cross-service.
    pub correlation_id: Option<String>,

    /// شناسه‌ی tenant (اگر multi-tenant است).
    pub tenant_id: Option<String>,

    /// منطقه/Residency درخواست (برخلاف `service.region` که منطقه‌ی استقرار سرویس است).
    ///
    /// این مقدار از `RequestContext.region()` (rhelma-core v5.2) پر می‌شود.
    pub request_region: Option<String>,

    /// شناسه‌ی کاربر (معمولاً UUID).
    pub user_id: Option<String>,

    /// شناسه‌ی session (وب‌سوکت / سشن UI).
    pub session_id: Option<String>,

    /// شناسه‌های tracing (W3C).
    pub trace_id: Option<String>,
    /// Field `span_id`.
    pub span_id: Option<String>,

    // -------- Operation Context --------
    /// نام عملیات (مثلاً "user.login" یا "search.query").
    pub operation_name: Option<String>,

    /// نوع عملیات (query / command / background / http / ws / cron / ...).
    pub operation_kind: Option<String>,

    /// وضعیت عملیات (success / failure / retry / timeout / ...).
    pub operation_status: Option<String>,

    // -------- Error Context --------
    /// کد خطا (مثلاً rhelma-core error label، یا کد اپلیکیشن).
    pub error_code: Option<String>,

    /// پیام خطا (جهت انسان).
    pub error_message: Option<String>,

    /// نوع خطا (مثلاً "rhelma_error", "sqlx_error", "io_error").
    pub error_type: Option<String>,

    /// شدت خطا (low / medium / high / critical).
    pub error_severity: Option<String>,

    // -------- Custom Structured Fields --------
    /// فیلدهای اضافی ساختار‌یافته (key/value).
    ///
    /// مثال:
    /// - "http.method" → "GET"
    /// - "db.query_time_ms" → 12.3
    /// - "tags" → ["search", "user-facing"]
    #[serde(default)]
    pub fields: Map<String, Value>,

    /// فهرست کلیدهایی که PII روی آن‌ها redacted شده است.
    ///
    /// مثال:
    /// - "user.email"
    /// - "user.tokens[0].value"
    #[serde(default)]
    pub redacted_fields: Vec<String>,

    /// آیا حداقل یک فیلد PII روی این رویداد redacted شده است.
    #[serde(default)]
    pub pii_redacted: bool,

    /// Tags سبک برای فیلتر/جستجوی سریع.
    #[serde(default)]
    pub tags: Vec<String>,

    /// مدت عملیات (میلی‌ثانیه) اگر از بیرون اندازه‌گیری شده باشد.
    #[serde(default)]
    pub duration_ms: Option<u64>,

    /// Incident/Case ID (در صورت وجود) برای SRE/SOC.
    #[serde(default)]
    pub incident_id: Option<String>,

    /// Command ID در الگوی CQRS (در صورت وجود).
    #[serde(default)]
    pub command_id: Option<String>,
}

// -----------------------------------------------------------------------------
// v5.2 JSON output (nested objects): service / request / trace / context / error
// -----------------------------------------------------------------------------
//
// We keep the Rust struct largely "flat" for ergonomics, but the emitted JSON is
// aligned to Rhelma Observability v5.2.
//
// Notes:
// - Unknown fields are allowed by most log backends; we keep `schema_version`.
// - `context` maps to `fields`.
// - `trace` is emitted only if `trace_id` exists.
//
#[derive(Serialize)]
struct OutService<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub instance_id: &'a str,
    pub region: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<&'a str>,
}

#[derive(Serialize)]
struct OutRequest<'a> {
    pub request_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<&'a str>,
}

#[derive(Serialize)]
struct OutTrace<'a> {
    pub trace_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<&'a str>,
}

#[derive(Serialize)]
struct OutError<'a> {
    pub code: &'a str,
    pub message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<&'a str>,
}

#[derive(Serialize)]
struct LogEventV52Out<'a> {
    pub schema_version: u8,
    #[serde(with = "crate::event_time_ms")]
    pub timestamp: OffsetDateTime,
    pub level: LogLevel,
    pub message: &'a str,

    pub service: OutService<'a>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<OutRequest<'a>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<OutTrace<'a>>,

    #[serde(rename = "context")]
    pub fields: &'a Map<String, Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OutError<'a>>,

    pub pii_redacted: bool,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: &'a Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    // Extra optional fields (not in the minimal schema, but useful operationally).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub redacted_fields: &'a Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incident_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_kind: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_status: Option<&'a str>,
}

impl serde::Serialize for LogEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let service = OutService {
            name: self.service_name.as_str(),
            version: self.service_version.as_str(),
            instance_id: self.service_instance_id.as_str(),
            region: self.region.as_str(),
            environment: if self.environment.is_empty() {
                None
            } else {
                Some(self.environment.as_str())
            },
        };

        // Only emit request object if we have a request_id.
        let request = self.request_id.as_deref().map(|rid| OutRequest {
            request_id: rid,
            correlation_id: self.correlation_id.as_deref(),
            tenant_id: self.tenant_id.as_deref(),
            user_id: self.user_id.as_deref(),
            session_id: self.session_id.as_deref(),
            region: self.request_region.as_deref(),
        });

        let trace = self.trace_id.as_deref().map(|tid| OutTrace {
            trace_id: tid,
            span_id: self.span_id.as_deref(),
        });

        let error = self.error_code.as_deref().map(|code| OutError {
            code,
            message: self.error_message.as_deref().unwrap_or(""),
            error_type: self.error_type.as_deref(),
            severity: self.error_severity.as_deref(),
        });

        let out = LogEventV52Out {
            schema_version: self.schema_version,
            timestamp: self.timestamp,
            level: self.level,
            message: self.message.as_str(),
            service,
            request,
            trace,
            fields: &self.fields,
            error,
            pii_redacted: self.pii_redacted,
            tags: &self.tags,
            duration_ms: self.duration_ms,
            redacted_fields: &self.redacted_fields,
            incident_id: self.incident_id.as_deref(),
            command_id: self.command_id.as_deref(),
            operation_name: self.operation_name.as_deref(),
            operation_kind: self.operation_kind.as_deref(),
            operation_status: self.operation_status.as_deref(),
        };

        out.serialize(serializer)
    }
}

impl LogEvent {
    /// مقدار پیش‌فرض schema_version برای v5.1.
    pub const CURRENT_SCHEMA_VERSION: u8 = 3;

    /// سازنده‌ی کمکی برای ساختن event خالی با مقداردهی اولیه معقول.
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            timestamp: OffsetDateTime::now_utc(),
            level,
            message: message.into(),

            service_name: String::new(),
            service_version: String::new(),
            service_instance_id: String::new(),
            environment: String::new(),
            region: String::new(),

            request_id: None,
            correlation_id: None,
            tenant_id: None,
            request_region: None,
            user_id: None,
            session_id: None,
            trace_id: None,
            span_id: None,

            operation_name: None,
            operation_kind: None,
            operation_status: None,

            error_code: None,
            error_message: None,
            error_type: None,
            error_severity: None,

            fields: Map::new(),
            redacted_fields: Vec::new(),
            pii_redacted: false,
            tags: Vec::new(),
            duration_ms: None,
            incident_id: None,
            command_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_event_new_defaults() {
        let ev = LogEvent::new(LogLevel::Info, "hello");

        // Schema version
        assert_eq!(ev.schema_version, LogEvent::CURRENT_SCHEMA_VERSION);

        // Base fields
        assert_eq!(ev.level, LogLevel::Info);
        assert_eq!(ev.message, "hello");

        // Timestamp should be valid
        let now = OffsetDateTime::now_utc();
        assert!(ev.timestamp <= now);

        // Context defaults
        assert!(ev.service_name.is_empty());
        assert!(ev.service_version.is_empty());
        assert!(ev.environment.is_empty());
        assert!(ev.region.is_empty());

        assert!(ev.request_id.is_none());
        assert!(ev.correlation_id.is_none());
        assert!(ev.tenant_id.is_none());
        assert!(ev.user_id.is_none());
        assert!(ev.session_id.is_none());
        assert!(ev.trace_id.is_none());
        assert!(ev.span_id.is_none());

        // Operation defaults
        assert!(ev.operation_name.is_none());
        assert!(ev.operation_kind.is_none());
        assert!(ev.operation_status.is_none());

        // Error defaults
        assert!(ev.error_code.is_none());
        assert!(ev.error_message.is_none());
        assert!(ev.error_type.is_none());
        assert!(ev.error_severity.is_none());

        // Custom fields should be empty
        assert!(ev.fields.is_empty());
        assert!(ev.redacted_fields.is_empty());
        assert!(!ev.pii_redacted);
        assert!(ev.tags.is_empty());
        assert!(ev.duration_ms.is_none());
        assert!(ev.incident_id.is_none());
        assert!(ev.command_id.is_none());
    }
}
