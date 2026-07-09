#![forbid(unsafe_code)]

use rhelma_db::{
    ResolvedWorkspaceContext, WorkspaceAccessPolicy, WorkspaceAgentPolicy, WorkspaceAiPolicy,
    WorkspaceRole, WorkspaceVisibility,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason_code: String,
    pub reason: String,
}

impl PolicyDecision {
    pub fn allow(reason_code: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            allowed: true,
            reason_code: reason_code.into(),
            reason: reason.into(),
        }
    }

    pub fn deny(reason_code: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason_code: reason_code.into(),
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyActorContext {
    pub role: Option<WorkspaceRole>,
    pub is_owner: bool,
}

impl PolicyActorContext {
    pub fn owner() -> Self {
        Self {
            role: Some(WorkspaceRole::Owner),
            is_owner: true,
        }
    }

    pub fn member(role: WorkspaceRole) -> Self {
        Self {
            role: Some(role),
            is_owner: false,
        }
    }

    fn is_workspace_member(&self) -> bool {
        self.is_owner || self.role.is_some()
    }

    fn has_admin_role(&self) -> bool {
        self.is_owner || matches!(self.role, Some(WorkspaceRole::Owner | WorkspaceRole::Admin))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceReadRequest {
    pub service_name: String,
    pub metadata_only: bool,
}

impl ServiceReadRequest {
    pub fn metadata(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            metadata_only: true,
        }
    }

    pub fn content(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            metadata_only: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentActionPolicyRequest {
    pub action_type: String,
    pub risk_level: AgentActionRisk,
    pub confirmation_provided: bool,
}

impl AgentActionPolicyRequest {
    pub fn new(
        action_type: impl Into<String>,
        risk_level: AgentActionRisk,
        confirmation_provided: bool,
    ) -> Self {
        Self {
            action_type: action_type.into(),
            risk_level,
            confirmation_provided,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceAiPurpose {
    Metadata,
    WorkspaceContent,
    CodeRemediation,
}

pub fn can_view_workspace(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
) -> PolicyDecision {
    match workspace.visibility {
        WorkspaceVisibility::Private if actor.is_workspace_member() => PolicyDecision::allow(
            "workspace_private_member",
            "actor is workspace owner or member",
        ),
        WorkspaceVisibility::Private => PolicyDecision::deny(
            "workspace_private",
            "private workspace requires owner or member access",
        ),
        WorkspaceVisibility::Unlisted => PolicyDecision::allow(
            "workspace_unlisted_direct",
            "unlisted workspace resolved directly",
        ),
        WorkspaceVisibility::PublicProfile
        | WorkspaceVisibility::PublicReadable
        | WorkspaceVisibility::PublicJoinable => {
            PolicyDecision::allow("workspace_public", "workspace visibility allows view")
        }
    }
}

pub fn can_join_workspace(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
) -> PolicyDecision {
    if actor.is_workspace_member() {
        return PolicyDecision::allow("already_member", "actor already belongs to workspace");
    }

    match workspace.access_policy {
        WorkspaceAccessPolicy::OwnerOnly => {
            PolicyDecision::deny("join_owner_only", "workspace is owner-only")
        }
        WorkspaceAccessPolicy::InviteOnly => {
            PolicyDecision::deny("join_invite_only", "workspace requires an invitation")
        }
        WorkspaceAccessPolicy::RequestToJoin => {
            PolicyDecision::allow("join_request_allowed", "actor may request to join")
        }
        WorkspaceAccessPolicy::OpenJoin => {
            PolicyDecision::allow("join_open", "workspace allows open join")
        }
    }
}

pub fn can_read_service(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
    service: &ServiceReadRequest,
) -> PolicyDecision {
    let view = can_view_workspace(actor, workspace);
    if !view.allowed {
        return view;
    }

    let service_enabled = workspace
        .enabled_services
        .iter()
        .any(|enabled| enabled.service_name == service.service_name);
    if !service_enabled {
        return PolicyDecision::deny("service_disabled", "service is not enabled in workspace");
    }

    match workspace.visibility {
        WorkspaceVisibility::Private => PolicyDecision::allow(
            "service_private_member",
            "actor may read private workspace service",
        ),
        WorkspaceVisibility::PublicProfile if service.metadata_only => PolicyDecision::allow(
            "service_metadata_public",
            "public profile metadata is readable",
        ),
        WorkspaceVisibility::PublicProfile => PolicyDecision::deny(
            "service_content_not_public",
            "public profile workspaces expose metadata only",
        ),
        WorkspaceVisibility::Unlisted
        | WorkspaceVisibility::PublicReadable
        | WorkspaceVisibility::PublicJoinable => PolicyDecision::allow(
            "service_public_read",
            "workspace visibility allows service read",
        ),
    }
}

pub fn can_execute_agent_action(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
    action: &AgentActionPolicyRequest,
) -> PolicyDecision {
    if !actor.is_workspace_member() {
        return PolicyDecision::deny(
            "agent_actor_not_member",
            "agent action requires workspace owner or member context",
        );
    }

    match workspace.agent_policy {
        WorkspaceAgentPolicy::Disabled => {
            PolicyDecision::deny("agent_disabled", "agent actions are disabled for workspace")
        }
        WorkspaceAgentPolicy::OwnerConfirmed => {
            if !actor.is_owner {
                return PolicyDecision::deny(
                    "agent_owner_required",
                    "owner-confirmed actions require the workspace owner",
                );
            }
            require_confirmation(action, "agent_owner_confirmed")
        }
        WorkspaceAgentPolicy::RoleConfirmed => {
            if !actor.has_admin_role() {
                return PolicyDecision::deny(
                    "agent_role_required",
                    "role-confirmed actions require owner or admin role",
                );
            }
            require_confirmation(action, "agent_role_confirmed")
        }
        WorkspaceAgentPolicy::AutoLowRisk if action.risk_level == AgentActionRisk::Low => {
            PolicyDecision::allow("agent_auto_low_risk", "low-risk agent action is allowed")
        }
        WorkspaceAgentPolicy::AutoLowRisk => PolicyDecision::deny(
            "agent_risk_too_high",
            "auto-low-risk policy allows only low-risk actions",
        ),
    }
}

// ---------------------------------------------------------------------------
// Wallet policy (Stage 14). The wallet is a workspace-owned service.
//
// Reads require workspace membership. Financial *mutations* (entry, refund,
// payout) additionally require explicit confirmation and elevated roles, on top
// of the capability/entitlement gates enforced elsewhere.
// ---------------------------------------------------------------------------

/// A member (owner/admin/member/viewer) may view the wallet balance.
pub fn can_view_wallet(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
) -> PolicyDecision {
    wallet_member_read(actor, workspace, "wallet_view")
}

/// A member may read the ledger history.
pub fn can_view_ledger(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
) -> PolicyDecision {
    wallet_member_read(actor, workspace, "ledger_view")
}

/// Creating a ledger entry (deposit/withdrawal/adjustment) is a financial
/// mutation: requires an admin-or-owner role and explicit confirmation.
pub fn can_create_wallet_entry(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
    confirmation_provided: bool,
) -> PolicyDecision {
    wallet_financial_mutation(actor, workspace, confirmation_provided, "wallet_entry")
}

/// Creating a refund is a financial mutation.
pub fn can_request_refund(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
    confirmation_provided: bool,
) -> PolicyDecision {
    wallet_financial_mutation(actor, workspace, confirmation_provided, "wallet_refund")
}

/// Requesting a payout is the highest-sensitivity mutation: owner only, with
/// explicit confirmation.
pub fn can_request_payout(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
    confirmation_provided: bool,
) -> PolicyDecision {
    if !actor.is_workspace_member() {
        return PolicyDecision::deny(
            "wallet_actor_not_member",
            "wallet action requires workspace owner or member context",
        );
    }
    if !actor.is_owner {
        return PolicyDecision::deny(
            "wallet_payout_owner_required",
            "payout requests require the workspace owner",
        );
    }
    if !confirmation_provided {
        return PolicyDecision::deny(
            "wallet_confirmation_required",
            "financial mutation requires explicit confirmation",
        );
    }
    PolicyDecision::allow("wallet_payout_allowed", "owner confirmed payout request")
}

fn wallet_member_read(
    actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
    allow_code: &'static str,
) -> PolicyDecision {
    // Reads are gated on the same workspace-view rule (private ⇒ member only).
    let view = can_view_workspace(actor, workspace);
    if !view.allowed {
        return view;
    }
    PolicyDecision::allow(allow_code, "workspace member may read wallet")
}

fn wallet_financial_mutation(
    actor: &PolicyActorContext,
    _workspace: &ResolvedWorkspaceContext,
    confirmation_provided: bool,
    allow_code: &'static str,
) -> PolicyDecision {
    if !actor.is_workspace_member() {
        return PolicyDecision::deny(
            "wallet_actor_not_member",
            "wallet action requires workspace owner or member context",
        );
    }
    if !actor.has_admin_role() {
        return PolicyDecision::deny(
            "wallet_role_required",
            "financial mutations require owner or admin role",
        );
    }
    if !confirmation_provided {
        return PolicyDecision::deny(
            "wallet_confirmation_required",
            "financial mutation requires explicit confirmation",
        );
    }
    PolicyDecision::allow(allow_code, "authorized confirmed wallet mutation")
}

pub fn can_use_ai_on_workspace(
    _actor: &PolicyActorContext,
    workspace: &ResolvedWorkspaceContext,
    purpose: WorkspaceAiPurpose,
) -> PolicyDecision {
    match workspace.ai_policy {
        WorkspaceAiPolicy::Disabled => {
            PolicyDecision::deny("ai_disabled", "AI usage is disabled for workspace")
        }
        WorkspaceAiPolicy::MetadataOnly if purpose == WorkspaceAiPurpose::Metadata => {
            PolicyDecision::allow("ai_metadata_allowed", "AI may use workspace metadata")
        }
        WorkspaceAiPolicy::MetadataOnly => {
            PolicyDecision::deny("ai_content_blocked", "AI policy allows metadata only")
        }
        WorkspaceAiPolicy::WorkspaceContentAllowed => PolicyDecision::allow(
            "ai_workspace_content_allowed",
            "AI may use workspace content for requested purpose",
        ),
    }
}

fn require_confirmation(
    action: &AgentActionPolicyRequest,
    allowed_code: &'static str,
) -> PolicyDecision {
    if action.confirmation_provided {
        PolicyDecision::allow(allowed_code, "required confirmation was provided")
    } else {
        PolicyDecision::deny(
            "agent_confirmation_required",
            "agent action requires explicit confirmation",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhelma_db::{ResolvedWorkspaceService, WorkspaceAgentPolicy, WorkspaceAiPolicy};
    use serde_json::json;

    fn workspace(
        visibility: WorkspaceVisibility,
        ai_policy: WorkspaceAiPolicy,
        agent_policy: WorkspaceAgentPolicy,
    ) -> ResolvedWorkspaceContext {
        ResolvedWorkspaceContext {
            workspace_id: rhelma_db::types::WorkspaceId::parse(
                "018f6d97-248b-7c54-9a82-58aa7f2ef6df",
            )
            .unwrap(),
            owner_user_id: rhelma_db::types::UserId::parse("018f6d97-248b-7c54-9a82-58aa7f2ef6e0")
                .unwrap(),
            tenant_id: rhelma_db::types::TenantId::parse("tenant-one").unwrap(),
            visibility,
            access_policy: WorkspaceAccessPolicy::RequestToJoin,
            ai_policy,
            agent_policy,
            license_id: None,
            enabled_services: vec![ResolvedWorkspaceService {
                service_name: "profile-service".to_string(),
                config: json!({}),
            }],
        }
    }

    #[test]
    fn private_workspace_owner_allowed() {
        let workspace = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::Disabled,
        );
        let decision = can_view_workspace(&PolicyActorContext::owner(), &workspace);

        assert!(decision.allowed);
    }

    #[test]
    fn private_workspace_non_member_denied() {
        let workspace = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::Disabled,
        );
        let decision = can_view_workspace(&PolicyActorContext::default(), &workspace);

        assert!(!decision.allowed);
        assert_eq!(decision.reason_code, "workspace_private");
    }

    #[test]
    fn public_profile_metadata_allowed() {
        let workspace = workspace(
            WorkspaceVisibility::PublicProfile,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::Disabled,
        );
        let decision = can_read_service(
            &PolicyActorContext::default(),
            &workspace,
            &ServiceReadRequest::metadata("profile-service"),
        );

        assert!(decision.allowed);
        assert_eq!(decision.reason_code, "service_metadata_public");
    }

    #[test]
    fn ai_disabled_blocks_ai_usage() {
        let workspace = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::Disabled,
            WorkspaceAgentPolicy::Disabled,
        );
        let decision = can_use_ai_on_workspace(
            &PolicyActorContext::owner(),
            &workspace,
            WorkspaceAiPurpose::Metadata,
        );

        assert!(!decision.allowed);
        assert_eq!(decision.reason_code, "ai_disabled");
    }

    #[test]
    fn agent_disabled_blocks_agent_actions() {
        let workspace = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::Disabled,
        );
        let decision = can_execute_agent_action(
            &PolicyActorContext::owner(),
            &workspace,
            &AgentActionPolicyRequest::new("post.create", AgentActionRisk::Low, true),
        );

        assert!(!decision.allowed);
        assert_eq!(decision.reason_code, "agent_disabled");
    }

    #[test]
    fn owner_confirmed_requires_confirmation() {
        let workspace = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::OwnerConfirmed,
        );
        let denied = can_execute_agent_action(
            &PolicyActorContext::owner(),
            &workspace,
            &AgentActionPolicyRequest::new("post.create", AgentActionRisk::Low, false),
        );
        let allowed = can_execute_agent_action(
            &PolicyActorContext::owner(),
            &workspace,
            &AgentActionPolicyRequest::new("post.create", AgentActionRisk::Low, true),
        );

        assert!(!denied.allowed);
        assert_eq!(denied.reason_code, "agent_confirmation_required");
        assert!(allowed.allowed);
    }

    #[test]
    fn wallet_read_requires_membership() {
        let ws = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::Disabled,
        );
        assert!(can_view_wallet(&PolicyActorContext::owner(), &ws).allowed);
        assert!(can_view_ledger(&PolicyActorContext::member(WorkspaceRole::Viewer), &ws).allowed);
        // Non-member cannot read a private workspace's wallet.
        let denied = can_view_wallet(&PolicyActorContext::default(), &ws);
        assert!(!denied.allowed);
    }

    #[test]
    fn wallet_mutation_requires_role_and_confirmation() {
        let ws = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::Disabled,
        );
        // Viewer cannot mutate even with confirmation.
        let viewer =
            can_create_wallet_entry(&PolicyActorContext::member(WorkspaceRole::Viewer), &ws, true);
        assert!(!viewer.allowed);
        assert_eq!(viewer.reason_code, "wallet_role_required");

        // Admin without confirmation is denied.
        let no_conf =
            can_create_wallet_entry(&PolicyActorContext::member(WorkspaceRole::Admin), &ws, false);
        assert!(!no_conf.allowed);
        assert_eq!(no_conf.reason_code, "wallet_confirmation_required");

        // Admin with confirmation is allowed.
        let ok =
            can_create_wallet_entry(&PolicyActorContext::member(WorkspaceRole::Admin), &ws, true);
        assert!(ok.allowed);
    }

    #[test]
    fn payout_is_owner_only_confirmed() {
        let ws = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::Disabled,
        );
        let admin = can_request_payout(&PolicyActorContext::member(WorkspaceRole::Admin), &ws, true);
        assert!(!admin.allowed);
        assert_eq!(admin.reason_code, "wallet_payout_owner_required");

        assert!(can_request_payout(&PolicyActorContext::owner(), &ws, true).allowed);
        assert!(!can_request_payout(&PolicyActorContext::owner(), &ws, false).allowed);
    }

    #[test]
    fn auto_low_risk_allows_only_low_risk_actions() {
        let workspace = workspace(
            WorkspaceVisibility::Private,
            WorkspaceAiPolicy::MetadataOnly,
            WorkspaceAgentPolicy::AutoLowRisk,
        );
        let low = can_execute_agent_action(
            &PolicyActorContext::member(WorkspaceRole::Member),
            &workspace,
            &AgentActionPolicyRequest::new("post.react", AgentActionRisk::Low, false),
        );
        let medium = can_execute_agent_action(
            &PolicyActorContext::member(WorkspaceRole::Member),
            &workspace,
            &AgentActionPolicyRequest::new("profile.update", AgentActionRisk::Medium, true),
        );

        assert!(low.allowed);
        assert!(!medium.allowed);
        assert_eq!(medium.reason_code, "agent_risk_too_high");
    }
}
