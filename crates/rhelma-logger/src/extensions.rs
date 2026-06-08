use crate::event::LogEvent;
use thiserror::Error;

/// برای آینده (مثلاً HTTP/IO dispatcher)
#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("failed to dispatch log event: {0}")]
    /// Variant `Failed`.
    Failed(String),
}

// ---------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------
pub trait LogDispatcher: Send + Sync {
    /// ارسال یک event (بدون برگشت Result — خطا باید swallow شود)
    fn dispatch(&self, event: LogEvent);

    /// برای dispatcherهایی مثل FileDispatcher مفید است.
    /// پیش‌فرض هیچ‌کاری نمی‌کند.
    fn flush(&self) {}

    /// برای کلون‌کردن trait-object
    fn box_clone(&self) -> Box<dyn LogDispatcher + Send + Sync>;
}

// ---------------------------------------------------------
// Metrics Collector (اختیاری)
// ---------------------------------------------------------
pub trait LogMetricsCollector: Send + Sync {
    /// Called before sampling/backpressure decisions (attempted log emission).
    /// Default: no-op.
    fn record_attempted_event(&self) {}

    /// fn `record_dispatched_event`.
    fn record_dispatched_event(&self);
    /// fn `record_dropped_event`.
    fn record_dropped_event(&self);

    /// Called when a log was sampled out (not emitted).
    /// Default: counts as a dropped event.
    fn record_sampled_out(&self) {
        self.record_dropped_event();
    }

    /// fn `record_dropped_newest`.
    fn record_dropped_newest(&self) {
        self.record_dropped_event();
    }

    /// fn `record_dropped_oldest`.
    fn record_dropped_oldest(&self) {
        self.record_dropped_event();
    }

    /// fn `record_dropoldest_lock_poisoned`.
    fn record_dropoldest_lock_poisoned(&self) {
        self.record_dropped_event();
    }

    /// لازم برای clone trait object
    fn box_clone(&self) -> Box<dyn LogMetricsCollector + Send + Sync>;
}

// ---------------------------------------------------------
// Trace Provider (اختیاری)
// ---------------------------------------------------------
pub trait TraceContextProvider: Send + Sync {
    /// fn `request_id`.
    fn request_id(&self) -> Option<String>;
    /// fn `correlation_id`.
    fn correlation_id(&self) -> Option<String>;
    /// fn `tenant_id`.
    fn tenant_id(&self) -> Option<String>;
    /// fn `user_id`.
    fn user_id(&self) -> Option<String>;
    /// fn `session_id`.
    fn session_id(&self) -> Option<String>;
    /// fn `trace_id`.
    fn trace_id(&self) -> Option<String>;
    /// fn `span_id`.
    fn span_id(&self) -> Option<String>;

    /// fn `box_clone`.
    fn box_clone(&self) -> Box<dyn TraceContextProvider + Send + Sync>;
}
