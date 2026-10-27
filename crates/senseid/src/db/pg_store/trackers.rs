//! Issue-tracker connections, their sprints and tickets (spec
//! `docs/spec/daemon/tracker-jira-linkage.md`, R2/R3/R5). Every SQL statement the
//! tracker integration runs lives here; `crate::tracker` holds the provider
//! logic and calls these methods.

use crate::db::pg_store::PgStore;
use crate::tracker::link::StoredAlias;
use crate::tracker::sprint::{SprintRank, SprintState, pick_current_sprint};
use crate::tracker::{Link, PrevState, SessionText, Snapshot, TaskRow};

/// One `sensei.trackers` row.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackerRow {
    pub id: uuid::Uuid,
    pub workspace_id: uuid::Uuid,
    pub kind: String,
    /// `epic | sprint | task` (D40).
    pub level: String,
    pub base_url: String,
    pub account: String,
    pub external_scope: String,
    pub board_ref: String,
    pub params: serde_json::Value,
    pub secret_service: String,
    pub last_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_success_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    /// R11: transcript turns ingested up to here have been checked for unknown keys.
    pub discovered_through: chrono::DateTime<chrono::Utc>,
    /// The token owner's provider id (W1); NULL until known.
    pub owner_ref: Option<crate::tracker::AccountRef>,
    /// Why `owner_ref` is NULL (`sensei.tracker_owner_state`).
    pub owner_state: Option<String>,
    /// The last owner lookup's failure code (`sensei.tracker_error`).
    pub owner_error: Option<String>,
    /// Whether usage is written back (W8).
    pub writeback_enabled: bool,
    /// Why the last write-back pass could not run or ended early (W2).
    pub writeback_error: Option<String>,
}

impl<'r> sqlx_core::from_row::FromRow<'r, sqlx_postgres::PgRow> for TrackerRow {
    fn from_row(r: &'r sqlx_postgres::PgRow) -> Result<Self, sqlx_core::Error> {
        use sqlx_core::row::Row;
        Ok(TrackerRow {
            id: r.try_get("id")?,
            workspace_id: r.try_get("workspace_id")?,
            kind: r.try_get("kind")?,
            level: r.try_get("level")?,
            base_url: r.try_get("base_url")?,
            account: r.try_get("account")?,
            external_scope: r.try_get("external_scope")?,
            board_ref: r.try_get("board_ref")?,
            params: r.try_get("params")?,
            secret_service: r.try_get("secret_service")?,
            last_attempt_at: r.try_get("last_attempt_at")?,
            last_success_at: r.try_get("last_success_at")?,
            last_error: r.try_get("last_error")?,
            discovered_through: r.try_get("discovered_through")?,
            owner_ref: r
                .try_get::<Option<String>, _>("owner_ref")?
                .map(crate::tracker::AccountRef::new),
            owner_state: r.try_get("owner_state")?,
            owner_error: r.try_get("owner_error")?,
            writeback_enabled: r.try_get("writeback_enabled")?,
            writeback_error: r.try_get("writeback_error")?,
        })
    }
}

/// One ticket a write-back pass may write (spec W4), in pass order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WritebackTask {
    pub id: uuid::Uuid,
    pub external_id: String,
    pub comment_ref: Option<String>,
    pub hash: Option<String>,
    /// `sensei.tracker_writeback_status`, NULL when never attempted.
    pub status: Option<String>,
}

/// What the per-ticket re-check (W2) found just before a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteTarget {
    /// Write it.
    Go,
    /// No longer the token owner's: skip it, carry on.
    Reassigned,
    /// The tracker is gone, write-back is off, or the token changed: end the pass.
    Stop,
}

/// The workspace that already owns a board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardOwner {
    pub tracker_id: uuid::Uuid,
    pub workspace_id: uuid::Uuid,
    pub workspace_name: String,
}

/// A connect (or re-connect) to persist in one transaction.
#[derive(Debug, Clone)]
pub struct NewTracker {
    pub workspace_id: uuid::Uuid,
    pub kind: String,
    pub level: String,
    pub base_url: String,
    pub account: String,
    pub external_scope: String,
    pub board_ref: String,
    pub params: serde_json::Value,
    pub secret_service: String,
    /// Who the token belongs to, from `validate` (W1).
    pub owner: crate::tracker::OwnerLookup,
}

/// Why persisting a connect failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectDbError {
    /// Another workspace took the board meanwhile.
    BoardTaken,
    /// The new secret's pending tombstone was already drained: the secret may be
    /// gone, so the row must not point at it.
    PendingTombstoneGone,
    Db(String),
}

/// What an applied snapshot changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplyCounts {
    pub sprints: u32,
    pub tasks: u32,
    pub removed: u32,
    pub links: u32,
    pub skipped_members: u32,
}

/// One link of a session to a ticket, as the task read shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLinkRow {
    pub source: String,
    pub matched_key: String,
    pub mentions: i32,
}

/// A candidate session linked to a ticket, with its links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedSession {
    pub id: uuid::Uuid,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub links: Vec<SessionLinkRow>,
}

/// One ticket of a workspace's tracker and the candidate sessions linked to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDetail {
    pub id: uuid::Uuid,
    /// The tracker whose board holds the ticket.
    pub tracker_id: uuid::Uuid,
    pub key: String,
    pub summary: Option<String>,
    pub issue_type: Option<String>,
    pub status: Option<String>,
    pub removed: bool,
    /// Whether the token owner is the assignee (W1); NULL while the owner is unknown.
    pub assigned_to_me: Option<bool>,
    /// The last write-back outcome (W9), and when a write last landed.
    pub writeback_status: Option<String>,
    pub writeback_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sessions: Vec<LinkedSession>,
}

/// What an R11 apply added.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoverCounts {
    pub tasks: u32,
    pub links: u32,
    pub skipped_members: u32,
}

/// Why an apply did not commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// The tracker row no longer exists (disconnected mid-sync).
    Gone,
    /// The row exists under another credential (re-connected, or re-keyed with
    /// its site, mid-sync): this result belongs to the old token.
    Superseded,
    Db(String),
}

const TRACKER_SELECT: &str =
    "SELECT id, workspace_id, kind::text AS kind, level::text AS level, base_url, account,
        external_scope, board_ref, params, secret_service, last_attempt_at, last_success_at,
        last_error::text AS last_error, discovered_through, owner_ref,
        owner_state::text AS owner_state, owner_error::text AS owner_error, writeback_enabled,
        writeback_error::text AS writeback_error
   FROM sensei.trackers";

/// Last occurrence wins, so overlapping provider pages never make one upsert touch a row twice.
fn dedupe_by<T: Clone, K: std::hash::Hash + Eq>(items: &[T], key: impl Fn(&T) -> K) -> Vec<T> {
    let mut seen = std::collections::HashMap::new();
    for (i, item) in items.iter().enumerate() {
        seen.insert(key(item), i);
    }
    let mut keep: Vec<usize> = seen.into_values().collect();
    keep.sort_unstable();
    keep.into_iter().map(|i| items[i].clone()).collect()
}

/// The tracker's candidate sessions (spec R1): anchored in a repository of the
/// tracker's workspace, whichever repo. One row per session; `tracker` is the SQL
/// placeholder holding the tracker id.
pub(crate) fn candidate_sessions(tracker: &str) -> String {
    format!(
        "SELECT s.id AS session_id, fp.repository_id
           FROM activity.sessions s
           JOIN sensei.folder_workspaces fp ON fp.folder_id = s.repo_folder_id
           JOIN sensei.trackers tr ON tr.workspace_id = fp.workspace_id
          WHERE tr.id = {tracker}"
    )
}

