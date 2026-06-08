//! Global logger state & async worker for rhelma-logger v0.17
//!
//! مسئولیت‌ها:
//! - نگهداری dispatcher / redactor به‌صورت global
//! - مدیریت صف async (crossbeam-channel)
//! - پیاده‌سازی استراتژی‌های backpressure (DropNewest / DropOldest / Block)
//! - graceful shutdown و flush صف در پایان پروسه
//! - snapshot ساده برای تست و دیباگ
//!
//! ⚠️ مهم: این لایه فقط مربوط به logging است.
//! tracing و metrics باید در rhelma-tracing و rhelma-metrics پیاده شوند، نه اینجا.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

// Clippy: reduce type complexity for global handler slots.
type InternalErrorHandler = dyn Fn(&str) + Send + Sync + 'static;
type PiiViolationHandler = dyn Fn(&[String]) + Send + Sync + 'static;
type InternalErrorHandlerBox = Box<InternalErrorHandler>;
type PiiViolationHandlerBox = Box<PiiViolationHandler>;
type InternalErrorHandlerSlot = OnceLock<Mutex<Option<InternalErrorHandlerBox>>>;
type PiiViolationHandlerSlot = OnceLock<Mutex<Option<PiiViolationHandlerBox>>>;

use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{after, bounded, select, Receiver, Sender, TrySendError};
use rand;

use crate::config::{BackpressureStrategy, DispatchMode, Environment, LoggerConfig, LoggerError};
use crate::event::LogEvent;
use crate::extensions::{LogDispatcher, LogMetricsCollector};
use crate::pii::{DefaultPiiRedactor, PiiRedactor};

/// Global dispatcher (مثلاً StdoutDispatcher یا FileDispatcher)
static GLOBAL_DISPATCHER: OnceLock<Mutex<Option<Box<dyn LogDispatcher + Send + Sync>>>> =
    OnceLock::new();

/// Global PII redactor (پیش‌فرض: DefaultPiiRedactor)
static GLOBAL_REDACTOR: OnceLock<Mutex<Option<Box<dyn PiiRedactor + Send + Sync>>>> =
    OnceLock::new();

/// Global metrics collector hook (اختیاری)
static GLOBAL_METRICS: OnceLock<Mutex<Option<Box<dyn LogMetricsCollector + Send + Sync>>>> =
    OnceLock::new();

/// Optional internal error handler (avoid leaking events/PII to stderr by default).
static INTERNAL_ERROR_HANDLER: InternalErrorHandlerSlot = OnceLock::new();

/// Optional hook that fires when PII was redacted (for SOC/SIEM integration).
static PII_VIOLATION_HANDLER: PiiViolationHandlerSlot = OnceLock::new();

/// Sender برای صف async
static ASYNC_SENDER: OnceLock<Sender<LogEvent>> = OnceLock::new();

/// Receiver برای صف async (داخل Mutex برای DropOldest)
static ASYNC_RECEIVER: OnceLock<Mutex<Option<Receiver<LogEvent>>>> = OnceLock::new();

/// Worker thread برای async dispatch
static WORKER_HANDLE: OnceLock<Mutex<Option<JoinHandle<()>>>> = OnceLock::new();

/// کانال shutdown برای graceful exit
static SHUTDOWN_TX: OnceLock<Sender<()>> = OnceLock::new();

/// True after a successful init. Used to prevent double-init without relying on dispatcher presence.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// When true, async dispatch will avoid queueing and will best-effort sync-dispatch.
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// service_name برای snapshot / تست
static SERVICE_NAME: OnceLock<String> = OnceLock::new();

/// capacity صف async (برای snapshot)
static QUEUE_CAPACITY: OnceLock<usize> = OnceLock::new();

/// کانفیگ global (leaked) برای استفاده در builder و snapshot
static LOGGER_CONFIG: OnceLock<&'static LoggerConfig> = OnceLock::new();

