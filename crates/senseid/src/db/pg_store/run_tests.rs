//! DB-touching CRUD tests for the relay run-state model. Each test is
//! self-contained: `project_id` is `None` (nullable FK), and every created
//! run is cascade-deleted at the end (`run_events` cascade with the run).
//! Guarded like the neighbouring pg_store tests — a missing test DB means
//! the test no-ops rather than fails.
use super::*;

async fn delete_run(pg: &PgStore, id: &uuid::Uuid) {
    sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
        .bind(id)
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn create_get_and_defaults() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    // Minimal create — plan_ref/max_concurrency fall back to DDL defaults.
    let id = pg.create_run(&NewRun::default()).await.unwrap();
    let run = pg.get_run(&id).await.unwrap().expect("run exists");
    assert_eq!(run.id, id);
    assert_eq!(run.project_id, None);
    assert_eq!(run.plan_ref, "", "plan_ref defaults to ''");
    assert_eq!(run.status, RelayRunStatus::Running, "status defaults to running");
    assert_eq!(run.max_concurrency, 1, "max_concurrency defaults to 1");
    assert!(run.paused_until.is_none());
    assert!(run.completed_at.is_none());
    assert!(run.started_at.contains('T'), "started_at is RFC-3339 text");
    assert!(run.created_at.contains('T'));

    // Unknown id → None, not an error.
    assert!(pg.get_run(&uuid::Uuid::new_v4()).await.unwrap().is_none());

    delete_run(&pg, &id).await;
}

#[tokio::test]
async fn create_with_fields() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let session = uuid::Uuid::new_v4();
    let id = pg
        .create_run(&NewRun {
            project_id: None,
            plan_ref: Some("docs/plan/P3.md".into()),
            goal: Some("ship relay".into()),
            dojo_session_id: Some(session),
            max_concurrency: Some(3),
            author_name: Some("Sensei HQ".into()),
            author_email: Some("dev@sensei-hq.com".into()),
            plan_graph: Some(serde_json::json!({
                "phases": [{ "title": "P", "tasks": [{ "id": "t1", "title": "x" }] }]
            })),
        })
        .await
        .unwrap();
    let run = pg.get_run(&id).await.unwrap().unwrap();
    assert_eq!(run.plan_ref, "docs/plan/P3.md");
    assert_eq!(run.goal.as_deref(), Some("ship relay"));
    assert_eq!(run.dojo_session_id, Some(session));
    assert_eq!(
        pg.run_author(&id).await.unwrap(),
        (Some("Sensei HQ".into()), Some("dev@sensei-hq.com".into())),
        "create_run stamps + run_author reads the git author back"
    );
    assert_eq!(run.max_concurrency, 3);
    // plan_graph stored + read back on demand (off the 16-col RUN_SELECT).
    let g = pg.run_plan_graph(&id).await.unwrap().expect("plan_graph stored");
    assert_eq!(g["phases"][0]["tasks"][0]["id"], serde_json::json!("t1"));
    // set_run_plan_graph overwrites it (the update_task_status write-back path).
    pg.set_run_plan_graph(&id, &serde_json::json!({ "phases": [] })).await.unwrap();
    assert_eq!(pg.run_plan_graph(&id).await.unwrap().unwrap(), serde_json::json!({ "phases": [] }));
    delete_run(&pg, &id).await;
}

#[tokio::test]
async fn set_run_dojo_session_id_persists_the_cloud_join() {
    // The P1 run→relay bridge persists the cloud session id after the first
    // successful publish, so the local run joins to its relay session.
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let id = pg.create_run(&NewRun::default()).await.unwrap();
    // Fresh run has no cloud session yet.
    assert!(pg.get_run(&id).await.unwrap().unwrap().dojo_session_id.is_none());

    let cloud = uuid::Uuid::new_v4();
    pg.set_run_dojo_session_id(&id, &cloud).await.unwrap();
    assert_eq!(
        pg.get_run(&id).await.unwrap().unwrap().dojo_session_id,
        Some(cloud),
        "the cloud session id is persisted onto the run"
    );

    delete_run(&pg, &id).await;
}

