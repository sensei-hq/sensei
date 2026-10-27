//! The provider-neutral sync coordinator (spec R5): guard → fetch with no DB
//! connection held → apply in one short transaction → record the outcome.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::trackers::{ApplyCounts, ApplyError};
use crate::tracker::link::known_tasks;
use crate::tracker::{
    BoardScope, GetTransport, KnownTask, Link, SessionText, Snapshot, TrackerAdapter, TrackerError,
    needs_owner_backfill,
};

/// Candidate sessions read per chunk when computing links.
pub const LINK_CHUNK: i64 = 200;

/// Whole-sync deadline for the fetch and link phases.
pub const SYNC_DEADLINE: Duration = Duration::from_secs(10 * 60);

/// One in-process lock per tracker, shared by the API and the scheduled tick.
/// The daemon is the only writer, so this is enough to keep two syncs of one
/// tracker from overlapping, without holding a DB lock across network calls.
#[derive(Default)]
pub struct TrackerLocks {
    inner: std::sync::Mutex<HashMap<uuid::Uuid, Arc<tokio::sync::Mutex<()>>>>,
}

impl TrackerLocks {
    /// The tracker's lock if it is free; `None` while another sync holds it.
    pub fn try_lock(&self, tracker_id: uuid::Uuid) -> Option<tokio::sync::OwnedMutexGuard<()>> {
        let lock = self
            .inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .entry(tracker_id)
            .or_default()
            .clone();
        lock.try_lock_owned().ok()
    }
}

/// Why a sync did not complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// Another sync of this tracker is running.
    Busy,
    /// The tracker no longer exists.
    Gone,
    /// The tracker's credential changed since this sync's token was read (a
    /// re-connect, or its site re-keyed): nothing is recorded; sync again.
    Superseded,
    /// The provider or credential failed; recorded as `last_error`.
    Failed(TrackerError),
    /// The database failed; recorded as `db` when the row is still reachable.
    Db(String),
}

/// Sync one tracker end to end, on whatever credential its row names now.
#[cfg(test)]
pub async fn sync_tracker(
    pg: &PgStore,
    locks: &TrackerLocks,
    tracker_id: uuid::Uuid,
    adapter: &'static dyn TrackerAdapter,
    transport: &dyn GetTransport,
    deadline: Duration,
) -> Result<ApplyCounts, SyncError> {
    run(pg, locks, tracker_id, adapter, transport, deadline, None).await
}

/// [`sync_tracker`] with a transport built from the token `secret_service`
/// names: once the row names another, the sync is superseded and records nothing.
pub async fn sync_with_credential(
    pg: &PgStore,
    locks: &TrackerLocks,
    tracker_id: uuid::Uuid,
    adapter: &'static dyn TrackerAdapter,
    transport: &dyn GetTransport,
    deadline: Duration,
    secret_service: &str,
) -> Result<ApplyCounts, SyncError> {
    run(pg, locks, tracker_id, adapter, transport, deadline, Some(secret_service)).await
}

async fn run(
    pg: &PgStore,
    locks: &TrackerLocks,
    tracker_id: uuid::Uuid,
    adapter: &'static dyn TrackerAdapter,
    transport: &dyn GetTransport,
    deadline: Duration,
    credential: Option<&str>,
) -> Result<ApplyCounts, SyncError> {
    // 1. Guard.
    let Some(_guard) = locks.try_lock(tracker_id) else {
        return Err(SyncError::Busy);
    };
    let row = pg.tracker_by_id(tracker_id).await.map_err(SyncError::Db)?.ok_or(SyncError::Gone)?;
    if credential.is_some_and(|c| c != row.secret_service) {
        return Err(SyncError::Superseded);
    }
    pg.mark_tracker_attempt(tracker_id).await.map_err(SyncError::Db)?;
    let prev = pg.tracker_prev_state(tracker_id).await.map_err(SyncError::Db)?;
    // The whole-sync deadline covers the owner lookup and steps 2-3.
    let until = tokio::time::Instant::now() + deadline;
    if needs_owner_backfill(row.owner_ref.as_ref().map(|o| o.as_str()), row.owner_state.as_deref())
    {
        backfill_owner(pg, tracker_id, adapter, transport, &row.secret_service, until).await;
    }

    // 2. Fetch, with no connection checked out, under the whole-sync deadline
    // (shared with step 3). Every write below is conditional on this credential
    // still being current.
    let secret_service = row.secret_service;
    let board = BoardScope { base_url: row.base_url, board_ref: row.board_ref };
    let snapshot = match tokio::time::timeout_at(
        until,
        adapter.fetch_snapshot(transport, &board, &prev),
    )
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(record_failure(pg, tracker_id, &secret_service, e).await),
        Err(_) => {
            return Err(
                record_failure(pg, tracker_id, &secret_service, TrackerError::Transport).await
            );
        }
    };

    // 3. Compute links, with no transaction open, under the same deadline.
    let links =
        match tokio::time::timeout_at(until, compute_links(pg, tracker_id, adapter, &snapshot))
            .await
        {
            Ok(Ok(links)) => links,
            Ok(Err(e)) => return Err(record_db(pg, tracker_id, &secret_service, e).await),
            Err(_) => {
                return Err(record_failure(
                    pg,
                    tracker_id,
                    &secret_service,
                    TrackerError::Transport,
                )
                .await);
            }
        };
    // 4. Apply in one short transaction; success is recorded inside it.
    let patch = match &snapshot.board_name {
        Some(name) => serde_json::json!({ "board_name": name }),
        None => serde_json::json!({}),
    };
    match pg.apply_tracker_snapshot(tracker_id, &secret_service, &snapshot, &patch, &links).await {
        Ok(counts) => Ok(counts),
        Err(ApplyError::Gone) => Err(SyncError::Gone),
        Err(ApplyError::Superseded) => Err(SyncError::Superseded),
        Err(ApplyError::Db(e)) => Err(record_db(pg, tracker_id, &secret_service, e).await),
    }
}

