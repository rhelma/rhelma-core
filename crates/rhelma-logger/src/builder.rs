use std::fmt;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::config::{Environment, LoggerConfig};
use crate::event::{LogEvent, LogLevel};
use crate::pii::PiiRedactor;
use crate::state;

// از rhelma-core فقط برای خطا و context استفاده می‌کنیم
use rhelma_core::RequestContext;
use rhelma_core::RhelmaError;

/// اطلاعات خطا که توسط builder نگه داشته می‌شود تا در LogEvent پر شود.
#[derive(Debug, Clone)]
struct ErrorInfo {
    code: Option<String>,
    message: Option<String>,
    type_: Option<String>,
    severity: Option<String>,
}

/// Builder برای ساخت و ارسال LogEvent.
pub struct LogBuilder {
    cfg: Option<&'static LoggerConfig>,
    level: LogLevel,
    message: String,

    // Context
    request_context: Option<RequestContext>,

    // Operation
    operation_name: Option<String>,
    operation_kind: Option<String>,
    operation_status: Option<String>,

    // Error
    error: Option<ErrorInfo>,

    // Custom fields
    fields: Map<String, Value>,

    // Light tags
    tags: Vec<String>,

    // Optional performance & incident correlation
    duration_ms: Option<u64>,
    incident_id: Option<String>,
    command_id: Option<String>,
}

