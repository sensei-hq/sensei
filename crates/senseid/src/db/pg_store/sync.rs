//! Sync bookkeeping — what has been shared with dōjō, and what still needs to be.
//!
//! The transport is `dojo_client/user_plane.rs` (`push_metrics`) driven by
//! `tasks/dojo_sync.rs::push_allowed`; this module is the state it reads and
//! writes. Keeping them separate means the decision logic — *should this row
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

/// One `sensei.sync_state` row — the shape [`PgStore::sync_state_rows`] returns.
///
/// Both timestamps travel, and they answer different questions: `synced_at` is
/// when the two sides last AGREED, `attempted_at` is when it was last TRIED.
/// Together they separate "failing repeatedly" from "nobody has run it", which a
/// single timestamp cannot.
///
/// `last_error` is `None` on a `synced` row because the writer clears it — a
/// stale message beside a success reads as "it failed". On a `skipped` row it
/// carries the REASON, which is not a fault: `skipped` is deliberate (a private
/// repository), and conflating the two would either cry wolf about every private
/// repo or stay silent about real failures.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncStateRow {
    pub entity: String,
    pub entity_key: String,
    pub direction: String,
    pub state: String,
    pub last_error: Option<String>,
    pub attempted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub synced_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// The push query's column tuple. Named for the same reason `ScheduleRow` and
/// `TaskExecutionRow` are: ten positional columns inline is a type nobody can
/// read, and clippy is right to say so.
type PushableRow = (
    uuid::Uuid,
    String,
    String,
    String,
    String,
    chrono::NaiveDate,
    f64,
    Option<String>,
    serde_json::Value,
    String,
);

/// The user-scoped rows a push could consider: this machine's, unsent or changed
/// since, on a repository the plan allows (`$2`). `$1` and `$3` are bound by
/// [`PROVABLY_THEIRS`].
const USER_ROWS_IN_PLAY: &str = "rm.computed_by = 'local' AND rm.scope = 'user' \
     AND r.repo_key = ANY($2) AND (rm.shared_at IS NULL OR rm.modified_at > rm.shared_at)";

/// A user-scoped row provably the persona's (`$1`): no identity on a repository
/// only this persona elected (`$3`), or an author address the persona claimed.
const PROVABLY_THEIRS: &str = "(rm.identity IS NULL AND r.repo_key = ANY($3)) \
     OR lower(rm.identity) IN (SELECT lower(e.email) FROM sensei.persona_emails e \
                               WHERE e.persona_id = $1 AND e.source = 'claimed' \
                                 AND e.removed_at IS NULL)";

/// A selected row as the push carries it.
fn pushable(row: PushableRow) -> PushableMetric {
    let (id, repo_key, metric, scope, grain, computed_on, value, commit_sha, props, source) = row;
    PushableMetric {
        id,
        repo_key,
        metric,
        scope,
        grain,
        computed_on,
        value,
        commit_sha,
        props,
        source,
    }
}