/// Who the token belongs to, for a tracker with no owner on record (spec W1). Never
/// fails the sync: a failed lookup is recorded as `owner_error`, and a failed store
/// is only logged, leaving `assigned_to_me` NULL for this sync.
async fn backfill_owner(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    adapter: &dyn TrackerAdapter,
    transport: &dyn GetTransport,
    secret_service: &str,
    until: tokio::time::Instant,
) {
    let looked = tokio::time::timeout_at(until, adapter.owner(transport))
        .await
        .unwrap_or(Err(TrackerError::Transport));
    let stored = match &looked {
        Ok(owner) => pg.set_tracker_owner(tracker_id, secret_service, owner).await,
        Err(e) => pg.set_tracker_owner_error(tracker_id, secret_service, e.code()).await,
    };
    if let Err(e) = looked {
        tracing::warn!(tracker_id = %tracker_id, code = e.code(), "tracker sync: owner lookup failed");
    }
    if let Err(db) = stored {
        tracing::warn!(tracker_id = %tracker_id, "tracker sync: could not record the owner lookup: {db}");
    }
}

/// Step 3: every candidate session's links to the known keys, read in chunks and
/// matched by the adapter on a blocking thread.
async fn compute_links(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    adapter: &'static dyn TrackerAdapter,
    snapshot: &Snapshot,
) -> Result<Vec<Link>, String> {
    let aliases = pg.tracker_key_aliases(tracker_id).await?;
    let known = Arc::new(known_tasks(&aliases, &snapshot.tasks));
    links_to(pg, tracker_id, adapter, known, None).await
}

