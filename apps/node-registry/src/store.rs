#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::warn;

use crate::config::NodeRegistryPolicy;
use crate::error::RegistryError;
use crate::models::{
    NodeHeartbeatRequestV1, NodeManifestV1, NodeReportOutcomeV1, NodeReportRequestV1, NodeSummaryV1,
};

#[cfg(feature = "federation-sync")]
use crate::models::FederationNodeRecordV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeStatus {
    Active,
    Probation,
    Suspended,
}

impl NodeStatus {
    fn as_str(&self) -> &'static str {
        match self {
            NodeStatus::Active => "active",
            NodeStatus::Probation => "probation",
            NodeStatus::Suspended => "suspended",
        }
    }

    fn from_str(s: &str) -> NodeStatus {
        if s.eq_ignore_ascii_case("active") {
            NodeStatus::Active
        } else if s.eq_ignore_ascii_case("suspended") {
            NodeStatus::Suspended
        } else {
            NodeStatus::Probation
        }
    }
}

#[derive(Debug, Clone)]
struct NodeRecord {
    manifest: NodeManifestV1,
    registered_at: DateTime<Utc>,
    last_heartbeat_at: DateTime<Utc>,
    reputation: i32,
    attested: bool,
    status: NodeStatus,
    // Local-only suspension tracking; not part of federation contract v1.
    suspended_until: Option<DateTime<Utc>>,
}

/// Filters for node discovery queries (kept as a struct to avoid `clippy::too_many_arguments`).
#[derive(Debug, Clone, Copy, Default)]
pub struct DiscoverFilter<'a> {
    /// Field `capability`.
    pub capability: Option<&'a str>,
    /// Field `region`.
    pub region: Option<&'a str>,
    /// Field `residency`.
    pub residency: Option<&'a str>,
    /// Field `min_reputation`.
    pub min_reputation: Option<i32>,
    /// Field `require_attested`.
    pub require_attested: Option<bool>,
    /// Field `only_status`.
    pub only_status: Option<&'a str>,
    /// Field `limit`.
    pub limit: usize,
}

pub struct InMemoryNodeStore {
    inner: Arc<RwLock<HashMap<String, NodeRecord>>>,
    max_nodes: usize,
}

