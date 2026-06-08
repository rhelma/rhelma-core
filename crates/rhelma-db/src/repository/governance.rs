//! Governance-related repositories.
//!
//! These repositories are intentionally minimal and generic so apps can
//! adopt them incrementally.

use crate::repository::BaseRepository;
use crate::DbResult;
use rhelma_core::RequestContext;
use serde_json::Value;

/// Parameters for inserting/upserting a policy bundle.
///
/// This groups related fields to keep repository method signatures small
/// (Clippy: `too_many_arguments`).
#[derive(Clone, Copy, Debug)]
pub struct PolicyBundleInsert<'a> {
    pub bundle_id: &'a str,
    pub version: &'a str,
    pub hash: &'a str,
    pub quorum_signatures: &'a Value,
    pub issuer: Option<&'a str>,
    pub payload: &'a Value,
}

/// Governance repository (policy bundles + succession records).
#[derive(Clone)]
pub struct GovernanceRepository {
    base: BaseRepository,
}

impl GovernanceRepository {
    pub fn new(base: BaseRepository) -> Self {
        Self { base }
    }

    /// Insert a policy bundle row.
    pub async fn insert_policy_bundle(
        &self,
        ctx: &RequestContext,
        insert: PolicyBundleInsert<'_>,
    ) -> DbResult<()> {
        let table = Some("policy_bundles");
        self.base
            .run_db(ctx, crate::metrics::DbOperation::Insert, table, |pool| async move {
                sqlx::query(
                    r#"
                    INSERT INTO policy_bundles (bundle_id, version, hash, quorum_signatures, issuer, payload)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (bundle_id) DO UPDATE
                      SET version = EXCLUDED.version,
                          hash = EXCLUDED.hash,
                          quorum_signatures = EXCLUDED.quorum_signatures,
                          issuer = EXCLUDED.issuer,
                          payload = EXCLUDED.payload,
                          issued_at = NOW()
                    "#,
                )
                .bind(insert.bundle_id)
                .bind(insert.version)
                .bind(insert.hash)
                .bind(insert.quorum_signatures)
                .bind(insert.issuer)
                .bind(insert.payload)
                .execute(&pool)
                .await
                .map(|_| ())
            })
            .await
    }

    /// Fetch the most recently issued policy bundle.
    pub async fn get_current_policy_bundle(
        &self,
        ctx: &RequestContext,
    ) -> DbResult<Option<(String, String, String, Value)>> {
        let table = Some("policy_bundles");
        self.base
            .run_db(
                ctx,
                crate::metrics::DbOperation::Select,
                table,
                |pool| async move {
                    let row = sqlx::query(
                        r#"
                    SELECT bundle_id, version, hash, payload
                    FROM policy_bundles
                    ORDER BY issued_at DESC
                    LIMIT 1
                    "#,
                    )
                    .fetch_optional(&pool)
                    .await?;

                    Ok(row.map(|r| {
                        let bundle_id: String = r.get("bundle_id");
                        let version: String = r.get("version");
                        let hash: String = r.get("hash");
                        let payload: Value = r.get("payload");
                        (bundle_id, version, hash, payload)
                    }))
                },
            )
            .await
    }

    /// Upsert a succession record.
    pub async fn upsert_succession_record(
        &self,
        ctx: &RequestContext,
        creator_id: &str,
        record_version: &str,
        successor_id: &str,
        signature: &str,
    ) -> DbResult<()> {
        let table = Some("succession_records");
        self.base
            .run_db(ctx, crate::metrics::DbOperation::Insert, table, |pool| async move {
                sqlx::query(
                    r#"
                    INSERT INTO succession_records (creator_id, record_version, successor_id, signature)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT (creator_id, record_version) DO UPDATE
                      SET successor_id = EXCLUDED.successor_id,
                          signature = EXCLUDED.signature,
                          created_at = NOW()
                    "#,
                )
                .bind(creator_id)
                .bind(record_version)
                .bind(successor_id)
                .bind(signature)
                .execute(&pool)
                .await
                .map(|_| ())
            })
            .await
    }

    /// Activate a specific succession record (best-effort single active record).
    pub async fn activate_succession_record(
        &self,
        ctx: &RequestContext,
        creator_id: &str,
        record_version: &str,
    ) -> DbResult<()> {
        let table = Some("succession_records");
        self.base
            .run_db(
                ctx,
                crate::metrics::DbOperation::Update,
                table,
                |pool| async move {
                    let mut tx = pool.begin().await?;

                    sqlx::query(
                        r#"
                    UPDATE succession_records
                    SET activated = false, activated_at = NULL
                    WHERE creator_id = $1
                    "#,
                    )
                    .bind(creator_id)
                    .execute(&mut *tx)
                    .await?;

                    sqlx::query(
                        r#"
                    UPDATE succession_records
                    SET activated = true, activated_at = NOW()
                    WHERE creator_id = $1 AND record_version = $2
                    "#,
                    )
                    .bind(creator_id)
                    .bind(record_version)
                    .execute(&mut *tx)
                    .await?;

                    tx.commit().await?;
                    Ok(())
                },
            )
            .await
    }
}

// sqlx::Row is only needed within this module.
use sqlx::Row;
