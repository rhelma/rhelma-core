use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::error::RhelmaError;
use validator::ValidateEmail;

// ==============================================================
// Shared validator for all strong text-based IDs (Tenant/Region)
// ==============================================================

/// Validate a Rhelma v5.1 identifier.
///
/// Rules:
/// - lower-case ASCII letters only
/// - digits allowed
/// - hyphens allowed
/// - minimum length: 3
/// - must not contain whitespace or unicode
/// - trimmed before validation
///
/// This function MUST NOT leak raw invalid input in the error message.
fn validate_identifier(raw: &str, field: &str) -> Result<String, RhelmaError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(RhelmaError::Validation(format!(
            "{field} must not be empty"
        )));
    }

    if trimmed.len() < 3 {
        return Err(RhelmaError::Validation(format!("{field} is too short")));
    }

    if !trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(RhelmaError::Validation(format!(
            "{field} must contain only lowercase ASCII letters, digits, or '-'"
        )));
    }

    Ok(trimmed.to_string())
}

// ==============================================================
// UserId (UUID-based strong ID)
// ==============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    /// Create a random UserId (internal use only).
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a UserId from a string (external input).
    pub fn parse(s: &str) -> Result<Self, RhelmaError> {
        let uuid = Uuid::parse_str(s)
            .map_err(|_| RhelmaError::Validation("invalid user_id format".into()))?;
        Ok(Self(uuid))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ==============================================================
// WorkspaceId (UUID-based strong ID)
// ==============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct WorkspaceId(pub Uuid);

impl WorkspaceId {
    /// Create a random WorkspaceId (internal use only).
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a WorkspaceId from a string (external input).
    pub fn parse(s: &str) -> Result<Self, RhelmaError> {
        let uuid = Uuid::parse_str(s)
            .map_err(|_| RhelmaError::Validation("invalid workspace_id format".into()))?;
        Ok(Self(uuid))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for WorkspaceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ==============================================================
// TenantId (Strong-ID)
// ==============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TenantId(pub String);

impl TenantId {
    /// Parse a tenant ID according to Rhelma v5.1 strict identifier rules.
    pub fn parse(s: &str) -> Result<Self, RhelmaError> {
        Ok(Self(validate_identifier(s, "tenant_id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ==============================================================
// RegionId (Strong-ID)
// ==============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct RegionId(pub String);

impl RegionId {
    pub fn parse(s: &str) -> Result<Self, RhelmaError> {
        Ok(Self(validate_identifier(s, "region")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ==============================================================
// Email (strict, dependency-free validator)
// ==============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Email(pub String);

impl Email {
    /// Parse and validate an email address.
    pub fn parse(s: &str) -> Result<Self, RhelmaError> {
        let trimmed = s.trim();
        if !is_valid_email(trimmed) {
            return Err(RhelmaError::Validation("invalid email address".into()));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Redacted email for safe logs.
    ///
    /// Example:  
    /// `user@example.com` → `u***@example.com`
    pub fn redacted(&self) -> String {
        let [user, domain] = self.0.split('@').collect::<Vec<_>>()[..] else {
            return "***".to_string();
        };

        let first = user.chars().next().unwrap_or('*');
        format!("{first}***@{domain}")
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ==============================================================
// Minimal deterministic email validator (Rhelma v5.1)
// ==============================================================

fn is_valid_email(s: &str) -> bool {
    // trim to avoid leading/trailing spaces
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }

    // validator 0.18 replaced the free `validate_email` fn with the `ValidateEmail` trait.
    trimmed.validate_email()
}
