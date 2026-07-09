#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceCapabilityManifest {
    pub service_name: String,
    #[serde(default)]
    pub service_version: String,
    #[serde(default)]
    pub manifest_version: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub agent_actions: Vec<AgentActionDefinition>,
    pub events: Vec<String>,
    pub permissions: Vec<String>,
    #[serde(default)]
    pub required_entitlements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentActionDefinition {
    pub action_type: String,
    pub service_name: String,
    #[serde(default)]
    pub canonical_action_type: String,
    #[serde(default)]
    pub legacy_aliases: Vec<String>,
    pub input_schema: InputSchema,
    pub risk_level: ActionRiskLevel,
    pub requires_confirmation: bool,
    pub required_permissions: Vec<String>,
    pub required_entitlements: Vec<String>,
    #[serde(default)]
    pub dry_run_supported: bool,
    #[serde(default = "default_execute_supported")]
    pub execute_supported: bool,
}

fn default_execute_supported() -> bool {
    true
}

impl AgentActionDefinition {
    pub fn canonical_type(&self) -> &str {
        if self.canonical_action_type.trim().is_empty() {
            &self.action_type
        } else {
            &self.canonical_action_type
        }
    }
}

impl ServiceCapabilityManifest {
    pub fn validate(&self) -> Result<(), CapabilityError> {
        if self.service_name.trim().is_empty() {
            return Err(CapabilityError::InvalidManifest {
                reason: "service_name is required".to_string(),
            });
        }
        if self.service_version.trim().is_empty() {
            return Err(CapabilityError::InvalidManifest {
                reason: "service_version is required".to_string(),
            });
        }
        if self.manifest_version.trim().is_empty() {
            return Err(CapabilityError::InvalidManifest {
                reason: "manifest_version is required".to_string(),
            });
        }

        validate_entitlements(&self.required_entitlements)?;
        validate_strings("capability", &self.capabilities)?;
        validate_strings("event", &self.events)?;
        validate_strings("permission", &self.permissions)?;

        let mut canonical_actions = BTreeSet::new();
        for action in &self.agent_actions {
            if action.action_type.trim().is_empty() {
                return Err(CapabilityError::InvalidManifest {
                    reason: "action_type is required".to_string(),
                });
            }
            if action.service_name.trim().is_empty() {
                return Err(CapabilityError::InvalidManifest {
                    reason: format!("service_name is required for action {}", action.action_type),
                });
            }
            if action.service_name != self.service_name {
                return Err(CapabilityError::InvalidManifest {
                    reason: format!(
                        "action {} declares service {}, expected {}",
                        action.action_type, action.service_name, self.service_name
                    ),
                });
            }
            let canonical = action.canonical_type();
            if canonical != action.action_type {
                return Err(CapabilityError::InvalidManifest {
                    reason: format!(
                        "canonical_action_type for {} must equal action_type; use legacy_aliases for aliases",
                        action.action_type
                    ),
                });
            }
            if !canonical_actions.insert(canonical.to_string()) {
                return Err(CapabilityError::InvalidManifest {
                    reason: format!("duplicate action_type: {canonical}"),
                });
            }
            validate_strings("required_permission", &action.required_permissions)?;
            validate_entitlements(&action.required_entitlements)?;
            validate_strings("legacy_alias", &action.legacy_aliases)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl ActionRiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputSchema {
    pub required: Vec<String>,
    pub any_required: Vec<Vec<String>>,
    pub properties: BTreeMap<String, SchemaValueType>,
    pub allow_additional: bool,
}

impl InputSchema {
    pub fn new() -> Self {
        Self {
            required: Vec::new(),
            any_required: Vec::new(),
            properties: BTreeMap::new(),
            allow_additional: true,
        }
    }

    pub fn required(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.required = fields.into_iter().map(Into::into).collect();
        self
    }

    pub fn any_required(
        mut self,
        groups: impl IntoIterator<Item = impl IntoIterator<Item = impl Into<String>>>,
    ) -> Self {
        self.any_required = groups
            .into_iter()
            .map(|group| group.into_iter().map(Into::into).collect())
            .collect();
        self
    }

    pub fn property(mut self, name: impl Into<String>, value_type: SchemaValueType) -> Self {
        self.properties.insert(name.into(), value_type);
        self
    }

    pub fn validate(&self, payload: &Value) -> Result<(), CapabilityError> {
        let object = payload
            .as_object()
            .ok_or_else(|| CapabilityError::InvalidPayloadSchema {
                reason: "payload must be a JSON object".to_string(),
            })?;

        for field in &self.required {
            if missing_or_null(object.get(field)) {
                return Err(CapabilityError::InvalidPayloadSchema {
                    reason: format!("missing required field: {field}"),
                });
            }
        }

        for alternatives in &self.any_required {
            if !alternatives
                .iter()
                .any(|field| !missing_or_null(object.get(field)))
            {
                return Err(CapabilityError::InvalidPayloadSchema {
                    reason: format!("payload must include one of: {}", alternatives.join(", ")),
                });
            }
        }

        for (field, value) in object {
            let Some(value_type) = self.properties.get(field) else {
                if self.allow_additional {
                    continue;
                }
                return Err(CapabilityError::InvalidPayloadSchema {
                    reason: format!("unknown field: {field}"),
                });
            };
            if !value_type.matches(value) {
                return Err(CapabilityError::InvalidPayloadSchema {
                    reason: format!("field has wrong type: {field}"),
                });
            }
        }

        Ok(())
    }
}

impl Default for InputSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SchemaValueType {
    Any,
    String,
    Uuid,
    Boolean,
    Array,
    Object,
}

impl SchemaValueType {
    fn matches(self, value: &Value) -> bool {
        if value.is_null() {
            return true;
        }

        match self {
            Self::Any => true,
            Self::String => value.is_string(),
            Self::Uuid => value
                .as_str()
                .and_then(|raw| Uuid::parse_str(raw).ok())
                .is_some(),
            Self::Boolean => value.is_boolean(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAgentAction {
    pub manifest: ServiceCapabilityManifest,
    pub definition: AgentActionDefinition,
    pub requested_action_type: String,
    pub canonical_action_type: String,
    pub used_alias: bool,
}

#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    manifests: BTreeMap<String, ServiceCapabilityManifest>,
    aliases: BTreeMap<String, String>,
}

impl CapabilityRegistry {
    pub fn new(manifests: Vec<ServiceCapabilityManifest>) -> Self {
        Self::try_new(manifests).expect("built-in capability manifests must be valid")
    }

    pub fn try_new(manifests: Vec<ServiceCapabilityManifest>) -> Result<Self, CapabilityError> {
        let mut by_service = BTreeMap::new();
        let mut aliases = legacy_aliases();

        for manifest in manifests {
            manifest.validate()?;
            if by_service.contains_key(&manifest.service_name) {
                return Err(CapabilityError::InvalidManifest {
                    reason: format!("duplicate service manifest: {}", manifest.service_name),
                });
            }
            for action in &manifest.agent_actions {
                let canonical = action.canonical_type().to_string();
                for alias in &action.legacy_aliases {
                    insert_alias(&mut aliases, alias, &canonical)?;
                }
            }
            by_service.insert(manifest.service_name.clone(), manifest);
        }

        Ok(Self {
            manifests: by_service,
            aliases,
        })
    }

    pub fn built_in() -> Self {
        Self::new(vec![social_manifest(), profile_manifest()])
    }

    pub fn manifests(&self) -> impl Iterator<Item = &ServiceCapabilityManifest> {
        self.manifests.values()
    }

    pub fn resolve_action(
        &self,
        action_type: &str,
    ) -> Result<ResolvedAgentAction, CapabilityError> {
        let canonical = self
            .aliases
            .get(action_type)
            .map(String::as_str)
            .unwrap_or(action_type);
        let used_alias = canonical != action_type;

        for manifest in self.manifests.values() {
            if let Some(definition) = manifest
                .agent_actions
                .iter()
                .find(|definition| definition.canonical_type() == canonical)
            {
                return Ok(ResolvedAgentAction {
                    manifest: manifest.clone(),
                    definition: definition.clone(),
                    requested_action_type: action_type.to_string(),
                    canonical_action_type: canonical.to_string(),
                    used_alias,
                });
            }
        }

        Err(CapabilityError::UnknownAction {
            action_type: action_type.to_string(),
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("unknown action: {action_type}")]
    UnknownAction { action_type: String },
    #[error("invalid payload schema: {reason}")]
    InvalidPayloadSchema { reason: String },
    #[error("invalid capability manifest: {reason}")]
    InvalidManifest { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DryRunMode {
    Static,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteDryRunRequest {
    pub workspace_id: Uuid,
    pub tenant_id: String,
    pub user_id: Uuid,
    pub service_name: String,
    pub action_type: String,
    pub canonical_action_type: String,
    pub payload: Value,
    pub request_id: Uuid,
    pub correlation_id: String,
    pub dry_run_mode: DryRunMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteDryRunResponse {
    pub allowed: bool,
    pub would_execute: bool,
    pub risk_level: ActionRiskLevel,
    pub requires_confirmation: bool,
    pub validation_errors: Vec<String>,
    pub policy_notes: Vec<String>,
    pub estimated_side_effects: Vec<String>,
    pub preview: Value,
    pub correlation_id: String,
}

impl RemoteDryRunResponse {
    pub fn unsupported(
        request: &RemoteDryRunRequest,
        risk_level: ActionRiskLevel,
        requires_confirmation: bool,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            allowed: false,
            would_execute: false,
            risk_level,
            requires_confirmation,
            validation_errors: Vec::new(),
            policy_notes: vec![reason.into()],
            estimated_side_effects: Vec::new(),
            preview: Value::Null,
            correlation_id: request.correlation_id.clone(),
        }
    }

    pub fn invalid(
        request: &RemoteDryRunRequest,
        risk_level: ActionRiskLevel,
        requires_confirmation: bool,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            allowed: false,
            would_execute: false,
            risk_level,
            requires_confirmation,
            validation_errors: vec![reason.into()],
            policy_notes: Vec::new(),
            estimated_side_effects: Vec::new(),
            preview: Value::Null,
            correlation_id: request.correlation_id.clone(),
        }
    }
}

pub fn legacy_alias(action_type: &str) -> Option<&'static str> {
    match action_type {
        "create_post" => Some("post.create"),
        "create_comment" => Some("comment.create"),
        "react_to_post" => Some("post.react"),
        "follow_user" => Some("user.follow"),
        "update_profile" => Some("profile.update"),
        _ => None,
    }
}

fn legacy_aliases() -> BTreeMap<String, String> {
    [
        ("create_post", "post.create"),
        ("create_comment", "comment.create"),
        ("react_to_post", "post.react"),
        ("follow_user", "user.follow"),
        ("update_profile", "profile.update"),
    ]
    .into_iter()
    .map(|(legacy, canonical)| (legacy.to_string(), canonical.to_string()))
    .collect()
}

fn insert_alias(
    aliases: &mut BTreeMap<String, String>,
    alias: &str,
    canonical: &str,
) -> Result<(), CapabilityError> {
    if alias == canonical {
        return Err(CapabilityError::InvalidManifest {
            reason: format!("legacy alias {alias} must not equal canonical action"),
        });
    }

    match aliases.get(alias) {
        Some(existing) if existing == canonical => Ok(()),
        Some(existing) => Err(CapabilityError::InvalidManifest {
            reason: format!("legacy alias {alias} conflicts: {existing} vs {canonical}"),
        }),
        None => {
            aliases.insert(alias.to_string(), canonical.to_string());
            Ok(())
        }
    }
}

fn validate_strings(kind: &str, values: &[String]) -> Result<(), CapabilityError> {
    for value in values {
        if value.trim().is_empty() {
            return Err(CapabilityError::InvalidManifest {
                reason: format!("{kind} must not be empty"),
            });
        }
        if value.chars().any(char::is_whitespace) {
            return Err(CapabilityError::InvalidManifest {
                reason: format!("{kind} must not contain whitespace: {value}"),
            });
        }
    }
    Ok(())
}

fn validate_entitlements(values: &[String]) -> Result<(), CapabilityError> {
    for value in values {
        if !is_known_entitlement_or_quota_key(value) {
            return Err(CapabilityError::InvalidManifest {
                reason: format!("unknown entitlement key: {value}"),
            });
        }
    }
    Ok(())
}

pub fn is_known_entitlement_or_quota_key(key: &str) -> bool {
    matches!(
        key,
        "workspace.custom_domain"
            | "workspace.public_visibility"
            | "workspace.members.max"
            | "workspace.services.social"
            | "agent.enabled"
            | "agent.actions.monthly"
            | "ai.orchestrator.enabled"
            | "ai.code_remediation.enabled"
            | "storage.quota_mb"
    )
}

pub fn social_manifest() -> ServiceCapabilityManifest {
    ServiceCapabilityManifest {
        service_name: "social".to_string(),
        service_version: "0.1.0".to_string(),
        manifest_version: "v1".to_string(),
        version: "v1".to_string(),
        capabilities: vec![
            "posts".to_string(),
            "comments".to_string(),
            "reactions".to_string(),
            "follows".to_string(),
        ],
        agent_actions: vec![
            AgentActionDefinition {
                action_type: "post.create".to_string(),
                service_name: "social".to_string(),
                canonical_action_type: "post.create".to_string(),
                legacy_aliases: vec!["create_post".to_string()],
                input_schema: InputSchema::new()
                    .any_required([["body", "url"]])
                    .property("kind", SchemaValueType::String)
                    .property("status", SchemaValueType::String)
                    .property("title", SchemaValueType::String)
                    .property("body", SchemaValueType::String)
                    .property("url", SchemaValueType::String)
                    .property("tags", SchemaValueType::Array),
                risk_level: ActionRiskLevel::Medium,
                requires_confirmation: true,
                required_permissions: vec!["social.post.create".to_string()],
                required_entitlements: vec!["workspace.services.social".to_string()],
                dry_run_supported: true,
                execute_supported: true,
            },
            AgentActionDefinition {
                action_type: "comment.create".to_string(),
                service_name: "social".to_string(),
                canonical_action_type: "comment.create".to_string(),
                legacy_aliases: vec!["create_comment".to_string()],
                input_schema: InputSchema::new()
                    .required(["body"])
                    .any_required([["post_id"]])
                    .property("post_id", SchemaValueType::Uuid)
                    .property("body", SchemaValueType::String),
                risk_level: ActionRiskLevel::Medium,
                requires_confirmation: true,
                required_permissions: vec!["social.comment.create".to_string()],
                required_entitlements: vec!["workspace.services.social".to_string()],
                dry_run_supported: true,
                execute_supported: true,
            },
            AgentActionDefinition {
                action_type: "post.react".to_string(),
                service_name: "social".to_string(),
                canonical_action_type: "post.react".to_string(),
                legacy_aliases: vec!["react_to_post".to_string()],
                input_schema: InputSchema::new()
                    .required(["post_id", "kind"])
                    .property("post_id", SchemaValueType::Uuid)
                    .property("kind", SchemaValueType::String)
                    .property("active", SchemaValueType::Boolean),
                risk_level: ActionRiskLevel::Low,
                requires_confirmation: false,
                required_permissions: vec!["social.post.react".to_string()],
                required_entitlements: vec!["workspace.services.social".to_string()],
                dry_run_supported: true,
                execute_supported: true,
            },
            AgentActionDefinition {
                action_type: "user.follow".to_string(),
                service_name: "social".to_string(),
                canonical_action_type: "user.follow".to_string(),
                legacy_aliases: vec!["follow_user".to_string()],
                input_schema: InputSchema::new()
                    .any_required([["user_id", "following_id"]])
                    .property("user_id", SchemaValueType::Uuid)
                    .property("following_id", SchemaValueType::Uuid),
                risk_level: ActionRiskLevel::Medium,
                requires_confirmation: true,
                required_permissions: vec!["social.user.follow".to_string()],
                required_entitlements: vec!["workspace.services.social".to_string()],
                dry_run_supported: true,
                execute_supported: true,
            },
        ],
        events: vec![
            "social.post.created.v1".to_string(),
            "social.comment.created.v1".to_string(),
            "social.post.reacted.v1".to_string(),
            "social.user.followed.v1".to_string(),
        ],
        permissions: vec![
            "social.post.create".to_string(),
            "social.comment.create".to_string(),
            "social.post.react".to_string(),
            "social.user.follow".to_string(),
        ],
        required_entitlements: vec!["workspace.services.social".to_string()],
    }
}

pub fn profile_manifest() -> ServiceCapabilityManifest {
    ServiceCapabilityManifest {
        service_name: "profile".to_string(),
        service_version: "0.1.0".to_string(),
        manifest_version: "v1".to_string(),
        version: "v1".to_string(),
        capabilities: vec!["profile.read".to_string(), "profile.update".to_string()],
        agent_actions: vec![
            AgentActionDefinition {
                action_type: "profile.read".to_string(),
                service_name: "profile".to_string(),
                canonical_action_type: "profile.read".to_string(),
                legacy_aliases: Vec::new(),
                input_schema: InputSchema::new(),
                risk_level: ActionRiskLevel::Low,
                requires_confirmation: false,
                required_permissions: vec!["profile.read".to_string()],
                required_entitlements: Vec::new(),
                dry_run_supported: true,
                execute_supported: true,
            },
            AgentActionDefinition {
                action_type: "profile.update".to_string(),
                service_name: "profile".to_string(),
                canonical_action_type: "profile.update".to_string(),
                legacy_aliases: vec!["update_profile".to_string()],
                input_schema: InputSchema::new()
                    .any_required([["display_name", "bio", "avatar_url", "location", "website"]])
                    .property("display_name", SchemaValueType::String)
                    .property("bio", SchemaValueType::String)
                    .property("avatar_url", SchemaValueType::String)
                    .property("location", SchemaValueType::String)
                    .property("website", SchemaValueType::String),
                risk_level: ActionRiskLevel::Medium,
                requires_confirmation: true,
                required_permissions: vec!["profile.update".to_string()],
                required_entitlements: Vec::new(),
                dry_run_supported: true,
                execute_supported: true,
            },
        ],
        events: vec![
            "profile.read.v1".to_string(),
            "profile.updated.v1".to_string(),
        ],
        permissions: vec!["profile.read".to_string(), "profile.update".to_string()],
        required_entitlements: Vec::new(),
    }
}

/// Capability manifest for wallet-service (Stage 13).
///
/// Declares the wallet's agent-visible actions so the internal
/// `GET /internal/capabilities` endpoint returns a validated contract and the
/// internal dry-run can simulate against it. Money-moving actions are
/// `High` risk and always require confirmation. This describes existing wallet
/// behavior only; it does not redesign the wallet (that is Stage 14).
pub fn wallet_manifest() -> ServiceCapabilityManifest {
    ServiceCapabilityManifest {
        service_name: "wallet".to_string(),
        service_version: "0.1.0".to_string(),
        manifest_version: "v1".to_string(),
        version: "v1".to_string(),
        capabilities: vec![
            "balance".to_string(),
            "deposit".to_string(),
            "transfer".to_string(),
        ],
        agent_actions: vec![
            AgentActionDefinition {
                action_type: "wallet.read".to_string(),
                service_name: "wallet".to_string(),
                canonical_action_type: "wallet.read".to_string(),
                legacy_aliases: Vec::new(),
                input_schema: InputSchema::new(),
                risk_level: ActionRiskLevel::Low,
                requires_confirmation: false,
                required_permissions: vec!["wallet.read".to_string()],
                required_entitlements: Vec::new(),
                dry_run_supported: true,
                execute_supported: true,
            },
            AgentActionDefinition {
                action_type: "wallet.deposit".to_string(),
                service_name: "wallet".to_string(),
                canonical_action_type: "wallet.deposit".to_string(),
                legacy_aliases: Vec::new(),
                input_schema: InputSchema::new()
                    .required(["amount_minor"])
                    .property("amount_minor", SchemaValueType::Any)
                    .property("memo", SchemaValueType::String),
                risk_level: ActionRiskLevel::High,
                requires_confirmation: true,
                required_permissions: vec!["wallet.deposit".to_string()],
                required_entitlements: Vec::new(),
                dry_run_supported: true,
                execute_supported: true,
            },
            AgentActionDefinition {
                action_type: "wallet.transfer".to_string(),
                service_name: "wallet".to_string(),
                canonical_action_type: "wallet.transfer".to_string(),
                legacy_aliases: Vec::new(),
                input_schema: InputSchema::new()
                    .required(["to_user_id", "amount_minor"])
                    .property("to_user_id", SchemaValueType::Uuid)
                    .property("amount_minor", SchemaValueType::Any)
                    .property("memo", SchemaValueType::String),
                risk_level: ActionRiskLevel::High,
                requires_confirmation: true,
                required_permissions: vec!["wallet.transfer".to_string()],
                required_entitlements: Vec::new(),
                dry_run_supported: true,
                execute_supported: true,
            },
        ],
        events: vec![
            "wallet.deposited.v1".to_string(),
            "wallet.transferred.v1".to_string(),
        ],
        permissions: vec![
            "wallet.read".to_string(),
            "wallet.deposit".to_string(),
            "wallet.transfer".to_string(),
        ],
        required_entitlements: Vec::new(),
    }
}

pub fn manifest_permission_set(manifest: &ServiceCapabilityManifest) -> BTreeSet<String> {
    manifest.permissions.iter().cloned().collect()
}

fn missing_or_null(value: Option<&Value>) -> bool {
    value.is_none_or(Value::is_null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_action_rejected() {
        let err = CapabilityRegistry::built_in()
            .resolve_action("post.delete")
            .unwrap_err();

        assert!(matches!(err, CapabilityError::UnknownAction { .. }));
    }

    #[test]
    fn legacy_alias_resolves_to_canonical_action() {
        let resolved = CapabilityRegistry::built_in()
            .resolve_action("create_post")
            .unwrap();

        assert_eq!(resolved.canonical_action_type, "post.create");
        assert!(resolved.used_alias);
    }

    #[test]
    fn invalid_payload_schema_rejected() {
        let resolved = CapabilityRegistry::built_in()
            .resolve_action("comment.create")
            .unwrap();

        let err = resolved
            .definition
            .input_schema
            .validate(&json!({"post_id":"not-a-uuid"}))
            .unwrap_err();

        assert!(matches!(err, CapabilityError::InvalidPayloadSchema { .. }));
    }

    #[test]
    fn social_create_post_accepted_through_manifest_alias() {
        let resolved = CapabilityRegistry::built_in()
            .resolve_action("create_post")
            .unwrap();

        resolved
            .definition
            .input_schema
            .validate(&json!({"body":"hello"}))
            .unwrap();
        assert_eq!(resolved.definition.service_name, "social");
        assert_eq!(resolved.definition.risk_level, ActionRiskLevel::Medium);
    }

    #[test]
    fn social_post_create_accepted_through_canonical_manifest_action() {
        let resolved = CapabilityRegistry::built_in()
            .resolve_action("post.create")
            .unwrap();

        resolved
            .definition
            .input_schema
            .validate(&json!({"kind":"link","url":"https://example.test"}))
            .unwrap();
        assert_eq!(resolved.canonical_action_type, "post.create");
    }

    #[test]
    fn profile_update_profile_accepted_through_manifest_alias() {
        let resolved = CapabilityRegistry::built_in()
            .resolve_action("update_profile")
            .unwrap();

        resolved
            .definition
            .input_schema
            .validate(&json!({"bio":"updated"}))
            .unwrap();
        assert_eq!(resolved.canonical_action_type, "profile.update");
    }

    #[test]
    fn dynamic_manifest_validates() {
        let manifest = social_manifest();

        manifest.validate().unwrap();
        assert_eq!(manifest.manifest_version, "v1");
        assert!(manifest
            .agent_actions
            .iter()
            .all(|action| action.dry_run_supported && action.execute_supported));
    }

    #[test]
    fn invalid_manifest_rejected() {
        let mut manifest = social_manifest();
        manifest.manifest_version.clear();

        let err = manifest.validate().unwrap_err();
        assert!(matches!(err, CapabilityError::InvalidManifest { .. }));
    }

    #[test]
    fn alias_conflict_rejected() {
        let mut social = social_manifest();
        let mut profile = profile_manifest();
        profile.agent_actions[1]
            .legacy_aliases
            .push("create_post".to_string());
        social.agent_actions[0]
            .legacy_aliases
            .push("profile_alias".to_string());

        let err = CapabilityRegistry::try_new(vec![social, profile]).unwrap_err();
        assert!(matches!(err, CapabilityError::InvalidManifest { .. }));
    }

    #[test]
    fn dry_run_request_response_serializes() {
        let request = RemoteDryRunRequest {
            workspace_id: Uuid::parse_str("018f6d97-248b-7c54-9a82-58aa7f2ef6df").unwrap(),
            tenant_id: "tenant-one".to_string(),
            user_id: Uuid::parse_str("018f6d97-248b-7c54-9a82-58aa7f2ef6e0").unwrap(),
            service_name: "social".to_string(),
            action_type: "create_post".to_string(),
            canonical_action_type: "post.create".to_string(),
            payload: json!({"body":"hello"}),
            request_id: Uuid::parse_str("018f6d97-248b-7c54-9a82-58aa7f2ef6e1").unwrap(),
            correlation_id: "corr-1".to_string(),
            dry_run_mode: DryRunMode::Remote,
        };

        let response = RemoteDryRunResponse {
            allowed: true,
            would_execute: false,
            risk_level: ActionRiskLevel::Medium,
            requires_confirmation: true,
            validation_errors: Vec::new(),
            policy_notes: vec!["read_only".to_string()],
            estimated_side_effects: vec!["would create post".to_string()],
            preview: json!({"canonical_action_type":"post.create"}),
            correlation_id: request.correlation_id.clone(),
        };

        let encoded = serde_json::to_value((&request, &response)).unwrap();
        let decoded: (RemoteDryRunRequest, RemoteDryRunResponse) =
            serde_json::from_value(encoded).unwrap();

        assert_eq!(decoded.0, request);
        assert_eq!(decoded.1, response);
    }
}
