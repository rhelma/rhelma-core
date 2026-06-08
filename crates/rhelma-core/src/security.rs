use crate::RhelmaError;

/// Zero-Trust password policy (no hashing logic).
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    /// Field `min_length`.
    pub min_length: usize,
    /// Field `max_length`.
    pub max_length: usize,
    /// Field `require_uppercase`.
    pub require_uppercase: bool,
    /// Field `require_lowercase`.
    pub require_lowercase: bool,
    /// Field `require_digit`.
    pub require_digit: bool,
    /// Field `require_symbol`.
    pub require_symbol: bool,

    /// Field `disallow_repeated_sequences`.
    pub disallow_repeated_sequences: bool,
    /// Field `reject_common_passwords`.
    pub reject_common_passwords: bool,
}

/// Strength levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordStrength {
    /// Variant `Weak`.
    Weak,
    /// Variant `Fair`.
    Fair,
    /// Variant `Good`.
    Good,
    /// Variant `Strong`.
    Strong,
}

impl PasswordPolicy {
    // ============================================================
    // VALIDATION
    // ============================================================

    pub fn validate(&self, password: &str) -> Result<(), RhelmaError> {
        if password
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(RhelmaError::Validation(
                "password contains whitespace or control characters".into(),
            ));
        }

        if password.contains('\u{200B}') || password.contains('\u{200C}') {
            return Err(RhelmaError::Validation(
                "password contains hidden unicode characters".into(),
            ));
        }

        let len = password.chars().count();

        if len < self.min_length {
            return Err(RhelmaError::Validation(format!(
                "password must be at least {} characters long",
                self.min_length
            )));
        }

        if self.max_length > 0 && len > self.max_length {
            return Err(RhelmaError::Validation(format!(
                "password must not exceed {} characters",
                self.max_length
            )));
        }

        if self.reject_common_passwords {
            const COMMON: [&str; 8] = [
                "12345678",
                "password",
                "qwerty123",
                "admin123",
                "letmein!",
                "test1234",
                "iloveyou",
                "123456789",
            ];
            if COMMON.iter().any(|w| w.eq_ignore_ascii_case(password)) {
                return Err(RhelmaError::Validation(
                    "password is too common or weak".into(),
                ));
            }
        }

        let mut has_upper = false;
        let mut has_lower = false;
        let mut has_digit = false;
        let mut has_symbol = false;

        for ch in password.chars() {
            if ch.is_ascii_uppercase() {
                has_upper = true;
            } else if ch.is_ascii_lowercase() {
                has_lower = true;
            } else if ch.is_ascii_digit() {
                has_digit = true;
            } else if Self::is_valid_symbol(ch) {
                has_symbol = true;
            }
        }

        if self.require_uppercase && !has_upper {
            return Err(RhelmaError::Validation(
                "password must include an uppercase letter".into(),
            ));
        }
        if self.require_lowercase && !has_lower {
            return Err(RhelmaError::Validation(
                "password must include a lowercase letter".into(),
            ));
        }
        if self.require_digit && !has_digit {
            return Err(RhelmaError::Validation(
                "password must include a digit".into(),
            ));
        }
        if self.require_symbol && !has_symbol {
            return Err(RhelmaError::Validation(
                "password must include a symbol".into(),
            ));
        }

        if self.disallow_repeated_sequences {
            let mut last = '\0';
            let mut streak = 1;

            for ch in password.chars() {
                if ch == last {
                    streak += 1;
                    if streak >= 3 {
                        return Err(RhelmaError::Validation(
                            "password contains repeated characters".into(),
                        ));
                    }
                } else {
                    streak = 1;
                }
                last = ch;
            }
        }

        Ok(())
    }

    // ============================================================
    // STRENGTH EVALUATION
    // ============================================================

    pub fn evaluate(&self, password: &str) -> Result<PasswordStrength, RhelmaError> {
        self.validate(password)?;

        let score = Self::score(password);

        let strength = match score {
            0..=2 => PasswordStrength::Weak,
            3..=4 => PasswordStrength::Fair,
            5..=6 => PasswordStrength::Good,
            _ => PasswordStrength::Strong,
        };

        Ok(strength)
    }

    fn score(password: &str) -> usize {
        let mut score = 0;
        // Important: use characters, NOT bytes
        let len = password.chars().count();

        if len >= 12 {
            score += 1;
        }
        if len >= 16 {
            score += 1;
        }
        if len >= 20 {
            score += 1;
        }

        if password.chars().any(|c| c.is_ascii_uppercase()) {
            score += 1;
        }
        if password.chars().any(|c| c.is_ascii_lowercase()) {
            score += 1;
        }
        if password.chars().any(|c| c.is_ascii_digit()) {
            score += 1;
        }
        if password.chars().any(|c| !c.is_ascii_alphanumeric()) {
            score += 1;
        }

        if !password
            .chars()
            .zip(password.chars().skip(1))
            .any(|(a, b)| a == b)
        {
            score += 1;
        }

        score
    }

    #[inline]
    fn is_valid_symbol(ch: char) -> bool {
        ch.is_ascii() && !ch.is_ascii_alphanumeric()
    }
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8, // Your preference
            max_length: 256,

            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_symbol: true,

            disallow_repeated_sequences: true,
            reject_common_passwords: true,
        }
    }
}