/// گرفتن redactor فعلی (برای استفاده در builder)
pub fn global_redactor() -> Option<Box<dyn PiiRedactor + Send + Sync>> {
    GLOBAL_REDACTOR
        .get()
        .and_then(|lock| lock.lock().ok())
        .and_then(|guard| guard.as_ref().map(|r| r.box_clone()))
}

/// ثبت metrics collector (از بیرون، مثلاً rhelma-metrics)
pub fn set_metrics_collector(collector: Box<dyn LogMetricsCollector + Send + Sync>) {
    let lock = GLOBAL_METRICS.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(collector);
    }
}

/// Set an internal error handler (for dispatcher failures, misconfiguration, etc.).
/// The handler must **never** panic.
pub fn set_internal_error_handler(handler: InternalErrorHandlerBox) {
    let lock = INTERNAL_ERROR_HANDLER.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(handler);
    }
}

/// Report an internal error safely.
pub fn report_internal_error(msg: &str) {
    if let Some(lock) = INTERNAL_ERROR_HANDLER.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(ref h) = *guard {
                h(msg);
                return;
            }
        }
    }

    // Default: print a minimal message (no event payloads).
    eprintln!("[rhelma-logger] {msg}");
}

/// Set a hook that fires when PII redaction happened.
pub fn set_pii_violation_handler(handler: PiiViolationHandlerBox) {
    let lock = PII_VIOLATION_HANDLER.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(handler);
    }
}

/// Internal: notify about PII redaction.
pub(crate) fn notify_pii_violation(redacted_fields: &[String]) {
    if redacted_fields.is_empty() {
        return;
    }
    if let Some(lock) = PII_VIOLATION_HANDLER.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(ref h) = *guard {
                h(redacted_fields);
            }
        }
    }
}

/// گرفتن metrics collector فعلی (clone به‌صورت Box جدید)
fn global_metrics_collector() -> Option<Box<dyn LogMetricsCollector + Send + Sync>> {
    GLOBAL_METRICS
        .get()
        .and_then(|lock| lock.lock().ok())
        .and_then(|guard| guard.as_ref().map(|m| m.box_clone()))
}

/// Snapshot ساده از وضعیت logger برای تست/دیباگ.
#[derive(Debug, Clone)]
pub struct LoggerStateSnapshot {
    /// Field `is_initialized`.
    pub is_initialized: bool,
    /// Field `dispatch_mode`.
    pub dispatch_mode: Option<DispatchMode>,
    /// Field `queue_capacity`.
    pub queue_capacity: Option<usize>,
    /// Field `queue_len`.
    pub queue_len: Option<usize>,
    /// Field `service_name`.
    pub service_name: Option<String>,
    /// Field `environment`.
    pub environment: Option<Environment>,
}

impl LoggerStateSnapshot {
    pub fn is_async(&self) -> bool {
        matches!(self.dispatch_mode, Some(DispatchMode::Async))
    }
}

/// global config که در install_globals ست می‌شود.
pub fn global_config() -> Option<&'static LoggerConfig> {
    LOGGER_CONFIG.get().copied()
}

/// دسترسی امن به redactor global (بدون clone و بدون دردسر lifetime).
///
/// استفاده:
/// ```ignore
/// state::with_redactor(|opt| {
///     if let Some(red) = opt {
///         // از red استفاده کن
///     }
/// });
/// ```
pub fn with_redactor<F, R>(f: F) -> R
where
    F: FnOnce(Option<&dyn PiiRedactor>) -> R,
{
    if let Some(lock) = GLOBAL_REDACTOR.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(ref boxed) = *guard {
                return f(Some(boxed.as_ref()));
            }
        }
    }
    f(None)
}

