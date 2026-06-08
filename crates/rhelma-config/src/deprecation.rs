//! Deprecation warning handling.
//!
//! `rhelma-config` must not assume any logging system is initialized.
//! By default, warnings go to stderr. Downstream services can override this
//! with [`set_deprecation_handler`] to route warnings into their logger.

#![forbid(unsafe_code)]

use once_cell::sync::OnceCell;

/// Callback type for deprecation warnings.
pub type DeprecationHandler = Box<dyn Fn(&str) + Send + Sync + 'static>;

static DEPRECATION_HANDLER: OnceCell<DeprecationHandler> = OnceCell::new();

/// Set a global deprecation handler.
///
/// Returns `true` if the handler was set, `false` if it was already set.
pub fn set_deprecation_handler<F>(handler: F) -> bool
where
    F: Fn(&str) + Send + Sync + 'static,
{
    DEPRECATION_HANDLER.set(Box::new(handler)).is_ok()
}

/// Emit a deprecation warning (routed to handler if installed, else stderr).
pub fn warn(msg: &str) {
    if let Some(handler) = DEPRECATION_HANDLER.get() {
        (handler)(msg);
    } else {
        eprintln!("[rhelma-config] {msg}");
    }
}

/// Emit a deprecation warning at most once for the provided `OnceCell`.
pub fn warn_once(once: &OnceCell<()>, msg: &str) {
    let _ = once.get_or_init(|| {
        warn(msg);
    });
}
