use rhelma_core::prelude::*;

#[test]
fn email_redaction_is_correct() {
    let email = Email::parse("john.doe@example.com").unwrap();
    assert_eq!(email.redacted(), "j***@example.com");
}

#[test]
fn email_invalid_formats_fail() {
    assert!(Email::parse("no-at-symbol").is_err());
    assert!(Email::parse("a b@example.com").is_err());
}