#[tokio::test]
async fn status_pause_progress_heartbeat_complete() {
    // Holds the shared resume lock: this test asserts a run STAYS paused, but
    // the global resume_due_runs (scheduler tests) would resume any paused run
    // whose paused_until has elapsed — so serialize against those callers.
    let _guard = crate::runs::resume_test_guard();
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let id = pg.create_run(&NewRun::default()).await.unwrap();

    // Pause with a FAR-FUTURE resume time + reason (never "due", so the
    // global resume sweep can't flip it mid-assertion).
    pg.update_run_status(
        &id,
        RelayRunStatus::Paused,
        Some("2999-07-17T11:29:00Z"),
        Some("weekly cap"),
    )
    .await
    .unwrap();
    let run = pg.get_run(&id).await.unwrap().unwrap();
    assert_eq!(run.status, RelayRunStatus::Paused);
    assert!(run.paused_until.as_deref().unwrap().contains("2999-07-17"));
    assert_eq!(run.pause_reason.as_deref(), Some("weekly cap"));

    // Resume clears the pause fields.
    pg.update_run_status(&id, RelayRunStatus::Running, None, None).await.unwrap();
    let run = pg.get_run(&id).await.unwrap().unwrap();
    assert_eq!(run.status, RelayRunStatus::Running);
    assert!(run.paused_until.is_none());
    assert!(run.pause_reason.is_none());

    // Progress markers.
    pg.set_run_progress(&id, Some("P3"), Some("run-state model")).await.unwrap();
    let run = pg.get_run(&id).await.unwrap().unwrap();
    assert_eq!(run.current_phase.as_deref(), Some("P3"));
    assert_eq!(run.current_feature.as_deref(), Some("run-state model"));

    // Heartbeat sets heartbeat_at.
    assert!(run.heartbeat_at.is_none());
    pg.touch_run_heartbeat(&id).await.unwrap();
    let run = pg.get_run(&id).await.unwrap().unwrap();
    assert!(run.heartbeat_at.as_deref().unwrap().contains('T'));

    // Terminal completion stamps completed_at.
    pg.complete_run(&id, RelayRunStatus::Done).await.unwrap();
    let run = pg.get_run(&id).await.unwrap().unwrap();
    assert_eq!(run.status, RelayRunStatus::Done);
    assert!(run.completed_at.as_deref().unwrap().contains('T'));

    delete_run(&pg, &id).await;
}

#[tokio::test]
async fn list_active_runs_filters_by_status() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let active = pg.create_run(&NewRun::default()).await.unwrap(); // running
    let paused = pg.create_run(&NewRun::default()).await.unwrap();
    pg.update_run_status(&paused, RelayRunStatus::Paused, None, None).await.unwrap();
    // A blocked run (waiting on a gate) must stay in the active set so the
    // scheduler keeps heartbeating it and GET /api/runs keeps showing it —
    // otherwise once P3.3 sets status='blocked' the run drops out and looks
    // crashed. (The advance_run handler has a Blocked-heartbeat branch.)
    let blocked = pg.create_run(&NewRun::default()).await.unwrap();
    pg.update_run_status(&blocked, RelayRunStatus::Blocked, None, None).await.unwrap();
    let terminal = pg.create_run(&NewRun::default()).await.unwrap();
    pg.complete_run(&terminal, RelayRunStatus::Done).await.unwrap();

    let ids: std::collections::HashSet<uuid::Uuid> =
        pg.list_active_runs().await.unwrap().into_iter().map(|r| r.id).collect();
    assert!(ids.contains(&active), "running run is active");
    assert!(ids.contains(&paused), "paused run is active");
    assert!(ids.contains(&blocked), "blocked run is active");
    assert!(!ids.contains(&terminal), "done run is excluded");

    for id in [active, paused, blocked, terminal] {
        delete_run(&pg, &id).await;
    }
}