/// فقط برای تست‌ها: reset کردن state داخلی (تا حد ممکن).
#[cfg(test)]
pub fn reset_logger_state_for_tests() {
    if let Some(lock) = GLOBAL_DISPATCHER.get() {
        if let Ok(mut g) = lock.lock() {
            *g = None;
        }
    }
    if let Some(lock) = GLOBAL_REDACTOR.get() {
        if let Ok(mut g) = lock.lock() {
            *g = None;
        }
    }
    if let Some(lock) = GLOBAL_METRICS.get() {
        if let Ok(mut g) = lock.lock() {
            *g = None;
        }
    }
    if let Some(lock) = ASYNC_RECEIVER.get() {
        if let Ok(mut g) = lock.lock() {
            *g = None;
        }
    }
    if let Some(lock) = WORKER_HANDLE.get() {
        if let Ok(mut g) = lock.lock() {
            *g = None;
        }
    }
    INITIALIZED.store(false, Ordering::SeqCst);
    SHUTTING_DOWN.store(false, Ordering::SeqCst);
    // NOTE: LOGGER_CONFIG / SERVICE_NAME / QUEUE_CAPACITY چون OnceLock هستند reset نمی‌شوند؛
    // ولی برای تست‌ها، کافی است dispatcher/worker/queue را خالی کنیم تا init مجدد کار کند.
    if let Some(lock) = INTERNAL_ERROR_HANDLER.get() {
        if let Ok(mut g) = lock.lock() {
            *g = None;
        }
    }
    if let Some(lock) = PII_VIOLATION_HANDLER.get() {
        if let Ok(mut g) = lock.lock() {
            *g = None;
        }
    }
}

/// ست‌کردن dispatcher سفارشی (برای تست یا wiring خاص)
pub fn set_dispatcher(dispatcher: Box<dyn LogDispatcher + Send + Sync>) {
    let lock = GLOBAL_DISPATCHER.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        // In async mode, worker holds a snapshot dispatcher; changing globals won't affect it.
        if ASYNC_SENDER.get().is_some() {
            report_internal_error(
                "set_dispatcher called after async init: async worker uses dispatcher snapshot (change may not take effect)"
            );
        }
        *guard = Some(dispatcher);
    }
}

/// ست‌کردن redactor سفارشی (برای تست یا override)
pub fn set_redactor(redactor: Box<dyn PiiRedactor + Send + Sync>) {
    let lock = GLOBAL_REDACTOR.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(redactor);
    }
}

/// اگر redactor ست نشده باشد، DefaultPiiRedactor را نصب می‌کند.
fn ensure_default_redactor() {
    let lock = GLOBAL_REDACTOR.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        if guard.is_none() {
            *guard = Some(Box::new(DefaultPiiRedactor));
        }
    }
}

/// گرفتن dispatcher فعلی (clone به‌صورت Box جدید)
fn current_dispatcher() -> Option<Box<dyn LogDispatcher + Send + Sync>> {
    GLOBAL_DISPATCHER
        .get()
        .and_then(|lock| lock.lock().ok())
        .and_then(|guard| guard.as_ref().map(|d| d.box_clone()))
}

/// Snapshot از وضعیت فعلی logger
pub fn get_state_snapshot() -> LoggerStateSnapshot {
    let is_initialized = INITIALIZED.load(Ordering::SeqCst);

    let has_async = ASYNC_SENDER.get().is_some();

    let dispatch_mode = if has_async {
        Some(DispatchMode::Async)
    } else if is_initialized {
        Some(DispatchMode::Sync)
    } else {
        None
    };

    let queue_capacity = QUEUE_CAPACITY.get().cloned();

    let queue_len = if has_async {
        ASYNC_RECEIVER
            .get()
            .and_then(|lock| lock.lock().ok())
            .and_then(|guard| guard.as_ref().map(|rx| rx.len()))
    } else {
        None
    };

    let environment = LOGGER_CONFIG.get().map(|cfg| cfg.environment);

    LoggerStateSnapshot {
        is_initialized,
        dispatch_mode,
        queue_capacity,
        queue_len,
        service_name: SERVICE_NAME.get().cloned(),
        environment,
    }
}

