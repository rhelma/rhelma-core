//! Startup enforcement helpers for governance.

use crate::governance::runtime::GovernanceRuntime;
use crate::governance::state::{current_policy_state, init_policy_state};
use crate::result::RhelmaResult;

use tracing::{info, warn};

/// Perform best-effort governance checks at service startup.
///
/// Behavior:
/// - Always logs whether emergency mode is active.
/// - Optionally validates presence of a policy bundle (fail-closed when configured).
///
/// Recommended use:
/// ```ignore
/// rhelma_core::governance::bootstrap::ensure_governance_ready("api-gateway")?;
/// ```
pub fn ensure_governance_ready(service_name: &str) -> RhelmaResult<()> {
    let rt = GovernanceRuntime::from_env();

    if rt.emergency_mode() {
        warn!(service = %service_name, "governance: emergency mode active");
    } else {
        info!(service = %service_name, "governance: emergency mode inactive");
    }

    match rt.validate_policy_bundle()? {
        Some(b) => {
            info!(service = %service_name, path = %b.path.display(), "governance: policy bundle configured");
        }
        None => {
            if rt.policy_required() {
                // validate_policy_bundle() would have errored already; this is defensive.
                warn!(service = %service_name, "governance: policy bundle required but not present");
            } else {
                info!(service = %service_name, "governance: policy bundle not configured (fail-open)");
            }
        }
    }

    // Load + verify policy bundle (if configured). The resulting state is cached
    // in-process for other modules (e.g., patch-applier safe mode gates).
    init_policy_state(service_name)?;

    if let Some(st) = current_policy_state() {
        if st.safe_mode {
            warn!(service = %service_name, "governance: SAFE MODE enabled");
        }
        if st.emergency_mode {
            warn!(service = %service_name, "governance: EMERGENCY MODE enabled");
        }
        for w in &st.warnings {
            warn!(service = %service_name, warning = %w, "governance: warning");
        }
    }

    Ok(())
}
