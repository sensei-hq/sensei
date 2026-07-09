//! Collective triage / promote — the receiving-side stage that CLOSES the Dōjō
//! loop.
//!
//! C6 (daemon) publishes contributions into `dojo.artifacts` at
//! `status='submitted'`; C7 (daemon) pulls only `status='published'` artifacts.
//! Nothing in between promotes `submitted → published`, so the loop is open.
//! This module is that promotion:
//!
//! 1. **Cluster by signature** — `submitted` artifacts sharing a `signature`
//!    (the dojo-protocol content hash) within a tenant are the SAME contribution
//!    from N contributors; `contributor_count` is the distinct contributor tally.
//! 2. **Score** — a pure, documented function of the cluster's confidence
//!    signals (contributor breadth + any observed FTR lift). See [`score`].
//! 3. **Auto-approve** — a cluster clearing [`AUTO_APPROVE_SCORE`] publishes its
//!    representative artifact (`status='published'`, `seq` advanced so C7 sees
//!    it) and records an automated `dojo.decisions` row.
//! 4. **Queue for humans** — a cluster below the bar lands in
//!    `dojo.triage_queue` (`state='queued'`) for a maintainer; it is NOT
//!    published.
//! 5. **k-anonymity gate** — for a `global`-scope tenant (the global-dojo
//!    collective) an artifact may auto-publish ONLY once it has at least
//!    [`K_ANONYMITY_MIN_CONTRIBUTORS`] distinct contributors, so a published
//!    global artifact can never de-anonymise a lone contributor.
//!
//! The pure decision logic ([`ClusterSignals`] / [`score`] / [`decide`]) is
//! unit-tested with no database. The store-backed runner ([`HiveStore`]
//! methods below) is transactional and idempotent: re-running never
//! double-publishes and never double-decides (it guards on artifact status /
//! triage state).

use crate::store::{HiveStore, ARTIFACTS_SEQ_LOCK};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx_postgres::PgConnection;
use uuid::Uuid;

// ── Pure triage logic (unit-tested, no DB) ───────────────────────────────────

/// Cluster score at or above which a cluster auto-publishes; below it, the
/// cluster is queued for a maintainer. Chosen so five independent contributors
/// alone clear it (the collective-intelligence "≥ 5 users" heuristic), while a
/// single strong contribution can also clear it on measured efficacy.
pub const AUTO_APPROVE_SCORE: f64 = 0.8;

/// Weight each distinct contributor adds to the score (breadth signal). Five
/// contributors → `5 * 0.16 = 0.80` = exactly the bar, matching the spec's
/// "contributed by ≥ 5 users" example.
pub const PER_CONTRIBUTOR_WEIGHT: f64 = 0.16;

/// Observed first-try-resolution delta that on its own earns full efficacy
/// credit (1.0). A pattern contribution with an FTR lift ≥ this is trusted
/// enough to publish from a single source. (The contribution floor for a
/// pattern is `+0.05`; a `+0.20` lift is a strong, publish-worthy signal.)
pub const FTR_DELTA_FULL_CREDIT: f64 = 0.20;

/// k-anonymity floor: a `global`-scope (global-dojo) artifact may auto-publish
/// only once at least this many DISTINCT contributors stand behind it. Below
/// it, the cluster stays queued regardless of score — never publish a global
/// artifact that could de-anonymise a lone contributor.
pub const K_ANONYMITY_MIN_CONTRIBUTORS: i64 = 3;

/// The confidence signals aggregated over one signature-cluster of `submitted`
/// artifacts. Pure input to [`score`] / [`decide`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusterSignals {
    /// Distinct contributors behind the cluster (drives breadth + k-anonymity).
    pub contributor_count: i64,
    /// Best observed FTR lift across the cluster's pattern payloads, if any.
    pub ftr_delta_observed: Option<f64>,
    /// True when the owning tenant is the `global` collective (k-anonymity
    /// applies).
    pub is_global: bool,
}

