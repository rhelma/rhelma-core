#![cfg(feature = "with-config")]

use std::sync::{Arc, Mutex};

use rhelma_logger::event::{LogEvent, LogLevel};
use rhelma_logger::state::set_dispatcher;
use rhelma_logger::LogDispatcher;
use rhelma_logger::{LogBuilder, RhelmaLogger};

use rhelma_core::{RegionId, RequestContext};

#[derive(Clone)]
struct CaptureDispatcher {
    events: Arc<Mutex<Vec<LogEvent>>>,
}

impl CaptureDispatcher {
    fn new(events: Arc<Mutex<Vec<LogEvent>>>) -> Self {
        Self { events }
    }
}

impl LogDispatcher for CaptureDispatcher {
    fn dispatch(&self, event: LogEvent) {
        if let Ok(mut g) = self.events.lock() {
            g.push(event);
        }
    }

    fn flush(&self) {}

    fn box_clone(&self) -> Box<dyn LogDispatcher + Send + Sync> {
        Box::new(self.clone())
    }
}

#[test]
fn test_init_from_unified_emits_v52_aligned_event() {
    // NOTE: This is an integration test binary with a single test,
    // so the global logger state is clean for this process.

    // Capture emitted events instead of printing to stdout.
    let events = Arc::new(Mutex::new(Vec::new()));
    set_dispatcher(Box::new(CaptureDispatcher::new(events.clone())));

    let mut unified = rhelma_config::UnifiedObservabilityConfig::baseline("test-svc".into());
    unified.region = "eu-central-1".into();
    unified.performance_profile = rhelma_config::PerformanceProfile::LowLatency; // force Sync dispatch for deterministic tests

    RhelmaLogger::init_from_unified(&unified, None).expect("init_from_unified failed");

    let ctx =
        RequestContext::empty().with_region(RegionId::parse("eu-west-1").expect("region parse"));

    LogBuilder::new(LogLevel::Info, "hello")
        .with_request_context(&ctx)
        .field("authorization", "Bearer secret-token")
        .field("user.email", "user@example.com")
        .emit();

    let ev = {
        let g = events.lock().expect("lock");
        assert_eq!(g.len(), 1, "expected exactly one event");
        g[0].clone()
    };

    // Service region must come from config, request region must come from RequestContext.
    assert_eq!(ev.region, "eu-central-1");
    assert_eq!(ev.request_region.as_deref(), Some("eu-west-1"));

    // Redaction must be fail-safe.
    let auth = ev
        .fields
        .get("authorization")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(auth, "***REDACTED***");

    let email = ev
        .fields
        .get("user.email")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        email.starts_with("sha256:"),
        "expected hashed email, got: {email}"
    );

    assert!(ev.pii_redacted, "expected pii_redacted flag to be true");
    assert!(ev.redacted_fields.iter().any(|f| f == "authorization"));
    assert!(ev.redacted_fields.iter().any(|f| f == "user.email"));
}
