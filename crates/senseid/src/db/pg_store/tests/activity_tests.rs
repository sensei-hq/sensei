use super::*;

// ── Activity pruner tests (#74) ────────────────────────────────────

#[tokio::test]
async fn prune_activity_keeps_unanalyzed_sessions_even_when_old() {
    let s = pg_store().await;
    let suffix = format!("prune_keep_unanalyzed_{}", uuid::Uuid::new_v4());
    let (_pid, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    let csid = format!("{}-csid", suffix);
    let sid = s
        .record_session_event(&crate::db::pg_store::SessionEventRow {
            client_session_id: &csid,
            folder_id: &fid,
            workspace_id: None,
            family: "claude",
            is_end: true,
        })
        .await
        .unwrap();
    // Age the session past the cutoff but leave analyzed_at NULL.
    sqlx_core::query::query(
        "UPDATE activity.sessions SET started_at = now() - interval '90 days' WHERE id = $1",
    )
    .bind(sid)
    .execute(s.pool())
    .await
    .unwrap();

    // Other tests may seed analyzed sessions, so the global count is not
    // useful — verify OUR session specifically survives. The analyzed-only
    // guard keeps it regardless of the capture-before-reclaim backstop
    // (backstop=60 here), so this assertion is unaffected by that guard.
    s.prune_activity(30, 60).await.unwrap();

    // The watermark must go with the turns. Leaving it behind is what made the
    // prune unrecoverable: the session re-appears without its transcript and
    // reports as fully ingested, forever (#125).
    let wm_left: (i64,) = sqlx_core::query_as::query_as(
        "SELECT COUNT(*) FROM activity.capture_watermarks WHERE session_id = $1",
    )
    .bind(&csid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(wm_left.0, 0, "prune left a watermark claiming turns it deleted");

    let exists: (bool,) = sqlx_core::query_as::query_as(
        "SELECT EXISTS(SELECT 1 FROM activity.sessions WHERE id = $1)",
    )
    .bind(sid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(exists.0, "unanalyzed session must survive prune");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(sid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn prune_activity_deletes_analyzed_sessions_past_cutoff_and_children() {
    let s = pg_store().await;
    let suffix = format!("prune_del_{}", uuid::Uuid::new_v4());
    let (pid, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    let csid = format!("{}-csid", suffix);
    let sid = s
        .record_session_event(&crate::db::pg_store::SessionEventRow {
            client_session_id: &csid,
            folder_id: &fid,
            workspace_id: None,
            family: "claude",
            is_end: true,
        })
        .await
        .unwrap();
    // Age to 40 days: past the 30-day retention window but INSIDE the 60-day
    // backstop, so the ONLY thing that makes it prune-eligible is the
    // capture path — its day must already exist in sensei.workspace_metrics.
    // (Previously aged 90 days, which pruned unconditionally; now the test
    // exercises capture-before-reclaim directly.)
    // Repo-grain: the capture guard keys on the session's repository, so give the
    // folder a repository and anchor the session to it via repo_folder_id.
    // `create_test_folder` already seeded `test/{suffix}` for this same suffix, so
    // this must adopt that row rather than insert a second one under the unique
    // `repositories_repo_key_key`.
    let (repo_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'prune-del') \
         ON CONFLICT (repo_key) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(format!("test/{suffix}"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET repository_id = $1 WHERE id = $2")
        .bind(repo_id)
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query(
        "UPDATE activity.sessions
                SET started_at = date_trunc('day', now() - interval '40 days'),
                    analyzed_at = now() - interval '39 days',
                    repo_folder_id = $2
              WHERE id = $1",
    )
    .bind(sid)
    .bind(fid)
    .execute(s.pool())
    .await
    .unwrap();
    // Seed a covering daily workspace_metrics row for the session's day (its OWN
    // repository, scope='user') so the repo-grain capture-before-reclaim guard is
    // satisfied (the durable snapshot exists).
    let day40: (chrono::NaiveDate,) = sqlx_core::query_as::query_as(
        "SELECT (date_trunc('day', now() - interval '40 days'))::date",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    let ftr_id: (uuid::Uuid,) =
        sqlx_core::query_as::query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
            .fetch_one(s.pool())
            .await
            .unwrap();
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_id.0,
        repository_id: &repo_id,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: day40.0,
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();
    // Seed a child transcript_turn keyed on client_session_id (no FK).
    sqlx_core::query::query(
            // `family` is NOT NULL — TranscriptAdapter::family() returns &'static str,
            // so the ingest path always supplies one and a fixture must too.
            "INSERT INTO activity.transcript_turns(session_id, source, family, turn_index, assistant_text)
             VALUES ($1, 'claude_code', 'claude', 0, 'hello')"
        ).bind(&csid).execute(s.pool()).await.unwrap();
    // Seed the capture watermark the ingest would have written for those turns.
    // Its survival is what made the prune self-sealing (#125): `ingest_one`
    // gates the turn re-import on it, so a pruned session never re-imported
    // while `synthesize_session` kept re-creating the session row.
    s.set_capture_watermark(&crate::db::pg_store::CaptureWatermarkRow {
        source: "claude_code",
        file_path: &csid,
        session_id: Some(&csid),
        mtime_ns: 1,
        turns: 1,
    })
    .await
    .unwrap();

    // Prove the fixture landed — an assertion that the row is GONE passes
    // trivially if it was never there.
    let wm_before: (i64,) = sqlx_core::query_as::query_as(
        "SELECT COUNT(*) FROM activity.capture_watermarks WHERE session_id = $1",
    )
    .bind(&csid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(wm_before.0, 1, "watermark fixture did not land");

    // Seed a hook event under the same client_session_id.
    s.insert_hook_event(&HookEventRow {
        session_id: &csid,
        assistant_family: "claude",
        event_type: "UserPromptSubmit",
        tool_name: None,
        cwd: None,
        ts: 1000,
        success: None,
        payload: &serde_json::json!({"prompt": "hi"}),
    })
    .await
    .unwrap();

    // Counts include any leftover analyzed+old data from other tests, so
    // don't assert exact numbers — assert OUR session (and its child
    // rows) are gone after the prune. backstop=60 > 40d age, so the capture
    // row (not the backstop) is what enables the prune. The covering row is
    // an `ftr` (session_outcomes = DAY-KEYED) metric, so it still counts as
    // captured under the scoped guard.
    s.prune_activity(30, 60).await.unwrap();

    let exists: (bool,) = sqlx_core::query_as::query_as(
        "SELECT EXISTS(SELECT 1 FROM activity.sessions WHERE id = $1)",
    )
    .bind(sid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(!exists.0, "analyzed + old session must be pruned");

    let tt: (i64,) = sqlx_core::query_as::query_as(
        "SELECT COUNT(*) FROM activity.transcript_turns WHERE session_id = $1",
    )
    .bind(&csid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(tt.0, 0, "transcript_turns keyed on this session must be gone");

    // cleanup: the covering metric row references the repository (FK) — remove both
    // so repo_key can't collide across runs of the shared test DB.
    sqlx_core::query::query("DELETE FROM sensei.repository_metrics WHERE repository_id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(repo_id)
        .execute(s.pool())
        .await
        .ok();
}

/// Repo-grain capture-before-reclaim (I20, P-A.3b): after the metric store flips
/// to REPOSITORY grain, the pruner's capture guard keys on the SESSION'S
/// REPOSITORY (`folders.repository_id` via `s.repo_folder_id`), a `scope='user'`
/// row, and `metrics.capture_source='session'` — NOT the workspace, NOT the
/// cadence. Four legs prove all three discriminators, each at retention=30 /
/// backstop=60:
///  (a) 40d, day captured by a `scope='user'` `capture_source='session'` (`ftr`)
///      row ON THE SESSION'S REPOSITORY → PRUNED.
///  (b) 45d, uncaptured on its own repo — a DECOY `scope='user'` session-metric
///      row exists for the SAME day on a DIFFERENT repository → KEPT (fails if the
///      guard drops the `pm.repository_id = rf.repository_id` match).
///  (c) 50d, covered ONLY by (i) a `capture_source='snapshot'` DAY-cadence
///      `rework_density` row (`scope='user'`) and (ii) a `capture_source='session'`
///      `ftr` row at the WRONG scope (`scope='repo'`), both on the right repo/day
///      → KEPT (fails if the guard keys on cadence='day' instead of
///      `capture_source`, or drops the `scope='user'` filter).
///  (d) 90d, past the backstop, uncaptured → PRUNED via the backstop arm.
#[tokio::test]
async fn prune_activity_captures_before_reclaim_repo_grain() {
    let s = pg_store().await;
    let suffix = format!("prune_cbr_repo_{}", uuid::Uuid::new_v4());
    let (pid, fid) = create_test_workspace_and_folder(&s, &suffix).await;

    // The session's repository (fid's repository_id) plus a DECOY repository used
    // to prove the guard matches the SESSION'S repository, not any repository.
    let (repo_a,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'cbr-a') RETURNING id",
    )
    .bind(format!("test/{suffix}-a"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    let (repo_b,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'cbr-b') RETURNING id",
    )
    .bind(format!("test/{suffix}-b"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET repository_id = $1 WHERE id = $2")
        .bind(repo_a)
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();

    // ftr = session_outcomes (capture_source='session'); rework_density = a
    // snapshot metric at DAY cadence (the cadence-vs-capture_source trap).
    let ftr_id: (uuid::Uuid,) =
        sqlx_core::query_as::query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
            .fetch_one(s.pool())
            .await
            .unwrap();
    let rework_id: (uuid::Uuid,) =
        sqlx_core::query_as::query_as("SELECT id FROM sensei.metrics WHERE key = 'rework_density'")
            .fetch_one(s.pool())
            .await
            .unwrap();

    // An analyzed session `age_days` old whose repo_folder_id anchors to `fid`
    // (whose repository_id is set), so the guard resolves its repository.
    async fn aged_repo_session(
        s: &PgStore,
        fid: &uuid::Uuid,
        suffix: &str,
        tag: &str,
        age_days: i32,
    ) -> uuid::Uuid {
        let csid = format!("{suffix}-{tag}");
        let sid = s
            .record_session_event(&crate::db::pg_store::SessionEventRow {
                client_session_id: &csid,
                folder_id: fid,
                workspace_id: None,
                family: "claude",
                is_end: true,
            })
            .await
            .unwrap();
        sqlx_core::query::query(
            "UPDATE activity.sessions
                    SET started_at = date_trunc('day', now() - (interval '1 day' * $2)),
                        analyzed_at = now() - (interval '1 day' * ($2 - 1)),
                        repo_folder_id = $3
                  WHERE id = $1",
        )
        .bind(sid)
        .bind(age_days)
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();
        sid
    }
    async fn day_ago(s: &PgStore, age_days: i32) -> chrono::NaiveDate {
        let d: (chrono::NaiveDate,) = sqlx_core::query_as::query_as(
            "SELECT (date_trunc('day', now() - (interval '1 day' * $1)))::date",
        )
        .bind(age_days)
        .fetch_one(s.pool())
        .await
        .unwrap();
        d.0
    }

    // (a) captured on its OWN repo by a scope=user session metric → PRUNED.
    let captured = aged_repo_session(&s, &fid, &suffix, "captured", 40).await;
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_id.0,
        repository_id: &repo_a,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: day_ago(&s, 40).await,
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();

    // (b) uncaptured on repo_a; a DECOY scope=user session metric for the SAME day
    //     lives on repo_b → KEPT (the guard must match the session's repository).
    let uncaptured = aged_repo_session(&s, &fid, &suffix, "uncaptured", 45).await;
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_id.0,
        repository_id: &repo_b,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: day_ago(&s, 45).await,
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();

    // (c) on repo_a/day50, TWO non-authorizing rows: a snapshot + DAY-cadence
    //     rework_density (scope=user) and a session-source ftr at scope=repo.
    //     Neither authorizes reclaim → KEPT. Fails if the guard keys on
    //     cadence='day' (the rework_density row would capture) or drops the
    //     scope='user' filter (the scope=repo ftr row would capture).
    let wrong_signal = aged_repo_session(&s, &fid, &suffix, "wrongsignal", 50).await;
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &rework_id.0,
        repository_id: &repo_a,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: day_ago(&s, 50).await,
        grain: "daily",
        value: 0.2,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_id.0,
        repository_id: &repo_a,
        scope: "repo",
        identity: None,
        commit_sha: None,
        computed_on: day_ago(&s, 50).await,
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();

    // (d) past the backstop, uncaptured → PRUNED via the backstop arm.
    let past_backstop = aged_repo_session(&s, &fid, &suffix, "backstop", 90).await;

    s.prune_activity(30, 60).await.unwrap();

    async fn alive(s: &PgStore, sid: uuid::Uuid) -> bool {
        let r: (bool,) = sqlx_core::query_as::query_as(
            "SELECT EXISTS(SELECT 1 FROM activity.sessions WHERE id = $1)",
        )
        .bind(sid)
        .fetch_one(s.pool())
        .await
        .unwrap();
        r.0
    }
    assert!(
        !alive(&s, captured).await,
        "(a) day captured by a scope=user session metric ON THE SESSION'S REPOSITORY → pruned"
    );
    assert!(
        alive(&s, uncaptured).await,
        "(b) uncaptured on its repo (the same-day metric is on ANOTHER repo) → kept: the guard must match the session's repository"
    );
    assert!(
        alive(&s, wrong_signal).await,
        "(c) covered only by a snapshot/day metric + a wrong-scope session metric → kept: the guard keys on capture_source + scope=user, never cadence"
    );
    assert!(
        !alive(&s, past_backstop).await,
        "(d) uncaptured but past the backstop → pruned so nothing lingers forever"
    );

    // Clean up survivors, the seeded metric rows, then the repositories (repo_key
    // uniqueness must not collide across runs of the shared test DB). workspace_metrics
    // first so its repository FK does not block the repositories delete.
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = ANY($1::uuid[])")
        .bind(vec![uncaptured, wrong_signal])
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repository_metrics WHERE repository_id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1::uuid[])")
        .bind(vec![repo_a, repo_b])
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn vacuum_activity_runs_via_simple_protocol() {
    // VACUUM cannot run through the prepared/extended protocol or inside a
    // transaction; vacuum_activity uses raw_sql (simple protocol). Assert it
    // executes without error against the activity tables (guards against a
    // regression to query()/extended protocol, which would fail at runtime).
    let s = pg_store().await;
    s.vacuum_activity().await.expect("VACUUM (ANALYZE) on activity tables succeeds");
}

#[tokio::test]
async fn prune_activity_prunes_orphan_events_by_ts() {
    let s = pg_store().await;
    // Insert an assistant_event with no matching session and old ts.
    let old_ts: i64 = (chrono::Utc::now() - chrono::Duration::days(90)).timestamp() * 1000;
    let orphan_csid = format!("orphan_prune_{}", uuid::Uuid::new_v4());
    s.insert_hook_event(&HookEventRow {
        session_id: &orphan_csid,
        assistant_family: "claude",
        event_type: "PostToolUse",
        tool_name: Some("Read"),
        cwd: None,
        ts: old_ts,
        success: None,
        payload: &serde_json::json!({}),
    })
    .await
    .unwrap();

    // prune_activity's returned count is GLOBAL across the shared test DB — a
    // sibling db-gated test may prune this row concurrently, so don't assert on
    // the count. The per-row check below deterministically proves our orphan
    // (unique csid) was pruned. Session-less orphan events are pruned by ts
    // alone (no capture-before-reclaim guard), so the backstop + day-keyed args
    // are inert.
    s.prune_activity(30, 60).await.unwrap();

    let orphaned: (i64,) = sqlx_core::query_as::query_as(
        "SELECT COUNT(*) FROM activity.assistant_events WHERE session_id = $1",
    )
    .bind(&orphan_csid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(orphaned.0, 0);
}

#[tokio::test]
async fn prune_activity_prunes_orphans_despite_a_null_client_session_id() {
    // Regression: step (6) of prune_activity used `session_id NOT IN (SELECT
    // client_session_id FROM activity.sessions)`. `client_session_id` is
    // NULLABLE, and under three-valued logic ONE NULL in the subquery makes
    // the predicate NULL for every row — the DELETE then matches nothing, so
    // orphan events were never reclaimed. Sessions with no client id are
    // normal (an AI-start anchor row has none), so this leaked in production
    // and merely LOOKED like a flaky test: it passed only on runs where no
    // such session existed.
    //
    // Assert the real thing: with a NULL-client_session_id session present
    // AND at least one prune-eligible session (so the non-empty branch runs),
    // an old orphan is still pruned.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();

    // A session carrying NO client_session_id — the NULL that poisoned NOT IN.
    let fid = uuid::Uuid::new_v4();
    s.execute_raw("INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001','/_test','_test','watching'::sensei.watch_status) ON CONFLICT DO NOTHING").await.unwrap();
    s.execute_raw(&format!(
            "INSERT INTO sensei.folders(id, root_id, kind, name, path, abs_path) \
             VALUES('{fid}','00000000-0000-0000-0000-000000000001','git'::sensei.folder_kind,'_np_{uniq}','_np_{uniq}','/_test/_np_{uniq}')"
        )).await.unwrap();
    let (null_sid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.sessions (folder_id, started_at, client_session_id) \
             VALUES ($1, now(), NULL) RETURNING id",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();

    // A prune-ELIGIBLE session (analyzed, past the 60-day backstop) so
    // prune_activity takes its non-empty branch and reaches step (6). Without
    // this the eligible set can be empty, the early-return path runs instead —
    // and that path always spelled the predicate correctly, so the test would
    // pass with the bug still in place.
    sqlx_core::query::query(
        "INSERT INTO activity.sessions (folder_id, started_at, analyzed_at, client_session_id) \
             VALUES ($1, now() - interval '90 days', now(), $2)",
    )
    .bind(fid)
    .bind(format!("eligible_{uniq}"))
    .execute(s.pool())
    .await
    .unwrap();

    let orphan_csid = format!("orphan_null_{uniq}");
    let old_ts: i64 = (chrono::Utc::now() - chrono::Duration::days(90)).timestamp() * 1000;
    s.insert_hook_event(&HookEventRow {
        session_id: &orphan_csid,
        assistant_family: "claude",
        event_type: "PostToolUse",
        tool_name: Some("Read"),
        cwd: None,
        ts: old_ts,
        success: None,
        payload: &serde_json::json!({}),
    })
    .await
    .unwrap();

    s.prune_activity(30, 60).await.unwrap();

    let (left,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT COUNT(*) FROM activity.assistant_events WHERE session_id = $1",
    )
    .bind(&orphan_csid)
    .fetch_one(s.pool())
    .await
    .unwrap();

    // Clean up before asserting so a failure doesn't leak fixture rows.
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(null_sid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(left, 0, "a NULL client_session_id must not block the orphan prune");
}

// ── Sessions + Events tests ────────────────────────────────────────

#[tokio::test]
async fn session_create_and_get() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "sess_create").await;
    let sid = s.create_session(&fid, "fix bug #42", Some("claude-code")).await.unwrap();
    let sess = s.get_session(&sid).await.unwrap().unwrap();
    assert_eq!(sess["task"], "fix bug #42");
    assert_eq!(sess["acp_id"], "claude-code");
    assert!(sess["outcome"].is_null());
    assert_eq!(sess["turns"], 0);
}

/// The agent closes its session with the id its SessionStart context gave it —
/// Claude Code's own session id, which is `client_session_id`, not the row id.
/// Before this, `complete_session` matched `id` only, so every agent's close was
/// an UPDATE of zero rows reported as `{"ok": true}` (#238).
///
/// Mutations that must break this: match `id` only, or report a miss as found.
#[tokio::test]
async fn a_session_closes_by_its_client_id_and_a_miss_is_a_miss() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "sess_client_close").await;
    let client = uuid::Uuid::new_v4();
    let row = s
        .record_session_event(&crate::db::pg_store::SessionEventRow {
            client_session_id: &client.to_string(),
            folder_id: &fid,
            workspace_id: None,
            family: "claude",
            is_end: false,
        })
        .await
        .unwrap();
    assert_ne!(row, client, "the row id is not the client id — which is the whole bug");

    let close = |id| SessionOutcomeRow {
        id,
        outcome: "completed",
        ftr: true,
        turns: 3,
        corrections: 0,
        summary: Some("closed by client id"),
        tokens_in: None,
        tokens_out: None,
    };
    assert!(s.complete_session(&close(&client)).await.unwrap(), "found by client id");
    let sess = s.get_session(&row).await.unwrap().unwrap();
    assert_eq!(sess["outcome"], "completed");

    let nobody = uuid::Uuid::new_v4();
    assert!(!s.complete_session(&close(&nobody)).await.unwrap(), "a miss is reported, not ok");
}

#[tokio::test]
async fn session_complete() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "sess_complete").await;
    let sid = s.create_session(&fid, "add feature", None).await.unwrap();
    s.complete_session(&SessionOutcomeRow {
        id: &sid,
        outcome: "completed",
        ftr: true,
        turns: 5,
        corrections: 0,
        summary: Some("shipped it"),
        tokens_in: Some(1200),
        tokens_out: Some(3400),
    })
    .await
    .unwrap();
    let sess = s.get_session(&sid).await.unwrap().unwrap();
    assert_eq!(sess["outcome"], "completed");
    assert_eq!(sess["ftr"], true);
    assert_eq!(sess["turns"], 5);
    assert!(sess["completed_at"].as_str().is_some());
    // summary + tokens actually PERSIST (were previously advertised-but-dropped).
    let meta: (Option<String>, Option<i64>, Option<i64>) = sqlx_core::query_as::query_as(
        "SELECT summary, tokens_in, tokens_out FROM activity.sessions WHERE id=$1",
    )
    .bind(sid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(meta.0.as_deref(), Some("shipped it"), "summary persists");
    assert_eq!(meta.1, Some(1200), "tokens_in persists");
    assert_eq!(meta.2, Some(3400), "tokens_out persists");
}

#[tokio::test]
async fn session_list_by_folder() {
    let s = pg_store().await;
    let suffix = format!("sess_list_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &suffix).await;
    s.create_session(&fid, "task 1", None).await.unwrap();
    s.create_session(&fid, "task 2", None).await.unwrap();
    let sessions = s.list_sessions_by_folder(&fid, 10).await.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn session_get_nonexistent() {
    let s = pg_store().await;
    assert!(s.get_session(&uuid::Uuid::new_v4()).await.unwrap().is_none());
}

// ── Hook events tests ─────────────────────────────────────────────

#[tokio::test]
async fn hook_event_insert_and_query() {
    let s = pg_store().await;
    let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
    let payload = serde_json::json!({
        "session_id": session_id,
        "hook_event_name": "PreToolUse",
        "assistant_family": "claude",
        "tool_name": "Read",
        "cwd": "/tmp/test",
    });
    let id = s
        .insert_hook_event(&HookEventRow {
            session_id: &session_id,
            assistant_family: "claude",
            event_type: "PreToolUse",
            tool_name: Some("Read"),
            cwd: Some("/tmp/test"),
            ts: chrono::Utc::now().timestamp_millis(),
            success: None,
            payload: &payload,
        })
        .await
        .unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn hook_event_post_tool_use_success() {
    let s = pg_store().await;
    let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
    let payload = serde_json::json!({"hook_event_name": "PostToolUse", "assistant_family": "claude", "tool_name": "Bash"});
    let id = s
        .insert_hook_event(&HookEventRow {
            session_id: &session_id,
            assistant_family: "claude",
            event_type: "PostToolUse",
            tool_name: Some("Bash"),
            cwd: None,
            ts: chrono::Utc::now().timestamp_millis(),
            success: Some(true),
            payload: &payload,
        })
        .await
        .unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn hook_event_no_tool_name() {
    let s = pg_store().await;
    let session_id = format!("test-session-{}", uuid::Uuid::new_v4());
    let payload = serde_json::json!({"hook_event_name": "SessionStart", "assistant_family": "claude", "model": "claude-sonnet-4"});
    let id = s
        .insert_hook_event(&HookEventRow {
            session_id: &session_id,
            assistant_family: "claude",
            event_type: "SessionStart",
            tool_name: None,
            cwd: Some("/home/user/workspace"),
            ts: chrono::Utc::now().timestamp_millis(),
            success: None,
            payload: &payload,
        })
        .await
        .unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn hook_event_cursor_family() {
    let s = pg_store().await;
    let session_id = format!("cursor-session-{}", uuid::Uuid::new_v4());
    let payload =
        serde_json::json!({"hook_event_name": "SessionStart", "assistant_family": "cursor"});
    let id = s
        .insert_hook_event(&HookEventRow {
            session_id: &session_id,
            assistant_family: "cursor",
            event_type: "SessionStart",
            tool_name: None,
            cwd: Some("/home/user/workspace"),
            ts: chrono::Utc::now().timestamp_millis(),
            success: None,
            payload: &payload,
        })
        .await
        .unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn unclassified_verdict_sessions_returns_only_in_window_unclassified() {
    use crate::tasks::handlers::tool_insights::HEALTH_VERDICT_WINDOW_DAYS;
    let s = pg_store().await;
    let now = chrono::Utc::now().timestamp_millis();
    let day_ms = 86_400_000i64;

    // (a) in-window PostToolUse, never classified → should appear.
    let pending_sid = format!("_test-unclassified-pending-{}", uuid::Uuid::new_v4());
    s.insert_hook_event(&HookEventRow {
        session_id: &pending_sid,
        assistant_family: "claude",
        event_type: "PostToolUse",
        tool_name: Some("Read"),
        cwd: None,
        ts: now,
        success: Some(true),
        payload: &serde_json::json!({"tool_response": "x"}),
    })
    .await
    .unwrap();

    // (b) in-window PostToolUse that already carries a verdict row → excluded.
    let classified_sid = format!("_test-unclassified-classified-{}", uuid::Uuid::new_v4());
    let ev_id = s
        .insert_hook_event(&HookEventRow {
            session_id: &classified_sid,
            assistant_family: "claude",
            event_type: "PostToolUse",
            tool_name: Some("Read"),
            cwd: None,
            ts: now,
            success: Some(true),
            payload: &serde_json::json!({"tool_response": "y"}),
        })
        .await
        .unwrap();
    s.upsert_verdicts_batch(&[(
        classified_sid.clone(),
        ev_id,
        Some("Read".to_string()),
        "used",
        0.9f32,
        "seed".to_string(),
    )])
    .await
    .unwrap();

    // (c) out-of-window PostToolUse (30 days old), unclassified → excluded.
    let old_sid = format!("_test-unclassified-old-{}", uuid::Uuid::new_v4());
    s.insert_hook_event(&HookEventRow {
        session_id: &old_sid,
        assistant_family: "claude",
        event_type: "PostToolUse",
        tool_name: Some("Read"),
        cwd: None,
        ts: now - 30 * day_ms,
        success: Some(true),
        payload: &serde_json::json!({"tool_response": "z"}),
    })
    .await
    .unwrap();

    let pending = s.unclassified_verdict_sessions(HEALTH_VERDICT_WINDOW_DAYS).await.unwrap();
    assert!(pending.contains(&pending_sid), "in-window unclassified session is pending");
    assert!(!pending.contains(&classified_sid), "already-classified session excluded");
    assert!(!pending.contains(&old_sid), "out-of-window session excluded");
}

// ── public.logs read path (Observatory · Logs) ───────────────────

/// Seed three log rows spanning levels / sources / timestamps and return
/// a marker (via `running_on`) so the assertions can isolate this run's
/// rows from anything already in the shared test DB.
async fn seed_logs(pg: &PgStore, marker: &str) {
    // Oldest → newest so `logged_at DESC` ordering is observable.
    let base = chrono::Utc::now() - chrono::Duration::hours(2);
    let rows = [
        ("info", format!("{marker}-a"), "scanner", base),
        ("warn", format!("{marker}-a"), "watcher", base + chrono::Duration::minutes(30)),
        ("error", format!("{marker}-b"), "analyzer", base + chrono::Duration::minutes(90)),
    ];
    for (level, running_on, module, ts) in rows {
        pg.insert_log(&LogRow {
            level,
            running_on: &running_on,
            module: Some(module),
            logged_at: &ts.to_rfc3339(),
            message: &format!("{marker} {level} message"),
            context: &serde_json::json!({}),
            data: &None,
            error: &None,
        })
        .await
        .unwrap();
    }
}

async fn cleanup_logs(pg: &PgStore, marker: &str) {
    sqlx_core::query::query("DELETE FROM public.logs WHERE running_on LIKE $1")
        .bind(format!("{marker}-%"))
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn query_logs_no_filter_newest_first_and_capped() {
    let pg = PgStore::test_store().await;
    let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
    seed_logs(&pg, &marker).await;

    // Scope to this run via the `source` (running_on) filter is not enough
    // (two distinct sources), so fetch broadly and filter in-memory.
    let all = pg
        .query_logs(&crate::db::pg_store::LogQuery {
            level: None,
            source: None,
            module: None,
            since: None,
            limit: 1000,
        })
        .await
        .unwrap();
    let mine: Vec<_> = all
        .iter()
        .filter(|r| r["source"].as_str().is_some_and(|s| s.starts_with(&marker)))
        .collect();
    assert_eq!(mine.len(), 3, "all three seeded rows returned");

    // Newest-first: the analyzer/error row (base+90m) precedes the others.
    assert_eq!(mine[0]["level"], "error");
    assert_eq!(mine[2]["level"], "info");

    // Stable wire shape: source mirrors running_on, module is a top-level column.
    assert_eq!(mine[0]["source"], format!("{marker}-b"));
    assert_eq!(mine[0]["module"], "analyzer");
    assert!(mine[0]["logged_at"].as_str().unwrap().contains('T'));

    cleanup_logs(&pg, &marker).await;
}

#[tokio::test]
async fn query_logs_level_and_source_and_module_filters() {
    let pg = PgStore::test_store().await;
    let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
    seed_logs(&pg, &marker).await;

    // level filter → only the warn row.
    let warns = pg
        .query_logs(&crate::db::pg_store::LogQuery {
            level: Some("warn"),
            source: Some(&format!("{marker}-a")),
            module: None,
            since: None,
            limit: 1000,
        })
        .await
        .unwrap();
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0]["level"], "warn");

    // source (running_on) filter → the two `-a` rows only.
    let a_rows = pg
        .query_logs(&crate::db::pg_store::LogQuery {
            level: None,
            source: Some(&format!("{marker}-a")),
            module: None,
            since: None,
            limit: 1000,
        })
        .await
        .unwrap();
    assert_eq!(a_rows.len(), 2);
    assert!(a_rows.iter().all(|r| r["source"] == format!("{marker}-a")));

    // module column filter → only the analyzer row.
    let ana = pg
        .query_logs(&crate::db::pg_store::LogQuery {
            level: None,
            source: None,
            module: Some("analyzer"),
            since: None,
            limit: 1000,
        })
        .await
        .unwrap();
    let mine: Vec<_> = ana
        .iter()
        .filter(|r| r["source"].as_str().is_some_and(|s| s.starts_with(&marker)))
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0]["module"], "analyzer");

    cleanup_logs(&pg, &marker).await;
}

#[tokio::test]
async fn query_logs_since_excludes_older_rows() {
    let pg = PgStore::test_store().await;
    let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
    seed_logs(&pg, &marker).await;

    // Cutoff at 1h ago excludes the two rows at base(-2h) and base+30m(-90m),
    // keeping only the base+90m(-30m) analyzer/error row.
    let since = chrono::Utc::now() - chrono::Duration::hours(1);
    let recent = pg
        .query_logs(&crate::db::pg_store::LogQuery {
            level: None,
            source: None,
            module: None,
            since: Some(since),
            limit: 1000,
        })
        .await
        .unwrap();
    let mine: Vec<_> = recent
        .iter()
        .filter(|r| r["source"].as_str().is_some_and(|s| s.starts_with(&marker)))
        .collect();
    assert_eq!(mine.len(), 1, "since cutoff drops the two older rows");
    assert_eq!(mine[0]["level"], "error");

    cleanup_logs(&pg, &marker).await;
}

#[tokio::test]
async fn query_logs_limit_is_honored() {
    let pg = PgStore::test_store().await;
    let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
    seed_logs(&pg, &marker).await;

    // Scope to this run's sources so the global limit is deterministic.
    let a = pg
        .query_logs(&crate::db::pg_store::LogQuery {
            level: None,
            source: Some(&format!("{marker}-a")),
            module: None,
            since: None,
            limit: 1,
        })
        .await
        .unwrap();
    assert_eq!(a.len(), 1, "limit=1 returns exactly one of the two -a rows");
    // Newest -a row is the warn/watcher one.
    assert_eq!(a[0]["level"], "warn");

    cleanup_logs(&pg, &marker).await;
}

#[tokio::test]
async fn query_logs_empty_result_is_empty_array() {
    let pg = PgStore::test_store().await;
    // A source that never exists → empty Vec, not an error.
    let none = pg
        .query_logs(&crate::db::pg_store::LogQuery {
            level: None,
            source: Some("_test:logs:does-not-exist"),
            module: None,
            since: None,
            limit: 200,
        })
        .await
        .unwrap();
    assert!(none.is_empty());
}

/// The staging import must be idempotent: the same staged event imported twice
/// is one row. It was a plain INSERT, and each `dbd import` appended the staging
/// file again — the live DB held 167,932 duplicate events (26%), tool calls
/// stored 12 times, which skewed every count over them (found via #278).
#[tokio::test]
async fn importing_the_same_staged_event_twice_keeps_one_row() {
    let s = pg_store().await;
    let session = format!("import-idem-{}", uuid::Uuid::new_v4());
    sqlx_core::query::query(
        "INSERT INTO staging.assistant_events (session_id, family, event_type, tool_name, ts, payload, created_at)
         VALUES ($1, 'claude', 'PostToolUse', 'Bash', 1781374265314, '{\"tool_use_id\":\"t1\"}', now())",
    )
    .bind(&session)
    .execute(s.pool())
    .await
    .unwrap();

    s.execute_raw("CALL staging.import_assistant_events()").await.unwrap();
    s.execute_raw("CALL staging.import_assistant_events()").await.unwrap();

    let (n,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM activity.assistant_events WHERE session_id = $1",
    )
    .bind(&session)
    .fetch_one(s.pool())
    .await
    .unwrap();
    for sql in [
        "DELETE FROM staging.assistant_events WHERE session_id = $1",
        "DELETE FROM activity.assistant_events WHERE session_id = $1",
    ] {
        sqlx_core::query::query(sql).bind(&session).execute(s.pool()).await.ok();
    }
    assert_eq!(n, 1, "a re-import must not add a second copy");
}

/// Instruments · Health: one row per tool source. Calls to a tool its server
/// no longer lists (a renamed or retired tool) belong to that server's row —
/// matched by the prefix the harness records — not to a second row with the
/// same key, which drew sensei twice and broke the whole grid on a duplicate
/// key. A prefix no server claims is still its own row.
#[tokio::test]
async fn a_servers_retired_tools_count_on_its_own_health_row() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4().simple().to_string();
    let key = format!("srv{}", &uniq[..10]);
    let prefix = format!("plugin_p{0}_p{0}", &uniq[..8]);
    let orphan = format!("orphan{}", &uniq[..10]);
    s.upsert_assistant_tool(&AssistantToolRow {
        assistant_family: "claude",
        source_type: "mcp",
        source_key: &key,
        tool_name: "kept",
        invoked_name: &format!("mcp__{prefix}__kept"),
        description: None,
        server_id: None,
    })
    .await
    .unwrap();
    let now = chrono::Utc::now().timestamp_millis();
    let csid = format!("health-{uniq}");
    for tool in [
        format!("mcp__{prefix}__kept"),
        format!("mcp__{prefix}__retired"),
        format!("mcp__{prefix}__retired"),
        format!("mcp__{orphan}__x"),
    ] {
        s.insert_hook_event(&HookEventRow {
            session_id: &csid,
            assistant_family: "claude",
            event_type: "PostToolUse",
            tool_name: Some(&tool),
            cwd: None,
            ts: now,
            success: Some(true),
            payload: &serde_json::json!({}),
        })
        .await
        .unwrap();
    }

    let rows = s.get_tools_health().await.unwrap();
    let mine: Vec<_> = rows
        .iter()
        .filter(|r| {
            [key.as_str(), prefix.as_str(), orphan.as_str()]
                .contains(&r["source_key"].as_str().unwrap())
        })
        .map(|r| {
            (
                r["source_key"].as_str().unwrap().to_string(),
                r["tools_registered"].as_i64(),
                r["tools_invoked_14d"].as_i64().unwrap(),
                r["calls_14d"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        mine,
        vec![(key.clone(), Some(1), 1, 3), (orphan.clone(), None, 1, 1),],
        "the retired tool's calls count on its server's row; the share stays registered-only"
    );
}