fn is_unique_violation(e: &sqlx_core::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505")
}

type Conn = sqlx_postgres::PgConnection;

/// Upsert tickets by `(tracker_id, external_id)`: a seen ticket is live again.
async fn upsert_tasks(
    c: &mut Conn,
    id: uuid::Uuid,
    tasks: &[TaskRow],
) -> Result<(), sqlx_core::Error> {
    let t_ext: Vec<String> = tasks.iter().map(|t| t.external_id.clone()).collect();
    let t_key: Vec<String> = tasks.iter().map(|t| t.key.clone()).collect();
    let t_scope: Vec<Option<String>> = tasks.iter().map(|t| t.scope.clone()).collect();
    let t_summary: Vec<Option<String>> = tasks.iter().map(|t| t.summary.clone()).collect();
    let t_type: Vec<Option<String>> = tasks.iter().map(|t| t.issue_type.clone()).collect();
    let t_status: Vec<Option<String>> = tasks.iter().map(|t| t.status.clone()).collect();
    // Compared with the owner here, in the apply's transaction; never stored (W1).
    let t_assignee: Vec<Option<&str>> =
        tasks.iter().map(|t| t.assignee.as_ref().map(|a| a.as_str())).collect();
    sqlx_core::query::query(
        "INSERT INTO sensei.tracker_tasks
                (tracker_id, external_id, key, scope, summary, issue_type, status, removed_at, last_seen_at,
                 assigned_to_me)
         SELECT $1, x.e, x.k, x.sc, x.su, x.ty, x.st, NULL, now(),
                CASE WHEN o.owner_ref IS NULL THEN NULL ELSE coalesce(x.a = o.owner_ref, false) END
           FROM unnest($2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[], $8::text[])
                AS x(e, k, sc, su, ty, st, a)
          CROSS JOIN (SELECT owner_ref FROM sensei.trackers WHERE id = $1) o
         ON CONFLICT (tracker_id, external_id) DO UPDATE
            SET key = excluded.key, scope = excluded.scope, summary = excluded.summary,
                issue_type = excluded.issue_type, status = excluded.status,
                removed_at = NULL, last_seen_at = now(), assigned_to_me = excluded.assigned_to_me",
    )
    .bind(id)
    .bind(&t_ext)
    .bind(&t_key)
    .bind(&t_scope)
    .bind(&t_summary)
    .bind(&t_type)
    .bind(&t_status)
    .bind(&t_assignee)
    .execute(&mut *c)
    .await
    .map(|_| ())
}

/// Every ticket's current sprint, by the shared ranking (R5).
async fn recompute_current_sprints(c: &mut Conn, id: uuid::Uuid) -> Result<(), sqlx_core::Error> {
    let listing = sqlx_core::query_as::query_as::<
        _,
        (uuid::Uuid, uuid::Uuid, String, Option<chrono::DateTime<chrono::Utc>>, i64, bool),
    >(
        "SELECT m.task_id, s.id, s.state, s.start_at, s.seq, s.removed_at IS NOT NULL
           FROM sensei.tracker_task_sprints m JOIN sensei.sprints s ON s.id = m.sprint_id
          WHERE s.tracker_id = $1",
    )
    .bind(id)
    .fetch_all(&mut *c)
    .await?;
    let mut by_task: std::collections::HashMap<uuid::Uuid, Vec<SprintRank>> =
        std::collections::HashMap::new();
    for (task, sprint, state, start_at, seq, removed) in listing {
        let Some(state) = SprintState::parse(&state) else { continue };
        by_task.entry(task).or_default().push(SprintRank {
            id: sprint,
            state,
            start_at,
            seq,
            removed,
        });
    }
    let (cur_task, cur_sprint): (Vec<uuid::Uuid>, Vec<uuid::Uuid>) = by_task
        .iter()
        .filter_map(|(task, ranks)| pick_current_sprint(ranks).map(|s| (*task, s)))
        .unzip();
    sqlx_core::query::query(
        "UPDATE sensei.tracker_tasks t SET current_sprint_id = v.s
           FROM unnest($2::uuid[], $3::uuid[]) AS v(t, s)
          WHERE t.tracker_id = $1 AND t.id = v.t AND t.current_sprint_id IS DISTINCT FROM v.s",
    )
    .bind(id)
    .bind(&cur_task)
    .bind(&cur_sprint)
    .execute(&mut *c)
    .await?;
    sqlx_core::query::query(
        "UPDATE sensei.tracker_tasks SET current_sprint_id = NULL
          WHERE tracker_id = $1 AND current_sprint_id IS NOT NULL AND id <> ALL($2::uuid[])",
    )
    .bind(id)
    .bind(&cur_task)
    .execute(&mut *c)
    .await
    .map(|_| ())
}

/// Every current key becomes an alias (former keys stay), and leaves the R11
/// negative cache.
async fn add_task_keys(c: &mut Conn, id: uuid::Uuid) -> Result<(), sqlx_core::Error> {
    sqlx_core::query::query(
        "INSERT INTO sensei.tracker_task_keys (task_id, key)
         SELECT id, key FROM sensei.tracker_tasks WHERE tracker_id = $1
         ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .execute(&mut *c)
    .await?;
    sqlx_core::query::query(
        "DELETE FROM sensei.tracker_unknown_keys u
          WHERE u.tracker_id = $1
            AND EXISTS (SELECT 1 FROM sensei.tracker_tasks t WHERE t.tracker_id = $1 AND t.key = u.key)",
    )
    .bind(id)
    .execute(&mut *c)
    .await
    .map(|_| ())
}

/// Write links for the sessions still present (locked against a concurrent
/// prune), resolving external ids in SQL. A link already there is refreshed.
async fn write_links(
    c: &mut Conn,
    id: uuid::Uuid,
    links: &[Link],
) -> Result<u64, sqlx_core::Error> {
    let mut wanted: Vec<uuid::Uuid> = links.iter().map(|l| l.session_id).collect();
    wanted.sort_unstable();
    wanted.dedup();
    let present: std::collections::HashSet<uuid::Uuid> =
        sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM activity.sessions WHERE id = ANY($1::uuid[]) FOR KEY SHARE",
        )
        .bind(&wanted)
        .fetch_all(&mut *c)
        .await?
        .into_iter()
        .map(|(s,)| s)
        .collect();
    let kept: Vec<&Link> = links.iter().filter(|l| present.contains(&l.session_id)).collect();
    let l_session: Vec<uuid::Uuid> = kept.iter().map(|l| l.session_id).collect();
    let l_task: Vec<String> = kept.iter().map(|l| l.task_external_id.clone()).collect();
    let l_source: Vec<String> = kept.iter().map(|l| l.source.as_str().to_string()).collect();
    let l_key: Vec<String> = kept.iter().map(|l| l.matched_key.clone()).collect();
    let l_mentions: Vec<i32> =
        kept.iter().map(|l| i32::try_from(l.mentions).unwrap_or(i32::MAX)).collect();
    sqlx_core::query::query(
        "INSERT INTO sensei.session_task_links (session_id, task_id, source, matched_key, mentions)
         SELECT x.s, t.id, x.src::sensei.link_source, x.k, x.m
           FROM unnest($2::uuid[], $3::text[], $4::text[], $5::text[], $6::int[]) AS x(s, e, src, k, m)
           JOIN sensei.tracker_tasks t ON t.tracker_id = $1 AND t.external_id = x.e
         ON CONFLICT (session_id, task_id, source) DO UPDATE
            SET matched_key = excluded.matched_key, mentions = excluded.mentions, linked_at = now()",
    )
    .bind(id)
    .bind(&l_session)
    .bind(&l_task)
    .bind(&l_source)
    .bind(&l_key)
    .bind(&l_mentions)
    .execute(&mut *c)
    .await
    .map(|r| r.rows_affected())
}

