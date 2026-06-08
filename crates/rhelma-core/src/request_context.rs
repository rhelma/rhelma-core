//! Unified Zero-Trust RequestContext for Rhelma Platform (v5.2).
//!
//! Responsibilities:
//! - Immutable request-scoped context model
//! - Zero-Trust safe header parsing
//! - tenant / region / user / email → invalid = ignored
//! - request_id → invalid = HARD FAIL
//! - Trace context extraction with safe fallback
//!
//! IMPORTANT (Contract v5.2):
//! - RequestContext itself does NOT enforce required fields.
//! - Enforcement (tenant_id / region mandatory) is the responsibility
//!   of the API Gateway / Edge layer.

#![forbid(unsafe_code)]

use crate::trace_context::TraceContext;
use crate::types::{Email, RegionId, TenantId, UserId};
use crate::RhelmaError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Sanitizes metadata like UA, IP, Device.
/// Returns None for invalid or unsafe raw values.
fn sanitize_raw(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Prevent log / header injection
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return None;
    }

    Some(trimmed.to_string())
}

fn parse_bool_header(s: &str) -> Option<bool> {
    let v = s.trim();
    if v.is_empty() {
        return None;
    }
    match v.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

/// Residency policy (Contract v5.2).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResidencyPolicy {
    /// Variant `Global`.
    Global,
    /// Variant `RegionalPreferred`.
    RegionalPreferred,
    /// Variant `RegionalStrict`.
    RegionalStrict,
}

pub type RequestResidency = ResidencyPolicy;

impl ResidencyPolicy {
    fn parse_loose(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "GLOBAL" => Some(Self::Global),
            "REGIONAL_PREFERRED" | "REGIONALPREFERRED" | "PREFERRED" => {
                Some(Self::RegionalPreferred)
            }
            "REGIONAL_STRICT" | "REGIONALSTRICT" | "STRICT" => Some(Self::RegionalStrict),
            _ => None,
        }
    }
}

/// Request flags (Contract v5.2).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestFlags {
    /// Field `read_only`.
    pub read_only: bool,
    /// Field `dry_run`.
    pub dry_run: bool,
    /// Field `ai_safe_mode`.
    pub ai_safe_mode: bool,
    /// Field `debug_mode`.
    pub debug_mode: bool,
}

impl RequestFlags {
    pub fn with_read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self
    }
    pub fn with_dry_run(mut self, value: bool) -> Self {
        self.dry_run = value;
        self
    }
    pub fn with_ai_safe_mode(mut self, value: bool) -> Self {
        self.ai_safe_mode = value;
        self
    }
    pub fn with_debug_mode(mut self, value: bool) -> Self {
        self.debug_mode = value;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    // ------------------------------
    // Tracing
    // ------------------------------
    request_id: Uuid,
    correlation_id: Option<String>,
    trace: TraceContext,

    // ------------------------------
    // Tenancy / Location / Residency
    // ------------------------------
    tenant_id: Option<TenantId>,
    region: Option<RegionId>,
    residency: Option<ResidencyPolicy>,

    // ------------------------------
    // Identity
    // ------------------------------
    user_id: Option<UserId>,
    user_email: Option<Email>,
    session_id: Option<String>,

    // ------------------------------
    // Flags (v5.2)
    // ------------------------------
    flags: RequestFlags,

    // ------------------------------
    // Security Metadata
    // ------------------------------
    client_ip: Option<String>,
    user_agent: Option<String>,
    device_id: Option<String>,
    scopes: Vec<String>,
    roles: Vec<String>,
    mfa_level: Option<String>,
    risk_level: Option<String>,

    // ------------------------------
    // UX
    // ------------------------------
    locale: Option<String>,
}

