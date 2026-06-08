use rhelma_core::prelude::*;

#[test]
fn random_bytes_do_not_panic_in_tenant_or_region_parse() {
    for byte in 0u8..=255 {
        let s = format!("x{}x", byte as char);
        let _ = TenantId::parse(&s);
        let _ = RegionId::parse(&s);
        // فقط مهم اینه که panic نکنه
    }
}