/// نصب global state بر اساس LoggerConfig.
/// از سوی RhelmaLogger::init_with_config فراخوانی می‌شود.
pub fn install_globals(cfg: &LoggerConfig) -> Result<(), LoggerError> {
    // 1) validation اولیه config
    cfg.validate()?;

    // 2) Prevent double init (independent of dispatcher presence)
    if INITIALIZED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(LoggerError::AlreadyInitialised);
    }

    // 3) Ensure a dispatcher exists (but do not treat a pre-set dispatcher as "already initialised")
    {
        let lock = GLOBAL_DISPATCHER.get_or_init(|| Mutex::new(None));
        let mut guard = lock
            .lock()
            .map_err(|_| LoggerError::Dispatcher("GLOBAL_DISPATCHER lock poisoned".into()))?;

        if guard.is_none() {
            *guard = Some(Box::new(crate::dispatchers::StdoutDispatcher));
        }
    }

    // 4) redactor پیش‌فرض
    ensure_default_redactor();

    // 5) async wiring (در صورت نیاز)
    if cfg.dispatch_mode == DispatchMode::Async {
        if let Err(e) = init_async_worker(cfg) {
            // Roll back init flag on failure so callers can retry.
            INITIALIZED.store(false, Ordering::SeqCst);
            return Err(e);
        }
    }

    // 6) service_name / queue_capacity را برای snapshot نگه دار
    let _ = SERVICE_NAME.set(cfg.service_name.clone());
    let _ = QUEUE_CAPACITY.set(cfg.queue_capacity);

    // 7) Store global config after successful init (avoid leaking on partial init failures).
    // Only set once; later calls are already blocked by INITIALIZED flag.
    if LOGGER_CONFIG.get().is_none() {
        let leaked: &'static LoggerConfig = Box::leak(Box::new(cfg.clone()));
        let _ = LOGGER_CONFIG.set(leaked);
    }

    Ok(())
}

/// راه‌اندازی worker async + کانال‌ها
fn init_async_worker(cfg: &LoggerConfig) -> Result<(), LoggerError> {
    let (tx, rx) = bounded::<LogEvent>(cfg.queue_capacity);

    ASYNC_SENDER
        .set(tx)
        .map_err(|_| LoggerError::Dispatcher("ASYNC_SENDER already set".into()))?;

    let rx_lock = ASYNC_RECEIVER.get_or_init(|| Mutex::new(None));
    {
        let mut guard = rx_lock
            .lock()
            .map_err(|_| LoggerError::Dispatcher("ASYNC_RECEIVER lock poisoned".into()))?;
        *guard = Some(rx.clone());
    }

    // کانال shutdown
    let (shutdown_tx, shutdown_rx) = bounded::<()>(1);
    let _ = SHUTDOWN_TX.set(shutdown_tx);

    // dispatcher فعلی برای worker
    let dispatcher = current_dispatcher()
        .ok_or_else(|| LoggerError::Dispatcher("GLOBAL_DISPATCHER not initialised".into()))?;

    let flush_interval_ms = cfg.flush_interval_ms;

    let handle =
        std::thread::spawn(move || worker_loop(rx, shutdown_rx, dispatcher, flush_interval_ms));

    let lock = WORKER_HANDLE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(handle);
    }

    Ok(())
}

