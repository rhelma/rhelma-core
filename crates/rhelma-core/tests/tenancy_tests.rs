// crates/rhelma-core/tests/tenancy_tests.rs
use rhelma_core::error::RhelmaError;
use rhelma_core::tenancy::{ResidencyPolicy, TenancyTier, TenantProfile};
use rhelma_core::types::{RegionId, TenantId};

#[test]
fn tenancy_tier_and_residency_roundtrip() {
    let tier = TenancyTier::Tier2SharedDbIsolatedSchema;
    let json = serde_json::to_string(&tier).unwrap();
    assert_eq!(json, "\"TIER2_SHARED_DB_ISOLATED_SCHEMA\"");
    let back: TenancyTier = serde_json::from_str(&json).unwrap();
    assert_eq!(back, tier);

    let residency = ResidencyPolicy::RegionalRequired;
    let json = serde_json::to_string(&residency).unwrap();
    assert_eq!(json, "\"REGIONAL_REQUIRED\"");
    let back: ResidencyPolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(back, residency);
}

fn sample_profile() -> TenantProfile {
    let tenant_id = TenantId::parse("tenant-1").unwrap();
    let primary_region = RegionId::parse("eu-west-1").unwrap();
    let backup = RegionId::parse("eu-central-1").unwrap();

    TenantProfile {
        tenant_id,
        name: "Test Tenant".to_string(),
        tier: TenancyTier::Tier3DedicatedDb,
        sla: None,
        dr_tier: None,
        residency: ResidencyPolicy::RegionalPreferred,
        primary_region,
        backup_regions: vec![backup],
        ai_allowed: true,
        logging_pii_allowed: true,
        metadata: serde_json::json!({}),
    }
}

#[test]
fn tenant_profile_helpers_work() {
    let profile = sample_profile();

    assert!(profile.is_isolated());
    assert!(profile.is_region_sensitive());
}

#[test]
fn residency_global_preferred_allows_any_region() {
    let mut profile = sample_profile();
    profile.residency = ResidencyPolicy::GlobalPreferred;

    let us = RegionId::parse("us-east-1").unwrap();
    assert!(profile.validate_residency(&us).is_ok());
}

#[test]
fn residency_regional_preferred_allows_primary_and_backup_only() {
    let profile = sample_profile();

    let primary = RegionId::parse("eu-west-1").unwrap();
    let backup = RegionId::parse("eu-central-1").unwrap();
    let forbidden = RegionId::parse("us-east-1").unwrap();

    assert!(profile.validate_residency(&primary).is_ok());
    assert!(profile.validate_residency(&backup).is_ok());

    let err = profile.validate_residency(&forbidden).unwrap_err();
    matches!(err, RhelmaError::SecurityPolicy(_));
}

#[test]
fn residency_regional_required_allows_only_primary() {
    let mut profile = sample_profile();
    profile.residency = ResidencyPolicy::RegionalRequired;

    let primary = RegionId::parse("eu-west-1").unwrap();
    let other = RegionId::parse("eu-central-1").unwrap();

    assert!(profile.validate_residency(&primary).is_ok());
    let err = profile.validate_residency(&other).unwrap_err();
    matches!(err, RhelmaError::SecurityPolicy(_));
}
