//! On-demand ticket discovery (spec R11): a ticket the board has but sensei has
//! not synced yet, named in a newly ingested session (or looked up by key), is
//! fetched by key — board-scoped, GET only — inserted, and linked, without
//! waiting for the next full sync. Discovery only ever adds; the full sync (R5)
//! stays the authority.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::db::pg_store::PgStore;
use crate::db::pg_store::trackers::ApplyError;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};
use crate::tracker::link::known_tasks;
use crate::tracker::runtime::discover_stored;
use crate::tracker::sync::{TrackerLocks, links_to, scan_sessions};
use crate::tracker::{
    BoardScope, Discovered, GetTransport, KnownTask, Link, SessionText, TrackerAdapter,
    TrackerError,
};

/// A key the board does not have is not asked for again within this.
pub const UNKNOWN_KEY_TTL: Duration = Duration::from_secs(6 * 60 * 60);
/// Keys asked for per run; more wait for the next run.
pub const DISCOVER_MAX_KEYS: usize = 50;
/// After a failed run the tracker's discovery pauses this long.
pub const DISCOVER_BACKOFF: Duration = Duration::from_secs(5 * 60);
/// Whole-run deadline for the provider requests.
pub const DISCOVER_DEADLINE: Duration = Duration::from_secs(60);

/// In-process pause per tracker after a failed run.
#[derive(Default)]
pub struct DiscoverBackoff {
    until: std::sync::Mutex<HashMap<uuid::Uuid, Instant>>,
}

/// What to look at.
#[derive(Debug, Clone, Copy)]
pub enum Scope<'a> {
    /// The candidate sessions ingested after the tracker's watermark.
    Ingested,
    /// These keys (a lookup); no session scan, the watermark is untouched.
    Keys(&'a [String]),
}

/// What a run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoverOutcome {
    /// Keys asked of the provider, sorted.
    pub asked: Vec<String>,
    /// Keys inserted as tickets, sorted.
    pub inserted: Vec<String>,
    /// Keys the board does not have, sorted (now negative-cached).
    pub not_on_board: Vec<String>,
    pub links: u32,
}

/// Why a run did not apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverError {
    /// A sync or another discovery holds the tracker.
    Busy,
    /// A recent failure paused this tracker's discovery.
    Paused,
    /// The tracker is gone or re-connected meanwhile.
    Gone,
    Failed(TrackerError),
    Db(String),
}

impl DiscoverBackoff {
    fn paused(&self, tracker_id: uuid::Uuid) -> bool {
        let until = self.until.lock().unwrap_or_else(|p| p.into_inner());
        until.get(&tracker_id).is_some_and(|t| Instant::now() < *t)
    }
    fn pause(&self, tracker_id: uuid::Uuid) {
        let mut until = self.until.lock().unwrap_or_else(|p| p.into_inner());
        until.insert(tracker_id, Instant::now() + DISCOVER_BACKOFF);
    }
}

/// One discovery run for a tracker (R11 steps 1–6).
pub async fn discover(
    pg: &PgStore,
    locks: &TrackerLocks,
    backoff: &DiscoverBackoff,
    tracker_id: uuid::Uuid,
    adapter: &'static dyn TrackerAdapter,
    transport: &dyn GetTransport,
    scope: Scope<'_>,
) -> Result<DiscoverOutcome, DiscoverError> {
    if backoff.paused(tracker_id) {
        return Err(DiscoverError::Paused);
    }
    // 1. Guard, shared with the full sync.
    let Some(_guard) = locks.try_lock(tracker_id) else {
        return Err(DiscoverError::Busy);
    };
    let result = run(pg, tracker_id, adapter, transport, scope).await;
    if matches!(result, Err(DiscoverError::Failed(_) | DiscoverError::Db(_))) {
        backoff.pause(tracker_id);
    }
    result
}