/// حلقه اصلی worker async با flush دوره‌ای
fn worker_loop(
    rx: Receiver<LogEvent>,
    shutdown_rx: Receiver<()>,
    dispatcher: Box<dyn LogDispatcher + Send + Sync>,
    flush_interval_ms: u64,
) {
    let mut last_flush = Instant::now();

    loop {
        // Use a small tick to avoid a busy loop, while still staying responsive.
        select! {
            recv(rx) -> msg => {
                match msg {
                    Ok(event) => dispatcher.dispatch(event),
                    Err(_) => break, // all senders dropped
                }
            }
            recv(shutdown_rx) -> _ => {
                // graceful shutdown: drain remaining queue
                let start = Instant::now();
                while let Ok(event) = rx.try_recv() {
                    dispatcher.dispatch(event);
                    // Safety valve: do not block shutdown forever.
                    if start.elapsed() > Duration::from_secs(5) {
                        let remaining = rx.len();
                        if remaining > 0 {
                            crate::state::report_internal_error(&format!(
                                "shutdown drain timeout; {remaining} events may be dropped"
                            ));
                        }
                        break;
                    }
                }
                dispatcher.flush();
                break;
            }
            // Tick
            recv(after(Duration::from_millis(2))) -> _ => {}
        }

        if flush_interval_ms > 0 && last_flush.elapsed() >= Duration::from_millis(flush_interval_ms)
        {
            dispatcher.flush();
            last_flush = Instant::now();
        }
    }
}

/// ارسال event به dispatcher (sync یا async) + sampling + metrics
pub fn dispatch_event(event: LogEvent, cfg: &LoggerConfig) {
    // 0) Metrics: attempted (before sampling/backpressure)
    if let Some(m) = global_metrics_collector() {
        m.record_attempted_event();
    }

    // If shutting down, avoid queueing; best-effort synchronous dispatch.
    if SHUTTING_DOWN.load(Ordering::SeqCst) {
        if let Some(dispatcher) = current_dispatcher() {
            dispatcher.dispatch(event);
            dispatcher.flush();
            if let Some(m) = global_metrics_collector() {
                m.record_dispatched_event();
            }
        }
        return;
    }

    // 1) Sampling
    if cfg.sampling_rate < 1.0 {
        let r: f64 = rand::random();
        if r > cfg.sampling_rate {
            if let Some(m) = global_metrics_collector() {
                m.record_sampled_out();
            }
            return; // skip log
        }
    }

    match cfg.dispatch_mode {
        DispatchMode::Sync => {
            if let Some(dispatcher) = current_dispatcher() {
                dispatcher.dispatch(event);

                if let Some(m) = global_metrics_collector() {
                    m.record_dispatched_event();
                }
            }
        }
        DispatchMode::Async => {
            if let Some(sender) = ASYNC_SENDER.get() {
                let sent_ok = match cfg.backpressure {
                    BackpressureStrategy::DropNewest => try_send_drop_newest(sender, event),
                    BackpressureStrategy::DropOldest => try_send_drop_oldest(sender, event),
                    BackpressureStrategy::Block => sender.send(event).is_ok(),
                };

                if sent_ok {
                    if let Some(m) = global_metrics_collector() {
                        m.record_dispatched_event();
                    }
                }
            } else if let Some(dispatcher) = current_dispatcher() {
                dispatcher.dispatch(event);
                if let Some(m) = global_metrics_collector() {
                    m.record_dispatched_event();
                }
            }
        }
    }
}

/// BackpressureStrategy::DropNewest – اگر صف پر است، event جدید drop می‌شود.
fn try_send_drop_newest(sender: &Sender<LogEvent>, event: LogEvent) -> bool {
    match sender.try_send(event) {
        Ok(()) => true,
        Err(TrySendError::Full(_)) => {
            if let Some(m) = global_metrics_collector() {
                m.record_dropped_newest();
            }
            false
        }
        Err(TrySendError::Disconnected(_)) => false,
    }
}

