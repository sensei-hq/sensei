use super::*;

// ── Libraries tests ──────────────────────────────────────────────

#[tokio::test]
async fn library_upsert_and_get() {
    let s = pg_store().await;
    let id = s
        .upsert_library("_test:tokio", "cargo", Some("1.0"), Some("async runtime"), None, None)
        .await
        .unwrap();
    let lib = s.get_library(&id).await.unwrap().unwrap();
    assert_eq!(lib["name"], "_test:tokio");
    assert_eq!(lib["ecosystem"], "cargo");
    assert_eq!(lib["version"], "1.0");
    s.delete_library(&id).await.unwrap();
}

#[tokio::test]
async fn upsert_folder_dependency_is_idempotent_and_stores_all_columns() {
    // D11: folder → folder edges must be idempotent on the composite PK
    // (from_folder, to_folder, source_manifest) and must preserve
    // source_protocol and resolved_target across upserts.
    let s = pg_store().await;
    let from_fid = create_test_folder(&s, &format!("fd-from-{}", uuid::Uuid::new_v4())).await;
    let to_fid = create_test_folder(&s, &format!("fd-to-{}", uuid::Uuid::new_v4())).await;

    // First upsert
    s.upsert_folder_dependency(&from_fid, &to_fid, "link", "package.json", Some("../actions"))
        .await
        .unwrap();
    // Repeat with a different resolved_target — same PK, so this must
    // update in place (last-writer wins on non-key columns).
    s.upsert_folder_dependency(
        &from_fid,
        &to_fid,
        "link",
        "package.json",
        Some("../actions-renamed"),
    )
    .await
    .unwrap();

    use sqlx_core::query_as::query_as;
    let rows: Vec<(String, Option<String>)> = query_as(
        "SELECT source_protocol, resolved_target
               FROM sensei.folder_dependencies
              WHERE from_folder_id = $1 AND to_folder_id = $2",
    )
    .bind(from_fid)
    .bind(to_fid)
    .fetch_all(s.pool())
    .await
    .unwrap();

    assert_eq!(rows.len(), 1, "composite PK must dedupe two upserts");
    assert_eq!(rows[0].0, "link", "protocol preserved");
    assert_eq!(rows[0].1.as_deref(), Some("../actions-renamed"), "target updated in place");

    // Cleanup
    sqlx_core::query::query("DELETE FROM sensei.folder_dependencies WHERE from_folder_id = $1")
        .bind(from_fid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn version_conflicts_view_flags_multi_version_pins_and_excludes_local() {
    // 1a Step 7-8: two folders in the same workspace pin the same library
    // at DIFFERENT versions → surfaces as a conflict. A third row tagged
    // local_source (as if declared via link:) with a different version
    // must NOT contribute to the conflict.
    let s = pg_store().await;
    let suffix = uuid::Uuid::new_v4();
    let pid = s.ensure_test_workspace(&format!("vc-{suffix}")).await.unwrap();
    let lib = s
        .upsert_library(&format!("_test:vc-lib-{suffix}"), "npm", Some("1.0"), None, None, None)
        .await
        .unwrap();

    // Two folders in the same workspace, different versions.
    let fid_a = create_test_folder(&s, &format!("vc-a-{suffix}")).await;
    let fid_b = create_test_folder(&s, &format!("vc-b-{suffix}")).await;
    // Attach folders to the workspace THROUGH THE PRODUCTION API, so membership
    // reaches `repositories_in_workspaces`. The view resolves via `folder_workspaces`,
    // which reads only the junction — a raw `UPDATE … SET workspace_id` leaves it
    // with nothing to find and the assertion below would be measuring an empty
    // set rather than the exclusion rule it names.
    s.set_folder_workspace(&fid_a, &pid, "root", None).await.unwrap();
    s.set_folder_workspace(&fid_b, &pid, "root", None).await.unwrap();

    s.upsert_referenced_library(&fid_a, &lib, Some("1.2.0"), None).await.unwrap();
    s.upsert_referenced_library(&fid_b, &lib, Some("1.3.0"), None).await.unwrap();

    // Third folder pins a local-source variant. This must be excluded.
    let fid_local = create_test_folder(&s, &format!("vc-local-{suffix}")).await;
    s.set_folder_workspace(&fid_local, &pid, "root", None).await.unwrap();
    s.upsert_referenced_library(
        &fid_local,
        &lib,
        Some("workspace-42"),
        Some(serde_json::json!({"local_source": "../lib"})),
    )
    .await
    .unwrap();

    let rows = s.list_workspace_library_version_conflicts(&pid).await.unwrap();
    assert_eq!(rows.len(), 1, "one lib with two registry-version pins → one row");
    let r = &rows[0];
    assert_eq!(r["library_id"].as_str().unwrap(), lib.to_string());
    let versions: Vec<String> =
        r["versions"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert_eq!(
        versions,
        vec!["1.2.0".to_string(), "1.3.0".to_string()],
        "versions must be sorted + distinct; workspace-42 excluded because local_source is tagged"
    );

    // Cleanup — library FK cascades referenced_libraries; then delete
    // workspace (folders cascade because workspace_id set null).
    s.delete_library(&lib).await.unwrap();
    s.delete_workspace(&pid).await.ok();
}

#[tokio::test]
async fn list_workspace_dependencies_joins_target_name_and_folder() {
    // D11: the stored edge is folder -> folder; the workspace-level answer is
    // DERIVED by joining both folders to their workspaces. This test proves the
    // derivation, which is why both folders must be wired to a workspace.
    let s = pg_store().await;
    let suffix = uuid::Uuid::new_v4();
    let (from_pid, from_fid) =
        create_test_workspace_and_folder(&s, &format!("lpd-from-{suffix}")).await;
    let (to_pid, to_fid) = create_test_workspace_and_folder(&s, &format!("lpd-to-{suffix}")).await;

    s.upsert_folder_dependency(&from_fid, &to_fid, "link", "package.json", Some("../actions"))
        .await
        .unwrap();

    let deps = s.list_workspace_dependencies(&from_pid).await.unwrap();

    assert_eq!(deps.len(), 1);
    let d = &deps[0];
    assert_eq!(d["to_workspace_id"].as_str().unwrap(), to_pid.to_string());
    assert!(
        d["to_workspace_name"].as_str().unwrap().starts_with("_test:lpd-to-"),
        "target workspace name must be joined in"
    );
    assert!(
        d["from_folder"].as_str().unwrap().starts_with("lpd-from-"),
        "source folder name must be joined in"
    );
    assert_eq!(d["source_protocol"], "link");
    assert_eq!(d["source_manifest"], "package.json");
    assert_eq!(d["resolved_target"], "../actions");

    // Reverse direction returns empty
    let none = s.list_workspace_dependencies(&to_pid).await.unwrap();
    assert!(none.is_empty(), "target workspace has no outgoing edges");

    sqlx_core::query::query("DELETE FROM sensei.folder_dependencies WHERE from_folder_id = $1")
        .bind(from_fid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_workspace(&from_pid).await.ok();
    s.delete_workspace(&to_pid).await.ok();
}

#[tokio::test]
async fn folder_dependency_records_an_intra_workspace_edge() {
    // THE REGRAIN'S REASON (D11). The old workspace-grained writer skipped an
    // edge whenever both folders sat in ONE workspace, which silently dropped
    // every monorepo workspace dependency — measured, all 384 folders of this
    // repo share one workspace, so the table held 0 rows while the feature was
    // live and tested. Folder-graining must record it.
    let s = pg_store().await;
    let suffix = uuid::Uuid::new_v4();
    let (pid, from_fid) = create_test_workspace_and_folder(&s, &format!("intra-a-{suffix}")).await;
    let to_fid = create_test_folder(&s, &format!("intra-b-{suffix}")).await;
    s.set_folder_workspace(&to_fid, &pid, "root", None).await.unwrap();

    s.upsert_folder_dependency(&from_fid, &to_fid, "path", "Cargo.toml", Some("../b"))
        .await
        .unwrap();

    let deps = s.list_workspace_dependencies(&pid).await.unwrap();
    assert_eq!(deps.len(), 1, "an intra-workspace folder edge MUST be recorded, not skipped");
    assert_eq!(deps[0]["to_workspace_id"].as_str().unwrap(), pid.to_string());

    sqlx_core::query::query("DELETE FROM sensei.folder_dependencies WHERE from_folder_id = $1")
        .bind(from_fid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_workspace(&pid).await.ok();
}

#[tokio::test]
async fn upsert_folder_dependency_rejects_self_edges() {
    // D11: the CHECK (from_folder_id <> to_folder_id) must reject a self-edge
    // at the write path.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("self-fid-{}", uuid::Uuid::new_v4())).await;

    let err = s.upsert_folder_dependency(&fid, &fid, "path", "Cargo.toml", Some(".")).await;

    assert!(err.is_err(), "self-edge must be rejected");
    assert!(err.unwrap_err().contains("check"), "err message must reference the check constraint");
}

#[tokio::test]
async fn upsert_referenced_library_merges_props() {
    // 1a Step 3: props must accumulate across passes, not overwrite. A
    // first pass tags {"local_source": "../actions"} for a link:/path=
    // dep; a later pass adding {"pinned": true} must produce the merged
    // {"local_source": "../actions", "pinned": true}.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("refprops_{}", uuid::Uuid::new_v4())).await;
    let lib = s
        .upsert_library(
            &format!("_test:refprops-{}", uuid::Uuid::new_v4()),
            "npm",
            Some("1.0"),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    s.upsert_referenced_library(
        &fid,
        &lib,
        Some("1.0"),
        Some(serde_json::json!({"local_source": "../actions"})),
    )
    .await
    .unwrap();

    s.upsert_referenced_library(&fid, &lib, Some("1.0"), Some(serde_json::json!({"pinned": true})))
        .await
        .unwrap();

    use sqlx_core::query_as::query_as;
    let (props,): (serde_json::Value,) = query_as(
        "SELECT props FROM sensei.referenced_libraries WHERE folder_id = $1 AND library_id = $2",
    )
    .bind(fid)
    .bind(lib)
    .fetch_one(s.pool())
    .await
    .unwrap();

    assert_eq!(props["local_source"], "../actions", "first pass tag must persist");
    assert_eq!(props["pinned"], true, "second pass tag must merge in");

    // Cleanup — library delete cascades referenced_libraries via FK.
    s.delete_library(&lib).await.unwrap();
}

#[tokio::test]
async fn workspace_library_promotion_shows_in_resolved_and_is_idempotent() {
    // #30: referenced_libraries (folder-grained) must roll up to
    // library_enablement so detected libs — incl. scoped @rokkit/* — show in
    // workspace_libraries_resolved (the Workspaces screen). Was never populated.
    let s = pg_store().await;
    let pid = s.ensure_test_workspace("proj-lib-promo").await.unwrap();
    let lib =
        s.upsert_library("_test:@rokkit/core", "npm", Some("1.2"), None, None, None).await.unwrap();
    // Promote twice — must be idempotent (no error, no duplicate row).
    s.upsert_workspace_library(&lib, &pid).await.unwrap();
    s.upsert_workspace_library(&lib, &pid).await.unwrap();
    let libs = s.get_library_enablement(&pid).await.unwrap();
    let hits = libs.iter().filter(|l| l["name"] == "_test:@rokkit/core").count();
    assert_eq!(
        hits, 1,
        "promoted scoped lib should appear exactly once in resolved view; got {libs:?}"
    );
    s.delete_library(&lib).await.unwrap(); // FK CASCADE removes the library_enablement row
}

#[tokio::test]
async fn ensure_test_workspace_is_namespaced_and_idempotent() {
    // #34: test fixtures must not accrete a new row per run, nor look like
    // real workspaces. Reuse one `_test:`-namespaced row per name.
    let s = pg_store().await;
    let a = s.ensure_test_workspace("dup-check").await.unwrap();
    let b = s.ensure_test_workspace("dup-check").await.unwrap();
    assert_eq!(a, b, "repeated ensure_test_workspace must reuse one row, not create a new one");
    let proj = s.get_workspace(&a).await.unwrap().unwrap();
    assert_eq!(proj["name"], "_test:dup-check", "test workspaces must be _test:-namespaced");
    s.delete_workspace(&a).await.ok();
}

#[tokio::test]
async fn find_folder_for_path_returns_nearest_ancestor() {
    // #31: a hook's cwd (often a subdir) must resolve to its indexed folder.
    let s = pg_store().await;
    let fid = create_test_folder(&s, "sess-nearest").await; // abs_path /_test/sess-nearest
    assert_eq!(
        s.find_folder_for_path("/_test/sess-nearest/src/auth").await.unwrap().map(|(id, _)| id),
        Some(fid),
        "subdir cwd resolves to ancestor folder"
    );
    assert_eq!(
        s.find_folder_for_path("/_test/sess-nearest").await.unwrap().map(|(id, _)| id),
        Some(fid),
        "exact path resolves too"
    );
    assert_eq!(
        s.find_folder_for_path("/_test/nonexistent-xyz/deep").await.unwrap(),
        None,
        "uncovered path resolves to nothing"
    );
}

/// The folder a repaired/created session row points at (test assertion helper).
pub(super) async fn session_row_folder(s: &PgStore, client_session_id: &str) -> Option<uuid::Uuid> {
    let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
        "SELECT folder_id FROM activity.sessions WHERE client_session_id = $1",
    )
    .bind(client_session_id)
    .fetch_optional(&s.pool)
    .await
    .unwrap();
    row.map(|(f,)| f)
}

// The repair operates on ALL orphaned sessions in the DB, and sensei_test is
// persistent + tests run in parallel, so a prior run's rows can linger. Each repair
// test clears its OWN session first, then asserts only the (deterministic) folder its
// session resolves to after repair — never a global "no row exists" precondition.
pub(super) async fn clear_test_session(s: &PgStore, sid: &str) {
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE client_session_id = $1")
        .bind(sid)
        .execute(&s.pool)
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id = $1")
        .bind(sid)
        .execute(&s.pool)
        .await
        .unwrap();
}

/// Serialises EVERY test that invokes a global session repair — both
/// `repair_sessions_from_transcripts` and `repair_orphaned_sessions`.
///
/// Both are deliberately global: they sweep every unattached session, which
/// is what makes them converge as folders become tracked. That also means one
/// test's repair operates on another's fixture. Two ways it bites, both
/// observed: test B's sweep creates the session test A has just asserted does
/// not exist yet (A fails its own precondition), and B's sweep attributes A's
/// half-built fixture to the wrong folder before A finishes building it.
///
/// Any new test that calls a global repair belongs here too. Gating only the
/// transcript pair left the two `repair_orphaned_sessions` tests racing, and
/// leaving `heal_duplicate_name_workspaces` ungated left its two tests racing
/// against every other test's setup — a workspace is a folderless duplicate for
/// the instant between being created and having its folder attached, which is
/// exactly the state the sweep is entitled to prune.
///
/// Blocking on purpose — see [`crate::tasks::test_support::TestGate`] for why
/// an async mutex loses wakers across per-test runtimes.
pub(super) static REPAIR_SWEEP_GATE: crate::tasks::test_support::TestGate =
    crate::tasks::test_support::TestGate::new();

#[tokio::test]
async fn repair_sessions_from_transcripts_creates_the_missing_session() {
    // The gap the events-based repair cannot close: prose was ingested but no
    // session was ever synthesized (the source reconstructed no events, or no
    // cwd resolved at the time). The turns then exist and can join nothing.
    let _gate = REPAIR_SWEEP_GATE.enter();
    let s = pg_store().await;
    let sess = "_test-repair-from-transcript";
    clear_test_session(&s, sess).await;
    sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id = $1")
        .bind(sess)
        .execute(&s.pool)
        .await
        .unwrap();
    let fid = create_test_folder(&s, "repair-tt").await; // /_test/repair-tt

    // A turn carrying its cwd in attrs, historical, with NO session row.
    let turn = crate::transcript::TranscriptTurn {
        turn_index: 1,
        user_text: Some("do the thing".into()),
        assistant_text: "done".into(),
        started_at: chrono::DateTime::from_timestamp_millis(1_700_000_000_000),
        attrs: serde_json::json!({ "cwd": "/_test/repair-tt" }),
        ..Default::default()
    };
    s.upsert_transcript_turns("claude_code", sess, "claude", None, None, &[turn]).await.unwrap();
    assert_eq!(session_row_folder(&s, sess).await, None, "no session before the repair");

    let n = s.repair_sessions_from_transcripts().await.unwrap();
    assert!(n >= 1, "the transcript-only session is created; got {n}");
    assert_eq!(
        session_row_folder(&s, sess).await,
        Some(fid),
        "resolved from the cwd the turn retained in attrs"
    );

    // Historical, not today — otherwise it pollutes recency and every
    // time-windowed metric.
    let (started,): (chrono::DateTime<chrono::Utc>,) = sqlx_core::query_as::query_as(
        "SELECT started_at FROM activity.sessions WHERE client_session_id = $1",
    )
    .bind(sess)
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert!(
        started < chrono::Utc::now() - chrono::Duration::days(365),
        "carries the turn's real timestamp, not now(); got {started}"
    );

    // Idempotent: a second pass finds nothing to do for this session.
    s.repair_sessions_from_transcripts().await.unwrap();
    let (rows,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM activity.sessions WHERE client_session_id = $1",
    )
    .bind(sess)
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert_eq!(rows, 1, "re-running the backfill does not duplicate the session");

    clear_test_session(&s, sess).await;
}

#[tokio::test]
async fn a_task_id_lookup_is_scoped_to_the_issuing_daemon_session() {
    // Task ids restart at 1 on every boot while `task_executions` accumulates
    // forever, so id 1 accrues rows from every session the daemon has ever
    // run. An unscoped lookup returned a pile of unrelated tasks of other
    // KINDS — a follower would read another session's failure as its own.
    let s = pg_store().await;
    let task_id = 999_777_555i64;
    sqlx_core::query::query("DELETE FROM activity.task_executions WHERE task_id = $1")
        .bind(task_id)
        .execute(&s.pool)
        .await
        .unwrap();

    let boundary = chrono::Utc::now();
    // One row from a "previous session" (before the boundary), one from this.
    for (kind, started) in [
        ("scan_root", boundary - chrono::Duration::hours(3)),
        ("backfill_coverage", boundary + chrono::Duration::seconds(1)),
    ] {
        sqlx_core::query::query(
            "INSERT INTO activity.task_executions(task_id, task_kind, status, started_at) \
                 VALUES($1, $2::sensei.task_execution_kind, 'completed', $3)",
        )
        .bind(task_id)
        .bind(kind)
        .bind(started)
        .execute(&s.pool)
        .await
        .unwrap();
    }

    let rows = s.task_execution_attempts(task_id, boundary).await.unwrap();

    sqlx_core::query::query("DELETE FROM activity.task_executions WHERE task_id = $1")
        .bind(task_id)
        .execute(&s.pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1, "only this session's row: {rows:?}");
    assert_eq!(rows[0]["kind"], "backfill_coverage");
}

#[tokio::test]
async fn repair_from_transcripts_leaves_an_unresolvable_cwd_alone() {
    // A wrong attribution is worse than a missing one: a cwd that resolves to
    // no tracked folder must NOT be attached to a guessed repository.
    let _gate = REPAIR_SWEEP_GATE.enter();
    let s = pg_store().await;
    let sess = "_test-repair-tt-unresolvable";
    clear_test_session(&s, sess).await;
    sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id = $1")
        .bind(sess)
        .execute(&s.pool)
        .await
        .unwrap();
    let turn = crate::transcript::TranscriptTurn {
        turn_index: 1,
        user_text: Some("x".into()),
        assistant_text: "y".into(),
        started_at: chrono::DateTime::from_timestamp_millis(1_700_000_000_000),
        attrs: serde_json::json!({ "cwd": "/_nowhere/not/tracked" }),
        ..Default::default()
    };
    s.upsert_transcript_turns("claude_code", sess, "claude", None, None, &[turn]).await.unwrap();
    s.repair_sessions_from_transcripts().await.unwrap();
    assert_eq!(
        session_row_folder(&s, sess).await,
        None,
        "left unattached rather than misattributed"
    );
    sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id = $1")
        .bind(sess)
        .execute(&s.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn repair_orphaned_sessions_reattaches_via_alias() {
    // A session captured under a since-renamed repo: its events survived but the
    // session row was cascade-deleted. The repair recreates the row, resolving the
    // folder from the (old) cwd via the alias.
    let _gate = REPAIR_SWEEP_GATE.enter();
    let s = pg_store().await;
    let sess = "_test-repair-orphan-session";
    clear_test_session(&s, sess).await;
    let fid = create_test_folder(&s, "repair-new").await; // /_test/repair-new
    s.add_folder_path_alias("/_test/repair-old", &fid, "rename").await.unwrap();
    // an orphaned event under the OLD path (a subdir) — no session row.
    s.insert_hook_event(&HookEventRow {
        session_id: sess,
        assistant_family: "claude",
        event_type: "PreToolUse",
        tool_name: None,
        cwd: Some("/_test/repair-old/src"),
        ts: 1_700_000_000,
        success: None,
        payload: &serde_json::json!({}),
    })
    .await
    .unwrap();
    let repaired = s.repair_orphaned_sessions().await.unwrap();
    assert!(repaired >= 1, "at least this orphaned session is re-attached; got {repaired}");
    assert_eq!(
        session_row_folder(&s, sess).await,
        Some(fid),
        "the session row now exists and points at the current folder (resolved via the alias)"
    );
    // The repaired row must carry the event's REAL historical timestamp, not now() —
    // else a long-dead session masquerades as "today" and pollutes recency + FTR windows.
    let (started, backfilled, repo_anchor): (chrono::DateTime<chrono::Utc>, bool, Option<uuid::Uuid>) =
            sqlx_core::query_as::query_as(
                "SELECT started_at, backfilled, repo_folder_id FROM activity.sessions WHERE client_session_id = $1",
            ).bind(sess).fetch_one(&s.pool).await.unwrap();
    assert!(backfilled, "a repaired historical session is marked backfilled");
    assert!(
        started < chrono::Utc::now() - chrono::Duration::days(365),
        "started_at is backfilled to the event's historical ts (1_700_000_000ms), not now(); got {started}"
    );
    assert_eq!(repo_anchor, Some(fid), "the repaired session anchors to its repo (P1)");
}

#[tokio::test]
async fn repair_prefers_the_renamed_subdir_over_a_live_parent() {
    // The defect this guards: a session with events under BOTH a still-live parent
    // (`/_test/shadow-parent`) AND a renamed subdir aliased to a different folder
    // (`/_test/shadow-parent/sub` → new folder). Most-specific-first must attribute
    // it to the renamed subdir's folder, not the shadowing parent.
    let _gate = REPAIR_SWEEP_GATE.enter();
    let s = pg_store().await;
    let sess = "_test-shadow-session";
    clear_test_session(&s, sess).await;
    let parent = create_test_folder(&s, "shadow-parent").await; // live /_test/shadow-parent
    let moved = create_test_folder(&s, "shadow-moved").await; // the renamed subdir's new home
    s.add_folder_path_alias("/_test/shadow-parent/sub", &moved, "rename").await.unwrap();
    // events under BOTH the live parent and the renamed subdir.
    s.insert_hook_event(&HookEventRow {
        session_id: sess,
        assistant_family: "claude",
        event_type: "PreToolUse",
        tool_name: None,
        cwd: Some("/_test/shadow-parent"),
        ts: 1_700_000_100,
        success: None,
        payload: &serde_json::json!({}),
    })
    .await
    .unwrap();
    s.insert_hook_event(&HookEventRow {
        session_id: sess,
        assistant_family: "claude",
        event_type: "PreToolUse",
        tool_name: None,
        cwd: Some("/_test/shadow-parent/sub/x"),
        ts: 1_700_000_200,
        success: None,
        payload: &serde_json::json!({}),
    })
    .await
    .unwrap();
    s.repair_orphaned_sessions().await.unwrap();
    assert_eq!(
        session_row_folder(&s, sess).await,
        Some(moved),
        "attributes to the renamed subdir (via its alias), NOT the shadowing live parent"
    );
    assert_ne!(session_row_folder(&s, sess).await, Some(parent));
}
