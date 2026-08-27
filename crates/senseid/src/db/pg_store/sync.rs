//! Sync bookkeeping — what has been shared with dōjō, and what still needs to be.
//!
//! The transport itself is Phase 7's remaining work; this is the state it reads
//! and writes. Keeping them separate means the decision logic — *should this row
//! be pushed, may this one be skipped* — is testable now, without a Supabase
//! session or a network.

use super::PgStore;

/// One thing's sync position, as the caller supplies it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncMark<'a> {
    /// `repository`, `repository_metric`, … — see `sensei.sync_entity`.
    pub entity: &'a str,
    /// The DURABLE cross-install key: `repo_key`, a metric key. Never a local
    /// uuid — those differ per machine, so the other side could not match it.
    pub key: &'a str,
    /// `push` or `pull`.
    pub direction: &'a str,
}

impl PgStore {
    /// Record that a sync attempt succeeded.
    ///
    /// Clears `last_error` on success — a stale error beside a `synced` state
    /// reads as "it failed" to anyone scanning the table.
    pub async fn mark_synced(&self, m: &SyncMark<'_>, version: Option<i64>) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.sync_state \
                 (entity, entity_key, direction, state, remote_version, attempted_at, synced_at, updated_at) \
             VALUES ($1::sensei.sync_entity, $2, $3::sensei.sync_direction, 'synced', $4, now(), now(), now()) \
             ON CONFLICT (entity, entity_key, direction) DO UPDATE SET \
                 state = 'synced', remote_version = EXCLUDED.remote_version, \
                 last_error = NULL, attempted_at = now(), synced_at = now(), updated_at = now()",
        )
        .bind(m.entity)
        .bind(m.key)
        .bind(m.direction)
        .bind(version)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_synced: {e}"))?;
        Ok(())
    }

    /// Record that a sync attempt failed.
    ///
    /// `synced_at` is deliberately NOT cleared: it still says when the two sides
    /// last agreed, which is the first thing worth knowing when a sync starts
    /// failing. Losing it would leave no way to tell a never-synced entity from
    /// one that has been broken since Tuesday.
    pub async fn mark_sync_error(&self, m: &SyncMark<'_>, error: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.sync_state \
                 (entity, entity_key, direction, state, last_error, attempted_at, updated_at) \
             VALUES ($1::sensei.sync_entity, $2, $3::sensei.sync_direction, 'error', $4, now(), now()) \
             ON CONFLICT (entity, entity_key, direction) DO UPDATE SET \
                 state = 'error', last_error = EXCLUDED.last_error, \
                 attempted_at = now(), updated_at = now()",
        )
        .bind(m.entity)
        .bind(m.key)
        .bind(m.direction)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_sync_error: {e}"))?;
        Ok(())
    }

    /// Record that an entity is deliberately not synced — a private repository, a
    /// deactivated metric.
    ///
    /// A distinct state from `error` on purpose. Both mean "not synced", but only
    /// one is a problem, and a dashboard that cannot tell them apart will either
    /// cry wolf about every private repo or stay silent about real failures.
    pub async fn mark_sync_skipped(&self, m: &SyncMark<'_>, reason: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.sync_state \
                 (entity, entity_key, direction, state, last_error, attempted_at, updated_at) \
             VALUES ($1::sensei.sync_entity, $2, $3::sensei.sync_direction, 'skipped', $4, now(), now()) \
             ON CONFLICT (entity, entity_key, direction) DO UPDATE SET \
                 state = 'skipped', last_error = EXCLUDED.last_error, \
                 attempted_at = now(), updated_at = now()",
        )
        .bind(m.entity)
        .bind(m.key)
        .bind(m.direction)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_sync_skipped: {e}"))?;
        Ok(())
    }

    /// Metric rows this machine computed and has not yet pushed, for repositories
    /// marked shared.
    ///
    /// Two filters carry the whole design:
    ///
    /// * `computed_by = 'local'` — never re-push what dōjō handed down. Without
    ///   it a pulled value is indistinguishable from an own one, so it gets
    ///   pushed back, pulled again, and the pair ping-pong forever.
    /// * `visibility = 'shared'` — a private repository is not a sync failure, it
    ///   is a choice, so its rows never enter the queue at all.
    ///
    /// `shared_at IS NULL OR modified_at > shared_at` catches both the never-sent
    /// and the changed-since-sent, so a recomputed day is re-pushed rather than
    /// left stale behind an already-synced marker.
    pub async fn unpushed_metric_rows(&self, limit: i64) -> Result<Vec<serde_json::Value>, String> {
        let rows: Vec<(uuid::Uuid, String, String, chrono::NaiveDate, f64)> =
            sqlx_core::query_as::query_as(
                "SELECT rm.id, r.repo_key, m.key, rm.computed_on, rm.value::float8 \
                   FROM sensei.repository_metrics rm \
                   JOIN sensei.repositories r ON r.id = rm.repository_id \
                   JOIN sensei.metrics m      ON m.id = rm.metric_id \
                  WHERE r.visibility = 'shared' \
                    AND rm.computed_by = 'local' \
                    AND (rm.shared_at IS NULL OR rm.modified_at > rm.shared_at) \
                  ORDER BY rm.computed_on DESC \
                  LIMIT $1",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("unpushed_metric_rows: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(id, repo_key, metric, day, value)| {
                serde_json::json!({
                    "id": id, "repoKey": repo_key, "metric": metric,
                    "computedOn": day, "value": value,
                })
            })
            .collect())
    }

    /// Stamp rows as pushed.
    pub async fn mark_metric_rows_shared(&self, ids: &[uuid::Uuid]) -> Result<u64, String> {
        if ids.is_empty() {
            return Ok(0);
        }
        let n = sqlx_core::query::query(
            "UPDATE sensei.repository_metrics SET shared_at = now() WHERE id = ANY($1)",
        )
        .bind(ids)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_metric_rows_shared: {e}"))?
        .rows_affected();
        Ok(n)
    }
}