/// BackpressureStrategy::DropOldest – اگر صف پر است، یک event قدیمی drop می‌شود، جدید اضافه می‌شود.
fn try_send_drop_oldest(sender: &Sender<LogEvent>, event: LogEvent) -> bool {
    match sender.try_send(event) {
        Ok(()) => true,
        Err(TrySendError::Full(ev)) => {
            // Drop one oldest item to free up space.
            let mut dropped_oldest = false;

            if let Some(rx_lock) = ASYNC_RECEIVER.get() {
                match rx_lock.lock() {
                    Ok(guard) => {
                        if let Some(ref rx) = *guard {
                            if rx.try_recv().is_ok() {
                                dropped_oldest = true;
                            }
                        }
                    }
                    Err(_) => {
                        // lock poisoned
                        if let Some(m) = global_metrics_collector() {
                            m.record_dropoldest_lock_poisoned();
                        }
                    }
                }
            }

            if dropped_oldest {
                if let Some(m) = global_metrics_collector() {
                    m.record_dropped_oldest();
                }
            } else {
                // We couldn't drop the oldest (no receiver / empty queue / lock issue).
                if let Some(m) = global_metrics_collector() {
                    m.record_dropoldest_lock_poisoned();
                }
            }

            // Try again to send the new event.
            sender.try_send(ev).is_ok()
        }
        Err(TrySendError::Disconnected(_)) => false,
    }
}