/// Confidence score in `0.0..=1.0` from a cluster's signals. Two additive,
/// each-saturating evidence sources:
///
/// * **breadth** = `contributor_count * PER_CONTRIBUTOR_WEIGHT` — more
///   independent contributors ⇒ more trust.
/// * **efficacy** = `max(0, ftr_delta_observed) / FTR_DELTA_FULL_CREDIT` — a
///   measured first-try-resolution lift ⇒ the contribution demonstrably helps.
///
/// The sum is clamped to `1.0`. A negative or absent FTR delta contributes no
/// efficacy (never penalises breadth).
pub fn score(s: &ClusterSignals) -> f64 {
    let breadth = s.contributor_count.max(0) as f64 * PER_CONTRIBUTOR_WEIGHT;
    let efficacy = s.ftr_delta_observed.unwrap_or(0.0).max(0.0) / FTR_DELTA_FULL_CREDIT;
    (breadth + efficacy).clamp(0.0, 1.0)
}

/// Why a cluster was queued instead of auto-approved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueReason {
    /// Score below [`AUTO_APPROVE_SCORE`].
    BelowScoreBar,
    /// A `global` cluster with fewer than [`K_ANONYMITY_MIN_CONTRIBUTORS`]
    /// distinct contributors.
    KAnonymity,
}

impl QueueReason {
    /// Stable string for logs / event detail.
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueReason::BelowScoreBar => "below_score_bar",
            QueueReason::KAnonymity => "k_anonymity",
        }
    }
}

/// The pure triage verdict for a cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Clears the bar (and k-anonymity, when global) → publish.
    AutoApprove,
    /// Below the bar or blocked by k-anonymity → queue for a maintainer.
    Queue(QueueReason),
}

/// Decide a cluster's fate from its signals: score must clear
/// [`AUTO_APPROVE_SCORE`], and a `global` cluster must additionally clear the
/// [`K_ANONYMITY_MIN_CONTRIBUTORS`] floor. The score bar is checked first so a
/// low-confidence global cluster reports the more informative `BelowScoreBar`.
pub fn decide(s: &ClusterSignals) -> Decision {
    if score(s) < AUTO_APPROVE_SCORE {
        return Decision::Queue(QueueReason::BelowScoreBar);
    }
    if s.is_global && s.contributor_count < K_ANONYMITY_MIN_CONTRIBUTORS {
        return Decision::Queue(QueueReason::KAnonymity);
    }
    Decision::AutoApprove
}

// ── Runner types ─────────────────────────────────────────────────────────────

/// Outcome of promoting one signature-cluster.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PromoteOutcome {
    /// The representative artifact was published (auto-approved).
    Published { artifact_id: String, seq: i64 },
    /// The candidates duplicated an already-published signature and were merged
    /// (archived) rather than re-published.
    Merged { into: String },
    /// Below the auto bar (or k-anonymity) — queued for a maintainer.
    Queued { reason: String },
    /// Nothing `submitted` for the signature — already resolved (idempotent).
    NoOp,
}

/// A maintainer's verdict on a queued cluster. Mirrors `dojo.decision_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecideStatus {
    Approve,
    Revise,
    Decline,
}

impl DecideStatus {
    /// Parse the wire value (`approve | revise | decline`).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "approve" => Some(DecideStatus::Approve),
            "revise" => Some(DecideStatus::Revise),
            "decline" => Some(DecideStatus::Decline),
            _ => None,
        }
    }

    /// The `dojo.decision_status` enum string.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            DecideStatus::Approve => "approve",
            DecideStatus::Revise => "revise",
            DecideStatus::Decline => "decline",
        }
    }
}

/// Outcome of a maintainer decision.
#[derive(Debug, Clone, PartialEq)]
pub enum DecideOutcome {
    /// Approved → the representative was published.
    Published { artifact_id: String, seq: i64 },
    /// Declined → the cluster's `submitted` artifacts were archived.
    Declined,
    /// Sent back for revision (no publish, no archive).
    Revised,
    /// No `submitted` cluster under the signature — nothing to decide (→ 404).
    NotFound,
}