/// Every candidate session's links to `known` (R6 matching), optionally only the
/// sessions with turns ingested after a time.
pub(crate) async fn links_to(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    adapter: &'static dyn TrackerAdapter,
    known: Arc<Vec<KnownTask>>,
    ingested_after: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<Link>, String> {
    if known.is_empty() {
        return Ok(Vec::new());
    }
    scan_sessions(pg, tracker_id, ingested_after, move |s| adapter.match_links(s, &known)).await
}

/// Run a pure per-session function over the tracker's candidate sessions, read
/// in chunks of [`LINK_CHUNK`] (R5 step 3) and evaluated on a blocking thread.
pub(crate) async fn scan_sessions<T, F>(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    ingested_after: Option<chrono::DateTime<chrono::Utc>>,
    per_session: F,
) -> Result<Vec<T>, String>
where
    T: Send + 'static,
    F: Fn(&SessionText) -> Vec<T> + Send + Sync + 'static,
{
    let per_session = Arc::new(per_session);
    let mut out = Vec::new();
    let mut after = None;
    loop {
        let chunk = pg.load_link_inputs(tracker_id, after, LINK_CHUNK, ingested_after).await?;
        let Some(last) = chunk.last() else { break };
        after = Some(last.session_id);
        let full = chunk.len() as i64 == LINK_CHUNK;
        let f = per_session.clone();
        let found = tokio::task::spawn_blocking(move || {
            chunk.iter().flat_map(|s| f(s)).collect::<Vec<_>>()
        })
        .await
        .map_err(|e| e.to_string())?;
        out.extend(found);
        if !full {
            break;
        }
    }
    Ok(out)
}

/// Step 5 for a database failure: recorded as `db`, conditional like the others.
async fn record_db(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    secret_service: &str,
    e: String,
) -> SyncError {
    if let Err(db) =
        pg.mark_tracker_error(tracker_id, secret_service, TrackerError::Db.code()).await
    {
        tracing::warn!(tracker_id = %tracker_id, "tracker sync: could not record db failure: {db}");
    }
    SyncError::Db(e)
}

/// Step 5: record a provider/credential failure. A row deleted meanwhile makes the
/// update a no-op, which is what a gone tracker should get.
async fn record_failure(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    secret_service: &str,
    e: TrackerError,
) -> SyncError {
    if let Err(db) = pg.mark_tracker_error(tracker_id, secret_service, e.code()).await {
        tracing::warn!(tracker_id = %tracker_id, code = e.code(), "tracker sync: could not record failure: {db}");
    }
    SyncError::Failed(e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::connect::tests::connected_tracker;
    use crate::tracker::jira::JiraAdapter;
    use crate::tracker::test_support::FakeBoard;
    use serde_json::json;

    async fn rows(
        pg: &PgStore,
        tracker: uuid::Uuid,
    ) -> Vec<(String, String, Option<String>, Option<bool>)> {
        // (kind, external_id, key/state, removed) — everything but last_seen_at.
        sqlx_core::query_as::query_as(
            "SELECT 'task', external_id, key || '|' || coalesce(summary,'') || '|' || coalesce(current_sprint_id::text,''),
                    removed_at IS NOT NULL
               FROM sensei.tracker_tasks WHERE tracker_id = $1
             UNION ALL
             SELECT 'sprint', external_id, state || '|' || coalesce(members_fetched_state,''), removed_at IS NOT NULL
               FROM sensei.sprints WHERE tracker_id = $1
             UNION ALL
             SELECT 'member', t.external_id, s.external_id, NULL
               FROM sensei.tracker_task_sprints m
               JOIN sensei.tracker_tasks t ON t.id = m.task_id
               JOIN sensei.sprints s ON s.id = m.sprint_id
              WHERE t.tracker_id = $1
             ORDER BY 1, 2, 3",
        )
        .bind(tracker)
        .fetch_all(pg.pool())
        .await
        .unwrap()
    }

    async fn current_sprint_of(pg: &PgStore, tracker: uuid::Uuid, issue: &str) -> Option<String> {
        sqlx_core::query_as::query_as::<_, (Option<String>,)>(
            "SELECT s.external_id FROM sensei.tracker_tasks t
               LEFT JOIN sensei.sprints s ON s.id = t.current_sprint_id
              WHERE t.tracker_id = $1 AND t.external_id = $2",
        )
        .bind(tracker)
        .bind(issue)
        .fetch_one(pg.pool())
        .await
        .unwrap()
        .0
    }

    async fn sync(
        pg: &PgStore,
        t: uuid::Uuid,
        board: &FakeBoard,
    ) -> Result<ApplyCounts, SyncError> {
        sync_tracker(pg, &TrackerLocks::default(), t, &JiraAdapter, board, SYNC_DEADLINE).await
    }

    #[tokio::test]
    async fn a_first_sync_mirrors_the_board() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let counts = sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        assert_eq!(
            counts,
            ApplyCounts { sprints: 2, tasks: 3, removed: 0, links: 0, skipped_members: 0 }
        );
        assert_eq!(current_sprint_of(&pg, t, "10001").await.as_deref(), Some("100"));
        assert_eq!(current_sprint_of(&pg, t, "10002").await.as_deref(), Some("101"));
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        assert!(row.last_success_at.is_some() && row.last_error.is_none());
        assert_eq!(row.params["board_name"], json!("AUTH Scrum"));
    }

    #[tokio::test]
    async fn a_second_identical_sync_changes_nothing_but_last_seen() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let before = rows(&pg, t).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        assert_eq!(rows(&pg, t).await, before);
    }

    #[tokio::test]
    async fn a_missing_issue_is_marked_removed_once() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let without_3 = || {
            FakeBoard::standard().with(
                "/rest/agile/1.0/board/42/issue",
                2,
                json!({"startAt": 2, "total": 2, "issues": []}),
            )
        };
        let counts = sync(&pg, t, &without_3()).await.unwrap();
        assert_eq!(counts.removed, 1);
        let removed_at = || async {
            sqlx_core::query_as::query_as::<_, (Option<chrono::DateTime<chrono::Utc>>,)>(
                "SELECT removed_at FROM sensei.tracker_tasks WHERE tracker_id = $1 AND external_id = '10003'",
            )
            .bind(t)
            .fetch_one(pg.pool())
            .await
            .unwrap()
            .0
        };
        let first = removed_at().await.expect("removed_at set");
        sync(&pg, t, &without_3()).await.unwrap();
        assert_eq!(removed_at().await, Some(first), "removed_at is never advanced");
    }

    #[tokio::test]
    async fn a_renamed_key_updates_the_same_row() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let renamed = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/issue",
            2,
            json!({"startAt": 2, "total": 3, "issues": [{"id": "10003", "key": "PLAT-9", "fields": {}}]}),
        );
        sync(&pg, t, &renamed).await.unwrap();
        let (n, key): (i64, String) = sqlx_core::query_as::query_as(
            "SELECT count(*), max(key) FROM sensei.tracker_tasks WHERE tracker_id = $1 AND external_id = '10003'",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!((n, key.as_str()), (1, "PLAT-9"));
    }

    #[tokio::test]
    async fn a_failed_page_changes_no_rows_and_records_the_error() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let before = rows(&pg, t).await;
        let ok_at = pg.tracker_by_id(t).await.unwrap().unwrap().last_success_at;
        let broken = FakeBoard::standard().fail_page(
            "/rest/agile/1.0/board/42/issue",
            2,
            TrackerError::HttpError,
        );
        assert_eq!(sync(&pg, t, &broken).await, Err(SyncError::Failed(TrackerError::HttpError)));
        assert_eq!(rows(&pg, t).await, before);
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        assert_eq!((row.last_error.as_deref(), row.last_success_at), (Some("http_error"), ok_at));
    }

    #[tokio::test]
    async fn a_removed_sprint_drops_its_members_and_skipped_members_are_counted() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        // 10001 sits in closed 100 and future 102.
        let with_future = FakeBoard::standard()
            .with("/rest/agile/1.0/board/42/sprint", 0, json!({"isLast": true, "values": [
                {"id": 100, "name": "Sprint 1", "state": "closed", "startDate": "2026-09-01T00:00:00.000Z"},
                {"id": 102, "name": "Sprint 3", "state": "future"}]}))
            .with("/rest/agile/1.0/board/42/sprint/102/issue", 0, json!({"total": 2, "issues": [
                {"id": "10001", "key": "AUTH-1", "fields": {}},
                {"id": "99999", "key": "OTHER-1", "fields": {}}]}));
        let counts = sync(&pg, t, &with_future).await.unwrap();
        assert_eq!(
            counts.skipped_members, 1,
            "a member that is not a board issue is skipped, not inserted"
        );
        assert_eq!(current_sprint_of(&pg, t, "10001").await.as_deref(), Some("102"));
        // 102 is deleted in Jira.
        let without = FakeBoard::standard().with("/rest/agile/1.0/board/42/sprint", 0, json!({"isLast": true, "values": [
            {"id": 100, "name": "Sprint 1", "state": "closed", "startDate": "2026-09-01T00:00:00.000Z"}]}));
        sync(&pg, t, &without).await.unwrap();
        assert_eq!(current_sprint_of(&pg, t, "10001").await.as_deref(), Some("100"));
        let (members,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.tracker_task_sprints m JOIN sensei.sprints s ON s.id = m.sprint_id
              WHERE s.tracker_id = $1 AND s.external_id = '102'",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(members, 0);
    }

    #[tokio::test]
    async fn a_tracker_deleted_mid_sync_is_gone() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let pg2 = pg.clone();
        let board = FakeBoard::standard().on_call(Box::new(move |_path| {
            let pg = pg2.clone();
            Box::pin(async move {
                pg.delete_tracker(p, t).await.unwrap();
            })
        }));
        assert_eq!(sync(&pg, t, &board).await, Err(SyncError::Gone));
    }

    #[tokio::test]
    async fn a_held_lock_is_busy() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let locks = TrackerLocks::default();
        let _held = locks.try_lock(t).unwrap();
        let r =
            sync_tracker(&pg, &locks, t, &JiraAdapter, &FakeBoard::standard(), SYNC_DEADLINE).await;
        assert_eq!(r, Err(SyncError::Busy));
        assert!(
            pg.tracker_by_id(t).await.unwrap().unwrap().last_attempt_at.is_none(),
            "busy records nothing"
        );
    }

    #[tokio::test]
    async fn no_db_connection_is_held_while_fetching() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let in_use = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (pg2, seen) = (pg.clone(), in_use.clone());
        let board = FakeBoard::standard().on_call(Box::new(move |_path| {
            let (pool, seen) = (pg2.pool().clone(), seen.clone());
            Box::pin(async move {
                // sqlx returns a dropped connection to the pool in a spawned task;
                // a real request yields long enough for it, so wait (bounded) for
                // the pool to settle. A connection genuinely held never settles.
                let in_use = || pool.size() as usize - pool.num_idle();
                for _ in 0..200 {
                    if in_use() == 0 {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                }
                seen.lock().unwrap().push(in_use());
            })
        }));
        sync(&pg, t, &board).await.unwrap();
        let seen = in_use.lock().unwrap().clone();
        assert!(
            !seen.is_empty() && seen.iter().all(|n| *n == 0),
            "connections in use during fetch: {seen:?}"
        );
    }

    #[tokio::test]
    async fn overrunning_the_deadline_is_transport() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let slow = FakeBoard::standard().delay(Duration::from_secs(5));
        let r = sync_tracker(
            &pg,
            &TrackerLocks::default(),
            t,
            &JiraAdapter,
            &slow,
            Duration::from_millis(100),
        )
        .await;
        assert_eq!(r, Err(SyncError::Failed(TrackerError::Transport)));
        assert_eq!(
            pg.tracker_by_id(t).await.unwrap().unwrap().last_error.as_deref(),
            Some("transport")
        );
    }

    /// The standard board without sprint 100 (10001 then sits in no sprint).
    fn without_sprint_100() -> FakeBoard {
        FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/sprint",
            0,
            json!({"isLast": true, "values": [
                {"id": 101, "name": "Sprint 2", "state": "active", "startDate": "2026-10-01T00:00:00.000Z"}]}),
        )
    }

    async fn members_of(pg: &PgStore, tracker: uuid::Uuid, sprint: &str) -> i64 {
        sqlx_core::query_as::query_as::<_, (i64,)>(
            "SELECT count(*) FROM sensei.tracker_task_sprints m JOIN sensei.sprints s ON s.id = m.sprint_id
              WHERE s.tracker_id = $1 AND s.external_id = $2",
        )
        .bind(tracker)
        .bind(sprint)
        .fetch_one(pg.pool())
        .await
        .unwrap()
        .0
    }

    #[tokio::test]
    async fn a_closed_sprint_that_returns_regains_its_members() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        // Two syncs: sprint 100 is closed and its members were fetched while closed.
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        // It disappears (removed, memberships deleted), then comes back closed.
        sync(&pg, t, &without_sprint_100()).await.unwrap();
        assert_eq!(members_of(&pg, t, "100").await, 0);
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        assert_eq!(
            members_of(&pg, t, "100").await,
            1,
            "a returning closed sprint is fetched again"
        );
        assert_eq!(current_sprint_of(&pg, t, "10001").await.as_deref(), Some("100"));
    }

    #[tokio::test]
    async fn a_missing_sprint_is_marked_removed_once() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        sync(&pg, t, &without_sprint_100()).await.unwrap();
        let removed_at = || async {
            sqlx_core::query_as::query_as::<_, (Option<chrono::DateTime<chrono::Utc>>,)>(
                "SELECT removed_at FROM sensei.sprints WHERE tracker_id = $1 AND external_id = '100'",
            )
            .bind(t)
            .fetch_one(pg.pool())
            .await
            .unwrap()
            .0
        };
        let first = removed_at().await.expect("removed_at set");
        sync(&pg, t, &without_sprint_100()).await.unwrap();
        assert_eq!(removed_at().await, Some(first), "a sprint's removed_at is never advanced");
    }

    /// A second, non-Jira adapter: GitLab-like `#12` keys relative to a project
    /// scope, and iteration-named sprints. The coordinator must handle it unchanged
    /// (gate 1l). Leaked so it is `'static`, like a registered adapter.
    struct FakeAdapter {
        scopes: [String; 2],
    }

    fn fake_adapter(scopes: [String; 2]) -> &'static FakeAdapter {
        Box::leak(Box::new(FakeAdapter { scopes }))
    }

    #[async_trait::async_trait]
    impl crate::tracker::TrackerAdapter for FakeAdapter {
        fn kind(&self) -> crate::tracker::TrackerKind {
            crate::tracker::TrackerKind::Jira
        }
        fn connect_params(
            &self,
            _body: &serde_json::Value,
        ) -> Result<crate::tracker::ConnectParams, crate::tracker::ParamError> {
            Err(crate::tracker::ParamError::InvalidBaseUrl)
        }
        fn auth(
            &self,
            account: &str,
            token: &crate::tracker::ApiToken,
        ) -> crate::tracker::AuthScheme {
            crate::tracker::AuthScheme::Basic { user: account.into(), token: token.clone() }
        }
        async fn owner(
            &self,
            _t: &dyn GetTransport,
        ) -> Result<crate::tracker::OwnerLookup, TrackerError> {
            Ok(crate::tracker::OwnerLookup::Unknown)
        }
        async fn find_comment(
            &self,
            _t: &dyn GetTransport,
            _task: &str,
            _owner: &crate::tracker::AccountRef,
        ) -> Result<Option<String>, TrackerError> {
            Err(TrackerError::UnsupportedTracker)
        }
        async fn create_comment(
            &self,
            _w: &dyn crate::tracker::WriteTransport,
            _task: &str,
            _report: &crate::tracker::writeback::Report,
        ) -> Result<String, TrackerError> {
            Err(TrackerError::UnsupportedTracker)
        }
        async fn update_comment(
            &self,
            _w: &dyn crate::tracker::WriteTransport,
            _task: &str,
            _comment: &str,
            _report: &crate::tracker::writeback::Report,
        ) -> Result<(), TrackerError> {
            Err(TrackerError::UnsupportedTracker)
        }
        async fn issue_exists(
            &self,
            _t: &dyn GetTransport,
            _task: &str,
        ) -> Result<bool, TrackerError> {
            Err(TrackerError::UnsupportedTracker)
        }
        fn field_ref_shape(&self, _f: &str) -> bool {
            false
        }
        async fn check_fields(
            &self,
            _t: &dyn GetTransport,
            _f: &[String],
        ) -> Result<Vec<crate::tracker::FieldCheck>, TrackerError> {
            Err(TrackerError::UnsupportedTracker)
        }
        async fn field_in_use(
            &self,
            _t: &dyn GetTransport,
            _b: &BoardScope,
            _f: &str,
        ) -> Result<bool, TrackerError> {
            Err(TrackerError::UnsupportedTracker)
        }
        async fn write_fields(
            &self,
            _w: &dyn crate::tracker::WriteTransport,
            _task: &str,
            _f: &[(String, crate::tracker::FieldValue)],
        ) -> Result<(), TrackerError> {
            Err(TrackerError::UnsupportedTracker)
        }
        async fn validate(
            &self,
            _t: &dyn GetTransport,
            _p: &crate::tracker::ConnectParams,
        ) -> Result<crate::tracker::Validated, crate::tracker::ValidateError> {
            Err(crate::tracker::ValidateError::BoardNotFound)
        }
        async fn fetch_snapshot(
            &self,
            _t: &dyn GetTransport,
            _board: &BoardScope,
            _prev: &crate::tracker::PrevState,
        ) -> Result<crate::tracker::Snapshot, TrackerError> {
            use crate::tracker::sprint::SprintState;
            use crate::tracker::{Snapshot, SprintRow, TaskRow};
            let task = |id: &str, key: &str, scope: &str| TaskRow {
                external_id: id.into(),
                key: key.into(),
                scope: Some(scope.into()),
                summary: None,
                issue_type: None,
                status: Some("opened".into()),
                assignee: None,
            };
            let iteration = |id: &str, seq: i64, state| SprintRow {
                external_id: id.into(),
                seq,
                name: format!("Iteration {seq}"),
                state,
                start_at: None,
                end_at: None,
                complete_at: None,
            };
            Ok(Snapshot {
                board_name: Some("Group board".into()),
                sprints: vec![iteration("gid://iteration/7", 7, SprintState::Active)],
                tasks: vec![
                    task("gid://issue/1", "#12", &self.scopes[0]),
                    task("gid://issue/2", "#12", &self.scopes[1]),
                ],
                members: vec![("gid://iteration/7".into(), Some(vec!["gid://issue/1".into()]))],
            })
        }
        /// `#N` in a prompt, for the task of that key whose scope is the session's repo.
        fn match_links(
            &self,
            session: &crate::tracker::SessionText,
            known: &[crate::tracker::KnownTask],
        ) -> Vec<crate::tracker::Link> {
            known
                .iter()
                .filter(|k| k.scope.is_some() && k.scope == session.repo_key)
                .filter_map(|k| {
                    let n: usize = session.prompts.iter().map(|p| p.matches(&k.key).count()).sum();
                    (n > 0).then(|| crate::tracker::Link {
                        session_id: session.session_id,
                        task_external_id: k.external_id.clone(),
                        source: crate::tracker::LinkSource::Prompt,
                        matched_key: k.key.clone(),
                        mentions: n as u32,
                    })
                })
                .collect()
        }
        fn unknown_keys(
            &self,
            _session: &crate::tracker::SessionText,
            _known: &[crate::tracker::KnownTask],
        ) -> Vec<String> {
            Vec::new()
        }
        async fn fetch_tasks(
            &self,
            _t: &dyn GetTransport,
            _board: &BoardScope,
            _keys: &[String],
        ) -> Result<crate::tracker::Discovered, TrackerError> {
            Ok(Default::default())
        }
    }

    #[tokio::test]
    async fn the_coordinator_handles_a_non_jira_adapter_unchanged() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let counts = sync_tracker(
            &pg,
            &TrackerLocks::default(),
            t,
            fake_adapter(["gitlab.com/group/a".into(), "gitlab.com/group/b".into()]),
            &FakeBoard::empty(),
            SYNC_DEADLINE,
        )
        .await
        .unwrap();
        assert_eq!((counts.sprints, counts.tasks, counts.skipped_members), (1, 2, 0));
        let (key, scope, sprint): (String, Option<String>, Option<String>) =
            sqlx_core::query_as::query_as(
                "SELECT t.key, t.scope, s.external_id FROM sensei.tracker_tasks t
               LEFT JOIN sensei.sprints s ON s.id = t.current_sprint_id
              WHERE t.tracker_id = $1 AND t.external_id = 'gid://issue/1'",
            )
            .bind(t)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(key, "#12", "the adapter owns key form; the coordinator stores it as given");
        assert_eq!(scope.as_deref(), Some("gitlab.com/group/a"));
        assert_eq!(sprint.as_deref(), Some("gid://iteration/7"));
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        assert_eq!(row.params["board_name"], json!("Group board"));
    }

    /// Simulate a re-connect landing mid-sync: swap the secret and reset status.
    fn reconnect_mid_sync(pg: &PgStore, t: uuid::Uuid) -> crate::tracker::test_support::Hook {
        let pg = pg.clone();
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        Box::new(move |_path| {
            let (pg, done) = (pg.clone(), done.clone());
            Box::pin(async move {
                if done.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                sqlx_core::query::query(
                    "UPDATE sensei.trackers SET secret_service = secret_service || '.new',
                            last_success_at = NULL, last_error = NULL WHERE id = $1",
                )
                .bind(t)
                .execute(pg.pool())
                .await
                .unwrap();
            })
        })
    }

    #[tokio::test]
    async fn a_sync_whose_transport_predates_a_rekey_is_superseded_and_records_nothing() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let stale = pg.tracker_by_id(t).await.unwrap().unwrap().secret_service;
        sqlx_core::query::query(
            "UPDATE sensei.trackers SET secret_service = secret_service || '.new' WHERE id = $1",
        )
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap();
        let revoked =
            FakeBoard::standard().fail("/rest/agile/1.0/board/42", TrackerError::Unauthorized);
        let r = sync_with_credential(
            &pg,
            &TrackerLocks::default(),
            t,
            &JiraAdapter,
            &revoked,
            SYNC_DEADLINE,
            &stale,
        )
        .await;
        assert_eq!(r, Err(SyncError::Superseded));
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        assert_eq!(row.last_error, None, "the old token's failure is not the new token's");
        assert!(revoked.calls().is_empty(), "nothing fetched on a superseded token");
    }

    #[tokio::test]
    async fn a_sync_on_the_old_credential_does_not_record_success_after_a_reconnect() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let board = FakeBoard::standard().on_call(reconnect_mid_sync(&pg, t));
        let r = sync(&pg, t, &board).await;
        assert_eq!(r, Err(SyncError::Superseded), "a superseded sync is discarded, not gone");
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        assert!(row.last_success_at.is_none(), "the new credential has not synced yet");
    }

    #[tokio::test]
    async fn a_sync_on_the_old_credential_does_not_record_its_failure_after_a_reconnect() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let board = FakeBoard::standard()
            .fail_page("/rest/agile/1.0/board/42/issue", 2, TrackerError::Unauthorized)
            .on_call(reconnect_mid_sync(&pg, t));
        let _ = sync(&pg, t, &board).await;
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        assert_eq!(row.last_error, None, "the old token's 401 must not land on the new connection");
    }

    // ── Owner and assignee (write-back spec W1) ──────────────────────────────

    use crate::tracker::test_support::{assigned, forget_owner, owner_of, rows_containing};

    #[tokio::test]
    async fn a_tracker_without_an_owner_looks_it_up_once_and_an_app_token_is_not_asked_again() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        forget_owner(&pg, t).await;
        let board = FakeBoard::standard().with_myself(json!({"accountType": "app"}));
        sync(&pg, t, &board).await.unwrap();
        sync(&pg, t, &board).await.unwrap();
        assert_eq!(board.myself_calls(), 1, "not_a_person is final");
        assert_eq!(owner_of(&pg, t).await, (None, Some("not_a_person".into()), None));
    }

    #[tokio::test]
    async fn a_tracker_without_an_owner_stores_the_person_and_the_same_sync_assigns_with_it() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        forget_owner(&pg, t).await;
        let board = FakeBoard::standard().with_assignees([Some("A1"), None, Some("B9")]);
        sync(&pg, t, &board).await.unwrap();
        assert_eq!(owner_of(&pg, t).await, (Some("A1".into()), None, None));
        assert_eq!(
            assigned(&pg, t).await,
            vec![
                ("10001".into(), Some(true)),
                ("10002".into(), Some(false)),
                ("10003".into(), Some(false))
            ]
        );
        sync(&pg, t, &board).await.unwrap();
        assert_eq!(board.myself_calls(), 1, "an owner on record is never re-asked");
    }

    #[tokio::test]
    async fn a_failing_owner_lookup_is_recorded_apart_and_never_fails_the_sync() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        forget_owner(&pg, t).await;
        let denied = FakeBoard::standard().fail("/rest/api/3/myself", TrackerError::Forbidden);
        for _ in 0..2 {
            assert!(sync(&pg, t, &denied).await.is_ok());
            let row = pg.tracker_by_id(t).await.unwrap().unwrap();
            assert_eq!(row.last_error, None, "the read sync succeeded");
            assert_eq!(owner_of(&pg, t).await, (None, None, Some("forbidden".into())));
        }
        assert_eq!(denied.myself_calls(), 2, "a failed lookup is retried, once per sync");
        assert!(assigned(&pg, t).await.iter().all(|(_, a)| a.is_none()), "no owner, no answer");
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        assert_eq!(
            owner_of(&pg, t).await,
            (Some("A1".into()), None, None),
            "a success clears the error"
        );
    }

    #[tokio::test]
    async fn an_owner_looked_up_on_a_token_replaced_mid_call_is_not_stored() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        forget_owner(&pg, t).await;
        // The first call of the sync is the owner lookup; the hook re-keys during it.
        let board = FakeBoard::standard().on_call(reconnect_mid_sync(&pg, t));
        let _ = sync(&pg, t, &board).await;
        assert_eq!(board.calls().first().map(String::as_str), Some("/rest/api/3/myself"));
        assert_eq!(owner_of(&pg, t).await.0, None, "the old token's owner never lands");
    }

    #[tokio::test]
    async fn the_owner_lookup_spends_the_sync_deadline() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        forget_owner(&pg, t).await;
        let slow: crate::tracker::test_support::Hook = Box::new(|path| {
            Box::pin(async move {
                if path == "/rest/api/3/myself" {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            })
        });
        let board = FakeBoard::standard().on_call(slow);
        let started = std::time::Instant::now();
        let r = sync_tracker(
            &pg,
            &TrackerLocks::default(),
            t,
            &JiraAdapter,
            &board,
            Duration::from_millis(300),
        )
        .await;
        assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
        assert_eq!(
            r,
            Err(SyncError::Failed(TrackerError::Transport)),
            "the fetch had no time left"
        );
        assert_eq!(owner_of(&pg, t).await.2.as_deref(), Some("transport"));
    }

    #[tokio::test]
    async fn a_failing_owner_store_never_fails_the_sync() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        forget_owner(&pg, t).await;
        // This test's own database: make any owner write fail.
        sqlx_core::query::query(
            "CREATE FUNCTION pg_temp_fail_owner() RETURNS trigger LANGUAGE plpgsql AS
               $$ BEGIN RAISE EXCEPTION 'owner store refused'; END $$",
        )
        .execute(pg.pool())
        .await
        .unwrap();
        sqlx_core::query::query(
            "CREATE TRIGGER fail_owner BEFORE UPDATE OF owner_ref, owner_state, owner_error
               ON sensei.trackers FOR EACH ROW EXECUTE FUNCTION pg_temp_fail_owner()",
        )
        .execute(pg.pool())
        .await
        .unwrap();
        let board = FakeBoard::standard().with_assignees([Some("A1"), None, None]);
        assert!(sync(&pg, t, &board).await.is_ok());
        assert!(
            assigned(&pg, t).await.iter().all(|(_, a)| a.is_none()),
            "no owner stored, no answer"
        );
    }

    #[tokio::test]
    async fn a_ticket_is_mine_only_while_the_owner_is_its_assignee_and_no_assignee_is_stored() {
        let pg = PgStore::test_store().await;
        let (t, _p) = connected_tracker(&pg).await;
        let board =
            FakeBoard::standard().with_assignees([Some("A1"), Some("zz-sentinel-b2"), None]);
        sync(&pg, t, &board).await.unwrap();
        assert_eq!(
            assigned(&pg, t).await,
            vec![
                ("10001".into(), Some(true)),
                ("10002".into(), Some(false)),
                ("10003".into(), Some(false))
            ]
        );
        let reassigned = FakeBoard::standard().with_assignees([
            Some("zz-sentinel-b2"),
            Some("zz-sentinel-b2"),
            None,
        ]);
        sync(&pg, t, &reassigned).await.unwrap();
        assert_eq!(assigned(&pg, t).await[0], ("10001".into(), Some(false)), "reassigned away");
        assert_eq!(rows_containing(&pg, "zz-sentinel-b2").await, Vec::<String>::new());
    }

    // ── Linking (F3, spec R5 step 3 + R6) ───────────────────────────────────

    use crate::tracker::test_support::{repo_in, session_with};

    /// (session, issue external id, source, matched_key, mentions), sorted.
    async fn links(
        pg: &PgStore,
        tracker: uuid::Uuid,
    ) -> Vec<(uuid::Uuid, String, String, String, i32)> {
        sqlx_core::query_as::query_as(
            "SELECT l.session_id, t.external_id, l.source::text, l.matched_key, l.mentions
               FROM sensei.session_task_links l JOIN sensei.tracker_tasks t ON t.id = l.task_id
              WHERE t.tracker_id = $1 ORDER BY 1, 2, 3",
        )
        .bind(tracker)
        .fetch_all(pg.pool())
        .await
        .unwrap()
    }

    fn link(
        s: uuid::Uuid,
        issue: &str,
        source: &str,
        key: &str,
        n: i32,
    ) -> (uuid::Uuid, String, String, String, i32) {
        (s, issue.into(), source.into(), key.into(), n)
    }

    fn sorted<T: Ord>(mut v: Vec<T>) -> Vec<T> {
        v.sort();
        v
    }

    #[tokio::test]
    async fn sessions_from_every_workspace_repo_link_by_branch_and_prompt() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let (r1, r2) = (repo_in(&pg, p).await, repo_in(&pg, p).await);
        let elsewhere = repo_in(&pg, crate::tracker::connect::tests::workspace(&pg).await).await;
        let on_branch = session_with(&pg, Some(r1), &["fix/auth-2-login"], &[]).await;
        let in_prompts =
            session_with(&pg, Some(r2), &["main"], &["look at AUTH-3", "AUTH-3 again"]).await;
        let other_workspace = session_with(&pg, Some(elsewhere), &["fix/AUTH-2"], &[]).await;
        let no_anchor = session_with(&pg, None, &["fix/AUTH-2"], &[]).await;
        let unknown_key = session_with(&pg, Some(r1), &[], &["UTF-8 and AUTH-99"]).await;
        let counts = sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        assert_eq!(counts.links, 2);
        assert_eq!(
            links(&pg, t).await,
            sorted(vec![
                link(on_branch, "10002", "branch", "AUTH-2", 1),
                link(in_prompts, "10003", "prompt", "AUTH-3", 2),
            ]),
            "assistant text never links; {other_workspace} {no_anchor} {unknown_key} are not candidates or match nothing"
        );
    }

    #[tokio::test]
    async fn relinking_is_a_full_idempotent_recompute() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let r = repo_in(&pg, p).await;
        let s = session_with(&pg, Some(r), &["fix/AUTH-2"], &[]).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let first = links(&pg, t).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        assert_eq!(links(&pg, t).await, first, "a second sync relinks the same");
        sqlx_core::query::query(
            "UPDATE activity.transcript_turns SET git_branch = 'main'
              WHERE session_id = (SELECT client_session_id FROM activity.sessions WHERE id = $1)",
        )
        .bind(s)
        .execute(pg.pool())
        .await
        .unwrap();
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        assert_eq!(links(&pg, t).await, vec![], "a link the text no longer supports is dropped");
    }

    #[tokio::test]
    async fn a_removed_issue_keeps_its_links() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let s = session_with(&pg, Some(repo_in(&pg, p).await), &["fix/AUTH-3"], &[]).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let without_3 = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/issue",
            2,
            json!({"startAt": 2, "total": 2, "issues": []}),
        );
        assert_eq!(sync(&pg, t, &without_3).await.unwrap().removed, 1);
        assert_eq!(links(&pg, t).await, vec![link(s, "10003", "branch", "AUTH-3", 1)]);
    }

    #[tokio::test]
    async fn a_renamed_issue_links_by_its_old_and_new_keys() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let r = repo_in(&pg, p).await;
        let old = session_with(&pg, Some(r), &["fix/auth-3"], &[]).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let renamed = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/issue",
            2,
            json!({"startAt": 2, "total": 3, "issues": [{"id": "10003", "key": "PLAT-9", "fields": {}}]}),
        );
        let new = session_with(&pg, Some(r), &[], &["PLAT-9"]).await;
        sync(&pg, t, &renamed).await.unwrap();
        assert_eq!(
            links(&pg, t).await,
            sorted(vec![
                link(old, "10003", "branch", "AUTH-3", 1),
                link(new, "10003", "prompt", "PLAT-9", 1),
            ])
        );
        let keys: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT k.key FROM sensei.tracker_task_keys k JOIN sensei.tracker_tasks t ON t.id = k.task_id
              WHERE t.tracker_id = $1 AND t.external_id = '10003' ORDER BY 1",
        )
        .bind(t)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(keys, vec![("AUTH-3".into(),), ("PLAT-9".into(),)]);
    }

    #[tokio::test]
    async fn candidates_are_read_in_chunks() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let r = repo_in(&pg, p).await;
        for _ in 0..205 {
            session_with(&pg, Some(r), &["AUTH-2"], &[]).await;
        }
        assert_eq!(sync(&pg, t, &FakeBoard::standard()).await.unwrap().links, 205);
    }

    #[tokio::test]
    async fn a_session_pruned_before_apply_is_skipped() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let live = session_with(&pg, Some(repo_in(&pg, p).await), &[], &[]).await;
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        let snapshot = JiraAdapter
            .fetch_snapshot(
                &FakeBoard::standard(),
                &BoardScope { base_url: row.base_url, board_ref: row.board_ref },
                &Default::default(),
            )
            .await
            .unwrap();
        let to = |s| crate::tracker::Link {
            session_id: s,
            task_external_id: "10002".into(),
            source: crate::tracker::LinkSource::Prompt,
            matched_key: "AUTH-2".into(),
            mentions: 1,
        };
        let counts = pg
            .apply_tracker_snapshot(
                t,
                &row.secret_service,
                &snapshot,
                &json!({}),
                &[to(live), to(uuid::Uuid::new_v4())],
            )
            .await
            .expect("no FK error");
        assert_eq!(counts.links, 1);
        assert_eq!(links(&pg, t).await, vec![link(live, "10002", "prompt", "AUTH-2", 1)]);
    }

    /// Jira, but every link it produces breaks the `mentions > 0` check.
    struct BrokenLinks;

    #[async_trait::async_trait]
    impl crate::tracker::TrackerAdapter for BrokenLinks {
        fn kind(&self) -> crate::tracker::TrackerKind {
            JiraAdapter.kind()
        }
        fn connect_params(
            &self,
            body: &serde_json::Value,
        ) -> Result<crate::tracker::ConnectParams, crate::tracker::ParamError> {
            JiraAdapter.connect_params(body)
        }
        fn auth(
            &self,
            account: &str,
            token: &crate::tracker::ApiToken,
        ) -> crate::tracker::AuthScheme {
            JiraAdapter.auth(account, token)
        }
        async fn owner(
            &self,
            t: &dyn GetTransport,
        ) -> Result<crate::tracker::OwnerLookup, TrackerError> {
            JiraAdapter.owner(t).await
        }
        async fn find_comment(
            &self,
            t: &dyn GetTransport,
            task: &str,
            owner: &crate::tracker::AccountRef,
        ) -> Result<Option<String>, TrackerError> {
            JiraAdapter.find_comment(t, task, owner).await
        }
        async fn create_comment(
            &self,
            w: &dyn crate::tracker::WriteTransport,
            task: &str,
            report: &crate::tracker::writeback::Report,
        ) -> Result<String, TrackerError> {
            JiraAdapter.create_comment(w, task, report).await
        }
        async fn update_comment(
            &self,
            w: &dyn crate::tracker::WriteTransport,
            task: &str,
            comment: &str,
            report: &crate::tracker::writeback::Report,
        ) -> Result<(), TrackerError> {
            JiraAdapter.update_comment(w, task, comment, report).await
        }
        async fn issue_exists(
            &self,
            t: &dyn GetTransport,
            task: &str,
        ) -> Result<bool, TrackerError> {
            JiraAdapter.issue_exists(t, task).await
        }
        fn field_ref_shape(&self, f: &str) -> bool {
            JiraAdapter.field_ref_shape(f)
        }
        async fn check_fields(
            &self,
            t: &dyn GetTransport,
            f: &[String],
        ) -> Result<Vec<crate::tracker::FieldCheck>, TrackerError> {
            JiraAdapter.check_fields(t, f).await
        }
        async fn field_in_use(
            &self,
            t: &dyn GetTransport,
            b: &BoardScope,
            f: &str,
        ) -> Result<bool, TrackerError> {
            JiraAdapter.field_in_use(t, b, f).await
        }
        async fn write_fields(
            &self,
            w: &dyn crate::tracker::WriteTransport,
            task: &str,
            f: &[(String, crate::tracker::FieldValue)],
        ) -> Result<(), TrackerError> {
            JiraAdapter.write_fields(w, task, f).await
        }
        async fn validate(
            &self,
            t: &dyn GetTransport,
            p: &crate::tracker::ConnectParams,
        ) -> Result<crate::tracker::Validated, crate::tracker::ValidateError> {
            JiraAdapter.validate(t, p).await
        }
        async fn fetch_snapshot(
            &self,
            t: &dyn GetTransport,
            board: &BoardScope,
            prev: &crate::tracker::PrevState,
        ) -> Result<crate::tracker::Snapshot, TrackerError> {
            JiraAdapter.fetch_snapshot(t, board, prev).await
        }
        fn match_links(
            &self,
            session: &crate::tracker::SessionText,
            known: &[crate::tracker::KnownTask],
        ) -> Vec<crate::tracker::Link> {
            JiraAdapter
                .match_links(session, known)
                .into_iter()
                .map(|l| crate::tracker::Link { mentions: 0, ..l })
                .collect()
        }
        fn unknown_keys(
            &self,
            session: &crate::tracker::SessionText,
            known: &[crate::tracker::KnownTask],
        ) -> Vec<String> {
            JiraAdapter.unknown_keys(session, known)
        }
        async fn fetch_tasks(
            &self,
            t: &dyn GetTransport,
            board: &BoardScope,
            keys: &[String],
        ) -> Result<crate::tracker::Discovered, TrackerError> {
            JiraAdapter.fetch_tasks(t, board, keys).await
        }
    }

    #[tokio::test]
    async fn a_database_failure_changes_no_rows_and_is_recorded_as_db() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        session_with(&pg, Some(repo_in(&pg, p).await), &["fix/AUTH-2"], &[]).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let (before, before_links) = (rows(&pg, t).await, links(&pg, t).await);
        let r = sync_tracker(
            &pg,
            &TrackerLocks::default(),
            t,
            &BrokenLinks,
            &FakeBoard::standard(),
            SYNC_DEADLINE,
        )
        .await;
        assert!(matches!(r, Err(SyncError::Db(_))), "{r:?}");
        assert_eq!((rows(&pg, t).await, links(&pg, t).await), (before, before_links));
        assert_eq!(pg.tracker_by_id(t).await.unwrap().unwrap().last_error.as_deref(), Some("db"));
    }

    #[tokio::test]
    async fn a_non_jira_adapter_links_by_repo_scope() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let n = uuid::Uuid::new_v4().simple().to_string();
        // repo_key goes through the production normaliser, from an SSH remote; the
        // adapter's scope comes from the HTTPS form of the same project.
        let key = |remote: String| crate::db::pg_store::normalize_repo_key(&remote).unwrap();
        let scope_a = key(format!("https://gitlab.com/Group/Sub/A{n}"));
        let scope_b = key(format!("https://gitlab.com/Group/Sub/B{n}"));
        let with_remote = |remote: Option<String>| {
            let pg = pg.clone();
            async move {
                let folder = repo_in(&pg, p).await;
                sqlx_core::query::query(
                    "UPDATE sensei.repositories SET repo_key = $2
                      WHERE id = (SELECT repository_id FROM sensei.folders WHERE id = $1)",
                )
                .bind(folder)
                .bind(remote.map(key))
                .execute(pg.pool())
                .await
                .unwrap();
                folder
            }
        };
        let in_a = session_with(
            &pg,
            Some(with_remote(Some(format!("git@gitlab.com:Group/Sub/A{n}.git"))).await),
            &[],
            &["#12"],
        )
        .await;
        let in_b = session_with(
            &pg,
            Some(with_remote(Some(format!("git@gitlab.com:Group/Sub/B{n}.git"))).await),
            &[],
            &["#12 #12"],
        )
        .await;
        session_with(
            &pg,
            Some(with_remote(Some(format!("git@gitlab.com:Group/Sub/C{n}.git"))).await),
            &[],
            &["#12"],
        )
        .await;
        session_with(&pg, Some(with_remote(None).await), &[], &["#12"]).await;
        let counts = sync_tracker(
            &pg,
            &TrackerLocks::default(),
            t,
            fake_adapter([scope_a, scope_b]),
            &FakeBoard::empty(),
            SYNC_DEADLINE,
        )
        .await
        .unwrap();
        assert_eq!(counts.links, 2);
        assert_eq!(
            links(&pg, t).await,
            sorted(vec![
                link(in_a, "gid://issue/1", "prompt", "#12", 1),
                link(in_b, "gid://issue/2", "prompt", "#12", 2),
            ]),
            "one key, two repos, two tasks; no matching scope or no repo_key links nothing"
        );
    }
}
