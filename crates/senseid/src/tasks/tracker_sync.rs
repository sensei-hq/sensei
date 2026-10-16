//! The scheduled tracker sync (`tracker_sync`, spec R5 scheduling): drain due
//! secret tombstones, then sync every tracker. One tracker's failure is recorded
//! on its row and never stops the others; the tick reports failure if any failed.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::tracker::runtime::TrackerRuntime;
use crate::tracker::sync::SyncError;

/// One scheduled pass.
pub async fn tick(pg: Arc<PgStore>, rt: Arc<TrackerRuntime>) -> Result<(), String> {
    tick_scoped(&pg, &rt, None).await
}

/// The pass over `only` (all trackers when `None`). Tests scope it to their own
/// trackers so a tick never touches another test's rows.
pub async fn tick_scoped(
    pg: &PgStore,
    rt: &TrackerRuntime,
    only: Option<&[uuid::Uuid]>,
) -> Result<(), String> {
    if let Err(e) = crate::tracker::connect::drain_tombstones(pg, rt.secrets.clone()).await {
        tracing::warn!("tracker_sync: tombstone drain failed: {e}");
    }
    let rows: Vec<_> = pg
        .list_trackers()
        .await?
        .into_iter()
        .filter(|r| only.is_none_or(|ids| ids.contains(&r.id)))
        .collect();
    let mut failed = 0usize;
    for row in &rows {
        match crate::tracker::runtime::sync_stored(pg, rt, row).await {
            // Busy: a manual sync is running; Gone: disconnected meanwhile.
            Ok(_) | Err(SyncError::Busy) | Err(SyncError::Gone) => {}
            Err(e) => {
                failed += 1;
                tracing::warn!(tracker_id = %row.id, "tracker_sync: {e:?}");
            }
        }
    }
    if failed > 0 { Err(format!("{failed}/{} trackers failed", rows.len())) } else { Ok(()) }
}

/// Spawn the scheduled loop. Cadence lives in `sensei.schedules` (`tracker_sync`).
pub fn spawn(pg: Arc<PgStore>, rt: Arc<TrackerRuntime>) {
    tokio::spawn(async move {
        crate::tasks::ticker::run_scheduled(pg.clone(), "tracker_sync", move || {
            let (pg, rt) = (pg.clone(), rt.clone());
            async move { tick(pg, rt).await }
        })
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::connect::connect;
    use crate::tracker::jira::JiraAdapter;
    use crate::tracker::runtime::test_support::fake_runtime;
    use crate::tracker::test_support::FakeBoard;
    use crate::tracker::{ApiToken, ConnectParams, TrackerError};

    async fn connected(pg: &PgStore, rt: &TrackerRuntime, site: &str) -> uuid::Uuid {
        let p = pg
            .create_project(&format!("_test:tracker-tick:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let params = ConnectParams {
            base_url: site.into(),
            board_ref: "42".into(),
            account: "dev@example.com".into(),
            token: ApiToken("t".into()),
            project_key: None,
        };
        connect(pg, rt.secrets.clone(), &JiraAdapter, &FakeBoard::standard(), p, params)
            .await
            .unwrap()
            .tracker_id
    }
    fn site() -> String {
        format!("https://t{}.atlassian.net", uuid::Uuid::new_v4().simple())
    }

    #[tokio::test]
    async fn one_failing_tracker_does_not_stop_the_others_and_fails_the_tick() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (rt, _store, factory) = fake_runtime();
        let (good, bad) = (site(), site());
        let good_id = connected(&pg, &rt, &good).await;
        let bad_id = connected(&pg, &rt, &bad).await;
        factory.serve(&good, FakeBoard::standard());
        factory.serve(
            &bad,
            FakeBoard::standard().fail("/rest/agile/1.0/board/42", TrackerError::Unauthorized),
        );

        let r = tick_scoped(&pg, &rt, Some(&[good_id, bad_id])).await;
        let err = r.expect_err("a failed tracker fails the tick");
        assert!(err.contains("trackers failed"), "{err}");
        let good_row = pg.tracker_by_id(good_id).await.unwrap().unwrap();
        let bad_row = pg.tracker_by_id(bad_id).await.unwrap().unwrap();
        assert!(good_row.last_success_at.is_some() && good_row.last_error.is_none());
        assert_eq!(bad_row.last_error.as_deref(), Some("unauthorized"));
    }

    #[tokio::test]
    async fn a_missing_secret_is_recorded_as_token_missing() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (rt, store, factory) = fake_runtime();
        let s = site();
        let id = connected(&pg, &rt, &s).await;
        factory.serve(&s, FakeBoard::standard());
        store.entries.lock().unwrap().clear();
        let _ = tick_scoped(&pg, &rt, Some(&[id])).await;
        assert_eq!(
            pg.tracker_by_id(id).await.unwrap().unwrap().last_error.as_deref(),
            Some("token_missing")
        );
    }

    #[tokio::test]
    async fn the_tick_drains_due_tombstones() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (rt, store, _factory) = fake_runtime();
        let service = format!("com.sensei.tracker.jira.tick-{}", uuid::Uuid::new_v4());
        pg.write_pending_tombstone(
            &service,
            "a",
            chrono::Utc::now() - chrono::Duration::seconds(1),
        )
        .await
        .unwrap();
        let _ = tick_scoped(&pg, &rt, Some(&[])).await;
        let (left,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
        )
        .bind(&service)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(left, 0);
        let _ = store;
    }
}