impl RequestContext {
    // ======================================================
    // Getters (Zero-Copy, Non-Mutating)
    // ======================================================
    pub fn request_id(&self) -> Uuid {
        self.request_id
    }
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }
    pub fn trace(&self) -> &TraceContext {
        &self.trace
    }

    pub fn tenant_id(&self) -> Option<&TenantId> {
        self.tenant_id.as_ref()
    }
    pub fn region(&self) -> Option<&RegionId> {
        self.region.as_ref()
    }
    pub fn residency(&self) -> Option<ResidencyPolicy> {
        self.residency
    }

    pub fn user_id(&self) -> Option<&UserId> {
        self.user_id.as_ref()
    }
    pub fn user_email(&self) -> Option<&Email> {
        self.user_email.as_ref()
    }

    pub fn flags(&self) -> &RequestFlags {
        &self.flags
    }

    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
    }

    pub fn has_tenant(&self) -> bool {
        self.tenant_id.is_some()
    }
    pub fn has_region(&self) -> bool {
        self.region.is_some()
    }

    // ======================================================
    // Constructors
    // ======================================================
    pub fn empty() -> Self {
        Self {
            request_id: Uuid::now_v7(),
            correlation_id: None,
            trace: TraceContext::generate(),

            tenant_id: None,
            region: None,
            residency: None,

            user_id: None,
            user_email: None,
            session_id: None,

            flags: RequestFlags::default(),

            client_ip: None,
            user_agent: None,
            device_id: None,
            scopes: Vec::new(),
            roles: Vec::new(),
            mfa_level: None,
            risk_level: None,

            locale: None,
        }
    }

    /// Zero-Trust header parsing (Rhelma v5.2).
    ///
    /// Compatibility:
    /// - Accepts both canonical v5.2 headers (`x-rhelma-*`) and legacy headers.
    ///
    /// Rules:
    /// - request_id → must be valid UUID → HARD FAIL on invalid
    /// - tenant / region / user / email → invalid MUST be ignored
    /// - trace extraction MUST always succeed (fallback = generate)
    pub fn from_headers<'a, H>(headers: H) -> Result<Self, RhelmaError>
    where
        H: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut ctx = Self::empty();
        let mut trace_headers: Vec<(&str, &str)> = Vec::new();

        for (header_key, header_val) in headers {
            let key = header_key.trim().to_ascii_lowercase();
            let val = header_val.trim();

            match key.as_str() {
                // ------------------------------------------------
                // Correlation
                // ------------------------------------------------
                "x-rhelma-correlation-id" | "x-correlation-id" => {
                    if !val.is_empty() {
                        ctx.correlation_id = Some(val.to_string());
                    }
                }

                // ------------------------------------------------
                // Required (invalid = FAIL)
                // ------------------------------------------------
                "x-rhelma-request-id" | "x-request-id" => {
                    ctx.request_id = Uuid::parse_str(val)
                        .map_err(|_| RhelmaError::BadRequest("invalid request_id".into()))?;
                }

                // ------------------------------------------------
                // Optional strong IDs (invalid = ignored)
                // ------------------------------------------------
                "x-tenant-id" | "x-rhelma-tenant-id" => {
                    if let Ok(t) = TenantId::parse(val) {
                        ctx.tenant_id = Some(t);
                    }
                }

                "x-region" | "x-rhelma-region" => {
                    if let Ok(r) = RegionId::parse(val) {
                        ctx.region = Some(r);
                    }
                }

                "x-residency" | "x-rhelma-residency" => {
                    ctx.residency = ResidencyPolicy::parse_loose(val);
                }

                "x-user-id" | "x-rhelma-user-id" => {
                    ctx.user_id = UserId::parse(val).ok();
                }

                "x-user-email" | "x-rhelma-user-email" => {
                    ctx.user_email = Email::parse(val).ok();
                }

                // ------------------------------------------------
                // Flags (v5.2)
                // ------------------------------------------------
                "x-rhelma-flag-read-only" | "x-rhelma-flags-read-only" => {
                    if let Some(b) = parse_bool_header(val) {
                        ctx.flags.read_only = b;
                    }
                }
                "x-rhelma-flag-dry-run" | "x-rhelma-flags-dry-run" => {
                    if let Some(b) = parse_bool_header(val) {
                        ctx.flags.dry_run = b;
                    }
                }
                "x-rhelma-flag-ai-safe-mode" | "x-rhelma-flags-ai-safe-mode" => {
                    if let Some(b) = parse_bool_header(val) {
                        ctx.flags.ai_safe_mode = b;
                    }
                }
                "x-rhelma-flag-debug-mode" | "x-rhelma-flags-debug-mode" => {
                    if let Some(b) = parse_bool_header(val) {
                        ctx.flags.debug_mode = b;
                    }
                }

                // ------------------------------------------------
                // Security metadata
                // ------------------------------------------------
                "x-session-id" | "x-rhelma-session-id" => ctx.session_id = sanitize_raw(val),
                "x-client-ip" | "x-forwarded-for" => ctx.client_ip = sanitize_raw(val),
                "x-user-agent" | "user-agent" => ctx.user_agent = sanitize_raw(val),
                "x-device-id" | "x-rhelma-device-id" => ctx.device_id = sanitize_raw(val),

                // ------------------------------------------------
                // Localization
                // ------------------------------------------------
                "x-locale" | "x-rhelma-locale" => ctx.locale = sanitize_raw(val),

                // ------------------------------------------------
                // Trace headers (collect first)
                // ------------------------------------------------
                "traceparent" | "x-trace-id" | "x-span-id" | "x-rhelma-trace-id"
                | "x-rhelma-span-id" => {
                    trace_headers.push((header_key, header_val));
                }

                _ => {}
            }
        }

        if !trace_headers.is_empty() {
            ctx.trace = TraceContext::extract_from_headers(trace_headers);
        }

        Ok(ctx)
    }

    // ======================================================
    // Builder-style helpers (non-mutating semantics)
    // ======================================================
    pub fn with_tenant(mut self, t: TenantId) -> Self {
        self.tenant_id = Some(t);
        self
    }

    pub fn with_region(mut self, r: RegionId) -> Self {
        self.region = Some(r);
        self
    }

    pub fn with_residency(mut self, r: ResidencyPolicy) -> Self {
        self.residency = Some(r);
        self
    }

    pub fn with_flags(mut self, flags: RequestFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_user(mut self, uid: UserId, email: Option<Email>) -> Self {
        self.user_id = Some(uid);
        self.user_email = email;
        self
    }

    pub fn with_locale<S: Into<String>>(mut self, loc: S) -> Self {
        self.locale = Some(loc.into());
        self
    }

    pub fn add_scope<S: Into<String>>(mut self, val: S) -> Self {
        self.scopes.push(val.into());
        self
    }

    pub fn add_role<S: Into<String>>(mut self, val: S) -> Self {
        self.roles.push(val.into());
        self
    }
}
