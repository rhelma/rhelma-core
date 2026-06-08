use std::{
    ffi::OsString,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    sync::{Mutex, OnceLock},
};

/// Global lock for tests that mutate process-wide environment variables.
///
/// Rust tests run in parallel by default, and `std::env::*` is process-global.
/// Without a lock, env-based tests can race and become flaky.
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn with_env_lock<R>(f: impl FnOnce() -> R) -> R {
    let m = ENV_LOCK.get_or_init(|| Mutex::new(()));

    // If a prior test panicked while holding the lock, the mutex may be poisoned.
    // Env-based tests are best-effort isolated; recover the inner guard.
    let guard = m.lock().unwrap_or_else(|e| e.into_inner());

    // Prevent poisoning by catching panics while the guard is held, then re-raising
    // after we drop the guard.
    let res = catch_unwind(AssertUnwindSafe(f));
    drop(guard);

    match res {
        Ok(v) => v,
        Err(p) => resume_unwind(p),
    }
}

/// Run `f` while isolating (snapshotting + clearing + restoring) environment variables
/// that match the given `prefix`.
///
/// We clear both `PREFIX__*` and `PREFIX_*` because the upstream `config` crate
/// may treat `_` as a prefix separator even when `separator("__")` is used for
/// nested keys. Clearing both patterns prevents user/system env contamination.
///
/// This protects tests from:
/// - Parallel execution races (by acquiring `ENV_LOCK`)
/// - User/system environment contamination (by clearing `PREFIX__*`)
/// - Leaking env vars to other tests (by restoring the original snapshot)
#[allow(dead_code)]
pub fn with_isolated_prefix_env<R>(prefix: &str, f: impl FnOnce() -> R) -> R {
    with_env_lock(|| {
        let prefix_pat_double = format!("{}__", prefix);
        let prefix_pat_single = format!("{}_", prefix);
        let prefix_exact = prefix.to_string();

        // Collect keys first; don't mutate while iterating.
        let keys: Vec<OsString> = std::env::vars_os()
            .map(|(k, _)| k)
            .filter(|k| {
                let ks = k.to_string_lossy();
                ks == prefix_exact
                    || ks.starts_with(&prefix_pat_double)
                    || ks.starts_with(&prefix_pat_single)
            })
            .collect();

        // Snapshot old values.
        let snapshot: Vec<(OsString, Option<OsString>)> = keys
            .iter()
            .map(|k| (k.clone(), std::env::var_os(k)))
            .collect();

        // Clear matching keys before running.
        for (k, _) in &snapshot {
            std::env::remove_var(k);
        }

        struct Restore {
            prefix_pat_double: String,
            prefix_pat_single: String,
            prefix_exact: String,
            snapshot: Vec<(OsString, Option<OsString>)>,
        }
        impl Drop for Restore {
            fn drop(&mut self) {
                // Remove any keys created during the test for this prefix.
                let to_remove: Vec<OsString> = std::env::vars_os()
                    .map(|(k, _)| k)
                    .filter(|k| {
                        let ks = k.to_string_lossy();
                        ks == self.prefix_exact
                            || ks.starts_with(&self.prefix_pat_double)
                            || ks.starts_with(&self.prefix_pat_single)
                    })
                    .collect();
                for k in to_remove {
                    std::env::remove_var(k);
                }

                // Restore prior state.
                for (k, v) in self.snapshot.iter() {
                    match v {
                        Some(val) => std::env::set_var(k, val),
                        None => std::env::remove_var(k),
                    }
                }
            }
        }

        let _restore = Restore {
            prefix_pat_double,
            prefix_pat_single,
            prefix_exact,
            snapshot,
        };

        f()
    })
}