/// A triage-queue row for the maintainer console (`GET .../triage`).
#[derive(Debug, Clone, Serialize)]
pub struct TriageRow {
    pub signature: String,
    pub artifact_id: String,
    pub kind: String,
    pub title: String,
    pub owner_scope: Value,
    pub confidence: Option<f64>,
    pub contributor_count: i32,
    pub similarity: Option<f64>,
    pub nearest_artifact_id: Option<String>,
    pub state: String,
    pub created_at: String,
}

// ── Store-backed runner (transactional, idempotent) ──────────────────────────

impl HiveStore {
    /// Promote one signature-cluster within `tenant_id`. Runs in a single
    /// transaction holding [`ARTIFACTS_SEQ_LOCK`] (the same lock publish takes,
    /// so a published `seq` stays gap-free for the pull cursor).
    ///
    /// Idempotent: if nothing is `submitted` under the signature it is a no-op
    /// (the cluster was already resolved). Auto-approve publishes ONE
    /// representative and archives the remaining cluster members as merged
    /// duplicates, so a re-run finds nothing to publish.
    pub async fn promote_cluster(
        &self,
        tenant_id: &Uuid,
        signature: &str,
    ) -> Result<PromoteOutcome, String> {
        let mut tx = self.pool().begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query("SELECT pg_advisory_xact_lock($1)")
            .bind(ARTIFACTS_SEQ_LOCK)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        // Tenant scope decides whether the k-anonymity gate applies.
        let is_global: Option<(bool,)> =
            sqlx_core::query_as::query_as("SELECT scope = 'global' FROM dojo.tenants WHERE id = $1")
                .bind(tenant_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        let Some((is_global,)) = is_global else {
            // Unknown tenant — nothing to promote (handler validated existence).
            return Ok(PromoteOutcome::NoOp);
        };

        // Aggregate the SUBMITTED cluster: how many, distinct contributors, best
        // observed FTR lift, and the representative (earliest by seq).
        let (n_submitted, contributor_count, ftr_delta, rep_id): (
            i64,
            i64,
            Option<f64>,
            Option<Uuid>,
        ) = sqlx_core::query_as::query_as(
            "SELECT count(*)::bigint,
                    count(distinct coalesce(a.contributed_by::text,
                                            a.attribution->>'anonymous_id'))::bigint,
                    max((a.payload->>'ftr_delta_observed')::float8),
                    (array_agg(a.id ORDER BY a.seq))[1]
             FROM dojo.artifacts a
             WHERE a.tenant_id = $1 AND a.signature = $2 AND a.status = 'submitted'",
        )
        .bind(tenant_id)
        .bind(signature)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        if n_submitted == 0 {
            // Already published/archived — re-running neither double-publishes
            // nor double-decides.
            tx.commit().await.map_err(|e| e.to_string())?;
            return Ok(PromoteOutcome::NoOp);
        }
        let rep_id = rep_id
            .ok_or_else(|| "cluster has submitted rows but no representative".to_string())?;
        let owner_scope = artifact_scope(&mut tx, &rep_id).await?;

        // If the signature is already published, the candidates are re-submissions
        // of live content: merge (archive) them rather than publish a duplicate.
        let already_published: Option<(Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM dojo.artifacts
             WHERE tenant_id = $1 AND signature = $2 AND status = 'published'
             ORDER BY seq LIMIT 1",
        )
        .bind(tenant_id)
        .bind(signature)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        let signals = ClusterSignals {
            contributor_count,
            ftr_delta_observed: ftr_delta,
            is_global,
        };
        let confidence = score(&signals);

        if let Some((into,)) = already_published {
            archive_submitted_cluster(&mut tx, tenant_id, signature, None).await?;
            upsert_triage(
                &mut tx,
                tenant_id,
                &rep_id,
                signature,
                &owner_scope,
                confidence,
                contributor_count,
                Some(1.0),
                Some(into),
                "merged",
            )
            .await?;
            insert_event(
                &mut tx,
                tenant_id,
                None,
                Some(&into),
                "merged",
                json!({
                    "signature": signature,
                    "merged_into": into.to_string(),
                    "candidates": n_submitted,
                    "automated": true,
                }),
            )
            .await?;
            tx.commit().await.map_err(|e| e.to_string())?;
            return Ok(PromoteOutcome::Merged {
                into: into.to_string(),
            });
        }

        match decide(&signals) {
            Decision::AutoApprove => {
                let pub_seq = publish_representative(&mut tx, &rep_id).await?;
                archive_submitted_cluster(&mut tx, tenant_id, signature, Some(rep_id)).await?;
                let (triage_id, _) = upsert_triage(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    signature,
                    &owner_scope,
                    confidence,
                    contributor_count,
                    None,
                    None,
                    "auto_approved",
                )
                .await?;
                insert_decision(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    Some(triage_id),
                    None,
                    DecideStatus::Approve,
                    Some("auto-approved: cleared the collective trust bar"),
                    Some(&owner_scope),
                    true,
                )
                .await?;
                insert_event(
                    &mut tx,
                    tenant_id,
                    None,
                    Some(&rep_id),
                    "approved",
                    json!({
                        "automated": true,
                        "score": confidence,
                        "contributor_count": contributor_count,
                    }),
                )
                .await?;
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(PromoteOutcome::Published {
                    artifact_id: rep_id.to_string(),
                    seq: pub_seq,
                })
            }
            Decision::Queue(reason) => {
                let (_, inserted) = upsert_triage(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    signature,
                    &owner_scope,
                    confidence,
                    contributor_count,
                    None,
                    None,
                    "queued",
                )
                .await?;
                // Log 'queued' only when the row is newly created — re-running a
                // sweep on an already-queued cluster must not spam the audit log.
                if inserted {
                    insert_event(
                        &mut tx,
                        tenant_id,
                        None,
                        Some(&rep_id),
                        "queued",
                        json!({
                            "reason": reason.as_str(),
                            "score": confidence,
                            "contributor_count": contributor_count,
                        }),
                    )
                    .await?;
                }
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(PromoteOutcome::Queued {
                    reason: reason.as_str().to_string(),
                })
            }
        }
    }

    /// Sweep the whole tenant: promote every signature that has at least one
    /// `submitted` artifact. Each cluster is promoted in its own transaction
    /// (via [`Self::promote_cluster`]), so the sweep is idempotent.
    pub async fn promote_tenant(
        &self,
        tenant_id: &Uuid,
    ) -> Result<Vec<(String, PromoteOutcome)>, String> {
        let sigs: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT signature FROM dojo.artifacts
             WHERE tenant_id = $1 AND status = 'submitted'",
        )
        .bind(tenant_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| e.to_string())?;

        let mut out = Vec::with_capacity(sigs.len());
        for (sig,) in sigs {
            let outcome = self.promote_cluster(tenant_id, &sig).await?;
            out.push((sig, outcome));
        }
        Ok(out)
    }