impl PgStore {
    /// Record which dōjō tenant a repository is enrolled with (D2).
    ///
    /// Matched on `repo_key`, the durable cross-install identity — never a local
    /// uuid, which differs per machine and is meaningless to the dōjō.
    ///
    /// Returns the number of rows written, so a caller can tell "stored" from
    /// "that repo_key is not in this database" instead of assuming success. A
    /// mapping for a repository we do not have is not an error, but it is also
    /// not a write, and reporting it as one would hide a real mismatch.
    pub async fn set_repository_tenant(
        &self,
        repo_key: &str,
        tenant_id: uuid::Uuid,
    ) -> Result<u64, String> {
        let r = sqlx_core::query::query(
            "UPDATE sensei.repositories SET tenant_id = $2, modified_at = now() \
              WHERE repo_key = $1 AND tenant_id IS DISTINCT FROM $2",
        )
        .bind(repo_key)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("set_repository_tenant({repo_key}): {e}"))?;
        Ok(r.rows_affected())
    }

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

    /// When this mark was last ATTEMPTED, successfully or not — `None` if never.
    /// What a cadence keys on, so a thing that keeps failing waits its interval
    /// rather than retrying every cycle.
    pub async fn sync_attempted_at(
        &self,
        m: &SyncMark<'_>,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
        let row: Option<(Option<chrono::DateTime<chrono::Utc>>,)> = sqlx_core::query_as::query_as(
            "SELECT attempted_at FROM sensei.sync_state \
              WHERE entity = $1::sensei.sync_entity AND entity_key = $2 \
                AND direction = $3::sensei.sync_direction",
        )
        .bind(m.entity)
        .bind(m.key)
        .bind(m.direction)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("sync_attempted_at: {e}"))?;
        Ok(row.and_then(|(t,)| t))
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

    /// Record that a sync agreed except for a named part — a graph push that
    /// landed every file but one over the request cap (#225).
    ///
    /// An agreement, so `synced_at` is set; `what_is_missing` rides in
    /// `last_error` as the row's detail. Not `error`: that reads "failing · never
    /// agreed" over a graph that is all but one file there.
    pub async fn mark_sync_partial(
        &self,
        m: &SyncMark<'_>,
        what_is_missing: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.sync_state \
                 (entity, entity_key, direction, state, last_error, attempted_at, synced_at, updated_at) \
             VALUES ($1::sensei.sync_entity, $2, $3::sensei.sync_direction, 'partial', $4, now(), now(), now()) \
             ON CONFLICT (entity, entity_key, direction) DO UPDATE SET \
                 state = 'partial', last_error = EXCLUDED.last_error, \
                 attempted_at = now(), synced_at = now(), updated_at = now()",
        )
        .bind(m.entity)
        .bind(m.key)
        .bind(m.direction)
        .bind(what_is_missing)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_sync_partial: {e}"))?;
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

    /// Every `sensei.sync_state` row: what has been agreed with the dōjō, when,
    /// and what went wrong where it has not.
    ///
    /// The first READER of this table. Three writers existed
    /// ([`Self::mark_synced`], [`Self::mark_sync_error`],
    /// [`Self::mark_sync_skipped`]) and nothing read it back, so the answer to
    /// "when did this last sync?" lived only in the table and was never surfaced.
    ///
    /// Carries `last_error` and BOTH timestamps deliberately:
    ///
    /// * `synced_at` survives a failure ([`Self::mark_sync_error`] does not clear
    ///   it), so it still says when the two sides last agreed. Dropping it here
    ///   would waste that — a broken-since-Tuesday entity would look identical to
    ///   one that has never synced.
    /// * `attempted_at` says when it was last TRIED, which with `synced_at` is the
    ///   difference between "failing repeatedly" and "nobody has run it".
    ///
    /// Ordered worst-first — `error`, then `pending`, then `skipped`, then
    /// `synced` — so a caller rendering the head of the list sees what needs
    /// attention. Within a state, most-recently-attempted first.
    ///
    /// Unbounded by design: one row per (entity × key × direction), which is
    /// three on this install and grows with repositories, not with metric rows.
    /// If that changes, the caller wants a summary, not a limit that would hide
    /// exactly the failing rows this exists to show.
    pub async fn sync_state_rows(&self) -> Result<Vec<SyncStateRow>, String> {
        /// `sensei.sync_state` in [`SyncStateRow`] field order. Both enums are
        /// projected as text so this decodes without the Postgres types.
        type Row = (
            String,
            String,
            String,
            String,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
            chrono::DateTime<chrono::Utc>,
        );
        let rows: Vec<Row> = sqlx_core::query_as::query_as(
            "SELECT entity::text, entity_key, direction::text, state, last_error,
                    attempted_at, synced_at, updated_at
               FROM sensei.sync_state
              ORDER BY CASE state
                         WHEN 'error'   THEN 0
                         WHEN 'pending' THEN 1
                         WHEN 'partial' THEN 2
                         WHEN 'skipped' THEN 3
                         ELSE                4
                       END,
                       attempted_at DESC NULLS LAST,
                       entity::text, entity_key",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("sync_state_rows: {e}"))?;

        Ok(rows
            .into_iter()
            .map(|r| SyncStateRow {
                entity: r.0,
                entity_key: r.1,
                direction: r.2,
                state: r.3,
                last_error: r.4,
                attempted_at: r.5,
                synced_at: r.6,
                updated_at: r.7,
            })
            .collect())
    }

    /// Metric rows this machine computed and has not yet pushed, for the
    /// repositories the dōjō's plan allows.
    ///
    /// Two filters carry the whole design:
    ///
    /// * `computed_by = 'local'` — never re-push what dōjō handed down. Without
    ///   it a pulled value is indistinguishable from an own one, so it gets
    ///   pushed back, pulled again, and the pair ping-pong forever.
    /// * `repo_key = ANY(allowed_keys)` — **the allow-list is the authority.**
    ///
    /// **`visibility = 'shared'` used to sit beside them and was REMOVED**
    /// (`docs/spec/dojo/daemon-sync.md` §8a, finding B2). The reasoning, because it
    /// is a narrowing of a promise and must not be rediscovered from a diff:
    ///
    /// 1. **It was a second derivation of a question the dōjō had already
    ///    answered.** `allowed_keys` comes from `dojo.all_my_repositories`, whose
    ///    whole purpose (§8c) is to be the single place `may_share AND elected` is
    ///    computed, so the daemon, the API and the console cannot disagree. ANDing
    ///    a local flag back in restores the disagreement the view exists to remove
    ///    — and it is the local copy that would be wrong, since the user's election
    ///    lives in `dojo.repository_elections` and the org's mandate has no local
    ///    representation at all.
    /// 2. **Keeping it "for user-authority repositories only" is not expressible
    ///    here.** The daemon holds no fact that distinguishes user authority from
    ///    org authority — that is derived from `dojo.tenants.origin` and the forge
    ///    visibility, neither of which is mirrored locally. A subset predicate
    ///    would need the plan to carry per-repository authority, which it does not;
    ///    inventing the field before the dōjō emits it would degrade to either
    ///    "always apply the local gate" (B2 unfixed) or "never" (this, with dead
    ///    code attached).
    /// 3. **The allow-list still bounds it absolutely.** The daemon syncs the set
    ///    it was HANDED. It cannot include a repository it never offered, and the
    ///    offer set is what was cloned. Offline still degrades to no-sync.
    ///
    /// What is genuinely given up: the local flag no longer independently vetoes a
    /// push. *"Nothing leaves the machine without local consent"* became *"…without
    /// local consent, or an organization's mandate on that organization's own
    /// private code"*, and the daemon now trusts the dōjō to tell the two apart.
    ///
    /// **DEPLOYMENT ORDER, and it is not optional.** This query's only remaining
    /// gate is whatever `dojo.all_my_repositories` puts in `plan.allowed`. Against
    /// the version of that view which hardcoded `sync_enabled = true`, "allowed"
    /// meant *every registered repository* — so a daemon carrying this change would
    /// have pushed every clone's metrics with no election anywhere. The computing
    /// view must be deployed before a daemon that trusts it, and must never be
    /// rolled back behind one. Same ordering hazard §8a records for the authority
    /// rule, one layer out; verified deployed before this shipped.
    ///
    /// `shared_at IS NULL OR modified_at > shared_at` catches both the never-sent
    /// and the changed-since-sent, so a recomputed day is re-pushed rather than
    /// left stale behind an already-synced marker.
    ///
    /// **BOTH `scopes` and `allowed_keys` filter in SQL, before the LIMIT, and that
    /// ordering is the point.** `allowed_keys` is the dōjō's allow-list; filtering
    /// it in Rust after the fetch had the same defect the scope filter was fixed
    /// for — 218 of 500 slots went to repositories that can NEVER be in the plan
    /// (self-hosted hosts, unconnected orgs), whose rows therefore never drain and
    /// permanently occupy the window. Measured: the 500th-newest such row was
    /// months old, so every allowed row older than it was unreachable forever.
    ///
    /// `ORDER BY … , rm.id` gives a total order, so the window is deterministic
    /// rather than varying per run on ties.
    /// Filtering after the fetch lets rows the caller cannot push crowd
    /// out ones it can: with 596 user-scoped rows held back and a limit of 500,
    /// a pass fetched 500, found only 66 pushable in that window, and pushed 66
    /// of 132 — and had the held-back rows filled the window entirely, it would
    /// have pushed NOTHING while reporting success. Observed live at exactly
    /// those numbers.
    pub async fn unpushed_metric_rows(
        &self,
        scopes: &[&str],
        allowed_keys: &[&str],
        limit: i64,
    ) -> Result<Vec<PushableMetric>, String> {
        let rows: Vec<PushableRow> = sqlx_core::query_as::query_as(
            "SELECT rm.id, r.repo_key, m.key, rm.scope::text, rm.grain::text, rm.computed_on \
                  , rm.value::float8, rm.commit_sha, rm.props, rm.source::text \
               FROM sensei.repository_metrics rm \
               JOIN sensei.repositories r ON r.id = rm.repository_id \
               JOIN sensei.metrics m      ON m.id = rm.metric_id \
              WHERE rm.computed_by = 'local' \
                AND rm.scope::text = ANY($1) \
                AND r.repo_key = ANY($2) \
                AND (rm.shared_at IS NULL OR rm.modified_at > rm.shared_at) \
              ORDER BY rm.computed_on DESC, rm.id \
              LIMIT $3",
        )
        .bind(scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .bind(allowed_keys.iter().map(|k| k.to_string()).collect::<Vec<_>>())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("unpushed_metric_rows: {e}"))?;
        Ok(rows.into_iter().map(pushable).collect())
    }

    /// The signed-in person's user-scoped rows that may leave the machine (#240):
    /// only what is PROVABLY theirs. A row with no identity is this machine's own
    /// session metric — theirs when the persona is the only signed-in one that
    /// elected the repository (`exclusive_keys`). A row with an author email is
    /// theirs when that address is one of the persona's `claimed` (provider-
    /// verified) addresses; an unverified `git` address proves nothing — anyone
    /// can write any author — and a colleague's address is never theirs. The dōjō
    /// stamps the person from the session; nothing here names one.
    ///
    /// Same window discipline as [`Self::unpushed_metric_rows`]: every filter in
    /// SQL before the LIMIT. [`Self::unprovable_user_metric_count`] counts the
    /// complement over the same population.
    pub async fn unpushed_user_metric_rows(
        &self,
        persona: &uuid::Uuid,
        allowed_keys: &[&str],
        exclusive_keys: &[&str],
        limit: i64,
    ) -> Result<Vec<PushableMetric>, String> {
        let rows: Vec<PushableRow> = sqlx_core::query_as::query_as(&format!(
            "SELECT rm.id, r.repo_key, m.key, rm.scope::text, rm.grain::text, rm.computed_on \
                  , rm.value::float8, rm.commit_sha, rm.props, rm.source::text \
               FROM sensei.repository_metrics rm \
               JOIN sensei.repositories r ON r.id = rm.repository_id \
               JOIN sensei.metrics m      ON m.id = rm.metric_id \
              WHERE {USER_ROWS_IN_PLAY} AND ({PROVABLY_THEIRS}) \
              ORDER BY rm.computed_on DESC, rm.id \
              LIMIT $4"
        ))
        .bind(persona)
        .bind(allowed_keys.iter().map(|k| k.to_string()).collect::<Vec<_>>())
        .bind(exclusive_keys.iter().map(|k| k.to_string()).collect::<Vec<_>>())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("unpushed_user_metric_rows: {e}"))?;
        Ok(rows.into_iter().map(pushable).collect())
    }

    /// The user-scoped rows held back because they are not provably the
    /// persona's — the complement of [`Self::unpushed_user_metric_rows`] over the
    /// same population, so "pushed N, held M" adds up.
    pub async fn unprovable_user_metric_count(
        &self,
        persona: &uuid::Uuid,
        allowed_keys: &[&str],
        exclusive_keys: &[&str],
    ) -> Result<i64, String> {
        let row: (i64,) = sqlx_core::query_as::query_as(&format!(
            "SELECT count(*) \
               FROM sensei.repository_metrics rm \
               JOIN sensei.repositories r ON r.id = rm.repository_id \
              WHERE {USER_ROWS_IN_PLAY} AND NOT coalesce(({PROVABLY_THEIRS}), false)"
        ))
        .bind(persona)
        .bind(allowed_keys.iter().map(|k| k.to_string()).collect::<Vec<_>>())
        .bind(exclusive_keys.iter().map(|k| k.to_string()).collect::<Vec<_>>())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("unprovable_user_metric_count: {e}"))?;
        Ok(row.0)
    }

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