#[tokio::test]
async fn append_and_list_events_newest_first() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let id = pg.create_run(&NewRun::default()).await.unwrap();

    let e1 = pg
        .append_run_event(&id, RunEventKind::PhaseStarted, Some("P3"), None, &serde_json::json!({}))
        .await
        .unwrap();
    let e2 = pg
        .append_run_event(
            &id,
            RunEventKind::PausedOnLimit,
            Some("P3"),
            Some("run-state"),
            &serde_json::json!({ "reset_at": "2026-07-17T11:29:00Z" }),
        )
        .await
        .unwrap();
    assert!(e2 > e1, "bigserial is monotonic");

    let events = pg.list_run_events(&id, 10).await.unwrap();
    assert_eq!(events.len(), 2);
    // Newest first — the paused_on_limit event leads.
    assert_eq!(events[0].id, e2);
    assert_eq!(events[0].kind, RunEventKind::PausedOnLimit);
    assert_eq!(events[0].feature.as_deref(), Some("run-state"));
    assert_eq!(events[0].detail["reset_at"], serde_json::json!("2026-07-17T11:29:00Z"));
    assert_eq!(events[1].kind, RunEventKind::PhaseStarted);
    assert_eq!(events[1].detail, serde_json::json!({}), "detail defaults to {{}}");
    assert!(events[0].created_at.contains('T'));

    // limit caps the result.
    assert_eq!(pg.list_run_events(&id, 1).await.unwrap().len(), 1);

    delete_run(&pg, &id).await; // cascades run_events
    assert!(pg.list_run_events(&id, 10).await.unwrap().is_empty());
}

#[tokio::test]
async fn resume_due_runs_flips_only_elapsed_pauses() {
    // resume_due_runs is a global UPDATE; serialize with the scheduler test
    // that also creates due-paused runs (see runs::resume_test_guard).
    let _guard = crate::runs::resume_test_guard();
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    // Clear any stray due pauses so our set assertions are exact.
    pg.resume_due_runs().await.unwrap();

    // Due: paused with paused_until in the past → should resume.
    let due = pg.create_run(&NewRun::default()).await.unwrap();
    pg.update_run_status(
        &due,
        RelayRunStatus::Paused,
        Some("2000-01-01T00:00:00Z"),
        Some("elapsed cap"),
    )
    .await
    .unwrap();

    // Not-yet-due: paused with paused_until far in the future.
    let future = pg.create_run(&NewRun::default()).await.unwrap();
    pg.update_run_status(
        &future,
        RelayRunStatus::Paused,
        Some("2999-01-01T00:00:00Z"),
        Some("weekly cap"),
    )
    .await
    .unwrap();

    // Indefinite: paused with NULL paused_until (manual pause) → never auto-resumes.
    let indefinite = pg.create_run(&NewRun::default()).await.unwrap();
    pg.update_run_status(&indefinite, RelayRunStatus::Paused, None, None).await.unwrap();

    let resumed: std::collections::HashSet<uuid::Uuid> =
        pg.resume_due_runs().await.unwrap().into_iter().collect();
    assert!(resumed.contains(&due), "elapsed pause resumes");
    assert!(!resumed.contains(&future), "future pause stays paused");
    assert!(!resumed.contains(&indefinite), "indefinite pause stays paused");

    // The due run is now running with its pause fields cleared.
    let run = pg.get_run(&due).await.unwrap().unwrap();
    assert_eq!(run.status, RelayRunStatus::Running);
    assert!(run.paused_until.is_none(), "paused_until cleared on resume");
    assert!(run.pause_reason.is_none(), "pause_reason cleared on resume");
    // The future run is untouched.
    assert_eq!(pg.get_run(&future).await.unwrap().unwrap().status, RelayRunStatus::Paused);

    // Idempotent: a second call resumes nothing (nothing left due).
    assert!(pg.resume_due_runs().await.unwrap().into_iter().all(|id| id != due));

    for id in [due, future, indefinite] {
        delete_run(&pg, &id).await;
    }
}