    /// List the tenant's open triage rows (`queued` / `in_review`) with cluster
    /// info for the maintainer console. Scoped strictly to `tenant_id`.
    pub async fn list_triage(&self, tenant_id: &Uuid) -> Result<Vec<TriageRow>, String> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            String,         // signature
            String,         // artifact_id
            String,         // kind
            String,         // title
            Value,          // owner_scope
            Option<f64>,    // confidence
            i32,            // contributor_count
            Option<f64>,    // similarity
            Option<String>, // nearest_artifact_id
            String,         // state
            String,         // created_at
        )> = sqlx_core::query_as::query_as(
            "SELECT q.signature, q.artifact_id::text, a.kind::text, a.title,
                    q.owner_scope, q.confidence::float8, q.contributor_count,
                    q.similarity::float8, q.nearest_artifact_id::text, q.state::text,
                    to_char(q.created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
             FROM dojo.triage_queue q
             JOIN dojo.artifacts a ON a.id = q.artifact_id
             WHERE q.tenant_id = $1 AND q.state IN ('queued', 'in_review')
             ORDER BY q.created_at",
        )
        .bind(tenant_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    signature,
                    artifact_id,
                    kind,
                    title,
                    owner_scope,
                    confidence,
                    contributor_count,
                    similarity,
                    nearest_artifact_id,
                    state,
                    created_at,
                )| TriageRow {
                    signature,
                    artifact_id,
                    kind,
                    title,
                    owner_scope,
                    confidence,
                    contributor_count,
                    similarity,
                    nearest_artifact_id,
                    state,
                    created_at,
                },
            )
            .collect())
    }

    /// Record a maintainer decision on a queued signature-cluster and apply it:
    /// `approve` publishes the representative (and archives the duplicates),
    /// `decline` archives the cluster, `revise` notes it back for revision.
    ///
    /// The handler validates the required fields (approve → `distribution_scope`;
    /// decline → non-empty `reason`) and returns 400; this method also guards
    /// them defensively. Returns [`DecideOutcome::NotFound`] when nothing is
    /// `submitted` under the signature (→ 404). Runs in one transaction holding
    /// [`ARTIFACTS_SEQ_LOCK`].
    pub async fn decide_triage(
        &self,
        tenant_id: &Uuid,
        signature: &str,
        status: DecideStatus,
        distribution_scope: Option<Value>,
        reason: Option<String>,
        maintainer_id: Option<Uuid>,
    ) -> Result<DecideOutcome, String> {
        let mut tx = self.pool().begin().await.map_err(|e| e.to_string())?;
        sqlx_core::query::query("SELECT pg_advisory_xact_lock($1)")
            .bind(ARTIFACTS_SEQ_LOCK)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        // Representative = earliest submitted artifact for the signature.
        let rep: Option<(Uuid, Value)> = sqlx_core::query_as::query_as(
            "SELECT id, scope FROM dojo.artifacts
             WHERE tenant_id = $1 AND signature = $2 AND status = 'submitted'
             ORDER BY seq LIMIT 1",
        )
        .bind(tenant_id)
        .bind(signature)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        let Some((rep_id, owner_scope)) = rep else {
            return Ok(DecideOutcome::NotFound);
        };

        // Distinct-contributor tally for the resolved triage row.
        let (contributor_count,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(distinct coalesce(a.contributed_by::text,
                                            a.attribution->>'anonymous_id'))::bigint
             FROM dojo.artifacts a
             WHERE a.tenant_id = $1 AND a.signature = $2 AND a.status = 'submitted'",
        )
        .bind(tenant_id)
        .bind(signature)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        match status {
            DecideStatus::Approve => {
                let dist = distribution_scope
                    .ok_or_else(|| "approve requires distribution_scope".to_string())?;
                let pub_seq = publish_representative(&mut tx, &rep_id).await?;
                archive_submitted_cluster(&mut tx, tenant_id, signature, Some(rep_id)).await?;
                let (triage_id, _) = upsert_triage(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    signature,
                    &owner_scope,
                    1.0,
                    contributor_count,
                    None,
                    None,
                    "resolved",
                )
                .await?;
                insert_decision(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    Some(triage_id),
                    maintainer_id,
                    DecideStatus::Approve,
                    reason.as_deref(),
                    Some(&dist),
                    false,
                )
                .await?;
                insert_event(
                    &mut tx,
                    tenant_id,
                    maintainer_id,
                    Some(&rep_id),
                    "approved",
                    json!({ "automated": false, "distribution_scope": dist }),
                )
                .await?;
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(DecideOutcome::Published {
                    artifact_id: rep_id.to_string(),
                    seq: pub_seq,
                })
            }
            DecideStatus::Decline => {
                let reason = reason
                    .filter(|r| !r.trim().is_empty())
                    .ok_or_else(|| "decline requires a non-empty reason".to_string())?;
                archive_submitted_cluster(&mut tx, tenant_id, signature, None).await?;
                let (triage_id, _) = upsert_triage(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    signature,
                    &owner_scope,
                    0.0,
                    contributor_count,
                    None,
                    None,
                    "resolved",
                )
                .await?;
                insert_decision(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    Some(triage_id),
                    maintainer_id,
                    DecideStatus::Decline,
                    Some(&reason),
                    None,
                    false,
                )
                .await?;
                insert_event(
                    &mut tx,
                    tenant_id,
                    maintainer_id,
                    Some(&rep_id),
                    "declined",
                    json!({ "reason": reason }),
                )
                .await?;
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(DecideOutcome::Declined)
            }
            DecideStatus::Revise => {
                let (triage_id, _) = upsert_triage(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    signature,
                    &owner_scope,
                    0.0,
                    contributor_count,
                    None,
                    None,
                    "in_review",
                )
                .await?;
                insert_decision(
                    &mut tx,
                    tenant_id,
                    &rep_id,
                    Some(triage_id),
                    maintainer_id,
                    DecideStatus::Revise,
                    reason.as_deref(),
                    None,
                    false,
                )
                .await?;
                insert_event(
                    &mut tx,
                    tenant_id,
                    maintainer_id,
                    Some(&rep_id),
                    "revised",
                    json!({ "reason": reason }),
                )
                .await?;
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(DecideOutcome::Revised)
            }
        }
    }
}