async fn run(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    adapter: &'static dyn TrackerAdapter,
    transport: &dyn GetTransport,
    scope: Scope<'_>,
) -> Result<DiscoverOutcome, DiscoverError> {
    let db = DiscoverError::Db;
    let row = pg.tracker_by_id(tracker_id).await.map_err(db)?.ok_or(DiscoverError::Gone)?;
    // 2. What is known, and what the board already said it lacks.
    let aliases = pg.tracker_key_aliases(tracker_id).await.map_err(db)?;
    let known = Arc::new(known_tasks(&aliases, &[]));
    let cached: BTreeSet<String> = pg
        .tracker_unknown_keys_fresh(tracker_id, UNKNOWN_KEY_TTL)
        .await
        .map_err(db)?
        .into_iter()
        .collect();

    // 3. Find: keys shaped like this board's that no ticket has.
    let scan_started = chrono::Utc::now();
    let (found, mut watermark): (BTreeSet<String>, _) = match scope {
        Scope::Keys(keys) => {
            // The adapter decides what a key of this board looks like.
            let probe = SessionText { prompts: keys.to_vec(), ..Default::default() };
            (adapter.unknown_keys(&probe, &known).into_iter().collect(), None)
        }
        Scope::Ingested => {
            let k = known.clone();
            let keys = scan_sessions(pg, tracker_id, Some(row.discovered_through), move |s| {
                adapter.unknown_keys(s, &k)
            })
            .await
            .map_err(db)?;
            (keys.into_iter().collect(), Some(scan_started))
        }
    };
    let mut asked: Vec<String> = found.difference(&cached).cloned().collect();
    if asked.len() > DISCOVER_MAX_KEYS {
        // The rest wait for the next run: the watermark stays where it is.
        asked.truncate(DISCOVER_MAX_KEYS);
        watermark = None;
    }
    if asked.is_empty() {
        if watermark.is_some() {
            apply(pg, &row, &Discovered::default(), &[], &[], watermark).await?;
        }
        return Ok(DiscoverOutcome::default());
    }

    // 4. Fetch, board-scoped, under the deadline; no DB connection held.
    let board = BoardScope { base_url: row.base_url.clone(), board_ref: row.board_ref.clone() };
    let fetched = match tokio::time::timeout(
        DISCOVER_DEADLINE,
        adapter.fetch_tasks(transport, &board, &asked),
    )
    .await
    {
        Ok(Ok(d)) => d,
        Ok(Err(e)) => return Err(DiscoverError::Failed(e)),
        Err(_) => return Err(DiscoverError::Failed(TrackerError::Transport)),
    };
    let got: BTreeSet<String> = fetched.tasks.iter().map(|t| t.key.clone()).collect();
    let not_on_board: Vec<String> = asked.iter().filter(|k| !got.contains(*k)).cloned().collect();

    // Every candidate session's links to the new tickets — a lookup's, and a
    // session ingested before the watermark, included.
    let new_known: Vec<KnownTask> = fetched
        .tasks
        .iter()
        .map(|t| KnownTask {
            external_id: t.external_id.clone(),
            key: t.key.clone(),
            scope: t.scope.clone(),
            current: true,
        })
        .collect();
    let links = links_to(pg, tracker_id, adapter, Arc::new(new_known), None).await.map_err(db)?;

    // 5. Apply, adding only.
    let counts = apply(pg, &row, &fetched, &links, &not_on_board, watermark).await?;
    Ok(DiscoverOutcome {
        asked,
        inserted: got.into_iter().collect(),
        not_on_board,
        links: counts.links,
    })
}