impl LogBuilder {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        let cfg = state::global_config();
        Self {
            cfg,
            level,
            message: message.into(),
            request_context: None,
            operation_name: None,
            operation_kind: None,
            operation_status: None,
            error: None,
            fields: Map::new(),
            tags: Vec::new(),
            duration_ms: None,
            incident_id: None,
            command_id: None,
        }
    }

    pub fn with_request_context(mut self, ctx: &RequestContext) -> Self {
        self.request_context = Some(ctx.clone());
        self
    }

    pub fn operation(
        mut self,
        name: impl Into<String>,
        kind: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        self.operation_name = Some(name.into());
        self.operation_kind = Some(kind.into());
        self.operation_status = Some(status.into());
        self
    }

    /// Audit log (Rhelma v5.1.1)
    ///
    /// Example:
    /// log_audit!(
    ///     "user updated profile",
    ///     "user",          // actor_type
    ///     "update_profile",// operation
    ///     "user_profile",  // resource_type
    ///     user_id
    /// );
    pub fn audit(
        mut self,
        actor_type: impl Into<String>,
        operation: impl Into<String>,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        self.operation_name = Some(operation.into());
        self.operation_kind = Some("audit".into());
        self.operation_status = Some("success".into());

        self = self
            .field("audit.actor_type", actor_type.into())
            .field("audit.resource_type", resource_type.into())
            .field("audit.resource_id", resource_id.into());

        self
    }

    /// Heartbeat event (service liveness)
    ///
    /// Example:
    /// log_heartbeat!("service alive");
    pub fn heartbeat(mut self) -> Self {
        self.operation_name = Some("system.heartbeat".into());
        self.operation_kind = Some("heartbeat".into());
        self.operation_status = Some("success".into());
        self
    }

    pub fn with_rhelma_error(mut self, err: &RhelmaError) -> Self {
        let code = Some(err.as_str().to_string());
        let message = Some(err.to_string());
        let type_ = Some("rhelma_error".to_string());

        self.error = Some(ErrorInfo {
            code,
            message,
            type_,
            severity: None,
        });
        self
    }

    pub fn with_error<E>(mut self, err: &E) -> Self
    where
        E: fmt::Display + ?Sized,
    {
        self.error = Some(ErrorInfo {
            code: None,
            message: Some(err.to_string()),
            type_: Some("error".to_string()),
            severity: None,
        });
        self
    }

    pub fn with_error_code(
        mut self,
        code: impl Into<String>,
        message: impl Into<String>,
        type_: impl Into<String>,
    ) -> Self {
        self.error = Some(ErrorInfo {
            code: Some(code.into()),
            message: Some(message.into()),
            type_: Some(type_.into()),
            severity: None,
        });
        self
    }

    pub fn with_error_severity(mut self, severity: impl Into<String>) -> Self {
        if let Some(ref mut e) = self.error {
            e.severity = Some(severity.into());
        } else {
            self.error = Some(ErrorInfo {
                code: None,
                message: None,
                type_: None,
                severity: Some(severity.into()),
            });
        }
        self
    }

    pub fn field<V>(mut self, key: &str, value: V) -> Self
    where
        V: Serialize,
    {
        match serde_json::to_value(value) {
            Ok(v) => {
                self.fields.insert(key.to_string(), v);
            }
            Err(e) => {
                self.fields.insert(
                    key.to_string(),
                    Value::String(format!("<serialization-error: {e}>")),
                );
            }
        }
        self
    }

    pub fn fields_from<T>(mut self, prefix: &str, value: &T) -> Self
    where
        T: Serialize,
    {
        match serde_json::to_value(value) {
            Ok(Value::Object(map)) => {
                for (k, v) in map {
                    self.fields.insert(format!("{prefix}.{k}"), v);
                }
            }
            Ok(v) => {
                self.fields.insert(prefix.to_string(), v);
            }
            Err(e) => {
                self.fields.insert(
                    prefix.to_string(),
                    Value::String(format!("__serialization_error__:{e}")),
                );
            }
        }
        self
    }

    /// Add a lightweight tag for filtering (e.g. "audit", "payments", "slow").
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Set operation duration (milliseconds).
    pub fn duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    /// Attach an incident/case ID (SRE/SOC).
    pub fn incident_id(mut self, id: impl Into<String>) -> Self {
        self.incident_id = Some(id.into());
        self
    }

    /// Attach a command ID for CQRS-style correlation.
    pub fn command_id(mut self, id: impl Into<String>) -> Self {
        self.command_id = Some(id.into());
        self
    }

    pub fn build(self) -> LogEvent {
        let mut event = LogEvent::new(self.level, self.message);

        // 1) Service / env context از LoggerConfig (اگر موجود باشد)
        if let Some(cfg) = self.cfg {
            event.service_name = cfg.service_name.clone();
            event.service_version = cfg.service_version.clone();
            event.service_instance_id = cfg.service_instance_id.clone().unwrap_or_default();
            event.region = cfg.region.clone();
            event.environment = match cfg.environment {
                Environment::Local => "local".to_string(),
                Environment::Development => "development".to_string(),
                Environment::Staging => "staging".to_string(),
                Environment::Production => "production".to_string(),
                Environment::Test => "test".to_string(),
                Environment::Unknown => "unknown".to_string(),
            };
        }

        // 2) RequestContext (اگر ست شده)
        if let Some(ctx) = self.request_context {
            // request_id: اجباری و همیشه Uuid معتبر (طبق قرارداد)
            event.request_id = Some(ctx.request_id().to_string());

            // correlation_id: optional
            event.correlation_id = ctx.correlation_id().map(|s| s.to_string());

            // tenancy / residency
            if let Some(tid) = ctx.tenant_id() {
                event.tenant_id = Some(tid.to_string());
            }
            if let Some(rid) = ctx.region() {
                event.request_region = Some(rid.to_string());
            }

            // identity
            if let Some(uid) = ctx.user_id() {
                event.user_id = Some(uid.to_string());
            }
            if let Some(email) = ctx.user_email() {
                event.fields.insert(
                    "user.email".to_string(),
                    serde_json::Value::String(email.to_string()),
                );
            }

            // locale را هم به‌صورت فیلد کم‌ریسک لاگ می‌کنیم
            if let Some(loc) = ctx.locale() {
                event.fields.insert(
                    "user.locale".to_string(),
                    serde_json::Value::String(loc.to_string()),
                );
            }

            // trace context → از TraceContext داخل RequestContext
            let trace_ctx = ctx.trace();
            // trace_id استاندارد (W3C) از helper خود TraceContext
            event.trace_id = trace_ctx.current_trace_id().map(|s| s.to_string());
            // span_id چون pub است، مستقیم از فیلد می‌خوانیم
            event.span_id = trace_ctx.span_id.as_deref().map(|s| s.to_string());
        }

        // 3) Operation context
        event.operation_name = self.operation_name;
        event.operation_kind = self.operation_kind;
        event.operation_status = self.operation_status;

        // 4) Error context
        if let Some(err) = self.error {
            event.error_code = err.code;
            event.error_message = err.message;
            event.error_type = err.type_;
            event.error_severity = err.severity;
        }

        // 5) Custom fields
        event.fields.extend(self.fields);

        // 5.1) Common optional fields
        event.tags = self.tags;
        event.duration_ms = self.duration_ms;
        event.incident_id = self.incident_id;
        event.command_id = self.command_id;

        // 6) PII Redaction (deep) از طریق redactor global
        if let Some(redactor) = state::global_redactor() {
            apply_pii_redaction(&mut event, redactor.as_ref());
        }

        // Derived flags
        event.pii_redacted = !event.redacted_fields.is_empty();
        if event.pii_redacted {
            state::notify_pii_violation(&event.redacted_fields);
        }

        event
    }

    pub fn emit(self) {
        // ⚠️ اول cfg را جدا کنیم تا self بعداً move شود
        let cfg = self.cfg;
        if let Some(cfg) = cfg {
            // Level filtering (global standard: do not pay serialization cost for suppressed levels).
            let threshold = parse_level_threshold(&cfg.log_level);
            if self.level < threshold {
                return;
            }

            let event = self.build();
            state::dispatch_event(event, cfg);
        } else {
            crate::state::report_internal_error(
                "emit() called without global LoggerConfig; dropping log event",
            );
        }
    }
}