/// Graceful shutdown: worker را متوقف می‌کند و صف async را flush می‌کند.
/// در RhelmaLogger::flush_and_shutdown wrap می‌شود.
pub fn flush_and_shutdown() {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);

    // 1) سیگنال shutdown
    if let Some(tx) = SHUTDOWN_TX.get() {
        let _ = tx.send(());
    }

    // 2) join worker
    if let Some(handle_lock) = WORKER_HANDLE.get() {
        if let Ok(mut guard) = handle_lock.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
    }

    // Best-effort flush for sync mode / current dispatcher
    if let Some(dispatcher) = current_dispatcher() {
        dispatcher.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::config::{LogFormat, PerformanceProfile};
    use crate::event::{LogEvent, LogLevel};

    // یک dispatcher ساده که فقط event را می‌گیرد و چیزی نمی‌نویسد
    #[derive(Debug, Clone, Default)]
    struct NoopDispatcher;

    impl LogDispatcher for NoopDispatcher {
        fn dispatch(&self, _event: LogEvent) {}

        fn flush(&self) {}

        fn box_clone(&self) -> Box<dyn LogDispatcher + Send + Sync> {
            Box::new(Self)
        }
    }

    #[derive(Debug, Default)]
    struct Counts {
        dispatched: usize,
        dropped: usize,
        dropped_newest: usize,
        dropped_oldest: usize,
        drop_oldest_poisoned: usize,
    }

    #[derive(Clone)]
    struct TestMetricsCollector {
        inner: Arc<Mutex<Counts>>,
    }

    impl TestMetricsCollector {
        fn new() -> (Self, Arc<Mutex<Counts>>) {
            let inner = Arc::new(Mutex::new(Counts::default()));
            (
                Self {
                    inner: inner.clone(),
                },
                inner,
            )
        }
    }

    impl LogMetricsCollector for TestMetricsCollector {
        fn record_dispatched_event(&self) {
            if let Ok(mut g) = self.inner.lock() {
                g.dispatched += 1;
            }
        }

        fn record_dropped_event(&self) {
            if let Ok(mut g) = self.inner.lock() {
                g.dropped += 1;
            }
        }

        fn record_dropped_newest(&self) {
            if let Ok(mut g) = self.inner.lock() {
                g.dropped_newest += 1;
                g.dropped += 1;
            }
        }

        fn record_dropped_oldest(&self) {
            if let Ok(mut g) = self.inner.lock() {
                g.dropped_oldest += 1;
                g.dropped += 1;
            }
        }

        fn record_dropoldest_lock_poisoned(&self) {
            if let Ok(mut g) = self.inner.lock() {
                g.drop_oldest_poisoned += 1;
                g.dropped += 1;
            }
        }

        fn box_clone(&self) -> Box<dyn LogMetricsCollector + Send + Sync> {
            Box::new(self.clone())
        }
    }

    fn cfg_sync() -> LoggerConfig {
        LoggerConfig {
            service_name: "test-svc".into(),
            service_version: "1.0.0".into(),
            service_instance_id: None,
            environment: Environment::Development,
            region: "local".into(),
            log_level: "info".into(),
            log_format: LogFormat::Json,
            json_enabled: true,
            console_enabled: false,
            sampling_rate: 1.0,
            performance_profile: PerformanceProfile::Balanced,
            dispatch_mode: DispatchMode::Sync,
            queue_capacity: 8,
            backpressure: BackpressureStrategy::DropNewest,
            flush_interval_ms: 0,
        }
    }

    #[test]
    fn install_globals_sets_snapshot_for_sync_mode() {
        reset_logger_state_for_tests();
        let cfg = cfg_sync();

        let res = install_globals(&cfg);
        assert!(res.is_ok());

        let snap = get_state_snapshot();
        assert!(snap.is_initialized);
        assert_eq!(snap.dispatch_mode, Some(DispatchMode::Sync));
        assert_eq!(snap.service_name.as_deref(), Some("test-svc"));
        assert_eq!(snap.environment, Some(Environment::Development));
    }

    #[test]
    fn install_globals_prevents_double_init() {
        reset_logger_state_for_tests();
        let cfg = cfg_sync();

        assert!(install_globals(&cfg).is_ok());
        let second = install_globals(&cfg);
        assert!(matches!(second, Err(LoggerError::AlreadyInitialised)));
    }

    #[test]
    fn dispatch_event_sync_records_metrics() {
        reset_logger_state_for_tests();

        let cfg = cfg_sync();

        let _ = install_globals(&cfg);

        // IMPORTANT: override dispatcher AFTER install
        set_dispatcher(Box::new(NoopDispatcher));

        // IMPORTANT: install metrics collector AFTER dispatcher override
        let (collector, inner) = TestMetricsCollector::new();
        set_metrics_collector(Box::new(collector));

        dispatch_event(LogEvent::new(LogLevel::Info, "hello"), &cfg);

        let counts = inner.lock().unwrap();
        assert_eq!(counts.dispatched, 1);
    }

    #[test]
    fn drop_newest_records_drop_metrics_when_queue_full() {
        reset_logger_state_for_tests();

        // نصب clean metrics collector
        let (collector, inner) = TestMetricsCollector::new();
        set_metrics_collector(Box::new(collector));

        // ایجاد صف پر
        let (tx, _rx) = bounded::<LogEvent>(1);
        tx.send(LogEvent::new(LogLevel::Info, "first")).unwrap();

        // این یکی باید drop شود
        let sent = try_send_drop_newest(&tx, LogEvent::new(LogLevel::Info, "second"));
        assert!(!sent);

        let counts = inner.lock().unwrap();

        assert!(counts.dropped >= 1);
        assert!(counts.dropped_newest >= 1);
    }

    #[test]
    fn drop_oldest_records_drop_oldest_metric_when_queue_full() {
        reset_logger_state_for_tests();

        // نصب clean metrics collector
        let (collector, inner) = TestMetricsCollector::new();
        set_metrics_collector(Box::new(collector));

        // ایجاد صف پر + تنظیم ASYNC_RECEIVER برای drop-oldest logic
        let (tx, rx) = bounded::<LogEvent>(1);
        {
            let lock = ASYNC_RECEIVER.get_or_init(|| Mutex::new(None));
            let mut guard = lock.lock().unwrap();
            *guard = Some(rx.clone());
        }

        tx.send(LogEvent::new(LogLevel::Info, "first")).unwrap();

        // این یکی باید drop-oldest انجام دهد و بعد ارسال شود
        let sent = try_send_drop_oldest(&tx, LogEvent::new(LogLevel::Info, "second"));
        assert!(sent);

        let counts = inner.lock().unwrap();
        assert!(counts.dropped >= 1);
        assert_eq!(counts.dropped_oldest, 1);
    }
}