async fn apply(
    pg: &PgStore,
    row: &crate::db::pg_store::trackers::TrackerRow,
    discovered: &Discovered,
    links: &[Link],
    not_found: &[String],
    watermark: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<crate::db::pg_store::trackers::DiscoverCounts, DiscoverError> {
    pg.apply_discovered(
        row.id,
        &crate::db::pg_store::DiscoveryApply {
            secret_service: &row.secret_service,
            discovered,
            links,
            not_found,
            watermark,
        },
    )
    .await
    .map_err(|e| match e {
        ApplyError::Gone | ApplyError::Superseded => DiscoverError::Gone,
        ApplyError::Db(e) => DiscoverError::Db(e),
    })
}

/// Ingest hook: queue one discovery per tracker whose candidates include this
/// client session, unless one is already pending. Returns how many were queued.
pub async fn after_ingest(
    pg: &PgStore,
    queue: &TaskQueue,
    client_session_id: &str,
) -> Result<u32, String> {
    let mut queued = 0;
    for tracker_id in pg.trackers_for_client_session(client_session_id).await? {
        let path = tracker_id.to_string();
        if queue.has_pending_kind_path(TaskKind::TrackerDiscover, &path).await {
            continue;
        }
        queue.enqueue(Task::new(TaskKind::TrackerDiscover, "", &path)).await;
        queued += 1;
    }
    Ok(queued)
}

/// `TaskKind::TrackerDiscover`: `task.path` is the tracker id. A busy or paused
/// tracker is not an error — the next ingest queues it again.
pub async fn run_task(
    ctx: &crate::tasks::executor::TaskContext,
    task: &Task,
) -> Result<u32, String> {
    let id: uuid::Uuid =
        task.path.parse().map_err(|e| format!("tracker id {:?}: {e}", task.path))?;
    let pg = ctx.pg();
    let Some(row) = pg.tracker_by_id(id).await? else { return Ok(0) };
    match discover_stored(pg, &ctx.app_state.trackers, &row, Scope::Ingested).await {
        Ok(out) => Ok(out.inserted.len() as u32),
        Err(DiscoverError::Busy | DiscoverError::Paused | DiscoverError::Gone) => Ok(0),
        Err(DiscoverError::Failed(e)) => {
            tracing::warn!(tracker_id = %id, code = e.code(), "tracker discovery failed");
            Err(format!("tracker discovery: {}", e.code()))
        }
        Err(DiscoverError::Db(e)) => Err(format!("tracker discovery: database: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::connect::tests::{connected_tracker, workspace};
    use crate::tracker::jira::JiraAdapter;
    use crate::tracker::sync::{SYNC_DEADLINE, sync_tracker};
    use crate::tracker::test_support::{FakeBoard, repo_in, session_with};
    use serde_json::json;

    /// A board-search issue: `sprint` is its current sprint, `closed` its past ones.
    fn issue(id: &str, key: &str, sprint: Option<u64>, closed: &[u64]) -> serde_json::Value {
        json!({"id": id, "key": key, "fields": {
            "summary": format!("summary {key}"),
            "issuetype": {"name": "Bug"}, "status": {"name": "To Do"},
            "sprint": sprint.map(|s| json!({"id": s, "state": "active"})),
            "closedSprints": closed.iter().map(|c| json!({"id": c, "state": "closed"})).collect::<Vec<_>>()}})
    }

    async fn run(
        pg: &PgStore,
        t: uuid::Uuid,
        board: &FakeBoard,
        backoff: &DiscoverBackoff,
        scope: Scope<'_>,
    ) -> Result<DiscoverOutcome, DiscoverError> {
        discover(pg, &TrackerLocks::default(), backoff, t, &JiraAdapter, board, scope).await
    }

    async fn task_row(
        pg: &PgStore,
        t: uuid::Uuid,
        key: &str,
    ) -> Option<(String, Option<String>, bool)> {
        // (external id, current sprint ext id, removed)
        sqlx_core::query_as::query_as(
            "SELECT tt.external_id, s.external_id, tt.removed_at IS NOT NULL
               FROM sensei.tracker_tasks tt LEFT JOIN sensei.sprints s ON s.id = tt.current_sprint_id
              WHERE tt.tracker_id = $1 AND tt.key = $2",
        )
        .bind(t)
        .bind(key)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
    }

    /// (session, task key, source)
    async fn links(pg: &PgStore, t: uuid::Uuid) -> Vec<(uuid::Uuid, String, String)> {
        sqlx_core::query_as::query_as(
            "SELECT l.session_id, tt.key, l.source::text FROM sensei.session_task_links l
               JOIN sensei.tracker_tasks tt ON tt.id = l.task_id
              WHERE tt.tracker_id = $1 ORDER BY 1, 2, 3",
        )
        .bind(t)
        .fetch_all(pg.pool())
        .await
        .unwrap()
    }

    async fn cached(pg: &PgStore, t: uuid::Uuid) -> Vec<String> {
        sqlx_core::query_as::query_as::<_, (String,)>(
            "SELECT key FROM sensei.tracker_unknown_keys WHERE tracker_id = $1 ORDER BY 1",
        )
        .bind(t)
        .fetch_all(pg.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|(k,)| k)
        .collect()
    }

    /// A connected tracker, fully synced to the standard board (AUTH-1..3), with
    /// one repo in its workspace.
    async fn synced(pg: &PgStore) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let (t, p) = connected_tracker(pg).await;
        sync_tracker(
            pg,
            &TrackerLocks::default(),
            t,
            &JiraAdapter,
            &FakeBoard::standard(),
            SYNC_DEADLINE,
        )
        .await
        .unwrap();
        let repo = repo_in(pg, p).await;
        (t, p, repo)
    }

    #[tokio::test]
    async fn a_session_naming_an_unsynced_key_gets_the_ticket_and_its_link() {
        let pg = PgStore::test_store().await;
        let (t, _p, repo) = synced(&pg).await;
        let earlier = session_with(&pg, Some(repo), &["fix/AUTH-2"], &[]).await;
        sync_tracker(
            &pg,
            &TrackerLocks::default(),
            t,
            &JiraAdapter,
            &FakeBoard::standard(),
            SYNC_DEADLINE,
        )
        .await
        .unwrap();
        let before = links(&pg, t).await;
        let s = session_with(&pg, Some(repo), &["fix/auth-900-login"], &["also AUTH-2"]).await;
        let board = FakeBoard::standard()
            .with_jql("key in (AUTH-900)", json!([issue("10900", "AUTH-900", Some(101), &[100])]));

        let watermark = pg.tracker_by_id(t).await.unwrap().unwrap().discovered_through;
        let out = run(&pg, t, &board, &DiscoverBackoff::default(), Scope::Ingested).await.unwrap();
        assert_eq!(board.jql_calls(), vec!["key in (AUTH-900)"], "one board-scoped request");
        assert!(
            pg.tracker_by_id(t).await.unwrap().unwrap().discovered_through > watermark,
            "the watermark moves past what was checked"
        );
        assert_eq!(
            (out.inserted.clone(), out.not_on_board.clone()),
            (vec!["AUTH-900".to_string()], vec![])
        );
        assert_eq!(
            task_row(&pg, t, "AUTH-900").await,
            Some(("10900".into(), Some("101".into()), false))
        );
        let now = links(&pg, t).await;
        assert!(now.contains(&(s, "AUTH-900".into(), "branch".into())), "{now:?}");
        for l in &before {
            assert!(now.contains(l), "an existing link was touched: {l:?}");
        }
        assert!(now.iter().any(|(sid, k, _)| *sid == earlier && k == "AUTH-2"));
        for key in ["AUTH-1", "AUTH-2", "AUTH-3"] {
            assert_eq!(task_row(&pg, t, key).await.map(|r| r.2), Some(false), "{key} not removed");
        }

        // The watermark moved: the same sessions are not looked at again.
        let again = FakeBoard::standard();
        let out = run(&pg, t, &again, &DiscoverBackoff::default(), Scope::Ingested).await.unwrap();
        assert_eq!((out.asked, again.jql_calls()), (vec![], Vec::<String>::new()));
    }

    #[tokio::test]
    async fn a_session_with_only_known_keys_asks_nothing() {
        let pg = PgStore::test_store().await;
        let (t, _p, repo) = synced(&pg).await;
        session_with(&pg, Some(repo), &["fix/AUTH-2"], &["UTF-8 and Q-5 and AUTH-3"]).await;
        let board = FakeBoard::standard();
        let watermark = pg.tracker_by_id(t).await.unwrap().unwrap().discovered_through;
        let out = run(&pg, t, &board, &DiscoverBackoff::default(), Scope::Ingested).await.unwrap();
        assert_eq!(out, DiscoverOutcome::default());
        assert!(
            pg.tracker_by_id(t).await.unwrap().unwrap().discovered_through > watermark,
            "checked sessions are not read again, even when nothing was asked"
        );
        assert!(board.jql_calls().is_empty());
    }

    #[tokio::test]
    async fn a_key_the_board_lacks_is_asked_once_within_the_ttl() {
        let pg = PgStore::test_store().await;
        let (t, _p, repo) = synced(&pg).await;
        session_with(&pg, Some(repo), &[], &["see AUTH-404"]).await;
        let board = FakeBoard::standard();
        let out = run(&pg, t, &board, &DiscoverBackoff::default(), Scope::Ingested).await.unwrap();
        assert_eq!(out.not_on_board, vec!["AUTH-404".to_string()]);
        assert_eq!(cached(&pg, t).await, vec!["AUTH-404".to_string()]);
        assert_eq!(task_row(&pg, t, "AUTH-404").await, None);

        session_with(&pg, Some(repo), &[], &["AUTH-404 again"]).await;
        let later = FakeBoard::standard();
        run(&pg, t, &later, &DiscoverBackoff::default(), Scope::Ingested).await.unwrap();
        assert!(later.jql_calls().is_empty(), "negative-cached: not asked again");
    }

    #[tokio::test]
    async fn a_rejected_batch_is_asked_one_key_at_a_time() {
        let pg = PgStore::test_store().await;
        let (t, _p, repo) = synced(&pg).await;
        let s = session_with(&pg, Some(repo), &[], &["AUTH-404 and AUTH-900"]).await;
        let board = FakeBoard::standard()
            .fail_jql("key in (AUTH-404,AUTH-900)", TrackerError::BadRequest)
            .fail_jql("key in (AUTH-404)", TrackerError::BadRequest)
            .with_jql("key in (AUTH-900)", json!([issue("10900", "AUTH-900", None, &[])]));
        let out = run(&pg, t, &board, &DiscoverBackoff::default(), Scope::Ingested).await.unwrap();
        assert_eq!(
            board.jql_calls(),
            vec!["key in (AUTH-404,AUTH-900)", "key in (AUTH-404)", "key in (AUTH-900)"]
        );
        assert_eq!(
            (out.inserted, out.not_on_board),
            (vec!["AUTH-900".to_string()], vec!["AUTH-404".to_string()])
        );
        assert!(links(&pg, t).await.contains(&(s, "AUTH-900".into(), "prompt".into())));
    }

    #[tokio::test]
    async fn a_held_lock_skips_and_keeps_the_watermark() {
        let pg = PgStore::test_store().await;
        let (t, _p, repo) = synced(&pg).await;
        session_with(&pg, Some(repo), &["AUTH-900"], &[]).await;
        let board = FakeBoard::standard()
            .with_jql("key in (AUTH-900)", json!([issue("10900", "AUTH-900", None, &[])]));
        let locks = TrackerLocks::default();
        let held = locks.try_lock(t).unwrap();
        let backoff = DiscoverBackoff::default();
        let r = discover(&pg, &locks, &backoff, t, &JiraAdapter, &board, Scope::Ingested).await;
        assert_eq!(r, Err(DiscoverError::Busy));
        assert!(board.jql_calls().is_empty());
        drop(held);
        let out = discover(&pg, &locks, &backoff, t, &JiraAdapter, &board, Scope::Ingested)
            .await
            .unwrap();
        assert_eq!(
            out.inserted,
            vec!["AUTH-900".to_string()],
            "the skipped session is still pending"
        );
    }

    #[tokio::test]
    async fn a_failure_applies_nothing_records_nothing_and_pauses() {
        let pg = PgStore::test_store().await;
        let (t, _p, repo) = synced(&pg).await;
        session_with(&pg, Some(repo), &["AUTH-900"], &[]).await;
        let watermark = pg.tracker_by_id(t).await.unwrap().unwrap().discovered_through;
        let board = FakeBoard::standard().fail_jql("key in (AUTH-900)", TrackerError::HttpError);
        let backoff = DiscoverBackoff::default();
        let r = run(&pg, t, &board, &backoff, Scope::Ingested).await;
        assert_eq!(r, Err(DiscoverError::Failed(TrackerError::HttpError)));
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        assert_eq!(
            (row.last_error, row.discovered_through),
            (None, watermark),
            "no status, watermark kept"
        );
        assert_eq!(task_row(&pg, t, "AUTH-900").await, None);
        assert!(cached(&pg, t).await.is_empty(), "a 5xx is not \"not on the board\"");

        let r = run(&pg, t, &board, &backoff, Scope::Ingested).await;
        assert_eq!(r, Err(DiscoverError::Paused));
        assert_eq!(board.jql_calls().len(), 1, "paused: not asked again");
    }

    #[tokio::test]
    async fn the_next_full_sync_keeps_what_was_discovered_and_clears_the_cache() {
        let pg = PgStore::test_store().await;
        let (t, _p, repo) = synced(&pg).await;
        let s = session_with(&pg, Some(repo), &["AUTH-900"], &["and AUTH-404"]).await;
        let board = FakeBoard::standard().with_jql(
            "key in (AUTH-404,AUTH-900)",
            json!([issue("10900", "AUTH-900", Some(101), &[])]),
        );
        run(&pg, t, &board, &DiscoverBackoff::default(), Scope::Ingested).await.unwrap();
        assert_eq!(cached(&pg, t).await, vec!["AUTH-404".to_string()]);

        // AUTH-404 joins the board later; the full sync lists both.
        let full = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/issue",
            2,
            json!({"startAt": 2, "total": 5, "issues": [
                {"id": "10003", "key": "AUTH-3", "fields": {}},
                {"id": "10900", "key": "AUTH-900", "fields": {}},
                {"id": "10404", "key": "AUTH-404", "fields": {}}]}),
        );
        sync_tracker(&pg, &TrackerLocks::default(), t, &JiraAdapter, &full, SYNC_DEADLINE)
            .await
            .unwrap();
        assert_eq!(task_row(&pg, t, "AUTH-900").await.map(|r| r.2), Some(false));
        let l = links(&pg, t).await;
        assert!(l.contains(&(s, "AUTH-900".into(), "branch".into())));
        assert!(
            l.contains(&(s, "AUTH-404".into(), "prompt".into())),
            "the full relink links it now"
        );
        assert!(cached(&pg, t).await.is_empty(), "a key the sync now has leaves the cache");
    }

    #[tokio::test]
    async fn a_lookup_fetches_one_key_and_links_every_session_naming_it() {
        let pg = PgStore::test_store().await;
        let (t, _p, repo) = synced(&pg).await;
        let old = session_with(&pg, Some(repo), &["AUTH-900"], &[]).await;
        // The old session predates the watermark: ingest-path discovery would not see it.
        sqlx_core::query::query(
            "UPDATE sensei.trackers SET discovered_through = now() WHERE id = $1",
        )
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap();
        let board = FakeBoard::standard()
            .with_jql("key in (AUTH-900)", json!([issue("10900", "AUTH-900", None, &[])]));
        let keys = vec!["AUTH-900".to_string()];
        let out =
            run(&pg, t, &board, &DiscoverBackoff::default(), Scope::Keys(&keys)).await.unwrap();
        assert_eq!(out.inserted, keys);
        assert!(links(&pg, t).await.contains(&(old, "AUTH-900".into(), "branch".into())));
    }

    #[tokio::test]
    async fn ingest_queues_one_discovery_per_tracker_while_one_is_pending() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let repo = repo_in(&pg, p).await;
        let s = session_with(&pg, Some(repo), &["AUTH-900"], &[]).await;
        let client: (Option<String>,) = sqlx_core::query_as::query_as(
            "SELECT client_session_id FROM activity.sessions WHERE id = $1",
        )
        .bind(s)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let client = client.0.unwrap();
        let queue = TaskQueue::new();
        assert_eq!(after_ingest(&pg, &queue, &client).await, Ok(1));
        assert_eq!(after_ingest(&pg, &queue, &client).await, Ok(0), "coalesced");
        assert!(
            queue
                .has_pending_kind_path(crate::tasks::TaskKind::TrackerDiscover, &t.to_string())
                .await
        );

        // A session outside every tracker's workspace queues nothing.
        let elsewhere = repo_in(&pg, workspace(&pg).await).await;
        let o = session_with(&pg, Some(elsewhere), &["AUTH-900"], &[]).await;
        let other: (Option<String>,) = sqlx_core::query_as::query_as(
            "SELECT client_session_id FROM activity.sessions WHERE id = $1",
        )
        .bind(o)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(after_ingest(&pg, &TaskQueue::new(), &other.0.unwrap()).await, Ok(0));
    }
}
