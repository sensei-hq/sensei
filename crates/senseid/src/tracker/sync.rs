//! The provider-neutral sync coordinator (spec R5): guard → fetch with no DB
//! connection held → apply in one short transaction → record the outcome.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::trackers::{ApplyCounts, ApplyError};
use crate::tracker::{BoardScope, GetTransport, TrackerAdapter, TrackerError};

/// Whole-sync deadline for the fetch phase.
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
    /// The provider or credential failed; recorded as `last_error`.
    Failed(TrackerError),
    /// The database failed; nothing recorded.
    Db(String),
}

/// Sync one tracker end to end.
pub async fn sync_tracker(
    pg: &PgStore,
    locks: &TrackerLocks,
    tracker_id: uuid::Uuid,
    adapter: &dyn TrackerAdapter,
    transport: &dyn GetTransport,
    deadline: Duration,
) -> Result<ApplyCounts, SyncError> {
    // 1. Guard.
    let Some(_guard) = locks.try_lock(tracker_id) else {
        return Err(SyncError::Busy);
    };
    let row = pg.tracker_by_id(tracker_id).await.map_err(SyncError::Db)?.ok_or(SyncError::Gone)?;
    pg.mark_tracker_attempt(tracker_id).await.map_err(SyncError::Db)?;
    let prev = pg.tracker_prev_state(tracker_id).await.map_err(SyncError::Db)?;

    // 2. Fetch, with no connection checked out, under the whole-sync deadline.
    let board = BoardScope { base_url: row.base_url, board_ref: row.board_ref };
    let snapshot = match tokio::time::timeout(
        deadline,
        adapter.fetch_snapshot(transport, &board, &prev),
    )
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(record_failure(pg, tracker_id, e).await),
        Err(_) => return Err(record_failure(pg, tracker_id, TrackerError::Transport).await),
    };

    // 3. Links are computed here once linking lands (F3).
    // 4. Apply in one short transaction; success is recorded inside it.
    let patch = match &snapshot.board_name {
        Some(name) => serde_json::json!({ "board_name": name }),
        None => serde_json::json!({}),
    };
    pg.apply_tracker_snapshot(tracker_id, &snapshot, &patch).await.map_err(|e| match e {
        ApplyError::Gone => SyncError::Gone,
        ApplyError::Db(e) => SyncError::Db(e),
    })
}

/// Step 5: record a provider/credential failure. A row deleted meanwhile makes the
/// update a no-op, which is what a gone tracker should get.
async fn record_failure(pg: &PgStore, tracker_id: uuid::Uuid, e: TrackerError) -> SyncError {
    if let Err(db) = pg.mark_tracker_error(tracker_id, e.code()).await {
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
        let Ok(pg) = PgStore::connect_test().await else { return };
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
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, _p) = connected_tracker(&pg).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        let before = rows(&pg, t).await;
        sync(&pg, t, &FakeBoard::standard()).await.unwrap();
        assert_eq!(rows(&pg, t).await, before);
    }

    #[tokio::test]
    async fn a_missing_issue_is_marked_removed_once() {
        let Ok(pg) = PgStore::connect_test().await else { return };
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
        let Ok(pg) = PgStore::connect_test().await else { return };
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
        let Ok(pg) = PgStore::connect_test().await else { return };
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
        let Ok(pg) = PgStore::connect_test().await else { return };
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
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, p) = connected_tracker(&pg).await;
        let pg2 = pg.clone();
        let board = FakeBoard::standard().on_call(Box::new(move |_path| {
            let pg = pg2.clone();
            Box::pin(async move {
                pg.delete_tracker_for_project(p).await.unwrap();
            })
        }));
        assert_eq!(sync(&pg, t, &board).await, Err(SyncError::Gone));
    }

    #[tokio::test]
    async fn a_held_lock_is_busy() {
        let Ok(pg) = PgStore::connect_test().await else { return };
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
        let Ok(pg) = PgStore::connect_test().await else { return };
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
        let Ok(pg) = PgStore::connect_test().await else { return };
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
}