impl InMemoryNodeStore {
    pub fn new(max_nodes: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            max_nodes,
        }
    }

    pub async fn upsert_manifest(&self, manifest: NodeManifestV1) -> Result<(), RegistryError> {
        let mut map = self.inner.write().await;

        if !map.contains_key(&manifest.node_id) && map.len() >= self.max_nodes {
            return Err(RegistryError::conflict(
                "registry is full (max nodes reached)",
            ));
        }

        let now = Utc::now();

        if let Some(rec) = map.get_mut(&manifest.node_id) {
            rec.manifest = manifest;
            rec.last_heartbeat_at = now;
        } else {
            map.insert(
                manifest.node_id.clone(),
                NodeRecord {
                    manifest,
                    registered_at: now,
                    last_heartbeat_at: now,
                    reputation: 0,
                    attested: false,
                    status: NodeStatus::Probation,
                    suspended_until: None,
                },
            );
        }
        Ok(())
    }

    pub async fn heartbeat(&self, hb: NodeHeartbeatRequestV1) -> Result<(), RegistryError> {
        let mut map = self.inner.write().await;
        if let Some(rec) = map.get_mut(&hb.node_id) {
            rec.last_heartbeat_at = Utc::now();
            // Auto-unsuspend on time.
            if rec.status == NodeStatus::Suspended {
                if let Some(until) = rec.suspended_until {
                    if Utc::now() >= until {
                        rec.status = NodeStatus::Probation;
                        rec.suspended_until = None;
                    }
                }
            }
            Ok(())
        } else {
            Err(RegistryError::bad_request("unknown node_id"))
        }
    }

    pub async fn set_attestation(
        &self,
        node_id: &str,
        attested: bool,
        manifest_attestation: crate::models::NodeAttestationV1,
    ) -> Result<(), RegistryError> {
        let mut map = self.inner.write().await;
        let rec = map
            .get_mut(node_id)
            .ok_or_else(|| RegistryError::bad_request("unknown node_id"))?;

        rec.manifest.attestation = Some(manifest_attestation);
        rec.attested = attested;
        Ok(())
    }

    pub async fn report_outcome(
        &self,
        report: NodeReportRequestV1,
        policy: &NodeRegistryPolicy,
    ) -> Result<(i32, String), RegistryError> {
        let mut map = self.inner.write().await;
        let rec = map
            .get_mut(&report.node_id)
            .ok_or_else(|| RegistryError::bad_request("unknown node_id"))?;

        let delta: i32 = match report.outcome {
            NodeReportOutcomeV1::Ok => policy.delta_ok,
            NodeReportOutcomeV1::Fail => policy.delta_fail,
            NodeReportOutcomeV1::Timeout => policy.delta_timeout,
            NodeReportOutcomeV1::BadResult => policy.delta_bad_result,
        };

        rec.reputation =
            (rec.reputation + delta).clamp(policy.min_reputation, policy.max_reputation);

        // Suspend on low reputation.
        if rec.reputation <= policy.suspend_threshold {
            rec.status = NodeStatus::Suspended;
            rec.suspended_until =
                Some(Utc::now() + chrono::Duration::seconds(policy.suspend_seconds as i64));
        } else if rec.reputation >= policy.promote_threshold {
            rec.status = NodeStatus::Active;
        } else {
            rec.status = NodeStatus::Probation;
        }

        Ok((rec.reputation, rec.status.as_str().to_string()))
    }

    pub async fn get(&self, node_id: &str) -> Option<NodeSummaryV1> {
        let map = self.inner.read().await;
        map.get(node_id).map(|rec| self.to_summary(rec))
    }
    pub async fn discover(&self, f: DiscoverFilter<'_>) -> Vec<NodeSummaryV1> {
        let map = self.inner.read().await;

        // Deterministic ordering: prefer freshest heartbeats, then older registrations.
        let mut recs: Vec<&NodeRecord> = map.values().collect();
        recs.sort_by(|a, b| {
            b.last_heartbeat_at
                .cmp(&a.last_heartbeat_at)
                .then_with(|| b.registered_at.cmp(&a.registered_at))
        });

        let parsed_status = f.only_status.map(NodeStatus::from_str);

        let mut out: Vec<NodeSummaryV1> = Vec::new();

        for rec in recs {
            // Lazy auto-unsuspend: we can't mutate under read lock; heartbeat/report will clear.
            if rec.status == NodeStatus::Suspended {
                if let Some(until) = rec.suspended_until {
                    if Utc::now() >= until {
                        // noop
                    }
                }
            }

            if let Some(ms) = f.min_reputation {
                if rec.reputation < ms {
                    continue;
                }
            }
            if let Some(req_att) = f.require_attested {
                if req_att && !rec.attested {
                    continue;
                }
            }
            if let Some(st) = parsed_status {
                if rec.status != st {
                    continue;
                }
            }
            if let Some(r) = f.region {
                if !rec.manifest.region.eq_ignore_ascii_case(r) {
                    continue;
                }
            }
            if let Some(c) = f.capability {
                if !rec
                    .manifest
                    .capabilities
                    .iter()
                    .any(|x| x.eq_ignore_ascii_case(c))
                {
                    continue;
                }
            }
            if let Some(res) = f.residency {
                if !rec
                    .manifest
                    .allowed_residencies
                    .iter()
                    .any(|x| x.eq_ignore_ascii_case(res))
                {
                    continue;
                }
            }

            out.push(self.to_summary(rec));

            if f.limit != 0 && out.len() >= f.limit {
                break;
            }
        }

        out
    }

    pub async fn prune_stale(&self, ttl: Duration) -> usize {
        let mut map = self.inner.write().await;
        let now = Utc::now();
        let before = map.len();

        map.retain(|_, rec| {
            let age = now.signed_duration_since(rec.last_heartbeat_at);
            let keep = age.num_seconds() < ttl.as_secs() as i64;
            if !keep {
                warn!("pruning stale node {}", rec.manifest.node_id);
            }
            keep
        });

        before.saturating_sub(map.len())
    }

    /// Export full registry state records for federation replication.
    /// The caller is expected to sort the returned list deterministically before hashing/signing.
    #[cfg(feature = "federation-sync")]
    pub async fn export_federation_records(&self) -> Vec<FederationNodeRecordV1> {
        let map = self.inner.read().await;
        let mut out = Vec::with_capacity(map.len());
        for rec in map.values() {
            out.push(FederationNodeRecordV1 {
                node_id: rec.manifest.node_id.clone(),
                manifest: rec.manifest.clone(),
                region: rec.manifest.region.clone(),
                allowed_residencies: rec.manifest.allowed_residencies.clone(),
                capabilities: rec.manifest.capabilities.clone(),
                endpoints: rec.manifest.endpoints.clone(),
                registered_at: rec.registered_at,
                last_heartbeat_at: rec.last_heartbeat_at,
                reputation: rec.reputation,
                status: rec.status.as_str().to_string(),
                attested: rec.attested,
            });
        }
        out
    }

    /// Merge federation records into the local registry.
    ///
    /// Conflict rule:
    /// - prefer the record with the newer `last_heartbeat_at`
    /// - if equal, prefer the newer `manifest.issued_at`
    #[cfg(feature = "federation-sync")]
    pub async fn merge_federation_records(
        &self,
        incoming: Vec<FederationNodeRecordV1>,
    ) -> Result<usize, RegistryError> {
        let mut map = self.inner.write().await;
        let mut merged = 0usize;

        for inc in incoming {
            if !map.contains_key(&inc.manifest.node_id) && map.len() >= self.max_nodes {
                // Stop merging if we've hit max capacity.
                break;
            }

            let inc_key = inc.manifest.node_id.clone();
            let inc_status = NodeStatus::from_str(&inc.status);

            match map.get_mut(&inc_key) {
                Some(cur) => {
                    let newer_seen = inc.last_heartbeat_at > cur.last_heartbeat_at;
                    let newer_manifest = inc.manifest.issued_at > cur.manifest.issued_at;

                    if newer_seen
                        || (inc.last_heartbeat_at == cur.last_heartbeat_at && newer_manifest)
                    {
                        cur.manifest = inc.manifest;
                        cur.last_heartbeat_at = inc.last_heartbeat_at;
                        cur.reputation = inc.reputation;
                        cur.attested = inc.attested;
                        cur.status = inc_status;
                        merged += 1;
                    }
                }
                None => {
                    map.insert(
                        inc_key,
                        NodeRecord {
                            manifest: inc.manifest,
                            registered_at: inc.registered_at,
                            last_heartbeat_at: inc.last_heartbeat_at,
                            reputation: inc.reputation,
                            attested: inc.attested,
                            status: inc_status,
                            suspended_until: None,
                        },
                    );
                    merged += 1;
                }
            }
        }

        Ok(merged)
    }

    fn to_summary(&self, rec: &NodeRecord) -> NodeSummaryV1 {
        NodeSummaryV1 {
            node_id: rec.manifest.node_id.clone(),
            region: rec.manifest.region.clone(),
            allowed_residencies: rec.manifest.allowed_residencies.clone(),
            capabilities: rec.manifest.capabilities.clone(),
            endpoints: rec.manifest.endpoints.clone(),
            last_seen_at: rec.last_heartbeat_at,
            reputation: rec.reputation,
            attested: rec.attested,
            status: rec.status.as_str().to_string(),
        }
    }
}