/// One repository this machine offers the dōjō, as gate 1 lets through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedRepo {
    /// The DURABLE cross-install identity — the normalized remote
    /// (`host/org/repo`). This is what the dōjō maps to a tenant, so it is the
    /// one field that cannot be missing.
    pub repo_key: String,
    /// A representative raw remote, for display and re-derivation.
    pub remote_url: Option<String>,
    /// Display name — typically the repository basename.
    pub name: String,
}

impl PgStore {
    /// The repositories the user has opted into sharing — GATE 1 (intent).
    ///
    /// The first of the three gates in spec §V.3, and the only one the daemon
    /// owns. Cost (the repo's visibility on the forge) and entitlement (claim,
    /// billing, seat) belong to the dōjō and are never mirrored here; the daemon
    /// simply never mentions a repository the user did not opt in.
    ///
    /// Two filters, both load-bearing:
    ///
    /// * `visibility = 'shared'` — a private repository is a CHOICE, not a
    ///   failure. Signing in must not start sharing a repo the user never
    ///   offered, which is exactly what `sensei.repo_visibility`'s own comment
    ///   says the column is for.
    /// * `repo_key IS NOT NULL` — a NULL key is the registry's marker for a
    ///   local-only repository with no remote. It has no cross-install identity,
    ///   so the dōjō would have nothing to map it to; sending one could only
    ///   produce an `unmapped` answer.
    pub async fn shared_repositories(&self, limit: i64) -> Result<Vec<SharedRepo>, String> {
        let rows: Vec<(String, Option<String>, String)> = sqlx_core::query_as::query_as(
            "SELECT repo_key, remote_url, name \
               FROM sensei.repositories \
              WHERE visibility = 'shared' \
                AND repo_key IS NOT NULL \
              ORDER BY repo_key \
              LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("shared_repositories: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(repo_key, remote_url, name)| SharedRepo { repo_key, remote_url, name })
            .collect())
    }
}
