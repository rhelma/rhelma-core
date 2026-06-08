//! Tenancy and residency primitives.
//!
//! Compliant with Rhelma Contract v5.1:
//! - StrongId-based isolation
//! - Zero-Trust residency enforcement
//! - DR/SLA tiering
//! - Tenant-level AI + PII data governance

use crate::types::{RegionId, TenantId};
use crate::RhelmaError;
use serde::{Deserialize, Serialize};

/// Levels of data isolation for a tenant.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenancyTier {
    /// Shared infra, shared schema.
    Tier1Shared,
    /// Shared DB, isolated schema.
    Tier2SharedDbIsolatedSchema,
    /// Dedicated DB per tenant.
    Tier3DedicatedDb,
}

impl TenancyTier {
    pub fn is_isolated(self) -> bool {
        matches!(
            self,
            Self::Tier2SharedDbIsolatedSchema | Self::Tier3DedicatedDb
        )
    }
}

/// Data residency policy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResidencyPolicy {
    /// Data can reside globally.
    GlobalPreferred,

    /// Prefer primary + backup regions.
    RegionalPreferred,

    /// Must remain in primary region only (strict compliance).
    RegionalRequired,
}

/// Tenant-level configuration record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantProfile {
    /// Field `tenant_id`.
    pub tenant_id: TenantId,
    /// Field `name`.
    pub name: String,

    /// Field `tier`.
    pub tier: TenancyTier,
    /// Field `sla`.
    pub sla: Option<SlaTarget>,
    /// Field `dr_tier`.
    pub dr_tier: Option<DrTier>,

    // Residency
    /// Field `residency`.
    pub residency: ResidencyPolicy,
    /// Field `primary_region`.
    pub primary_region: RegionId,
    /// Field `backup_regions`.
    pub backup_regions: Vec<RegionId>,

    // AI / Data control
    /// Field `ai_allowed`.
    pub ai_allowed: bool,
    /// Field `logging_pii_allowed`.
    pub logging_pii_allowed: bool,

    // Free-form metadata for expansion (v5.x)
    /// Field `metadata`.
    pub metadata: serde_json::Value,
}

impl TenantProfile {
    /// Does the tenant have isolated data boundaries?
    pub fn is_isolated(&self) -> bool {
        self.tier.is_isolated()
    }

    /// Does tenant enforce region restrictions?
    pub fn is_region_sensitive(&self) -> bool {
        !matches!(self.residency, ResidencyPolicy::GlobalPreferred)
    }

    /// Validate region residency per Rhelma v5.1 Zero-Trust rules.
    ///
    /// SAFE ERROR DESIGN:
    /// - NEVER expose sensitive infrastructure details.
    /// - Error message MUST include meaningful identifiers for audit logs.
    pub fn validate_residency(&self, region: &RegionId) -> Result<(), RhelmaError> {
        match self.residency {
            ResidencyPolicy::GlobalPreferred => {
                // Any region allowed
                Ok(())
            }

            // FIX: Changed from duplicate `GlobalPreferred` to `RegionalPreferred`
            ResidencyPolicy::RegionalPreferred => {
                if region == &self.primary_region || self.backup_regions.contains(region) {
                    Ok(())
                } else {
                    Err(RhelmaError::residency_violation(format!(
                        "tenant '{}' cannot use region '{}'",
                        self.tenant_id.as_str(),
                        region.as_str(),
                    )))
                }
            }

            ResidencyPolicy::RegionalRequired => {
                if region == &self.primary_region {
                    Ok(())
                } else {
                    Err(RhelmaError::residency_violation(format!(
                        "tenant '{}' requires primary region '{}', received '{}'",
                        self.tenant_id.as_str(),
                        self.primary_region.as_str(),
                        region.as_str(),
                    )))
                }
            }
        }
    }
}

/// DR tiers (optional SLA/DR model)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DrTier {
    /// Variant `Bronze`.
    Bronze,
    /// Variant `Silver`.
    Silver,
    /// Variant `Gold`.
    Gold,
    /// Variant `Platinum`.
    Platinum,
}

/// SLA target configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SlaTarget {
    /// Field `availability_percent`.
    pub availability_percent: f32,
    /// Field `rto_minutes`.
    pub rto_minutes: u32,
    /// Field `rpo_minutes`.
    pub rpo_minutes: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RegionId, TenantId};

    #[test]
    fn residency_error_message_is_informative_for_required() {
        let tenant_id = TenantId::parse("acme-corp").unwrap();
        let primary = RegionId::parse("eu-west-1").unwrap();
        let us_region = RegionId::parse("us-west-2").unwrap();

        let profile = TenantProfile {
            tenant_id,
            name: "Acme Corp".into(),
            tier: TenancyTier::Tier2SharedDbIsolatedSchema,
            sla: None,
            dr_tier: None,
            residency: ResidencyPolicy::RegionalRequired,
            primary_region: primary,
            backup_regions: vec![],
            ai_allowed: true,
            logging_pii_allowed: false,
            metadata: serde_json::json!({}),
        };

        let err = profile.validate_residency(&us_region).unwrap_err();
        let msg = err.to_string().to_lowercase();

        assert!(msg.contains("acme-corp"));
        assert!(msg.contains("eu-west-1"));
        assert!(msg.contains("us-west-2"));
        assert!(msg.contains("residency"));
    }
}
