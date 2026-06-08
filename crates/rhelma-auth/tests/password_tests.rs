use rhelma_auth::prelude::*;

#[test]
fn password_hash_roundtrip() {
    let pw = "StrongPass!234";
    let hash = hash_password(pw).expect("hash ok");
    let ok = verify_password(pw, &hash).expect("verify ok");
    assert!(ok);
}