/// One metric row queued for the dōjō.
///
/// A struct rather than the `serde_json::Value` this used to return. The receiving
/// endpoint keys on `(metric, repository, scope, principal, commit_sha,
/// computed_on, grain)` and stores `props` + `source`, so every one of those has
/// to survive the trip — and an untyped bag let five of them go missing silently
/// (`docs/spec/dojo/daemon-sync.md` claim C4).
#[derive(Debug, Clone, PartialEq)]
pub struct PushableMetric {
    /// The LOCAL row id. Never sent — it is meaningless to the dōjō — but needed
    /// to mark `shared_at` on exactly the rows that were accepted.
    pub id: uuid::Uuid,
    /// The durable cross-install repository identity.
    pub repo_key: String,
    /// `sensei.metrics.key`, not a uuid: metric ids are not guaranteed identical
    /// across installs, and a mismatch would file real numbers under the wrong
    /// metric silently.
    pub metric: String,
    /// `repo` or `user`. Part of the dōjō's unique key, so a missing scope would
    /// file a per-person row as a repository-wide one.
    pub scope: String,
    /// `daily`, … — also part of that key.
    pub grain: String,
    pub computed_on: chrono::NaiveDate,
    pub value: f64,
    /// Absent stays absent. Defaulting it would forge a commit association.
    pub commit_sha: Option<String>,
    pub props: serde_json::Value,
    /// `measured` vs `imputed`. Defaulting to `measured` would present an
    /// estimate as an observation.
    pub source: String,
}