// ── Transactional helpers (share one txn / the seq lock) ─────────────────────

/// Fetch an artifact's `scope` jsonb (the owning scope for the triage row and
/// the auto-approve distribution scope).
async fn artifact_scope(conn: &mut PgConnection, artifact_id: &Uuid) -> Result<Value, String> {
    let (scope,): (Value,) =
        sqlx_core::query_as::query_as("SELECT scope FROM dojo.artifacts WHERE id = $1")
            .bind(artifact_id)
            .fetch_one(conn)
            .await
            .map_err(|e| e.to_string())?;
    Ok(scope)
}

/// Publish the representative artifact, advancing `seq` under the caller's
/// [`ARTIFACTS_SEQ_LOCK`] so the pull cursor observes the status change.
/// Guards on `status='submitted'` so a re-run cannot re-publish.
async fn publish_representative(conn: &mut PgConnection, rep_id: &Uuid) -> Result<i64, String> {
    let row: Option<(i64,)> = sqlx_core::query_as::query_as(
        "UPDATE dojo.artifacts
         SET status = 'published', published_at = now(),
             seq = nextval('dojo.artifacts_seq'), updated_at = now()
         WHERE id = $1 AND status = 'submitted'
         RETURNING seq",
    )
    .bind(rep_id)
    .fetch_optional(conn)
    .await
    .map_err(|e| e.to_string())?;
    row.map(|(s,)| s)
        .ok_or_else(|| "representative artifact was not in submitted state".to_string())
}