fn apply_pii_redaction(event: &mut LogEvent, redactor: &dyn PiiRedactor) {
    let mut redacted = Vec::new();

    // Top-level scalar fields (safer than leaving them untouched).
    redact_top_level_scalar_fields(event, redactor, &mut redacted);

    for (k, v) in event.fields.iter_mut() {
        let key_prefix = k.as_str();
        redact_value_recursively(v, key_prefix, redactor, &mut redacted);
    }

    event.redacted_fields = redacted;
}

fn redact_top_level_scalar_fields(
    event: &mut LogEvent,
    redactor: &dyn PiiRedactor,
    redacted_fields: &mut Vec<String>,
) {
    // message
    if let Some(out) = redactor.redact("message", &event.message) {
        event.message = out;
        redacted_fields.push("message".to_string());
    }

    // error_message (common leakage vector via anyhow/reqwest errors)
    if let Some(ref mut msg) = event.error_message {
        if let Some(out) = redactor.redact("error_message", msg) {
            *msg = out;
            redacted_fields.push("error_message".to_string());
        }
    }

    // user_id
    if let Some(ref mut uid) = event.user_id {
        if let Some(out) = redactor.redact("user_id", uid) {
            *uid = out;
            redacted_fields.push("user_id".to_string());
        }
    }

    // session_id
    if let Some(ref mut sid) = event.session_id {
        if let Some(out) = redactor.redact("session_id", sid) {
            *sid = out;
            redacted_fields.push("session_id".to_string());
        }
    }

    // tenant_id (treat as identifier; many orgs consider it sensitive in multi-tenant logs)
    if let Some(ref mut tid) = event.tenant_id {
        if let Some(out) = redactor.redact("tenant_id", tid) {
            *tid = out;
            redacted_fields.push("tenant_id".to_string());
        }
    }
}