/// One repository this machine can name to the dōjō.
///
/// Returned by BOTH [`PgStore::offerable_repositories`] (the disclosure set —
/// everything cloned) and [`PgStore::shared_repositories`] (gate 1 — what the user
/// elected). The two answer different questions and stopped having the same answer
/// in `docs/spec/dojo/daemon-sync.md` §8a, so the type is named for what it
/// carries rather than for either gate.
/// One offered repository as read: `(repo_key, remote_url, name, project slug,
/// project name)`.
type OfferedRow = (String, Option<String>, String, Option<String>, Option<String>);

/// A sensei project as the dōjō knows it: the project-scope namespace slug and
/// the project's name — the same pair a relay run sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedProject {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedRepo {
    /// The DURABLE cross-install identity — the normalized remote
    /// (`host/org/repo`). This is what the dōjō maps to a tenant, so it is the
    /// one field that cannot be missing.
    pub repo_key: String,
    /// A representative raw remote, for display and re-derivation.
    pub remote_url: Option<String>,
    /// Display name — typically the repository basename.
    pub name: String,
    /// Its sensei project, named as a relay run names it (#299) — offered only
    /// when the repository is in exactly ONE project; two is never a guess.
    pub project: Option<OfferedProject>,
}

impl PgStore {
    /// Opt a repository into — or out of — sharing. The write side of GATE 1.
    ///
    /// Until this existed nothing could set `visibility` at all, so gate 1 was
    /// unreachable: every repository sat at the private default and the whole
    /// push would have moved zero rows while reporting success
    /// (`docs/spec/dojo/daemon-sync.md` claim C3).
    ///
    /// `visibility` is a text parameter cast to the enum rather than a Rust enum
    /// because the DATABASE owns the legal set (`sensei.repo_visibility`). A
    /// value outside it fails the cast and returns `Err` — never a silent no-op,
    /// which would leave the user believing they had shared something.
    ///
    /// Returns rows affected, so a caller can distinguish "set" from "this
    /// database has no such repo_key" instead of assuming success. Revoking is
    /// the same call with `'private'`: sharing has to be reversible, or D8's
    /// "the user may still turn it off" is not true.
    pub async fn set_repository_visibility(
        &self,
        repo_key: &str,
        visibility: &str,
    ) -> Result<u64, String> {
        let r = sqlx_core::query::query(
            "UPDATE sensei.repositories \
                SET visibility = $2::sensei.repo_visibility, modified_at = now() \
              WHERE repo_key = $1",
        )
        .bind(repo_key)
        .bind(visibility)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("set_repository_visibility({repo_key}, {visibility}): {e}"))?;
        Ok(r.rows_affected())
    }

    /// **THE OFFER SET** — every locally-scanned repository with a cross-install
    /// identity, i.e. every repository the user CLONED.
    ///
    /// This is what the daemon discloses to the dōjō, and it is deliberately WIDER
    /// than gate 1 ([`Self::shared_repositories`]). `docs/spec/dojo/daemon-sync.md`
    /// §8a, finding B1: an organization's mandate over its own private code is a
    /// decision only the dōjō can make — the daemon holds no local fact that
    /// answers "is this mandated" — so filtering the offer on the user's local
    /// election meant the cycle returned before the dōjō was ever asked, and the
    /// mandate was unreachable for exactly the population it serves.
    ///
    /// **What bounds it is the CLONE, not the forge.** `sensei.repositories` is
    /// populated by the scanner, never by a forge listing, so this set is what the
    /// user actually works on. Belonging to an organization whose repositories are
    /// not on this machine discloses nothing, because there is nothing to measure.
    /// A forge listing was considered and rejected in §8a: it would turn a sign-in
    /// into an inventory upload of everything the user can see.
    ///
    /// One filter, and it is the same one gate 1 carries: `repo_key IS NOT NULL`.
    /// A NULL key is the registry's marker for a local-only repository with no
    /// remote; it has no cross-install identity, so the dōjō would have nothing to
    /// map it to and could only answer `unmapped`.
    ///
    /// See [`Self::repositories_where`] for why the ORDER BY is not `repo_key`.
    pub async fn offerable_repositories(&self, limit: i64) -> Result<Vec<OfferedRepo>, String> {
        self.repositories_where("repo_key IS NOT NULL", limit)
            .await
            .map_err(|e| format!("offerable_repositories: {e}"))
    }

    /// The repositories the user has opted into sharing — GATE 1 (intent).
    ///
    /// The first of the three gates in spec §V.3, and the only one the daemon
    /// owns. Cost (the repo's visibility on the forge) and entitlement (claim,
    /// billing, seat) belong to the dōjō and are never mirrored here.
    ///
    /// **It currently filters NEITHER.** §8a removed the `visibility` term from
    /// the offer, intending it to move to the push. It did not: `push_allowed`
    /// says outright that `visibility` is "deliberately NOT re-tested here or in
    /// the query", and this function has no production caller at all — every call
    /// site is inside a `#[cfg(test)]` module. So gate 1 is dead scaffolding, and
    /// the doc that used to claim "this filters the PUSH, and only where the USER
    /// holds authority" was describing an intention rather than the code.
    ///
    /// What that means concretely: `PATCH /api/repositories/{key}` writes
    /// `visibility` and returns 200, and nothing reads it. The promise §8a
    /// narrowed to *"nothing leaves the machine without local consent, or an
    /// organization's mandate on that organization's own private code"* is not
    /// enforced for the user-authority half either. Restoring the term is a
    /// behaviour change with real consequences for what is shared, so it is
    /// raised with the user rather than made here.
    ///
    /// Two filters, both load-bearing:
    ///
    /// * `visibility = 'shared'` — a private repository is a CHOICE, not a
    ///   failure. Signing in must not start sharing a repo the user never
    ///   elected, which is exactly what `sensei.repo_visibility`'s own comment
    ///   says the column is for.
    /// * `repo_key IS NOT NULL` — see [`Self::offerable_repositories`].
    pub async fn shared_repositories(&self, limit: i64) -> Result<Vec<OfferedRepo>, String> {
        self.repositories_where("visibility = 'shared' AND repo_key IS NOT NULL", limit)
            .await
            .map_err(|e| format!("shared_repositories: {e}"))
    }

    /// The projection both sets share. Private, and `predicate` is a compile-time
    /// literal from the two callers above — never caller data, so there is no
    /// injection surface to widen later.
    ///
    /// **`ORDER BY` is what keeps the limit a throughput bound instead of a
    /// ceiling.** It used to be `repo_key`, which was harmless while the set was
    /// the few repositories a user had elected and is not once the set is every
    /// clone: an install with more repositories than the caller's limit would offer
    /// the alphabetically-first N *forever*, and a freshly cloned org-mandated
    /// repository sorting after them would never be registered — B1 reintroduced
    /// one layer down, and invisible on the 67-repository install it was measured
    /// on. So:
    ///
    /// * `tenant_id IS NULL DESC` — repositories the dōjō has not yet mapped go
    ///   first. Registration is what CREATES the `dojo.repositories` row the plan
    ///   is computed from, so an unregistered repository is the only kind that
    ///   *needs* the slot; a mapped one is already in the dōjō's answer whether or
    ///   not it is re-offered this pass.
    /// * `modified_at DESC` — then the repositories actually being worked on.
    /// * `repo_key` — a total order, so the window is deterministic on ties rather
    ///   than varying per run (the same reason `unpushed_metric_rows` tiebreaks).
    async fn repositories_where(
        &self,
        predicate: &str,
        limit: i64,
    ) -> Result<Vec<OfferedRepo>, String> {
        // The project: counted by PROJECT, not by namespace — two projects of
        // which only one has a namespace are still two, and still not a guess.
        let rows: Vec<OfferedRow> = sqlx_core::query_as::query_as(&format!(
            "SELECT r.repo_key, r.remote_url, r.name, pj.slug, pj.name \
                   FROM sensei.repositories r \
                   LEFT JOIN LATERAL ( \
                        SELECT CASE WHEN count(DISTINCT p.id) = 1 THEN max(n.slug) END AS slug, \
                               CASE WHEN count(DISTINCT p.id) = 1 THEN max(p.name) END AS name \
                          FROM sensei.repositories_in_workspaces rip \
                          JOIN sensei.workspaces p ON p.id = rip.workspace_id \
                          LEFT {join} \
                         WHERE rip.repository_id = r.id \
                   ) pj ON true \
                  WHERE {predicate} \
                  ORDER BY (r.tenant_id IS NULL) DESC, r.modified_at DESC, r.repo_key \
                  LIMIT $1",
            join = super::WORKSPACE_NAMESPACE_JOIN
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(repo_key, remote_url, name, slug, project_name)| OfferedRepo {
                repo_key,
                remote_url,
                name,
                project: slug.zip(project_name).map(|(slug, name)| OfferedProject { slug, name }),
            })
            .collect())
    }
}