/// Archive the cluster's `submitted` artifacts (optionally sparing `except` —
/// the just-published representative). These are merged duplicates; they were
/// never distributed, so their `seq` is left untouched.
async fn archive_submitted_cluster(
    conn: &mut PgConnection,
    tenant_id: &Uuid,
    signature: &str,
    except: Option<Uuid>,
) -> Result<u64, String> {
    let res = sqlx_core::query::query(
        "UPDATE dojo.artifacts
         SET status = 'archived', updated_at = now()
         WHERE tenant_id = $1 AND signature = $2 AND status = 'submitted'
           AND ($3::uuid IS NULL OR id <> $3)",
    )
    .bind(tenant_id)
    .bind(signature)
    .bind(except)
    .execute(conn)
    .await
    .map_err(|e| e.to_string())?;
    Ok(res.rows_affected())
}

/// Upsert the cluster's single `dojo.triage_queue` row (keyed by
/// `(tenant_id, signature)`). Done as select-then-insert/update because the
/// table has no unique constraint on that pair; the enclosing txn holds
/// [`ARTIFACTS_SEQ_LOCK`], so the check-then-write is race-free. Returns the
/// row id and whether it was newly inserted.
#[allow(clippy::too_many_arguments)]
async fn upsert_triage(
    conn: &mut PgConnection,
    tenant_id: &Uuid,
    artifact_id: &Uuid,
    signature: &str,
    owner_scope: &Value,
    confidence: f64,
    contributor_count: i64,
    similarity: Option<f64>,
    nearest: Option<Uuid>,
    state: &str,
) -> Result<(Uuid, bool), String> {
    let existing: Option<(Uuid,)> = sqlx_core::query_as::query_as(
        "SELECT id FROM dojo.triage_queue WHERE tenant_id = $1 AND signature = $2 LIMIT 1",
    )
    .bind(tenant_id)
    .bind(signature)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;

    if let Some((id,)) = existing {
        sqlx_core::query::query(
            "UPDATE dojo.triage_queue
             SET artifact_id = $2, owner_scope = $3::jsonb, confidence = $4::numeric,
                 contributor_count = $5::int, similarity = $6::numeric,
                 nearest_artifact_id = $7, state = $8::dojo.triage_state, updated_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(artifact_id)
        .bind(owner_scope)
        .bind(confidence)
        .bind(contributor_count)
        .bind(similarity)
        .bind(nearest)
        .bind(state)
        .execute(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
        Ok((id, false))
    } else {
        let (id,): (Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO dojo.triage_queue
               (tenant_id, artifact_id, signature, owner_scope, confidence,
                contributor_count, similarity, nearest_artifact_id, state)
             VALUES ($1, $2, $3, $4::jsonb, $5::numeric, $6::int, $7::numeric, $8,
                     $9::dojo.triage_state)
             RETURNING id",
        )
        .bind(tenant_id)
        .bind(artifact_id)
        .bind(signature)
        .bind(owner_scope)
        .bind(confidence)
        .bind(contributor_count)
        .bind(similarity)
        .bind(nearest)
        .bind(state)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
        Ok((id, true))
    }
}

/// Append a `dojo.decisions` row (the named triage verdict trail).
#[allow(clippy::too_many_arguments)]
async fn insert_decision(
    conn: &mut PgConnection,
    tenant_id: &Uuid,
    artifact_id: &Uuid,
    triage_id: Option<Uuid>,
    maintainer_id: Option<Uuid>,
    status: DecideStatus,
    reason: Option<&str>,
    distribution_scope: Option<&Value>,
    automated: bool,
) -> Result<(), String> {
    sqlx_core::query::query(
        "INSERT INTO dojo.decisions
           (tenant_id, artifact_id, triage_id, maintainer_id, status, reason,
            distribution_scope, automated)
         VALUES ($1, $2, $3, $4, $5::dojo.decision_status, $6, $7::jsonb, $8)",
    )
    .bind(tenant_id)
    .bind(artifact_id)
    .bind(triage_id)
    .bind(maintainer_id)
    .bind(status.as_db_str())
    .bind(reason)
    .bind(distribution_scope)
    .bind(automated)
    .execute(conn)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Append a `dojo.events` row (the append-only audit trail).
async fn insert_event(
    conn: &mut PgConnection,
    tenant_id: &Uuid,
    actor_id: Option<Uuid>,
    artifact_id: Option<&Uuid>,
    action: &str,
    detail: Value,
) -> Result<(), String> {
    sqlx_core::query::query(
        "INSERT INTO dojo.events (tenant_id, actor_id, artifact_id, action, detail)
         VALUES ($1, $2, $3, $4, $5::jsonb)",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(artifact_id)
    .bind(action)
    .bind(detail)
    .execute(conn)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals(contributor_count: i64, ftr: Option<f64>, is_global: bool) -> ClusterSignals {
        ClusterSignals {
            contributor_count,
            ftr_delta_observed: ftr,
            is_global,
        }
    }

    #[test]
    fn score_breadth_only_five_contributors_hits_the_bar() {
        // 5 * 0.16 = 0.80 = the bar (the spec's "≥ 5 users" heuristic).
        assert!((score(&signals(5, None, false)) - 0.80).abs() < 1e-9);
        assert!(score(&signals(5, None, false)) >= AUTO_APPROVE_SCORE);
        // 4 contributors alone fall short.
        assert!(score(&signals(4, None, false)) < AUTO_APPROVE_SCORE);
    }

    #[test]
    fn score_efficacy_a_single_strong_contribution_can_clear_the_bar() {
        // 1 contributor (0.16) + full FTR credit (1.0) → clamped 1.0.
        let s = score(&signals(1, Some(FTR_DELTA_FULL_CREDIT), false));
        assert!((s - 1.0).abs() < 1e-9);
        assert!(s >= AUTO_APPROVE_SCORE);
    }

    #[test]
    fn score_is_clamped_and_ignores_negative_ftr() {
        assert_eq!(score(&signals(100, Some(10.0), false)), 1.0);
        // A negative FTR delta contributes no efficacy (never penalises breadth).
        assert!((score(&signals(1, Some(-0.5), false))
            - score(&signals(1, None, false)))
        .abs()
            < 1e-9);
        // Below-zero contributor counts can't push the score negative.
        assert_eq!(score(&signals(-3, None, false)), 0.0);
    }

    #[test]
    fn decide_low_bar_single_contributor_is_queued() {
        // 1 contributor, no efficacy → 0.16 < bar → queued (below score bar).
        assert_eq!(
            decide(&signals(1, None, false)),
            Decision::Queue(QueueReason::BelowScoreBar)
        );
    }

    #[test]
    fn decide_high_bar_single_contributor_auto_approves_private() {
        // A private tenant has no k-anonymity floor → a strong single
        // contribution auto-approves.
        assert_eq!(
            decide(&signals(1, Some(FTR_DELTA_FULL_CREDIT), false)),
            Decision::AutoApprove
        );
    }

    #[test]
    fn decide_high_bar_five_contributors_auto_approves() {
        assert_eq!(decide(&signals(5, None, false)), Decision::AutoApprove);
    }

    #[test]
    fn decide_global_under_k_is_blocked_even_with_a_high_score() {
        // Score clears the bar (1 * 0.16 + full efficacy = 1.0), but the global
        // cluster has 1 < K distinct contributors → k-anonymity block.
        assert_eq!(
            decide(&signals(1, Some(FTR_DELTA_FULL_CREDIT), true)),
            Decision::Queue(QueueReason::KAnonymity)
        );
        // 2 contributors is still under K=3.
        assert_eq!(
            decide(&signals(2, Some(FTR_DELTA_FULL_CREDIT), true)),
            Decision::Queue(QueueReason::KAnonymity)
        );
    }

    #[test]
    fn decide_global_at_k_with_high_score_auto_approves() {
        // 3 contributors (= K) + score over the bar → publishes.
        assert_eq!(K_ANONYMITY_MIN_CONTRIBUTORS, 3);
        assert_eq!(
            decide(&signals(3, Some(FTR_DELTA_FULL_CREDIT), true)),
            Decision::AutoApprove
        );
    }

    #[test]
    fn decide_global_reports_score_bar_before_k_anonymity() {
        // A global cluster that also fails the score bar reports the more
        // informative BelowScoreBar (not KAnonymity).
        assert_eq!(
            decide(&signals(1, None, true)),
            Decision::Queue(QueueReason::BelowScoreBar)
        );
    }

    #[test]
    fn decide_status_round_trips() {
        for (s, txt) in [
            (DecideStatus::Approve, "approve"),
            (DecideStatus::Revise, "revise"),
            (DecideStatus::Decline, "decline"),
        ] {
            assert_eq!(s.as_db_str(), txt);
            assert_eq!(DecideStatus::parse(txt), Some(s));
        }
        assert_eq!(DecideStatus::parse("nope"), None);
    }
}
