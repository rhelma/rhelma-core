#![forbid(unsafe_code)]
//! Tests for `PasswordPolicy` & `PasswordStrength`.
use rhelma_core::{PasswordPolicy, PasswordStrength, RhelmaError};

//
// ---------------------------------------------------------
// Helper macro for matching RhelmaError::Validation
// ---------------------------------------------------------
macro_rules! assert_validation_err {
    ($expr:expr) => {
        match $expr {
            Err(RhelmaError::Validation(_)) => {}
            other => panic!("expected Validation error, got: {:?}", other),
        }
    };
}

//
// ---------------------------------------------------------
// VALIDATION TESTS
// ---------------------------------------------------------
#[test]
fn rejects_whitespace() {
    let policy = PasswordPolicy::default();

    assert_validation_err!(policy.validate("abc def"));
    assert_validation_err!(policy.validate("abc\tdef"));
    assert_validation_err!(policy.validate("abc\ndef"));
}

#[test]
fn rejects_control_characters() {
    let policy = PasswordPolicy::default();
    let bad = format!("abc{}def", '\u{0007}'); // bell

    assert_validation_err!(policy.validate(&bad));
}

#[test]
fn rejects_hidden_unicode_zero_width() {
    let policy = PasswordPolicy::default();

    assert_validation_err!(policy.validate("test\u{200B}pass"));
    assert_validation_err!(policy.validate("test\u{200C}pass"));
}

#[test]
fn enforces_minimum_length() {
    let policy = PasswordPolicy {
        min_length: 12,
        ..Default::default()
    };
    assert_validation_err!(policy.validate("Short1!"));
}

#[test]
fn enforces_maximum_length() {
    let policy = PasswordPolicy {
        max_length: 10,
        ..Default::default()
    };
    assert_validation_err!(policy.validate("THISPASSWORDISWAYTOOLONG123!"));
}

#[test]
fn rejects_common_passwords() {
    let policy = PasswordPolicy::default();

    let bad_list = [
        "12345678",
        "password",
        "qwerty123",
        "admin123",
        "letmein!",
        "test1234",
        "iloveyou",
        "123456789",
    ];

    for p in bad_list {
        assert_validation_err!(policy.validate(p));
    }
}

#[test]
fn requires_upper_lower_digit_symbol() {
    let policy = PasswordPolicy::default();

    // no uppercase
    assert_validation_err!(policy.validate("password123!"));

    // no lowercase
    assert_validation_err!(policy.validate("PASSWORD123!"));

    // no digit
    assert_validation_err!(policy.validate("Password!!!"));

    // no symbol
    assert_validation_err!(policy.validate("Password123"));
}

#[test]
fn rejects_repeated_sequences() {
    let policy = PasswordPolicy::default();

    assert_validation_err!(policy.validate("AAApassword123!"));
    assert_validation_err!(policy.validate("Passsword123!"));
    assert_validation_err!(policy.validate("111Password!!"));
}

#[test]
fn accepts_valid_strong_password() {
    let policy = PasswordPolicy::default();

    let ok = policy.validate("StrongPass123!Good");
    assert!(ok.is_ok());
}

//
// ---------------------------------------------------------
// STRENGTH EVALUATION TESTS
// ---------------------------------------------------------
#[test]
fn evaluates_weak_password() {
    let policy = PasswordPolicy {
        min_length: 8,
        require_uppercase: false,
        require_digit: false,
        require_symbol: false,
        ..Default::default()
    };
    assert_eq!(policy.evaluate("abcdefgh").unwrap(), PasswordStrength::Weak);
}

#[test]
fn evaluates_good_password() {
    let policy = PasswordPolicy::default();
    assert_eq!(
        policy.evaluate("ThisIsGood123!").unwrap(),
        PasswordStrength::Good
    );
}

#[test]
fn evaluates_strong_password() {
    let policy = PasswordPolicy::default();
    assert_eq!(
        policy.evaluate("CorrectHorseBatteryStaple123!").unwrap(),
        PasswordStrength::Strong
    );
}

//
// ---------------------------------------------------------
// ADDITIONAL EDGE CASES
// ---------------------------------------------------------
#[test]
fn repeated_chars_detection_edge_case() {
    let policy = PasswordPolicy::default();

    // two repeated is okay
    assert!(policy.validate("AAbb33!!StrongPass").is_ok());

    // three repeated not okay
    assert_validation_err!(policy.validate("AAAbbb11!!"));
}

#[test]
fn unicode_is_not_mistaken_as_symbol_unless_ascii() {
    let policy = PasswordPolicy::default();

    // Arabic char should NOT count as symbol
    assert_validation_err!(policy.validate("Password123ا"));
}

#[test]
fn long_passphrase_is_valid_and_strong() {
    let policy = PasswordPolicy {
        require_symbol: false,
        max_length: 256,
        ..Default::default()
    };
    let pass = "UltraMegaSuperStrongPhraseDigits123XYZQR";

    assert!(policy.validate(pass).is_ok());
    assert_eq!(policy.evaluate(pass).unwrap(), PasswordStrength::Strong);
}