fn redact_value_recursively(
    value: &mut Value,
    key: &str,
    redactor: &dyn PiiRedactor,
    redacted_fields: &mut Vec<String>,
) {
    match value {
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                let next_key = if key.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", key, k)
                };
                redact_value_recursively(v, &next_key, redactor, redacted_fields);
            }
        }
        Value::Array(arr) => {
            for (idx, v) in arr.iter_mut().enumerate() {
                let next_key = format!("{}[{}]", key, idx);
                redact_value_recursively(v, &next_key, redactor, redacted_fields);
            }
        }
        _ => {
            // هر نوع scalar را به String تبدیل می‌کنیم
            let current = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                _ => return,
            };

            if let Some(redacted) = redactor.redact(key, &current) {
                *value = Value::String(redacted);
                redacted_fields.push(key.to_string());
            }
        }
    }
}

fn parse_level_threshold(s: &str) -> LogLevel {
    match s.trim().to_ascii_lowercase().as_str() {
        "trace" => LogLevel::Trace,
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "warn" | "warning" => LogLevel::Warn,
        "error" => LogLevel::Error,
        "critical" | "fatal" => LogLevel::Critical,
        _ => LogLevel::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Environment, LoggerConfig};
    use crate::pii::DefaultPiiRedactor;
    use crate::state;

    // Helper init — فقط برای تست داخلی builder
    fn init_test_logger() {
        let cfg = LoggerConfig {
            service_name: "test-svc".into(),
            service_version: "1.0".into(),
            service_instance_id: None,
            environment: Environment::Development,
            region: "local".into(),
            log_level: "info".into(),
            log_format: crate::config::LogFormat::Json,
            json_enabled: true,
            console_enabled: false,
            sampling_rate: 1.0,
            performance_profile: crate::config::PerformanceProfile::Balanced,
            dispatch_mode: crate::config::DispatchMode::Sync,
            queue_capacity: 128,
            backpressure: crate::config::BackpressureStrategy::DropNewest,
            flush_interval_ms: 0,
        };

        let _ = state::install_globals(&cfg);
        state::set_redactor(Box::new(DefaultPiiRedactor));
    }

    // ----------------------------------------------------------
    // TEST 1 — Basic builder behavior
    // ----------------------------------------------------------
    #[test]
    fn test_builder_basic_event() {
        init_test_logger();

        let event = LogBuilder::new(LogLevel::Info, "hello world").build();

        assert_eq!(event.message, "hello world");
        assert_eq!(event.level, LogLevel::Info);
        assert_eq!(event.service_name, "test-svc"); // از cfg می‌آید
        assert_eq!(event.operation_name, None);
        assert_eq!(event.error_code, None);
    }

    // ----------------------------------------------------------
    // TEST 2 — RequestContext integration
    // ----------------------------------------------------------
    #[test]
    fn test_builder_with_request_context() {
        use rhelma_core::RequestContext;
        init_test_logger();

        let mut ctx = RequestContext::empty();
        ctx = ctx.with_locale("fa-IR");

        let event = LogBuilder::new(LogLevel::Info, "ctx-test")
            .with_request_context(&ctx)
            .build();

        assert!(event.request_id.is_some());
        assert_eq!(event.fields.get("user.locale").unwrap(), "fa-IR");
        assert!(event.trace_id.is_some());
    }

    // ----------------------------------------------------------
    // TEST 3 — PII/Secrets redaction on message and error_message
    // ----------------------------------------------------------
    #[test]
    fn test_builder_redacts_message_jwt() {
        init_test_logger();

        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.sflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let event = LogBuilder::new(LogLevel::Info, jwt).build();

        assert_eq!(event.message, "***REDACTED***");
        assert!(event.redacted_fields.iter().any(|f| f == "message"));
        assert!(event.pii_redacted);
    }

    #[test]
    fn test_builder_redacts_error_message_bearer() {
        init_test_logger();

        let err = format!("Bearer {}", "thisisaverylongtokenvalue1234567890");
        let event = LogBuilder::new(LogLevel::Error, "boom")
            .with_error(&err)
            .build();

        assert_eq!(event.error_message.as_deref(), Some("***REDACTED***"));
        assert!(event.redacted_fields.iter().any(|f| f == "error_message"));
        assert!(event.pii_redacted);
    }
}