#[cfg(test)]
mod offered_project_tests {
    use super::*;

    // #299: the sync files a repository under its sensei project. The project
    // offered is the SAME (slug, name) a relay run sends, and only when the
    // repository is in exactly one project — never a guess between two.
    async fn exec(pg: &PgStore, sql: &str, args: &[&str]) {
        let mut q = sqlx_core::query::query(sql);
        for a in args {
            q = q.bind(*a);
        }
        q.execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn a_repository_in_exactly_one_project_offers_that_project() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let tag = uuid::Uuid::new_v4().simple().to_string();
        let (one, two, none) = (
            format!("github.com/ztest-op/{tag}-one"),
            format!("github.com/ztest-op/{tag}-two"),
            format!("github.com/ztest-op/{tag}-none"),
        );
        let (pa, pb) = (format!("ztest-op-a-{tag}"), format!("ztest-op-b-{tag}"));
        for key in [&one, &two, &none] {
            exec(&pg, "INSERT INTO sensei.repositories (name, repo_key) VALUES ('r', $1)", &[key])
                .await;
        }
        for name in [&pa, &pb] {
            exec(&pg, "INSERT INTO sensei.workspaces (name) VALUES ($1)", &[name]).await;
            exec(
                &pg,
                "INSERT INTO sensei.namespaces (scope_key, name, slug) VALUES ('project', $1, $1 || '-slug')",
                &[name],
            )
            .await;
        }
        let link = "INSERT INTO sensei.repositories_in_workspaces (workspace_id, repository_id) \
                    SELECT p.id, r.id FROM sensei.workspaces p, sensei.repositories r \
                     WHERE p.name = $1 AND r.repo_key = $2";
        exec(&pg, link, &[&pa, &one]).await;
        exec(&pg, link, &[&pa, &two]).await;
        exec(&pg, link, &[&pb, &two]).await;

        let offered = pg.offerable_repositories(100_000).await.unwrap();
        let find = |k: &str| offered.iter().find(|r| r.repo_key == k).cloned().expect("offered");
        assert_eq!(
            find(&one).project,
            Some(OfferedProject { slug: format!("{pa}-slug"), name: pa.clone() })
        );
        assert_eq!(find(&two).project, None, "two projects: never a guess");
        assert_eq!(find(&none).project, None);

        for key in [&one, &two, &none] {
            exec(&pg, "DELETE FROM sensei.repositories WHERE repo_key = $1", &[key]).await;
        }
        for name in [&pa, &pb] {
            exec(
                &pg,
                "DELETE FROM sensei.namespaces WHERE scope_key = 'project' AND name = $1",
                &[name],
            )
            .await;
            exec(&pg, "DELETE FROM sensei.workspaces WHERE name = $1", &[name]).await;
        }
    }
}
