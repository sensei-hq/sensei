use super::*;

// ── activity.task_failures — the restart list ─────────────────────

/// The restart list must carry what is STILL broken, not what has ever broken.
///
/// A job that failed and later succeeded is fixed; listing it would put finished
/// work back on the queue. So the view keeps a job only when its most recent
/// execution is the failure — and it reports `attempts` (failures accumulated)
/// beside `max_retry` (the runner's counter), because the gap between them is
/// the poison-pill signal: a job re-discovered and re-failed each pass climbs
/// `attempts` while `max_retry` stays flat. That shape produced 17,577,049 rows
/// over 43,312 paths before the stage-3 barrier reached production.
#[tokio::test]
async fn task_failures_lists_only_jobs_whose_latest_run_failed_and_counts_their_attempts() {
    let s = pg_store().await;
    let fp = format!("/_test/failures/{}", uuid::Uuid::new_v4());
    let kind = crate::tasks::TaskKind::ProcessFile.to_string();

    // "stuck" — failed twice and never recovered. Belongs on the restart list,
    // with both failures counted.
    for attempt in 0..2 {
        let id =
            s.start_task_execution(1, None, &kind, &fp, "src/stuck.rs", attempt).await.unwrap();
        s.fail_task_execution(&id, 5, "boom: no files row for src/stuck.rs, metric churn")
            .await
            .unwrap();
    }

    // "recovered" — failed once, then succeeded. Must NOT appear.
    let failed = s.start_task_execution(2, None, &kind, &fp, "recovered.rs", 0).await.unwrap();
    s.fail_task_execution(&failed, 5, "boom: transient").await.unwrap();
    let ok = s.start_task_execution(2, None, &kind, &fp, "recovered.rs", 1).await.unwrap();
    s.complete_task_execution(&ok, 1, 5).await.unwrap();

    let rows: Vec<(String, i64, i32)> = sqlx_core::query_as::query_as(
        "SELECT path, attempts, max_retry FROM activity.task_failures \
          WHERE folder_path = $1 ORDER BY path",
    )
    .bind(&fp)
    .fetch_all(s.pool())
    .await
    .unwrap();

    assert_eq!(
        rows.iter().map(|(p, _, _)| p.as_str()).collect::<Vec<_>>(),
        vec!["src/stuck.rs"],
        "only the job whose latest run failed — a recovered job is finished work, not a restart"
    );
    assert_eq!(rows[0].1, 2, "both failures counted, so a poison pill is visible as one number");
    assert_eq!(rows[0].2, 1, "beside the runner's own retry counter");

    // The signature collapses the path out, so many files failing one way group
    // as one cause rather than one cause per file.
    let (sig,): (String,) = sqlx_core::query_as::query_as(
        "SELECT error_signature FROM activity.task_failures WHERE folder_path = $1 LIMIT 1",
    )
    .bind(&fp)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(!sig.contains("src/stuck.rs"), "path collapsed out of the signature, got {sig:?}");
    // And the NON-path subject survives, deliberately. `path` is overloaded the
    // way `folder_path` is — 32 of the live failing jobs carry a metric name
    // (`churn`, `duplication`, `quality`) rather than a file. Those are the
    // subject, like an fqn or a language, and collapsing them would merge
    // genuinely different failures into one unreadable group.
    assert!(sig.contains("metric churn"), "a non-path subject is kept, got {sig:?}");

    sqlx_core::query::query("DELETE FROM activity.task_executions WHERE folder_path = $1")
        .bind(&fp)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Task executions — boot reconcile (D6b) ────────────────────────

#[tokio::test]
async fn reconcile_orphaned_task_executions_terminates_only_prior_session_running() {
    // D6b: on boot, a `running` task_execution row left over from a dead
    // daemon session (started before this session's start time) can never
    // complete — its in-memory task is gone. Reconcile flips it to a
    // terminal `failed`; a row from THIS session (started at/after the
    // cutoff) and an already-terminal row are both left untouched.
    let s = pg_store().await;
    let fp = format!("/_test/reconcile/{}", uuid::Uuid::new_v4());

    // A — orphaned: running, started well before the cutoff (prior session).
    let a = s
        .start_task_execution(
            1,
            None,
            &crate::tasks::TaskKind::ProcessFile.to_string(),
            &fp,
            "a",
            0,
        )
        .await
        .unwrap();
    sqlx_core::query::query(
        "UPDATE activity.task_executions SET started_at = now() - interval '2 hours' WHERE id = $1",
    )
    .bind(a)
    .execute(s.pool())
    .await
    .unwrap();
    // B — this session: running, started at now() (after the cutoff).
    let b = s
        .start_task_execution(
            2,
            None,
            &crate::tasks::TaskKind::ProcessFile.to_string(),
            &fp,
            "b",
            0,
        )
        .await
        .unwrap();
    // C — already terminal from a prior session: must not be re-touched.
    let c = s
        .start_task_execution(
            3,
            None,
            &crate::tasks::TaskKind::ProcessFile.to_string(),
            &fp,
            "c",
            0,
        )
        .await
        .unwrap();
    sqlx_core::query::query(
            "UPDATE activity.task_executions SET status = 'completed', started_at = now() - interval '2 hours' WHERE id = $1")
            .bind(c).execute(s.pool()).await.unwrap();

    // Cutoff sits between the prior-session rows (−2h) and this session (now).
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
    // D — boundary: running, started EXACTLY at the cutoff. The sweep is
    // exclusive (`started_at < cutoff`), so a row at session_start belongs
    // to this session and must be left running — locks the `<` vs `<=` line.
    let d = s
        .start_task_execution(
            4,
            None,
            &crate::tasks::TaskKind::ProcessFile.to_string(),
            &fp,
            "d",
            0,
        )
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE activity.task_executions SET started_at = $2 WHERE id = $1")
        .bind(d)
        .bind(cutoff)
        .execute(s.pool())
        .await
        .unwrap();

    let n = s.reconcile_orphaned_task_executions(cutoff).await.unwrap();
    assert!(n >= 1, "at least the one orphaned running row is reconciled, got {n}");

    let (a_status, a_completed, a_err): (
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
    ) = query_as(
        "SELECT status, completed_at, error_message FROM activity.task_executions WHERE id = $1",
    )
    .bind(a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(a_status, "failed", "orphaned running row is marked failed");
    assert!(a_completed.is_some(), "reconciled row gets a completed_at");
    assert!(a_err.is_some(), "reconciled row records why it was terminated");

    let (b_status, b_completed): (String, Option<chrono::DateTime<chrono::Utc>>) =
        query_as("SELECT status, completed_at FROM activity.task_executions WHERE id = $1")
            .bind(b)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(b_status, "running", "this session's in-flight row is left running");
    assert!(b_completed.is_none(), "this session's row keeps a null completed_at");

    let (c_status,): (String,) =
        query_as("SELECT status FROM activity.task_executions WHERE id = $1")
            .bind(c)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(c_status, "completed", "an already-terminal row is not re-touched");

    let (d_status,): (String,) =
        query_as("SELECT status FROM activity.task_executions WHERE id = $1")
            .bind(d)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(d_status, "running", "a row exactly at the cutoff is this session's — left running");

    // Cleanup.
    sqlx_core::query::query("DELETE FROM activity.task_executions WHERE folder_path = $1")
        .bind(&fp)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn rollup_keeps_failures_raw_and_is_idempotent() {
    // The execution log grew to 4.8M rows / 1.5 GB in 69 days with nothing
    // pruning it. Retention rolls old rows into a daily bucket and deletes
    // them — except failures, whose error_message is the reason to keep a
    // log at all.
    let s = pg_store().await;
    let fp = format!("/_test/rollup/{}", uuid::Uuid::new_v4());
    let kind = crate::tasks::TaskKind::ProcessFile.to_string();

    // Three old rows: two completed, one failed. All past the 14d window.
    for (tid, status) in [(901i64, "completed"), (902, "completed"), (903, "failed")] {
        let id = s.start_task_execution(tid, None, &kind, &fp, "x", 0).await.unwrap();
        sqlx_core::query::query(
            "UPDATE activity.task_executions \
                    SET started_at = now() - interval '30 days', status = $2, duration_ms = 10 \
                  WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .execute(s.pool())
        .await
        .unwrap();
    }

    let (rolled, pruned) = s.rollup_and_prune_task_executions(14, 90).await.unwrap();
    assert!(rolled >= 1 && pruned >= 2, "rolled={rolled} pruned={pruned}");

    let remaining: (i64,) =
        query_as("SELECT count(*) FROM activity.task_executions WHERE folder_path = $1")
            .bind(&fp)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(remaining.0, 1, "the FAILED row survives; the completed ones are rolled up");

    let (status,): (String,) =
        query_as("SELECT status FROM activity.task_executions WHERE folder_path = $1")
            .bind(&fp)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(status, "failed", "and it is the failure that was kept");

    // Idempotency is the load-bearing property: the raw rows are GONE after
    // the first pass, so an additive upsert would double-count on every
    // re-run with no source left to correct it from.
    let day_runs = |s: PgStore, fp: String| async move {
        let r: (i64,) = query_as(
            "SELECT coalesce(sum(runs),0)::int8 FROM activity.task_execution_daily d \
                  WHERE d.day = (now() - interval '30 days')::date \
                    AND d.task_kind::text = 'process_file'",
        )
        .fetch_one(s.pool())
        .await
        .unwrap();
        let _ = fp;
        r.0
    };
    let first = day_runs(s.clone(), fp.clone()).await;
    s.rollup_and_prune_task_executions(14, 90).await.unwrap();
    let second = day_runs(s.clone(), fp.clone()).await;
    assert_eq!(first, second, "re-running the rollup must not double-count");

    sqlx_core::query::query("DELETE FROM activity.task_executions WHERE folder_path = $1")
        .bind(&fp)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn start_task_execution_records_retry_number() {
    // D6c: a re-driven task carries its attempt count, and the execution
    // record must persist it (`task_executions.retry_number`, currently
    // always 0) so retries are observable on the logs/health screen.
    let s = pg_store().await;
    let fp = format!("/_test/retrynum/{}", uuid::Uuid::new_v4());
    let id = s
        .start_task_execution(
            77,
            None,
            &crate::tasks::TaskKind::ProcessFile.to_string(),
            &fp,
            "a.rs",
            2,
        )
        .await
        .unwrap();

    let (rn,): (i32,) = query_as("SELECT retry_number FROM activity.task_executions WHERE id = $1")
        .bind(id)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(rn, 2, "the recorded retry_number matches the attempt");

    sqlx_core::query::query("DELETE FROM activity.task_executions WHERE folder_path = $1")
        .bind(&fp)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Schedules · the code↔table agreement ────────────────────────────────────
//
// `sensei.schedules` is user-editable, and `tasks::schedule::SCHEDULABLE` is the
// code-side list of workers that actually exist. They must agree in BOTH
// directions, and this is the test `api/handlers/scheduled_tasks.rs` asks for in
// its own comment — "Registry, not reflection — keep in step when a worker is
// added" — turned from a hope into a build failure.
//
// Mirrors the existing TaskKind::ALL ↔ sensei.task_execution_kind test.

#[tokio::test]
async fn every_schedulable_worker_has_a_schedule_row() {
    // A worker with no row never runs, and nothing would say so.
    let s = pg_store().await;
    let rows: Vec<(String,)> =
        query_as("SELECT name FROM sensei.schedules").fetch_all(s.pool()).await.unwrap();
    let seeded: Vec<String> = rows.into_iter().map(|(n,)| n).collect();
    for name in crate::tasks::schedule::SCHEDULABLE {
        assert!(
            seeded.iter().any(|n| n == name),
            "{name} is schedulable in code but has no sensei.schedules row — it would \
             never run. Add it to database/import/staging/schedules.jsonl."
        );
    }
}

#[tokio::test]
async fn every_schedule_row_names_a_real_worker() {
    // A row naming no worker is a typo that silently does nothing.
    let s = pg_store().await;
    // The scan is table-wide, so it also sees the throwaway rows of any test
    // running beside it. Those are skipped by prefix — and a worker may not take
    // the prefix, or the skip would hide the very drift this test exists for.
    for name in crate::tasks::schedule::SCHEDULABLE {
        assert!(
            !name.starts_with(crate::tasks::test_support::TEST_SCHEDULE_PREFIX),
            "{name} may not start with the test-row prefix — the scan below would skip it"
        );
    }
    let rows: Vec<(String,)> =
        query_as("SELECT name FROM sensei.schedules").fetch_all(s.pool()).await.unwrap();
    for (name,) in rows {
        if name.starts_with(crate::tasks::test_support::TEST_SCHEDULE_PREFIX) {
            continue;
        }
        assert!(
            crate::tasks::schedule::SCHEDULABLE.contains(&name.as_str()),
            "sensei.schedules has a row for {name:?}, but no such worker exists in \
             tasks::schedule::SCHEDULABLE — either add the worker or remove the row."
        );
    }
}

#[tokio::test]
async fn a_zero_interval_is_rejected_by_the_database() {
    // Not by a runtime fallback: a zero interval busy-loops a core, so it must be
    // unrepresentable rather than quietly corrected.
    let s = pg_store().await;
    let err = sqlx_core::query::query(
        "INSERT INTO sensei.schedules(name, interval_secs) VALUES('_test:zero', 0)",
    )
    .execute(s.pool())
    .await
    .unwrap_err();
    assert!(
        err.to_string().contains("interval_secs"),
        "a zero interval must violate the CHECK, got: {err}"
    );
}

#[tokio::test]
async fn a_day_outside_monday_to_sunday_is_rejected() {
    // ISO 1..7. An 8 would match no weekday and the task would silently never run.
    let s = pg_store().await;
    let err = sqlx_core::query::query(
        "INSERT INTO sensei.schedules(name, interval_secs, days) \
         VALUES('_test:day8', 60, ARRAY[8]::smallint[])",
    )
    .execute(s.pool())
    .await
    .unwrap_err();
    assert!(err.to_string().contains("days"), "day 8 must violate the CHECK, got: {err}");
}

// ── Schedules · load + record a run ─────────────────────────────────────────

#[tokio::test]
async fn load_schedule_reads_the_row_a_worker_runs_on() {
    let s = pg_store().await;
    // `metrics` is seeded, so this is the real path a worker takes on boot.
    let sched = s.load_schedule("metrics").await.unwrap().expect("metrics is seeded");
    assert_eq!(sched.name, "metrics");
    assert!(sched.enabled);
    assert!(sched.interval_secs > 0, "the CHECK guarantees this; the loader must not zero it");
}

#[tokio::test]
async fn load_schedule_is_none_for_an_unknown_worker() {
    // None, not a fabricated default: a worker with no row must be visibly
    // unscheduled rather than silently running on an invented cadence.
    let s = pg_store().await;
    assert!(s.load_schedule("_test:no-such-worker").await.unwrap().is_none());
}

#[tokio::test]
async fn marking_a_run_records_the_outcome_and_clears_a_stale_error() {
    let s = pg_store().await;
    let name = test_schedule_name();
    sqlx_core::query::query("INSERT INTO sensei.schedules(name, interval_secs) VALUES($1, 60)")
        .bind(&name)
        .execute(s.pool())
        .await
        .unwrap();

    // A failure is recorded with its reason.
    s.mark_schedule_run(&name, Err("boom".into()), 10).await.unwrap();
    let after_fail = s.load_schedule(&name).await.unwrap().unwrap();
    assert_eq!(after_fail.last_ok, Some(false));
    assert_eq!(after_fail.last_error.as_deref(), Some("boom"));
    assert!(after_fail.last_run_at.is_some(), "an attempt is a run, even a failed one");

    // A later success CLEARS it — a stale error sitting beside a healthy run
    // reads as "still broken" to anyone scanning the table.
    s.mark_schedule_run(&name, Ok(()), 10).await.unwrap();
    let after_ok = s.load_schedule(&name).await.unwrap().unwrap();
    assert_eq!(after_ok.last_ok, Some(true));
    assert_eq!(after_ok.last_error, None, "success must clear the previous error");

    sqlx_core::query::query("DELETE FROM sensei.schedules WHERE name = $1")
        .bind(&name)
        .execute(s.pool())
        .await
        .ok();
}

/// #96: how long a scheduled pass takes, on average. `None` until a pass has
/// been timed — runs from before durations were recorded are not invented.
#[tokio::test]
async fn a_schedule_averages_the_duration_of_its_timed_runs() {
    let s = pg_store().await;
    let name = test_schedule_name();
    sqlx_core::query::query("INSERT INTO sensei.schedules(name, interval_secs) VALUES($1, 60)")
        .bind(&name)
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(s.load_schedule(&name).await.unwrap().unwrap().avg_ms, None, "nothing timed yet");

    s.mark_schedule_run(&name, Ok(()), 100).await.unwrap();
    s.mark_schedule_run(&name, Err("slow".into()), 300).await.unwrap();
    assert_eq!(s.load_schedule(&name).await.unwrap().unwrap().avg_ms, Some(200));

    sqlx_core::query::query("DELETE FROM sensei.schedules WHERE name = $1")
        .bind(&name)
        .execute(s.pool())
        .await
        .ok();
}

/// #283: a pass in progress is visible — `running_since` is set when it starts
/// and cleared when it ends — and an overrun is a recorded failure that does not
/// pretend the pass completed (`last_run_at` is untouched until it does).
#[tokio::test]
async fn a_pass_in_progress_and_its_overrun_are_recorded() {
    let s = pg_store().await;
    let name = test_schedule_name();
    sqlx_core::query::query("INSERT INTO sensei.schedules(name, interval_secs) VALUES($1, 60)")
        .bind(&name)
        .execute(s.pool())
        .await
        .unwrap();

    s.mark_schedule_started(&name).await.unwrap();
    let running = s.load_schedule(&name).await.unwrap().unwrap();
    assert!(running.running_since.is_some(), "a started pass is visible");

    s.mark_schedule_overrun(&name, "still running after 1800s").await.unwrap();
    let over = s.load_schedule(&name).await.unwrap().unwrap();
    assert_eq!(over.last_ok, Some(false));
    assert_eq!(over.last_error.as_deref(), Some("still running after 1800s"));
    assert!(over.running_since.is_some(), "an overrun pass is still running");
    assert_eq!(over.last_run_at, None, "an overrun is not a completed pass");

    s.mark_schedule_run(&name, Ok(()), 5).await.unwrap();
    let done = s.load_schedule(&name).await.unwrap().unwrap();
    assert_eq!(done.running_since, None, "finishing clears it");
    assert_eq!(done.last_ok, Some(true), "the pass's own outcome replaces the overrun");

    s.mark_schedule_started(&name).await.unwrap();
    s.clear_schedule_running(&name).await.unwrap();
    assert_eq!(
        s.load_schedule(&name).await.unwrap().unwrap().running_since,
        None,
        "a boot clears a pass the last daemon left"
    );

    sqlx_core::query::query("DELETE FROM sensei.schedules WHERE name = $1")
        .bind(&name)
        .execute(s.pool())
        .await
        .ok();
}

// ── Schedules · reading them all, and editing one ───────────────────────────

/// `(interval_secs, modified_at)` for a schedule, straight from SQL — the two
/// values the seed guard compares. `table` selects the live row or the staged
/// datafile row, so a test can assert against what the deploy will actually
/// bring rather than against a hard-coded copy of the datafile.
async fn schedule_state(
    s: &PgStore,
    table: &str,
    name: &str,
) -> (i32, chrono::DateTime<chrono::Utc>) {
    query_as(&format!("SELECT interval_secs, modified_at FROM {table} WHERE name = $1"))
        .bind(name)
        .fetch_one(s.pool())
        .await
        .unwrap()
}

/// Force a schedule's `(interval_secs, modified_at)` — the two halves of the
/// seed guard — without going through the patch path that bumps the timestamp.
async fn set_schedule_row(s: &PgStore, name: &str, state: (i32, chrono::DateTime<chrono::Utc>)) {
    sqlx_core::query::query(
        "UPDATE sensei.schedules SET interval_secs = $2, modified_at = $3 WHERE name = $1",
    )
    .bind(name)
    .bind(state.0)
    .bind(state.1)
    .execute(s.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn list_schedules_returns_every_worker_with_its_state() {
    // The read behind GET /api/tasks/scheduled. It must return the TABLE, not a
    // code-side list — that static registry is what this slice retires.
    let s = pg_store().await;
    // Two probes, inserted in REVERSE name order. Without them the ordering
    // claim below only bites while this heap happens to be out of order: a
    // freshly deployed table is written in the datafile's own alphabetical
    // order, so an unsorted seq scan would return exactly the sorted list and
    // the assertion would pass against a query that had lost its ORDER BY. They
    // share a stem, so their relative order is the same under every collation.
    let stem = test_schedule_name();
    let (early, late) = (format!("{stem}:a"), format!("{stem}:b"));
    for name in [&late, &early] {
        sqlx_core::query::query("INSERT INTO sensei.schedules(name, interval_secs) VALUES($1, 60)")
            .bind(name)
            .execute(s.pool())
            .await
            .unwrap();
    }

    let all = s.list_schedules().await.unwrap();
    for name in crate::tasks::schedule::SCHEDULABLE {
        let row = all
            .iter()
            .find(|r| r.name == *name)
            .unwrap_or_else(|| panic!("{name} is schedulable but missing from list_schedules"));
        assert!(row.interval_secs > 0, "{name} must carry the cadence the CHECK guarantees");
    }
    let names: Vec<&str> = all.iter().map(|r| r.name.as_str()).collect();
    let pos = |n: &str| names.iter().position(|x| *x == n).unwrap_or_else(|| panic!("{n} missing"));
    assert!(
        pos(&early) < pos(&late),
        "the row written LAST but sorting first must come back first"
    );
    // Workers only for the whole-list claim: a throwaway row belongs to whatever
    // test is running beside this one, and the database's collation need not
    // agree with Rust's byte order on a punctuated name.
    let workers: Vec<&str> =
        names.iter().copied().filter(|n| !n.starts_with(TEST_SCHEDULE_PREFIX)).collect();
    let sorted = {
        let mut n = workers.clone();
        n.sort_unstable();
        n
    };
    assert_eq!(workers, sorted, "rows come back in name order so the UI needs no sort");

    for name in [&early, &late] {
        sqlx_core::query::query("DELETE FROM sensei.schedules WHERE name = $1")
            .bind(name)
            .execute(s.pool())
            .await
            .ok();
    }
}

#[tokio::test]
async fn update_schedule_writes_only_the_fields_it_was_given() {
    // PATCH semantics: an absent field is "leave alone", not "clear".
    let s = pg_store().await;
    let name = test_schedule_name();
    sqlx_core::query::query(
        "INSERT INTO sensei.schedules(name, interval_secs, days) \
         VALUES($1, 60, ARRAY[1,2]::smallint[])",
    )
    .bind(&name)
    .execute(s.pool())
    .await
    .unwrap();

    let patch = SchedulePatch { interval_secs: Some(900), ..Default::default() };
    let after = s.update_schedule(&name, &patch).await.unwrap().expect("the row exists");
    assert_eq!(after.interval_secs, 900);
    assert!(after.enabled, "enabled was not in the patch, so it must be untouched");
    assert_eq!(after.days, vec![1, 2], "days were not in the patch either");

    // A window set as a PAIR, then cleared back to "any time".
    let hm = |h, m| chrono::NaiveTime::from_hms_opt(h, m, 0).unwrap();
    let window = (hm(22, 0), hm(5, 0));
    let patch = SchedulePatch { window: Some(Some(window)), ..Default::default() };
    let after = s.update_schedule(&name, &patch).await.unwrap().unwrap();
    assert_eq!((after.window_start.unwrap(), after.window_end.unwrap()), window);
    assert_eq!(after.interval_secs, 900, "the earlier edit survives the next patch");

    let patch = SchedulePatch { window: Some(None), days: Some(None), ..Default::default() };
    let after = s.update_schedule(&name, &patch).await.unwrap().unwrap();
    assert_eq!(after.window_start, None, "an explicit clear means any time");
    assert_eq!(after.window_end, None);
    assert!(after.days.is_empty(), "and every day");

    sqlx_core::query::query("DELETE FROM sensei.schedules WHERE name = $1")
        .bind(&name)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn update_schedule_is_none_for_a_worker_with_no_row() {
    // None, not a fabricated insert: the legal names are code-side, and a row
    // conjured by a PATCH would name a worker nobody validated.
    let s = pg_store().await;
    let patch = SchedulePatch { enabled: Some(false), ..Default::default() };
    assert!(s.update_schedule("_test:no-such-worker", &patch).await.unwrap().is_none());
}

#[tokio::test]
async fn update_schedule_does_not_disturb_the_runtime_state() {
    // last_run_at / last_ok / last_error belong to the daemon. Editing a cadence
    // must not read as "it just ran", nor erase a recorded failure.
    let s = pg_store().await;
    let name = test_schedule_name();
    sqlx_core::query::query("INSERT INTO sensei.schedules(name, interval_secs) VALUES($1, 60)")
        .bind(&name)
        .execute(s.pool())
        .await
        .unwrap();
    s.mark_schedule_run(&name, Err("boom".into()), 10).await.unwrap();
    let before = s.load_schedule(&name).await.unwrap().unwrap();

    let patch = SchedulePatch { enabled: Some(false), ..Default::default() };
    let after = s.update_schedule(&name, &patch).await.unwrap().unwrap();
    assert!(!after.enabled);
    assert_eq!(after.last_run_at, before.last_run_at);
    assert_eq!(after.last_ok, Some(false));
    assert_eq!(after.last_error.as_deref(), Some("boom"));

    sqlx_core::query::query("DELETE FROM sensei.schedules WHERE name = $1")
        .bind(&name)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn a_user_edit_survives_the_seed_import() {
    // THE behaviour of this endpoint. Deploy order is apply → import, so the seed
    // otherwise has the last word: an edit that does not bump `modified_at` is
    // silently reverted by the next `dbd deploy` and the user never learns why.
    // Asserted through the REAL procedure, because the guard lives in SQL.
    let s = pg_store().await;
    // The datafile's own claim, read from staging rather than hard-coded, so this
    // still tests the guard after someone edits schedules.jsonl.
    let seed = schedule_state(&s, "staging.schedules", "contribute").await;
    // Start from the state a fresh deploy leaves: an earlier run's leftover edit
    // would otherwise make the control below pass for the wrong reason.
    crate::tasks::test_support::restore_seeded_schedule(&s, "contribute").await;

    let patch = SchedulePatch { interval_secs: Some(4242), ..Default::default() };
    s.update_schedule("contribute", &patch).await.unwrap().expect("contribute is seeded");
    let (_, edited_modified) = schedule_state(&s, "sensei.schedules", "contribute").await;
    assert!(
        edited_modified > seed.1,
        "the edit must bump modified_at past the datafile's — that is the whole guard"
    );

    s.execute_raw("CALL staging.import_schedules()").await.unwrap();
    assert_eq!(
        schedule_state(&s, "sensei.schedules", "contribute").await.0,
        4242,
        "the seed must not revert a user's cadence"
    );

    // The control: a row OLDER than the datafile is overwritten, which is what
    // an un-bumped edit would have been. Without it the assertion above would
    // also pass against an import that quietly does nothing. It restores the row
    // too — the import writes the seed's own values back.
    set_schedule_row(&s, "contribute", (4242, seed.1 - chrono::Duration::days(1))).await;
    s.execute_raw("CALL staging.import_schedules()").await.unwrap();
    assert_eq!(
        schedule_state(&s, "sensei.schedules", "contribute").await,
        seed,
        "a stale row is exactly what the datafile is allowed to overwrite"
    );
}

#[tokio::test]
async fn signing_out_stops_the_persona_being_enumerated_for_sync() {
    // Sign-out took no `State` and touched no database, so `session_slot` and
    // `verified_at` both survived it. `signed_in_personas` keys on exactly those
    // two columns, so the cycle kept listing a persona whose Keychain slot had
    // just been emptied: every 60s `live_access_token` returned SignedOut, and on
    // a single-persona install `failures.len() == total` made `tick` return Err.
    // A deliberate, clean sign-out red-lighted `schedules.dojo_sync` FOREVER,
    // surfaced in the app as "failing".
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let slot = format!("ztest-signout-{uniq}");
    let id = s.upsert_persona(&slot, true).await.unwrap();
    sqlx_core::query::query(
        "UPDATE sensei.personas SET verified_at = now(), session_slot = $2, \
                                    github_login = 'ztest-login' WHERE id = $1",
    )
    .bind(id)
    .bind(&slot)
    .execute(s.pool())
    .await
    .unwrap();
    assert!(
        s.signed_in_personas().await.unwrap().contains(&slot),
        "precondition: a signed-in persona is enumerated"
    );

    let cleared = s.clear_persona_session(&slot).await.unwrap();

    let listed = s.signed_in_personas().await.unwrap();
    let after: (Option<String>, Option<String>, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx_core::query_as::query_as(
            "SELECT session_slot, github_login, verified_at FROM sensei.personas WHERE id = $1",
        )
        .bind(id)
        .fetch_one(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();

    assert!(cleared, "the row was found and cleared");
    assert!(!listed.contains(&slot), "a signed-out persona must not be swept up by the cycle");
    assert_eq!(after.0, None, "the Keychain slot no longer names anything");
    // The VERIFIED IDENTITY is not destroyed. Which GitHub account this persona
    // is remains true after a sign-out, and re-deriving it would mean a second
    // OAuth round trip to learn something already proved. Both columns are
    // asserted: a mutation probe that nulled `verified_at` passed while only
    // `github_login` was checked, so half the claim was unguarded.
    assert_eq!(after.1.as_deref(), Some("ztest-login"), "signing out is not un-verifying");
    assert!(after.2.is_some(), "the proof of verification survives a sign-out too");
}

#[tokio::test]
async fn signing_out_a_slot_nobody_holds_is_reported_as_such() {
    // The daemon must be able to tell "I forgot your session" from "there was
    // nothing to forget" — a sign-out that reports success for an unknown slot
    // would hide a persona/slot mismatch, which is exactly the class of bug the
    // `session_slot` column was added to fix.
    let s = pg_store().await;
    let missing = format!("ztest-absent-{}", uuid::Uuid::new_v4());
    assert!(!s.clear_persona_session(&missing).await.unwrap());
}

#[tokio::test]
async fn signing_out_matches_the_slot_however_the_caller_cased_the_persona() {
    // The persona reaches `POST /api/auth/signout` from a QUERY STRING, and the
    // two halves of sign-out disagreed about case. The Keychain half lowercases
    // (`account_for`, with a test saying "Sensei-HQ" and "sensei-hq" must not
    // become two half-signed-in states); `link_persona_identity` also stores
    // `session_slot` lowercased. But the registry half compared the RAW
    // parameter, so `?persona=Sensei-HQ` deleted the credentials and matched no
    // row — leaving `session_slot` set.
    //
    // That is the precise failure the sign-out fix exists to prevent, restored
    // by a capital letter: `signed_in_personas` keys on that column, so the cycle
    // keeps selecting a persona whose Keychain slot is now empty, resolves
    // SignedOut every 60s, and on a single-persona install pins
    // `schedules.dojo_sync.last_ok = false` forever. And it is silent —
    // `clear_persona_session` returns `Ok(false)`, not an error, so the handler
    // reports `ok: true` over a sign-out that half happened.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let gh_id: i64 = (uniq.as_u128() % 1_000_000) as i64 + 700_000;
    // Mixed case throughout, exactly as it would arrive on the query string.
    let hint = format!("Ztest-SignOut-{uniq}");

    s.upsert_persona(&hint, true).await.unwrap();
    let id = s.link_persona_identity(&hint, "ztest-cased-login", gh_id, None, &[]).await.unwrap();

    let slot: Option<String> =
        query_as::<_, (Option<String>,)>("SELECT session_slot FROM sensei.personas WHERE id = $1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap()
            .0;
    // Not an assumption about the writer — read back, so this test fails loudly
    // if `link_persona_identity` ever stops normalising and the mismatch moves.
    assert_eq!(
        slot.as_deref(),
        Some(hint.to_lowercase().as_str()),
        "precondition: the sign-in stores the slot lowercased"
    );
    assert!(
        s.signed_in_personas().await.unwrap().contains(&hint.to_lowercase()),
        "precondition: the persona is enumerated for sync"
    );

    // What the handler passes: the query parameter, uncased.
    let cleared = s.clear_persona_session(&hint).await.unwrap();

    let listed = s.signed_in_personas().await.unwrap();
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();

    assert!(cleared, "signing out must find the row the sign-in wrote, whatever the caller's case");
    assert!(
        !listed.contains(&hint.to_lowercase()),
        "and the cycle must stop enumerating a persona whose credentials are gone"
    );
}
