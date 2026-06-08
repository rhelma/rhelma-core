use rhelma_core::prelude::*;

#[test]
fn tenant_id_validation_works() {
    assert!(TenantId::parse("valid-tenant").is_ok());
    assert!(TenantId::parse("INVALID TENANT").is_err());
}

#[test]
fn region_id_validation_works() {
    assert!(RegionId::parse("eu-west-1").is_ok());
    assert!(RegionId::parse("UPPERCASE").is_err());
}
