//! The ticket-history task (spec tracker-sdlc-metrics S4): after a successful
//! sync, under the same per-tracker guard as the write-back pass, read the board's
//! terms and each changed ticket's changelog into `sensei.tracker_task_changes`.
//!
//! Separate from sync by design (override of linkage R5): it commits per ticket,
//! so a backfill interrupted by its deadline resumes where it stopped, and it
//! records its own outcome in `history_error`, never `last_error`. GET only.

use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::tracker::runtime::SYNC_RETRIES;
use crate::tracker::secret::{KEYRING_TIMEOUT, bounded};
use crate::tracker::{BoardScope, GetTransport, TrackerAdapter, TrackerError};

/// How long one run may take: the rest continues after the next sync.
pub const HISTORY_DEADLINE: Duration = Duration::from_secs(5 * 60);

/// Tickets read per batch of candidates.
const BATCH: i64 = 200;

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryOutcome {
    /// Every changed ticket's history is read.
    Done,
    /// The deadline came first; the rest continues next run.
    OutOfTime,
    /// It stopped on this error, recorded in `history_error`.
    Failed(&'static str),
    /// The tracker is gone, or was re-keyed since the sync.
    Skipped,
}

/// One run for a tracker, while the caller holds its guard.
pub async fn run(
    pg: &PgStore,
    io: &crate::tracker::writeback::PassIo,
    tracker_id: uuid::Uuid,
    secret_service: &str,
    adapter: &'static dyn TrackerAdapter,
    deadline: Duration,
    _guard: &tokio::sync::OwnedMutexGuard<()>,
) -> HistoryOutcome {
    // §G8: an adapter that reads no history leaves the tracker's history state NULL.
    if !adapter.capabilities().history {
        return HistoryOutcome::Skipped;
    }
    let until = tokio::time::Instant::now() + deadline;
    let row = match pg.tracker_by_id(tracker_id).await {
        Ok(Some(row)) if row.secret_service == secret_service => row,
        Ok(_) => return HistoryOutcome::Skipped,
        Err(_) => return finish(pg, tracker_id, Some(TrackerError::Db.code())).await,
    };
    let (service, account) = (row.secret_service.clone(), row.account.clone());
    let token = match bounded(io.secrets.clone(), KEYRING_TIMEOUT, move |s| {
        s.get(&service, &account)
    })
    .await
    {
        Ok(t) => t,
        Err(e) => return finish(pg, tracker_id, Some(e.code())).await,
    };
    let transport = match io.transports.transport(
        &row.base_url,
        adapter.auth(&row.account, &token),
        SYNC_RETRIES,
    ) {
        Ok(t) => t,
        Err(e) => return finish(pg, tracker_id, Some(e.code())).await,
    };
    let board = BoardScope { base_url: row.base_url.clone(), board_ref: row.board_ref.clone() };
    read(pg, transport.as_ref(), &board, tracker_id, adapter, until).await
}

/// The run's body, given its transport (S4).
pub async fn read(
    pg: &PgStore,
    t: &dyn GetTransport,
    board: &BoardScope,
    tracker_id: uuid::Uuid,
    adapter: &'static dyn TrackerAdapter,
    until: tokio::time::Instant,
) -> HistoryOutcome {
    let terms = match tokio::time::timeout_at(until, adapter.board_terms(t, board)).await {
        Err(_) => return finish(pg, tracker_id, None).await.out_of_time(),
        Ok(Err(e)) => return finish(pg, tracker_id, Some(e.code())).await,
        Ok(Ok(terms)) => terms,
    };
    if pg.set_board_terms(tracker_id, &terms).await.is_err() {
        return finish(pg, tracker_id, Some(TrackerError::Db.code())).await;
    }
    loop {
        let batch = match pg.history_candidates(tracker_id, BATCH).await {
            Ok(b) => b,
            Err(_) => return finish(pg, tracker_id, Some(TrackerError::Db.code())).await,
        };
        if batch.is_empty() {
            return finish(pg, tracker_id, None).await;
        }
        for c in batch {
            let read = adapter.changelog(
                t,
                &c.external_id,
                terms.sprint_field.as_deref(),
                terms.estimation_field.as_deref(),
            );
            let changes = match tokio::time::timeout_at(until, read).await {
                Err(_) => return finish(pg, tracker_id, None).await.out_of_time(),
                Ok(Err(e)) => return finish(pg, tracker_id, Some(e.code())).await,
                Ok(Ok(changes)) => changes,
            };
            if pg.store_task_history(c.task_id, &changes, c.updated_at).await.is_err() {
                return finish(pg, tracker_id, Some(TrackerError::Db.code())).await;
            }
        }
    }
}

impl HistoryOutcome {
    fn out_of_time(self) -> HistoryOutcome {
        match self {
            HistoryOutcome::Done => HistoryOutcome::OutOfTime,
            other => other,
        }
    }
}

/// Record the run's end; the outcome is `Failed` when `error` is set.
async fn finish(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    error: Option<&'static str>,
) -> HistoryOutcome {
    if let Err(e) = pg.set_history_outcome(tracker_id, error).await {
        tracing::warn!(tracker_id = %tracker_id, error = %e, "tracker history: could not record the run");
    }
    match error {
        Some(code) => HistoryOutcome::Failed(code),
        None => HistoryOutcome::Done,
    }
}

/// A tracker row for these tests (the shape `pg_store::trackers` tests insert).
#[cfg(test)]
pub(crate) async fn test_tracker(pg: &PgStore) -> uuid::Uuid {
    let w = pg
        .create_workspace(&format!("_test:history:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.trackers (workspace_id, kind, base_url, account, external_scope, board_ref, secret_service)
         VALUES ($1, 'jira', $2, 'dev@example.com', 'AUTH', '42', $3) RETURNING id",
    )
    .bind(w)
    .bind(format!("https://h{}.atlassian.net", uuid::Uuid::new_v4().simple()))
    .bind(format!("svc-{}", uuid::Uuid::new_v4()))
    .fetch_one(pg.pool())
    .await
    .unwrap();
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::jira::JiraAdapter;
    use crate::tracker::test_support::FakeBoard;
    use serde_json::json;

    fn board() -> BoardScope {
        BoardScope { base_url: "https://acme.atlassian.net".into(), board_ref: "42".into() }
    }

    /// A board that answers the terms, and a one-entry changelog for each ticket.
    fn fake(issues: std::ops::RangeInclusive<u32>) -> FakeBoard {
        let mut b = FakeBoard::empty()
            .with("/rest/agile/1.0/board/42/configuration", 0, json!({"estimation": {"type": "field", "field": {"fieldId": "customfield_10016"}}}))
            .with("/rest/api/3/field", 0, json!([{"id": "customfield_10020", "schema": {"custom": "com.pyxis.greenhopper.jira:gh-sprint"}}]))
            .with("/rest/api/3/status", 0, json!([{"id": "3", "name": "Done", "statusCategory": {"key": "done"}}]));
        for i in issues {
            b = b.with(&format!("/rest/api/3/issue/{i}/changelog"), 0, json!({"startAt": 0, "isLast": true, "values": [
                {"id": format!("h{i}"), "created": "2026-09-01T10:00:00.000+0000",
                 "items": [{"field": "Sprint", "fieldId": "customfield_10020", "from": "", "to": "100"}]}]}));
        }
        b
    }

    async fn tracker_with_tasks(pg: &PgStore, n: u32) -> uuid::Uuid {
        let t = super::test_tracker(pg).await;
        for i in 0..n {
            sqlx_core::query::query(
                "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, issue_updated_at)
                 VALUES ($1, $2, $3, now() - make_interval(mins => $4))",
            )
            .bind(t)
            .bind((10_001 + i).to_string())
            .bind(format!("AUTH-{}", i + 1))
            .bind((n - i) as i32)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        t
    }

    async fn read_through(pg: &PgStore, t: uuid::Uuid) -> i64 {
        let (n,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.tracker_tasks WHERE tracker_id = $1 AND history_through IS NOT NULL",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        n
    }

    async fn errors(pg: &PgStore, t: uuid::Uuid) -> (Option<String>, Option<String>, bool) {
        sqlx_core::query_as::query_as(
            "SELECT history_error::text, last_error::text, history_at IS NOT NULL FROM sensei.trackers WHERE id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap()
    }

    fn changelog_calls(b: &FakeBoard) -> usize {
        b.calls().iter().filter(|c| c.ends_with("/changelog")).count()
    }

    /// Spec gate 8.
    #[tokio::test]
    async fn an_interrupted_backfill_resumes_and_an_unchanged_board_costs_nothing() {
        let pg = PgStore::test_store().await;
        let t = tracker_with_tasks(&pg, 30).await;
        let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = seen.clone();
        let hanging = fake(10_001..=10_030).on_call(Box::new(move |path: String| {
            let n = if path.ends_with("/changelog") {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
            } else {
                0
            };
            Box::pin(async move {
                if n > 10 {
                    std::future::pending::<()>().await;
                }
            })
        }));
        let until = tokio::time::Instant::now() + Duration::from_millis(1_500);
        assert_eq!(
            read(&pg, &hanging, &board(), t, &JiraAdapter, until).await,
            HistoryOutcome::OutOfTime
        );
        assert_eq!(read_through(&pg, t).await, 10, "per-ticket commits survive the deadline");
        assert_eq!(
            errors(&pg, t).await,
            (None, None, true),
            "out of time is no error, and last_error is untouched"
        );

        let rest = fake(10_001..=10_030);
        let until = tokio::time::Instant::now() + Duration::from_secs(30);
        assert_eq!(read(&pg, &rest, &board(), t, &JiraAdapter, until).await, HistoryOutcome::Done);
        assert_eq!((read_through(&pg, t).await, changelog_calls(&rest)), (30, 20));
        let (sprint_rows,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.tracker_task_changes c JOIN sensei.tracker_tasks k ON k.id = c.task_id
              WHERE k.tracker_id = $1 AND c.field = 'sprint'",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(sprint_rows, 30);

        let idle = fake(10_001..=10_030);
        assert_eq!(read(&pg, &idle, &board(), t, &JiraAdapter, until).await, HistoryOutcome::Done);
        assert_eq!(changelog_calls(&idle), 0, "nothing changed, nothing read");
        let (field, sprint): (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT estimation_field, sprint_field FROM sensei.trackers WHERE id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(
            (field.as_deref(), sprint.as_deref()),
            (Some("customfield_10016"), Some("customfield_10020"))
        );
    }

    #[tokio::test]
    async fn a_refused_board_records_history_error_not_last_error() {
        let pg = PgStore::test_store().await;
        let t = tracker_with_tasks(&pg, 2).await;
        let refused = fake(10_001..=10_002)
            .fail("/rest/agile/1.0/board/42/configuration", TrackerError::Forbidden);
        let until = tokio::time::Instant::now() + Duration::from_secs(30);
        let out = read(&pg, &refused, &board(), t, &JiraAdapter, until).await;
        assert_eq!(out, HistoryOutcome::Failed(TrackerError::Forbidden.code()));
        let (history, last, ran) = errors(&pg, t).await;
        assert_eq!(
            (history.as_deref(), last, ran),
            (Some(TrackerError::Forbidden.code()), None, true)
        );
        assert_eq!(read_through(&pg, t).await, 0);
    }

    #[tokio::test]
    async fn a_ticket_without_an_update_time_is_never_selected() {
        let pg = PgStore::test_store().await;
        let t = tracker_with_tasks(&pg, 1).await;
        sqlx_core::query::query(
            "UPDATE sensei.tracker_tasks SET issue_updated_at = NULL WHERE tracker_id = $1",
        )
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap();
        let b = fake(10_001..=10_001);
        let until = tokio::time::Instant::now() + Duration::from_secs(30);
        for _ in 0..2 {
            assert_eq!(read(&pg, &b, &board(), t, &JiraAdapter, until).await, HistoryOutcome::Done);
        }
        assert_eq!(changelog_calls(&b), 0);
    }
}

#[cfg(test)]
mod learning_tests {
    use super::*;
    use crate::tracker::Estimation;
    use crate::tracker::jira::JiraAdapter;
    use crate::tracker::test_support::FakeBoard;
    use serde_json::json;

    /// A run that could not read the board's terms must not be taken for "no
    /// estimation field": the next sync would clear every ticket's points.
    #[tokio::test]
    async fn the_estimation_is_learned_only_from_terms_actually_read() {
        let pg = PgStore::test_store().await;
        let t = super::test_tracker(&pg).await;
        let board =
            BoardScope { base_url: "https://acme.atlassian.net".into(), board_ref: "42".into() };
        let until = tokio::time::Instant::now() + Duration::from_secs(30);
        assert_eq!(pg.tracker_prev_state(t).await.unwrap().estimation, Estimation::NotLearned);

        let refused = FakeBoard::empty()
            .fail("/rest/agile/1.0/board/42/configuration", TrackerError::Forbidden);
        read(&pg, &refused, &board, t, &JiraAdapter, until).await;
        assert_eq!(
            pg.tracker_prev_state(t).await.unwrap().estimation,
            Estimation::NotLearned,
            "a failure teaches nothing"
        );

        let by_count = FakeBoard::empty()
            .with(
                "/rest/agile/1.0/board/42/configuration",
                0,
                json!({"estimation": {"type": "none"}}),
            )
            .with("/rest/api/3/field", 0, json!([]))
            .with("/rest/api/3/status", 0, json!([]));
        read(&pg, &by_count, &board, t, &JiraAdapter, until).await;
        assert_eq!(pg.tracker_prev_state(t).await.unwrap().estimation, Estimation::NoField);

        let points = FakeBoard::empty()
            .with(
                "/rest/agile/1.0/board/42/configuration",
                0,
                json!({"estimation": {"type": "field", "field": {"fieldId": "customfield_10016"}}}),
            )
            .with("/rest/api/3/field", 0, json!([]))
            .with("/rest/api/3/status", 0, json!([]));
        read(&pg, &points, &board, t, &JiraAdapter, until).await;
        assert_eq!(
            pg.tracker_prev_state(t).await.unwrap().estimation,
            Estimation::Field("customfield_10016".into())
        );
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;
    use crate::tracker::jira::JiraAdapter;
    use crate::tracker::test_support::FakeBoard;
    use serde_json::json;

    fn board() -> BoardScope {
        BoardScope { base_url: "https://acme.atlassian.net".into(), board_ref: "42".into() }
    }
    fn terms(statuses: serde_json::Value) -> FakeBoard {
        FakeBoard::empty()
            .with("/rest/agile/1.0/board/42/configuration", 0, json!({"estimation": {"type": "field", "field": {"fieldId": "customfield_10016"}}}))
            .with("/rest/api/3/field", 0, json!([{"id": "customfield_10020", "schema": {"custom": "com.pyxis.greenhopper.jira:gh-sprint"}}]))
            .with("/rest/api/3/status", 0, statuses)
    }
    async fn one_task(pg: &PgStore, t: uuid::Uuid, updated: &str) -> uuid::Uuid {
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, issue_updated_at)
             VALUES ($1, '10001', 'AUTH-1', $2::timestamptz) RETURNING id",
        )
        .bind(t)
        .bind(updated)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        id
    }
    fn until() -> tokio::time::Instant {
        tokio::time::Instant::now() + Duration::from_secs(30)
    }

    #[tokio::test]
    async fn statuses_are_stored_and_replaced() {
        let pg = PgStore::test_store().await;
        let t = test_tracker(&pg).await;
        let first = terms(json!([{"id": "1", "name": "To Do", "statusCategory": {"key": "new"}},
                                 {"id": "3", "name": "Done", "statusCategory": {"key": "done"}}]));
        read(&pg, &first, &board(), t, &JiraAdapter, until()).await;
        let second =
            terms(json!([{"id": "4", "name": "Shipped", "statusCategory": {"key": "done"}}]));
        read(&pg, &second, &board(), t, &JiraAdapter, until()).await;
        let rows: Vec<(String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT status_id, name, category FROM sensei.tracker_statuses WHERE tracker_id = $1 ORDER BY status_id",
        )
        .bind(t)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(rows, [("4".to_string(), "Shipped".to_string(), "done".to_string())]);
    }

    #[tokio::test]
    async fn change_values_are_stored_once_and_a_changed_ticket_is_read_again() {
        let pg = PgStore::test_store().await;
        let t = test_tracker(&pg).await;
        let task = one_task(&pg, t, "2026-09-10T00:00:00Z").await;
        let item = |id: &str, from: &str, to: &str, fs: &str, ts: &str| json!({"fieldId": id, "from": from, "to": to, "fromString": fs, "toString": ts});
        let log = || {
            terms(json!([])).with(
                "/rest/api/3/issue/10001/changelog",
                0,
                json!({"startAt": 0, "isLast": true, "values": [
                {"id": "h1", "created": "2026-09-01T10:00:00.000+0000", "items": [
                    item("customfield_10020", "", "100", "", "Sprint 1"),
                    item("status", "1", "3", "To Do", "Done"),
                    item("customfield_10016", "", "", "2", "5")]}]}),
            )
        };
        let b = log();
        assert_eq!(read(&pg, &b, &board(), t, &JiraAdapter, until()).await, HistoryOutcome::Done);
        type Change = (String, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>);
        let changes = || async {
            sqlx_core::query_as::query_as::<_, Change>(
                "SELECT field::text, from_value, to_value, at FROM sensei.tracker_task_changes WHERE task_id = $1 ORDER BY tracker_task_changes.field",
            )
            .bind(task)
            .fetch_all(pg.pool())
            .await
            .unwrap()
        };
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-01T10:00:00Z").unwrap().to_utc();
        assert_eq!(
            changes().await,
            [
                ("sprint".to_string(), None, Some("100".to_string()), at),
                ("status".to_string(), Some("1".to_string()), Some("3".to_string()), at),
                ("estimate".to_string(), Some("2".to_string()), Some("5".to_string()), at),
            ]
        );
        let (through,): (Option<chrono::DateTime<chrono::Utc>>,) = sqlx_core::query_as::query_as(
            "SELECT history_through FROM sensei.tracker_tasks WHERE id = $1",
        )
        .bind(task)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(
            through,
            Some(chrono::DateTime::parse_from_rfc3339("2026-09-10T00:00:00Z").unwrap().to_utc())
        );

        sqlx_core::query::query("UPDATE sensei.tracker_tasks SET issue_updated_at = issue_updated_at + interval '1 day' WHERE id = $1")
            .bind(task)
            .execute(pg.pool())
            .await
            .unwrap();
        let again = log();
        assert_eq!(
            read(&pg, &again, &board(), t, &JiraAdapter, until()).await,
            HistoryOutcome::Done
        );
        assert_eq!(
            again.calls().iter().filter(|c| c.ends_with("/changelog")).count(),
            1,
            "a changed ticket is read again"
        );
        assert_eq!(changes().await.len(), 3, "the same entries are not stored twice");
    }

    #[tokio::test]
    async fn a_successful_run_clears_the_last_error() {
        let pg = PgStore::test_store().await;
        let t = test_tracker(&pg).await;
        let refused = FakeBoard::empty()
            .fail("/rest/agile/1.0/board/42/configuration", TrackerError::Forbidden);
        read(&pg, &refused, &board(), t, &JiraAdapter, until()).await;
        read(&pg, &terms(json!([])), &board(), t, &JiraAdapter, until()).await;
        let (err,): (Option<String>,) = sqlx_core::query_as::query_as(
            "SELECT history_error::text FROM sensei.trackers WHERE id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(err, None);
    }

    /// S6: a ticket's estimate is trusted only when a sync asked for the field the
    /// board was learned to use; until then it is not synced (A6).
    #[tokio::test]
    async fn the_estimate_needs_a_sync_under_the_learned_field() {
        let pg = PgStore::test_store().await;
        let t = test_tracker(&pg).await;
        let task = one_task(&pg, t, "2026-09-10T00:00:00Z").await;
        let (w,): (uuid::Uuid,) =
            sqlx_core::query_as::query_as("SELECT workspace_id FROM sensei.trackers WHERE id = $1")
                .bind(t)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        let reason = || async {
            let figures =
                crate::sdlc::ticket::ticket_figures(&pg, t, w, &[task], &Default::default())
                    .await
                    .unwrap();
            let f = figures.get(&task).unwrap().clone();
            (f.estimated_points, f.reasons.get("estimated_points").map(|r| r.to_string()))
        };
        let set = |sql: &'static str| {
            let pg = &pg;
            async move { sqlx_core::query::query(sql).bind(t).execute(pg.pool()).await.unwrap() }
        };
        assert_eq!(reason().await, (None, Some("not_synced".into())), "nothing learned");
        set("UPDATE sensei.trackers SET estimation_field = 'customfield_10016', terms_at = now() WHERE id = $1").await;
        assert_eq!(
            reason().await,
            (None, Some("not_synced".into())),
            "learned, but no sync has asked for it yet"
        );
        set("UPDATE sensei.trackers SET points_field = 'customfield_10016' WHERE id = $1").await;
        assert_eq!(reason().await, (None, Some("not_estimated".into())));
        set("UPDATE sensei.tracker_tasks SET points = 3 WHERE tracker_id = $1").await;
        assert_eq!(reason().await, (Some(3.0), None));
        set("UPDATE sensei.trackers SET estimation_field = NULL, points_field = 'none' WHERE id = $1").await;
        assert_eq!(reason().await.1, Some("no_estimation_field".into()));
    }

    #[tokio::test]
    async fn a_tracker_whose_adapter_reads_no_history_is_skipped_untouched() {
        let pg = PgStore::test_store().await;
        let (t, _p) = crate::tracker::connect::tests::connected_tracker(&pg).await;
        let row = pg.tracker_by_id(t).await.unwrap().unwrap();
        let adapter =
            crate::tracker::test_support::leak(crate::tracker::test_support::FakeAdapter {
                capabilities: crate::tracker::Capabilities { history: false, writeback: false },
                ..crate::tracker::test_support::FakeAdapter::gitlab_like()
            });
        let io = crate::tracker::test_support::empty_pass_io();
        let locks = crate::tracker::sync::TrackerLocks::default();
        let guard = locks.try_lock(t).expect("the guard is free");
        let out = run(
            &pg,
            &io,
            t,
            &row.secret_service,
            adapter,
            std::time::Duration::from_secs(5),
            &guard,
        )
        .await;
        assert_eq!(out, HistoryOutcome::Skipped);
        let (err, at, terms): (
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        ) = sqlx_core::query_as::query_as(
            "SELECT history_error::text, history_at, terms_at FROM sensei.trackers WHERE id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!((err, at, terms), (None, None, None));
        // Control: a capable adapter on the same tracker gets past the guard and
        // fails at the (empty) keyring, recording it.
        let capable = crate::tracker::test_support::fake_adapter([String::new(), String::new()]);
        let out = run(
            &pg,
            &io,
            t,
            &row.secret_service,
            capable,
            std::time::Duration::from_secs(5),
            &guard,
        )
        .await;
        assert!(matches!(out, HistoryOutcome::Failed(_)), "{out:?}");
        let (err,): (Option<String>,) = sqlx_core::query_as::query_as(
            "SELECT history_error::text FROM sensei.trackers WHERE id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(err.is_some(), "the capable run recorded its failure");
    }
}