impl PgStore {
    pub async fn tracker_by_id(&self, id: uuid::Uuid) -> Result<Option<TrackerRow>, String> {
        sqlx_core::query_as::query_as::<_, TrackerRow>(&format!("{TRACKER_SELECT} WHERE id = $1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    /// The workspace's trackers, by level (epic, sprint, task) then age.
    pub async fn trackers_for_workspace(
        &self,
        workspace_id: uuid::Uuid,
    ) -> Result<Vec<TrackerRow>, String> {
        sqlx_core::query_as::query_as::<_, TrackerRow>(&format!(
            "{TRACKER_SELECT} WHERE workspace_id = $1 ORDER BY level, created_at, id"
        ))
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    pub async fn tracker_board_owner(
        &self,
        kind: &str,
        base_url: &str,
        board_ref: &str,
    ) -> Result<Option<BoardOwner>, String> {
        sqlx_core::query_as::query_as::<_, (uuid::Uuid, uuid::Uuid, String)>(
            "SELECT t.id, t.workspace_id, p.name
               FROM sensei.trackers t JOIN sensei.workspaces p ON p.id = t.workspace_id
              WHERE t.kind = $1::sensei.tracker_kind AND t.base_url = $2 AND t.board_ref = $3",
        )
        .bind(kind)
        .bind(base_url)
        .bind(board_ref)
        .fetch_optional(&self.pool)
        .await
        .map(|r| {
            r.map(|(tracker_id, workspace_id, workspace_name)| BoardOwner {
                tracker_id,
                workspace_id,
                workspace_name,
            })
        })
        .map_err(|e| e.to_string())
    }

    pub async fn mark_tracker_attempt(&self, id: uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query("UPDATE sensei.trackers SET last_attempt_at = now() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Record a failure code, only if the tracker still uses the credential the
    /// failing sync started with: a re-connect meanwhile supersedes the result.
    pub async fn mark_tracker_error(
        &self,
        id: uuid::Uuid,
        secret_service: &str,
        code: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.trackers SET last_error = $3::sensei.tracker_error
              WHERE id = $1 AND secret_service = $2",
        )
        .bind(id)
        .bind(secret_service)
        .bind(code)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    pub async fn tracker_prev_state(&self, id: uuid::Uuid) -> Result<PrevState, String> {
        let rows = sqlx_core::query_as::query_as::<_, (String, Option<String>)>(
            "SELECT external_id, members_fetched_state FROM sensei.sprints WHERE tracker_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(PrevState {
            members_fetched_state: rows
                .into_iter()
                .map(|(ext, state)| (ext, state.as_deref().and_then(SprintState::parse)))
                .collect(),
        })
    }

    pub async fn apply_tracker_snapshot(
        &self,
        id: uuid::Uuid,
        secret_service: &str,
        snapshot: &Snapshot,
        params_patch: &serde_json::Value,
        links: &[Link],
    ) -> Result<ApplyCounts, ApplyError> {
        let db = |e: sqlx_core::Error| ApplyError::Db(e.to_string());
        let sprints = dedupe_by(&snapshot.sprints, |s| s.external_id.clone());
        let tasks = dedupe_by(&snapshot.tasks, |t| t.external_id.clone());
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx_core::query::query("SET LOCAL statement_timeout = '60s'")
            .execute(&mut *tx)
            .await
            .map_err(db)?;

        // Gone, or re-keyed since the sync started: the result is superseded.
        let locked = sqlx_core::query_as::query_as::<_, (String,)>(
            "SELECT secret_service FROM sensei.trackers WHERE id = $1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match locked {
            None => return Err(ApplyError::Gone),
            Some((current,)) if current != secret_service => return Err(ApplyError::Superseded),
            Some(_) => {}
        }

        // Sprints.
        let s_ext: Vec<String> = sprints.iter().map(|s| s.external_id.clone()).collect();
        let s_seq: Vec<i64> = sprints.iter().map(|s| s.seq).collect();
        let s_name: Vec<String> = sprints.iter().map(|s| s.name.clone()).collect();
        let s_state: Vec<String> = sprints.iter().map(|s| s.state.as_str().to_string()).collect();
        let s_start: Vec<Option<chrono::DateTime<chrono::Utc>>> =
            sprints.iter().map(|s| s.start_at).collect();
        let s_end: Vec<Option<chrono::DateTime<chrono::Utc>>> =
            sprints.iter().map(|s| s.end_at).collect();
        let s_done: Vec<Option<chrono::DateTime<chrono::Utc>>> =
            sprints.iter().map(|s| s.complete_at).collect();
        let sprint_ids: Vec<(String, uuid::Uuid)> = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.sprints
                    (tracker_id, external_id, seq, name, state, start_at, end_at, complete_at, removed_at, last_seen_at)
             SELECT $1, x.e, x.q, x.n, x.s, x.a, x.b, x.c, NULL, now()
               FROM unnest($2::text[], $3::bigint[], $4::text[], $5::text[],
                           $6::timestamptz[], $7::timestamptz[], $8::timestamptz[]) AS x(e, q, n, s, a, b, c)
             ON CONFLICT (tracker_id, external_id) DO UPDATE
                SET seq = excluded.seq, name = excluded.name, state = excluded.state,
                    start_at = excluded.start_at, end_at = excluded.end_at, complete_at = excluded.complete_at,
                    removed_at = NULL, last_seen_at = now()
             RETURNING external_id, id",
        )
        .bind(id)
        .bind(&s_ext)
        .bind(&s_seq)
        .bind(&s_name)
        .bind(&s_state)
        .bind(&s_start)
        .bind(&s_end)
        .bind(&s_done)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let removed_sprints = sqlx_core::query::query(
            "UPDATE sensei.sprints SET removed_at = now(), members_fetched_state = NULL
              WHERE tracker_id = $1 AND removed_at IS NULL AND external_id <> ALL($2::text[])",
        )
        .bind(id)
        .bind(&s_ext)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        sqlx_core::query::query(
            "DELETE FROM sensei.tracker_task_sprints
              WHERE sprint_id IN (SELECT id FROM sensei.sprints WHERE tracker_id = $1 AND removed_at IS NOT NULL)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;

        // Tickets.
        upsert_tasks(&mut tx, id, &tasks).await.map_err(db)?;
        let t_ext: Vec<String> = tasks.iter().map(|t| t.external_id.clone()).collect();
        let removed_tasks = sqlx_core::query::query(
            "UPDATE sensei.tracker_tasks SET removed_at = now()
              WHERE tracker_id = $1 AND removed_at IS NULL AND external_id <> ALL($2::text[])",
        )
        .bind(id)
        .bind(&t_ext)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();

        // Memberships of every sprint fetched this time.
        let sprint_id: std::collections::HashMap<&str, uuid::Uuid> =
            sprint_ids.iter().map(|(e, i)| (e.as_str(), *i)).collect();
        let state_of: std::collections::HashMap<&str, &str> =
            sprints.iter().map(|s| (s.external_id.as_str(), s.state.as_str())).collect();
        let mut skipped: u32 = 0;
        for (sprint_ext, members) in &snapshot.members {
            let (Some(members), Some(sid)) = (members, sprint_id.get(sprint_ext.as_str())) else {
                continue;
            };
            let mut distinct: Vec<String> = members.clone();
            distinct.sort_unstable();
            distinct.dedup();
            sqlx_core::query::query("DELETE FROM sensei.tracker_task_sprints WHERE sprint_id = $1")
                .bind(sid)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            let inserted = sqlx_core::query::query(
                "INSERT INTO sensei.tracker_task_sprints (task_id, sprint_id)
                 SELECT t.id, $2 FROM sensei.tracker_tasks t
                  WHERE t.tracker_id = $1 AND t.external_id = ANY($3::text[])",
            )
            .bind(id)
            .bind(sid)
            .bind(&distinct)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
            skipped += (distinct.len() as u64).saturating_sub(inserted) as u32;
            sqlx_core::query::query(
                "UPDATE sensei.sprints SET members_fetched_state = $2 WHERE id = $1",
            )
            .bind(sid)
            .bind(state_of.get(sprint_ext.as_str()).copied())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        // Current sprint per ticket, ranked by the shared pure function.
        recompute_current_sprints(&mut tx, id).await.map_err(db)?;

        // Every current key becomes an alias; former keys stay. A key the board
        // has now leaves the R11 negative cache.
        add_task_keys(&mut tx, id).await.map_err(db)?;

        // Relink: a full replacement.
        sqlx_core::query::query(
            "DELETE FROM sensei.session_task_links
              WHERE task_id IN (SELECT id FROM sensei.tracker_tasks WHERE tracker_id = $1)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let linked = write_links(&mut tx, id, links).await.map_err(db)?;

        sqlx_core::query::query(
            "UPDATE sensei.trackers SET params = params || $2, last_success_at = now(), last_error = NULL
              WHERE id = $1",
        )
        .bind(id)
        .bind(params_patch)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;

        Ok(ApplyCounts {
            sprints: sprints.len() as u32,
            tasks: tasks.len() as u32,
            removed: (removed_sprints + removed_tasks) as u32,
            links: linked as u32,
            skipped_members: skipped,
        })
    }

    /// Every stored key alias of the tracker's tasks, removed tasks included.
    pub async fn tracker_key_aliases(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<Vec<StoredAlias>, String> {
        let rows = sqlx_core::query_as::query_as::<
            _,
            (String, String, Option<String>, bool, bool, Option<chrono::DateTime<chrono::Utc>>),
        >(
            "SELECT t.external_id, k.key, t.scope, k.key = t.key, t.removed_at IS NOT NULL, t.last_seen_at
               FROM sensei.tracker_task_keys k JOIN sensei.tracker_tasks t ON t.id = k.task_id
              WHERE t.tracker_id = $1
             UNION
             SELECT external_id, key, scope, true, removed_at IS NOT NULL, last_seen_at
               FROM sensei.tracker_tasks WHERE tracker_id = $1",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(external_id, key, scope, current, removed, last_seen_at)| StoredAlias {
                external_id,
                key,
                scope,
                current,
                removed,
                last_seen_at,
            })
            .collect())
    }

    /// One chunk of the tracker's candidate sessions (R1), keyset-paged by id:
    /// distinct branches and each turn's prompt (first 4 KB). Never `assistant_text`.
    pub async fn load_link_inputs(
        &self,
        tracker_id: uuid::Uuid,
        after: Option<uuid::Uuid>,
        limit: i64,
        ingested_after: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<SessionText>, String> {
        let rows = sqlx_core::query_as::query_as::<_, (uuid::Uuid, Option<String>, Vec<String>, Vec<String>)>(
            &format!("WITH c AS (
                SELECT s.id, s.client_session_id, r.repo_key
                  FROM ({candidates}) cs
                  JOIN activity.sessions s ON s.id = cs.session_id
                  JOIN sensei.repositories r ON r.id = cs.repository_id
                 WHERE ($2::uuid IS NULL OR cs.session_id > $2)
                   AND ($4::timestamptz IS NULL OR EXISTS (
                        SELECT 1 FROM activity.transcript_turns t
                         WHERE t.session_id = s.client_session_id AND t.created_at > $4))
                 ORDER BY cs.session_id
                 LIMIT $3)
             SELECT c.id, c.repo_key,
                    coalesce((SELECT array_agg(DISTINCT t.git_branch)
                                FROM activity.transcript_turns t
                               WHERE t.session_id = c.client_session_id AND t.git_branch IS NOT NULL),
                             '{{}}'),
                    coalesce((SELECT array_agg(left(t.user_text, 4096) ORDER BY t.turn_index)
                                FROM activity.transcript_turns t
                               WHERE t.session_id = c.client_session_id AND t.user_text IS NOT NULL),
                             '{{}}')
               FROM c ORDER BY c.id",
            candidates = candidate_sessions("$1"),
        ))
        .bind(tracker_id)
        .bind(after)
        .bind(limit)
        .bind(ingested_after)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(session_id, repo_key, branches, prompts)| SessionText {
                session_id,
                branches,
                prompts,
                repo_key,
            })
            .collect())
    }

    /// The workspace's ticket with this key (current or former, case-insensitive)
    /// and its linked candidate sessions; `None` when there is no such ticket.
    pub async fn tracker_task_detail(
        &self,
        workspace_id: uuid::Uuid,
        key: &str,
    ) -> Result<Option<TaskDetail>, String> {
        let err = |e: sqlx_core::Error| e.to_string();
        let Some((
            task_id,
            tracker_id,
            key,
            summary,
            issue_type,
            status,
            removed,
            assigned_to_me,
            writeback_status,
            writeback_at,
        )) =
            sqlx_core::query_as::query_as::<
                _,
                (
                    uuid::Uuid,
                    uuid::Uuid,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    bool,
                    Option<bool>,
                    Option<String>,
                    Option<chrono::DateTime<chrono::Utc>>,
                ),
            >(
                "SELECT t.id, t.tracker_id, t.key, t.summary, t.issue_type, t.status, t.removed_at IS NOT NULL,
                        t.assigned_to_me, t.writeback_status::text, t.writeback_at
                   FROM sensei.tracker_tasks t JOIN sensei.trackers tr ON tr.id = t.tracker_id
                  WHERE tr.workspace_id = $1
                    AND (upper(t.key) = upper($2)
                         OR EXISTS (SELECT 1 FROM sensei.tracker_task_keys k
                                     WHERE k.task_id = t.id AND upper(k.key) = upper($2)))
                  ORDER BY upper(t.key) = upper($2) DESC, t.removed_at IS NULL DESC,
                           tr.level DESC, tr.created_at, tr.id
                  LIMIT 1",
            )
            .bind(workspace_id)
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(err)?
        else {
            return Ok(None);
        };
        // Only the workspace's current candidates (R1), as usage counts them.
        let rows = sqlx_core::query_as::query_as::<
            _,
            (uuid::Uuid, Option<chrono::DateTime<chrono::Utc>>, String, String, i32),
        >(&format!(
            "SELECT s.id, s.started_at, l.source::text, l.matched_key, l.mentions
                   FROM sensei.session_task_links l
                   JOIN activity.sessions s ON s.id = l.session_id
                  WHERE l.task_id = $1
                    AND l.session_id IN (SELECT session_id FROM ({candidates}) c)
                  ORDER BY s.started_at DESC NULLS LAST, s.id, l.source",
            candidates =
                candidate_sessions("(SELECT tracker_id FROM sensei.tracker_tasks WHERE id = $1)"),
        ))
        .bind(task_id)
        .fetch_all(&self.pool)
        .await
        .map_err(err)?;
        let mut sessions: Vec<LinkedSession> = Vec::new();
        for (id, started_at, source, matched_key, mentions) in rows {
            let link = SessionLinkRow { source, matched_key, mentions };
            match sessions.last_mut() {
                Some(last) if last.id == id => last.links.push(link),
                _ => sessions.push(LinkedSession { id, started_at, links: vec![link] }),
            }
        }
        Ok(Some(TaskDetail {
            id: task_id,
            tracker_id,
            key,
            summary,
            issue_type,
            status,
            assigned_to_me,
            writeback_status,
            writeback_at,
            removed,
            sessions,
        }))
    }

    /// Keys the tracker's board answered it does not have, checked within `ttl`.
    pub async fn tracker_unknown_keys_fresh(
        &self,
        tracker_id: uuid::Uuid,
        ttl: std::time::Duration,
    ) -> Result<Vec<String>, String> {
        sqlx_core::query_as::query_as::<_, (String,)>(
            "SELECT key FROM sensei.tracker_unknown_keys
              WHERE tracker_id = $1 AND checked_at > now() - make_interval(secs => $2)
              ORDER BY key",
        )
        .bind(tracker_id)
        .bind(ttl.as_secs_f64())
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|(k,)| k).collect())
        .map_err(|e| e.to_string())
    }

    /// The trackers whose candidate sessions (R1) include this client session.
    pub async fn trackers_for_client_session(
        &self,
        client_session_id: &str,
    ) -> Result<Vec<uuid::Uuid>, String> {
        sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(&format!(
            // `o`, not `tr`: the fragment has its own `tr`, which would shadow it.
            "SELECT o.id FROM sensei.trackers o
              WHERE EXISTS (SELECT 1 FROM ({candidates}) c
                              JOIN activity.sessions s ON s.id = c.session_id
                             WHERE s.client_session_id = $1)
              ORDER BY o.id",
            candidates = candidate_sessions("o.id"),
        ))
        .bind(client_session_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|(id,)| id).collect())
        .map_err(|e| e.to_string())
    }

    /// R11 apply: add the discovered tickets, their aliases, memberships and the
    /// links to them, the keys the board lacks, and (ingest path) advance the
    /// watermark. Never marks anything removed, never deletes a link.
    pub async fn apply_discovered(
        &self,
        id: uuid::Uuid,
        secret_service: &str,
        discovered: &crate::tracker::Discovered,
        links: &[Link],
        not_found: &[String],
        watermark: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<DiscoverCounts, ApplyError> {
        let db = |e: sqlx_core::Error| ApplyError::Db(e.to_string());
        let tasks = dedupe_by(&discovered.tasks, |t| t.external_id.clone());
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx_core::query::query("SET LOCAL statement_timeout = '60s'")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let locked = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.trackers WHERE id = $1 AND secret_service = $2 FOR UPDATE",
        )
        .bind(id)
        .bind(secret_service)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if locked.is_none() {
            return Err(ApplyError::Gone);
        }

        upsert_tasks(&mut tx, id, &tasks).await.map_err(db)?;
        // Memberships of sprints sensei already has; others wait for the full sync.
        let (m_sprint, m_task): (Vec<String>, Vec<String>) =
            discovered.members.iter().cloned().unzip();
        let (known_members,): (i64,) = sqlx_core::query_as::query_as(
            "WITH m AS (SELECT DISTINCT se, te FROM unnest($2::text[], $3::text[]) AS m(se, te)),
                  ins AS (
                    INSERT INTO sensei.tracker_task_sprints (task_id, sprint_id)
                    SELECT t.id, s.id FROM m
                      JOIN sensei.sprints s ON s.tracker_id = $1 AND s.external_id = m.se AND s.removed_at IS NULL
                      JOIN sensei.tracker_tasks t ON t.tracker_id = $1 AND t.external_id = m.te
                    ON CONFLICT DO NOTHING)
             SELECT count(*) FROM m
               JOIN sensei.sprints s ON s.tracker_id = $1 AND s.external_id = m.se AND s.removed_at IS NULL",
        )
        .bind(id)
        .bind(&m_sprint)
        .bind(&m_task)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let distinct_members: std::collections::HashSet<&(String, String)> =
            discovered.members.iter().collect();
        let skipped = (distinct_members.len() as i64 - known_members).max(0) as u32;
        recompute_current_sprints(&mut tx, id).await.map_err(db)?;
        add_task_keys(&mut tx, id).await.map_err(db)?;
        let linked = write_links(&mut tx, id, links).await.map_err(db)?;
        sqlx_core::query::query(
            "INSERT INTO sensei.tracker_unknown_keys (tracker_id, key)
             SELECT $1, k FROM unnest($2::text[]) AS k
             ON CONFLICT (tracker_id, key) DO UPDATE SET checked_at = now()",
        )
        .bind(id)
        .bind(not_found)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(w) = watermark {
            sqlx_core::query::query(
                "UPDATE sensei.trackers SET discovered_through = $2 WHERE id = $1",
            )
            .bind(id)
            .bind(w)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(DiscoverCounts {
            tasks: tasks.len() as u32,
            links: linked as u32,
            skipped_members: skipped,
        })
    }

    pub async fn write_pending_tombstone(
        &self,
        secret_service: &str,
        account: &str,
        not_before: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.tracker_secret_tombstones (secret_service, account, not_before)
             VALUES ($1, $2, $3)
             ON CONFLICT (secret_service) DO UPDATE SET account = excluded.account, not_before = excluded.not_before",
        )
        .bind(secret_service)
        .bind(account)
        .bind(not_before)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    /// Persist a connect: `(tracker id, created, the site's other boards re-keyed to its token)`.
    pub async fn connect_tracker(
        &self,
        t: &NewTracker,
    ) -> Result<(uuid::Uuid, bool, Vec<uuid::Uuid>), ConnectDbError> {
        let db = |e: sqlx_core::Error| {
            if is_unique_violation(&e) {
                ConnectDbError::BoardTaken
            } else {
                ConnectDbError::Db(e.to_string())
            }
        };
        let mut tx = self.pool.begin().await.map_err(db)?;
        // Serialise connects for the workspace.
        let workspace = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.workspaces WHERE id = $1 FOR UPDATE",
        )
        .bind(t.workspace_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if workspace.is_none() {
            return Err(ConnectDbError::Db("workspace not found".into()));
        }
        let existing = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.trackers
              WHERE workspace_id = $1 AND kind = $2::sensei.tracker_kind AND base_url = $3 AND board_ref = $4",
        )
        .bind(t.workspace_id)
        .bind(&t.kind)
        .bind(&t.base_url)
        .bind(&t.board_ref)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;

        let (owner_ref, owner_state) = t.owner.columns();
        let owner_state = owner_state.map(|s| s.as_str());
        let outcome = match existing {
            Some((id,)) => {
                // The secret_service update fires the swap trigger, tombstoning the old secret.
                sqlx_core::query::query(
                    "UPDATE sensei.trackers
                        SET level = $2::sensei.tracker_level, account = $3, external_scope = $4,
                            params = params || $5, secret_service = $6,
                            last_success_at = NULL, last_error = NULL,
                            owner_ref = $7, owner_state = $8::sensei.tracker_owner_state, owner_error = NULL,
                            writeback_error = NULL
                      WHERE id = $1",
                )
                .bind(id)
                .bind(&t.level)
                .bind(&t.account)
                .bind(&t.external_scope)
                .bind(&t.params)
                .bind(&t.secret_service)
                .bind(owner_ref)
                .bind(owner_state)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (id, false)
            }
            None => {
                let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
                    "INSERT INTO sensei.trackers
                            (workspace_id, kind, level, base_url, account, external_scope, board_ref, params,
                             secret_service, owner_ref, owner_state)
                     VALUES ($1, $2::sensei.tracker_kind, $3::sensei.tracker_level, $4, $5, $6, $7, $8, $9,
                             $10, $11::sensei.tracker_owner_state)
                     RETURNING id",
                )
                .bind(t.workspace_id)
                .bind(&t.kind)
                .bind(&t.level)
                .bind(&t.base_url)
                .bind(&t.account)
                .bind(&t.external_scope)
                .bind(&t.board_ref)
                .bind(&t.params)
                .bind(&t.secret_service)
                .bind(owner_ref)
                .bind(owner_state)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                (id, true)
            }
        };

        // One credential per site (D14): the workspace's other boards on this site
        // move to the new token; the swap trigger tombstones each old secret. The
        // old token's failure no longer describes them; the caller re-syncs them.
        let rekeyed: Vec<uuid::Uuid> = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "UPDATE sensei.trackers SET account = $4, secret_service = $5, last_error = NULL,
                    owner_ref = NULL, owner_state = NULL, owner_error = NULL, writeback_error = NULL
              WHERE workspace_id = $1 AND kind = $2::sensei.tracker_kind AND base_url = $3
                AND secret_service <> $5
             RETURNING id",
        )
        .bind(t.workspace_id)
        .bind(&t.kind)
        .bind(&t.base_url)
        .bind(&t.account)
        .bind(&t.secret_service)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(id,)| id)
        .collect();

        // Whose tickets these are was decided against the old owner (W11).
        let reset: Vec<uuid::Uuid> = rekeyed.iter().copied().chain(Some(outcome.0)).collect();
        sqlx_core::query::query(
            "UPDATE sensei.tracker_tasks
                SET assigned_to_me = NULL, writeback_comment_ref = NULL, writeback_hash = NULL,
                    writeback_status = NULL
              WHERE tracker_id = ANY($1)",
        )
        .bind(&reset)
        .execute(&mut *tx)
        .await
        .map_err(db)?;

        let cleared = sqlx_core::query::query(
            "DELETE FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
        )
        .bind(&t.secret_service)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if cleared != 1 {
            return Err(ConnectDbError::PendingTombstoneGone);
        }
        tx.commit().await.map_err(db)?;
        Ok((outcome.0, outcome.1, rekeyed))
    }

    /// Store the owner a sync looked up (W1 backfill), only while the row still
    /// names the token it was looked up with and still has none. `Ok(false)` when a
    /// re-key or re-connect got there first.
    pub async fn set_tracker_owner(
        &self,
        id: uuid::Uuid,
        secret_service: &str,
        owner: &crate::tracker::OwnerLookup,
    ) -> Result<bool, String> {
        let (owner_ref, owner_state) = owner.columns();
        sqlx_core::query::query(
            "UPDATE sensei.trackers
                SET owner_ref = $3, owner_state = $4::sensei.tracker_owner_state, owner_error = NULL
              WHERE id = $1 AND secret_service = $2
                AND owner_ref IS NULL AND owner_state IS DISTINCT FROM 'not_a_person'",
        )
        .bind(id)
        .bind(secret_service)
        .bind(owner_ref)
        .bind(owner_state.map(|s| s.as_str()))
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() == 1)
        .map_err(|e| e.to_string())
    }

    /// Record why the owner lookup failed (W1), under the same conditions.
    pub async fn set_tracker_owner_error(
        &self,
        id: uuid::Uuid,
        secret_service: &str,
        code: &str,
    ) -> Result<bool, String> {
        sqlx_core::query::query(
            "UPDATE sensei.trackers SET owner_error = $3::sensei.tracker_error
              WHERE id = $1 AND secret_service = $2 AND owner_ref IS NULL",
        )
        .bind(id)
        .bind(secret_service)
        .bind(code)
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() == 1)
        .map_err(|e| e.to_string())
    }

    /// The tracker's write-back candidates (W4): live tickets assigned to the
    /// token owner, never-tried first, then by oldest attempt, then by id (W2).
    pub async fn writeback_candidates(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<Vec<WritebackTask>, String> {
        sqlx_core::query_as::query_as::<
            _,
            (uuid::Uuid, String, Option<String>, Option<String>, Option<String>),
        >(
            "SELECT id, external_id, writeback_comment_ref, writeback_hash, writeback_status::text
               FROM sensei.tracker_tasks
              WHERE tracker_id = $1 AND assigned_to_me AND removed_at IS NULL
              ORDER BY writeback_tried_at NULLS FIRST, id",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(id, external_id, comment_ref, hash, status)| WritebackTask {
                    id,
                    external_id,
                    comment_ref,
                    hash,
                    status,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
    }

    /// The per-ticket re-check before a write (W2), against the pass's token.
    pub async fn writeback_target(
        &self,
        task_id: uuid::Uuid,
        secret_service: &str,
    ) -> Result<WriteTarget, String> {
        let row = sqlx_core::query_as::query_as::<_, (bool, bool, Option<bool>)>(
            "SELECT tr.writeback_enabled, tr.secret_service = $2, t.assigned_to_me
               FROM sensei.tracker_tasks t JOIN sensei.trackers tr ON tr.id = t.tracker_id
              WHERE t.id = $1",
        )
        .bind(task_id)
        .bind(secret_service)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(match row {
            None | Some((false, _, _)) | Some((_, false, _)) => WriteTarget::Stop,
            Some((true, true, Some(true))) => WriteTarget::Go,
            Some((true, true, _)) => WriteTarget::Reassigned,
        })
    }

    /// A write landed: its hash, the comment it wrote, `ok` and the time, while the
    /// token is the pass's. `Ok(false)` when it isn't (superseded).
    pub async fn record_write_ok(
        &self,
        task_id: uuid::Uuid,
        secret_service: &str,
        hash: &str,
        comment_ref: Option<&str>,
    ) -> Result<bool, String> {
        sqlx_core::query::query(
            "UPDATE sensei.tracker_tasks t
                SET writeback_hash = $3, writeback_comment_ref = coalesce($4, t.writeback_comment_ref),
                    writeback_status = 'ok', writeback_at = now(), writeback_tried_at = now()
               FROM sensei.trackers tr
              WHERE t.id = $1 AND tr.id = t.tracker_id AND tr.secret_service = $2",
        )
        .bind(task_id)
        .bind(secret_service)
        .bind(hash)
        .bind(comment_ref)
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() == 1)
        .map_err(|e| e.to_string())
    }

    /// A write failed: only its status, under the same condition.
    pub async fn record_write_status(
        &self,
        task_id: uuid::Uuid,
        secret_service: &str,
        status: &str,
    ) -> Result<bool, String> {
        sqlx_core::query::query(
            "UPDATE sensei.tracker_tasks t
                SET writeback_status = $3::sensei.tracker_writeback_status, writeback_tried_at = now()
               FROM sensei.trackers tr
              WHERE t.id = $1 AND tr.id = t.tracker_id AND tr.secret_service = $2",
        )
        .bind(task_id)
        .bind(secret_service)
        .bind(status)
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() == 1)
        .map_err(|e| e.to_string())
    }

    /// Set (or, with `None`, clear) the pass-level failure (W2), under the token condition.
    pub async fn set_writeback_error(
        &self,
        tracker_id: uuid::Uuid,
        secret_service: &str,
        code: Option<&str>,
    ) -> Result<bool, String> {
        sqlx_core::query::query(
            "UPDATE sensei.trackers SET writeback_error = $3::sensei.tracker_error
              WHERE id = $1 AND secret_service = $2",
        )
        .bind(tracker_id)
        .bind(secret_service)
        .bind(code)
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() == 1)
        .map_err(|e| e.to_string())
    }

    /// Switch write-back on or off (W8). Switching it on clears what was waiting
    /// for that: the sticky statuses, and a removed comment's id and hash, so the
    /// next pass posts a new one. `Ok(false)` when there is no such tracker.
    pub async fn set_writeback_enabled(
        &self,
        tracker_id: uuid::Uuid,
        enabled: bool,
    ) -> Result<bool, String> {
        let err = |e: sqlx_core::Error| e.to_string();
        let mut tx = self.pool.begin().await.map_err(err)?;
        let Some((was,)) = sqlx_core::query_as::query_as::<_, (bool,)>(
            "SELECT writeback_enabled FROM sensei.trackers WHERE id = $1 FOR UPDATE",
        )
        .bind(tracker_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        else {
            return Ok(false);
        };
        sqlx_core::query::query("UPDATE sensei.trackers SET writeback_enabled = $2 WHERE id = $1")
            .bind(tracker_id)
            .bind(enabled)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        if !was && enabled {
            sqlx_core::query::query(
                "UPDATE sensei.tracker_tasks
                    SET writeback_comment_ref = CASE WHEN writeback_status = 'comment_removed' THEN NULL
                                                     ELSE writeback_comment_ref END,
                        writeback_hash = CASE WHEN writeback_status = 'comment_removed' THEN NULL
                                              ELSE writeback_hash END,
                        writeback_status = NULL
                  WHERE tracker_id = $1
                    AND writeback_status IN ('comment_removed', 'forbidden', 'not_found', 'field_invalid')",
            )
            .bind(tracker_id)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
        Ok(true)
    }

    /// `(assigned, failing)` for the board card (W8).
    pub async fn writeback_counts(&self, tracker_id: uuid::Uuid) -> Result<(i64, i64), String> {
        sqlx_core::query_as::query_as(
            "SELECT count(*) FILTER (WHERE assigned_to_me),
                    count(*) FILTER (WHERE assigned_to_me AND writeback_status <> 'ok')
               FROM sensei.tracker_tasks WHERE tracker_id = $1 AND removed_at IS NULL",
        )
        .bind(tracker_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Every tracker, for the scheduled sync.
    pub async fn list_trackers(&self) -> Result<Vec<TrackerRow>, String> {
        sqlx_core::query_as::query_as::<_, TrackerRow>(&format!(
            "{TRACKER_SELECT} ORDER BY created_at"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// `(tasks, sprints)` currently on the board (not removed).
    pub async fn tracker_counts(&self, id: uuid::Uuid) -> Result<(i64, i64), String> {
        sqlx_core::query_as::query_as::<_, (i64, i64)>(
            "SELECT (SELECT count(*) FROM sensei.tracker_tasks WHERE tracker_id = $1 AND removed_at IS NULL),
                    (SELECT count(*) FROM sensei.sprints WHERE tracker_id = $1 AND removed_at IS NULL)",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Delete one of the workspace's trackers. `false` when the workspace has no such tracker.
    pub async fn delete_tracker(
        &self,
        workspace_id: uuid::Uuid,
        tracker_id: uuid::Uuid,
    ) -> Result<bool, String> {
        sqlx_core::query::query("DELETE FROM sensei.trackers WHERE id = $1 AND workspace_id = $2")
            .bind(tracker_id)
            .bind(workspace_id)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .map_err(|e| e.to_string())
    }

    pub async fn due_tombstones(
        &self,
        prefix: &str,
        limit: i64,
    ) -> Result<Vec<(String, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT secret_service, account FROM sensei.tracker_secret_tombstones
              WHERE not_before <= now() AND starts_with(secret_service, $1)
              ORDER BY not_before LIMIT $2",
        )
        .bind(prefix)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Drop the tombstone of a secret a tracker still references, atomically: the
    /// tombstone row is locked for the check, so a trigger re-arming it for the
    /// last referencing tracker either waits for this to commit (and writes a
    /// fresh row) or commits first (and the check sees the tracker gone).
    /// `None`: no such tombstone; `Some(true)`: still referenced, tombstone
    /// dropped; `Some(false)`: unreferenced, tombstone kept for the delete.
    pub async fn release_referenced_tombstone(
        &self,
        secret_service: &str,
    ) -> Result<Option<bool>, String> {
        let err = |e: sqlx_core::Error| e.to_string();
        let mut tx = self.pool.begin().await.map_err(err)?;
        let held = sqlx_core::query_as::query_as::<_, (String,)>(
            "SELECT secret_service FROM sensei.tracker_secret_tombstones
              WHERE secret_service = $1 FOR UPDATE",
        )
        .bind(secret_service)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?;
        if held.is_none() {
            return Ok(None);
        }
        let (referenced,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS (SELECT 1 FROM sensei.trackers WHERE secret_service = $1)",
        )
        .bind(secret_service)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        if referenced {
            sqlx_core::query::query(
                "DELETE FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
            )
            .bind(secret_service)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
        Ok(Some(referenced))
    }

    /// Push a tombstone's next attempt back (its keyring delete failed).
    pub async fn defer_tombstone(
        &self,
        secret_service: &str,
        not_before: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.tracker_secret_tombstones SET not_before = $2 WHERE secret_service = $1",
        )
        .bind(secret_service)
        .bind(not_before)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    pub async fn delete_tombstone(&self, secret_service: &str) -> Result<(), String> {
        sqlx_core::query::query(
            "DELETE FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
        )
        .bind(secret_service)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod schema_tests {
    use crate::db::pg_store::PgStore;

    async fn workspace(pg: &PgStore, tag: &str) -> uuid::Uuid {
        pg.create_workspace(&format!("_test:tracker:{tag}:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap()
    }

    async fn insert_tracker(
        pg: &PgStore,
        workspace_id: uuid::Uuid,
        base_url: &str,
        board_ref: &str,
        secret_service: &str,
    ) -> Result<uuid::Uuid, String> {
        sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "INSERT INTO sensei.trackers
                 (workspace_id, kind, base_url, account, external_scope, board_ref, secret_service)
             VALUES ($1, 'jira', $2, 'dev@example.com', 'AUTH', $3, $4)
             RETURNING id",
        )
        .bind(workspace_id)
        .bind(base_url)
        .bind(board_ref)
        .bind(secret_service)
        .fetch_one(pg.pool())
        .await
        .map(|(id,)| id)
        .map_err(|e| e.to_string())
    }

    async fn tombstone(pg: &PgStore, secret_service: &str) -> Option<(String, bool)> {
        sqlx_core::query_as::query_as::<_, (String, bool)>(
            "SELECT account, not_before <= now()
               FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
        )
        .bind(secret_service)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
    }

    /// Run `sql` (one uuid bind, plus an optional text bind) and read the
    /// tombstones of `services` inside one transaction, then roll back. The
    /// trigger's tombstone is visible here but never to a concurrent drain.
    async fn within_rollback(
        pg: &PgStore,
        sql: &str,
        id: uuid::Uuid,
        text: Option<&str>,
        services: &[&str],
    ) -> Vec<Option<(String, bool)>> {
        let mut tx = pg.pool().begin().await.unwrap();
        let q = sqlx_core::query::query(sql).bind(id);
        let q = match text {
            Some(t) => q.bind(t.to_string()),
            None => q,
        };
        q.execute(&mut *tx).await.unwrap();
        let mut out = Vec::new();
        for s in services {
            out.push(
                sqlx_core::query_as::query_as::<_, (String, bool)>(
                    "SELECT account, not_before <= now()
                       FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
                )
                .bind(*s)
                .fetch_optional(&mut *tx)
                .await
                .unwrap(),
            );
        }
        tx.rollback().await.unwrap();
        out
    }

    async fn cleanup(pg: &PgStore, workspace_id: uuid::Uuid, services: &[&str]) {
        sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
            .bind(workspace_id)
            .execute(pg.pool())
            .await
            .ok();
        for s in services {
            sqlx_core::query::query(
                "DELETE FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
            )
            .bind(s)
            .execute(pg.pool())
            .await
            .ok();
        }
    }

    fn unique(tag: &str) -> String {
        format!("com.sensei.tracker.jira.test-{tag}-{}", uuid::Uuid::new_v4())
    }

    #[tokio::test]
    async fn a_workspace_holds_several_boards_and_a_board_defaults_to_the_sprint_level() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg, "several").await;
        let (s1, s2) = (unique("a"), unique("b"));
        let base = format!("https://t{}.atlassian.net", uuid::Uuid::new_v4().simple());
        let a = insert_tracker(&pg, p, &base, "1", &s1).await.unwrap();
        let b = insert_tracker(&pg, p, &base, "2", &s2).await.unwrap();
        let levels: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
            "SELECT id, level::text FROM sensei.trackers WHERE workspace_id = $1 ORDER BY board_ref",
        )
        .bind(p)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(levels, [(a, "sprint".to_string()), (b, "sprint".to_string())]);
        cleanup(&pg, p, &[&s1, &s2]).await;
    }

    #[tokio::test]
    async fn a_ticket_on_two_boards_is_read_from_the_narrower_board_whichever_synced_last() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg, "two-copies").await;
        let (s1, s2) = (unique("epic"), unique("sprint"));
        let base = format!("https://t{}.atlassian.net", uuid::Uuid::new_v4().simple());
        let sprint = insert_tracker(&pg, p, &base, "42", &s2).await.unwrap();
        let epic = insert_tracker(&pg, p, &base, "43", &s1).await.unwrap();
        sqlx_core::query::query("UPDATE sensei.trackers SET level = 'epic' WHERE id = $1")
            .bind(epic)
            .execute(pg.pool())
            .await
            .unwrap();
        for (tracker, seen) in [(sprint, "2026-10-01"), (epic, "2026-10-20")] {
            sqlx_core::query::query(
                "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, last_seen_at)
                 VALUES ($1, '10001', 'AUTH-1', $2::timestamptz)",
            )
            .bind(tracker)
            .bind(seen)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        let detail = pg.tracker_task_detail(p, "auth-1").await.unwrap().unwrap();
        assert_eq!(
            detail.tracker_id, sprint,
            "task, then sprint, then epic; never by sync recency"
        );
        cleanup(&pg, p, &[&s1, &s2]).await;
    }

    #[tokio::test]
    async fn a_board_belongs_to_at_most_one_workspace() {
        let pg = PgStore::test_store().await;
        let (p1, p2) = (workspace(&pg, "board-a").await, workspace(&pg, "board-b").await);
        let (s1, s2) = (unique("a"), unique("b"));
        let base = format!("https://t{}.atlassian.net", uuid::Uuid::new_v4().simple());
        insert_tracker(&pg, p1, &base, "42", &s1).await.unwrap();
        let second = insert_tracker(&pg, p2, &base, "42", &s2).await;
        assert!(
            second.is_err(),
            "the same (kind, base_url, board_ref) on two workspaces must be rejected"
        );
        cleanup(&pg, p1, &[&s1]).await;
        cleanup(&pg, p2, &[&s2]).await;
    }

    #[tokio::test]
    async fn deleting_a_tracker_tombstones_its_secret_immediately() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg, "delete").await;
        let s = unique("del");
        let t = insert_tracker(&pg, p, "https://d.atlassian.net", "7", &s).await.unwrap();
        let seen =
            within_rollback(&pg, "DELETE FROM sensei.trackers WHERE id = $1", t, None, &[&s]).await;
        assert_eq!(seen, [Some(("dev@example.com".into(), true))]);
        cleanup(&pg, p, &[&s]).await;
    }

    #[tokio::test]
    async fn deleting_the_workspace_cascades_and_still_tombstones_the_secret() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg, "cascade").await;
        let s = unique("cascade");
        insert_tracker(&pg, p, "https://c.atlassian.net", "8", &s).await.unwrap();
        let seen =
            within_rollback(&pg, "DELETE FROM sensei.workspaces WHERE id = $1", p, None, &[&s])
                .await;
        assert_eq!(seen, [Some(("dev@example.com".into(), true))]);
        cleanup(&pg, p, &[&s]).await;
    }

    #[tokio::test]
    async fn swapping_the_secret_tombstones_the_old_one_not_the_new_one() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg, "swap").await;
        let (old, new) = (unique("old"), unique("new"));
        let t = insert_tracker(&pg, p, "https://s.atlassian.net", "9", &old).await.unwrap();
        let seen = within_rollback(
            &pg,
            "UPDATE sensei.trackers SET secret_service = $2 WHERE id = $1",
            t,
            Some(&new),
            &[&old, &new],
        )
        .await;
        assert_eq!(seen[0], Some(("dev@example.com".into(), true)), "the old secret is tombstoned");
        assert_eq!(seen[1], None, "the live secret must never be tombstoned");
        cleanup(&pg, p, &[&old, &new]).await;
    }

    #[tokio::test]
    async fn updating_other_columns_writes_no_tombstone() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg, "noop").await;
        let s = unique("noop");
        let t = insert_tracker(&pg, p, "https://n.atlassian.net", "10", &s).await.unwrap();
        sqlx_core::query::query("UPDATE sensei.trackers SET last_attempt_at = now() WHERE id = $1")
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        assert_eq!(tombstone(&pg, &s).await, None);
        cleanup(&pg, p, &[&s]).await;
    }

    #[tokio::test]
    async fn sprints_tasks_and_memberships_are_keyed_and_cascade() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg, "children").await;
        let s = unique("children");
        let t = insert_tracker(&pg, p, "https://k.atlassian.net", "11", &s).await.unwrap();

        let (sprint,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.sprints (tracker_id, external_id, seq, name, state)
             VALUES ($1, '100', 100, 'Sprint 1', 'active') RETURNING id",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let dup_sprint = sqlx_core::query::query(
            "INSERT INTO sensei.sprints (tracker_id, external_id, seq, name, state)
             VALUES ($1, '100', 100, 'dup', 'active')",
        )
        .bind(t)
        .execute(pg.pool())
        .await;
        assert!(dup_sprint.is_err(), "sprints are unique by (tracker_id, external_id)");

        let (task,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, current_sprint_id)
             VALUES ($1, '10001', 'AUTH-420', $2) RETURNING id",
        )
        .bind(t)
        .bind(sprint)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let dup_task = sqlx_core::query::query(
            "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key) VALUES ($1, '10001', 'AUTH-999')",
        )
        .bind(t)
        .execute(pg.pool())
        .await;
        assert!(dup_task.is_err(), "tickets are unique by (tracker_id, external_id)");

        sqlx_core::query::query(
            "INSERT INTO sensei.tracker_task_sprints (task_id, sprint_id) VALUES ($1, $2)",
        )
        .bind(task)
        .bind(sprint)
        .execute(pg.pool())
        .await
        .unwrap();

        // Deleting the sprint drops its memberships and un-sets the ticket's current sprint.
        sqlx_core::query::query("DELETE FROM sensei.sprints WHERE id = $1")
            .bind(sprint)
            .execute(pg.pool())
            .await
            .unwrap();
        let (members, current): (i64, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
            "SELECT (SELECT count(*) FROM sensei.tracker_task_sprints WHERE task_id = $1),
                    (SELECT current_sprint_id FROM sensei.tracker_tasks WHERE id = $1)",
        )
        .bind(task)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!((members, current), (0, None));

        // Deleting the tracker removes its tickets.
        sqlx_core::query::query("DELETE FROM sensei.trackers WHERE id = $1")
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        let (tasks,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.tracker_tasks WHERE tracker_id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(tasks, 0);
        cleanup(&pg, p, &[&s]).await;
    }
}
