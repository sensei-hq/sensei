use super::*;
use sqlx_core::query_as::query_as;

// Every test here connects via `PgStore::connect_test()` — the tiny floorless
// pool. Do NOT reach for `PgStore::connect()`: that is the daemon's pool with a
// warm floor of `DB_POOL_MIN_CONNECTIONS`, and cargo runs these tests in
// parallel, so each one would hold 8 connections open and the suite would blow
// past Postgres's `max_connections` (it did: 80 of 223 tests failed to connect
// at default parallelism). `connect_test` also owns the URL default, including
// the never-point-at-`sensei` guard.

#[tokio::test]
async fn connect_to_pg() {
    let store = PgStore::connect_test().await.unwrap();
    let row: (i32,) = query_as("SELECT 1").fetch_one(store.pool()).await.unwrap();
    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn execute_raw_works() {
    let store = PgStore::connect_test().await.unwrap();
    store.execute_raw("SELECT 1").await.unwrap();
}

#[tokio::test]
async fn schema_exists() {
    let store = PgStore::connect_test().await.unwrap();
    let row: (bool,) = query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'sensei')",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(row.0, "sensei schema must exist — run `dbd apply` first");
}

// ── Config tests ───────────────────────────────────────────────

async fn pg_store() -> PgStore {
    PgStore::connect_test().await.unwrap()
}

/// Generate a unique key prefix for test isolation.
fn tkey(test: &str, key: &str) -> String {
    format!("_test:{}:{}", test, key)
}

#[tokio::test]
async fn config_set_and_get() {
    let s = pg_store().await;
    let k = tkey("set_get", "theme");
    s.set_config(&k, "dark").await.unwrap();
    assert_eq!(s.get_config(&k).await.unwrap(), Some("dark".into()));
    s.delete_config(&k).await.unwrap(); // cleanup
}

#[tokio::test]
async fn config_get_missing_returns_none() {
    let s = pg_store().await;
    assert_eq!(s.get_config("_test:missing:nonexistent").await.unwrap(), None);
}

#[tokio::test]
async fn config_set_overwrites() {
    let s = pg_store().await;
    let k = tkey("overwrite", "k");
    s.set_config(&k, "v1").await.unwrap();
    s.set_config(&k, "v2").await.unwrap();
    assert_eq!(s.get_config(&k).await.unwrap(), Some("v2".into()));
    s.delete_config(&k).await.unwrap();
}

#[tokio::test]
async fn config_delete() {
    let s = pg_store().await;
    let k = tkey("delete", "k");
    s.set_config(&k, "v").await.unwrap();
    s.delete_config(&k).await.unwrap();
    assert_eq!(s.get_config(&k).await.unwrap(), None);
}

#[tokio::test]
async fn config_delete_nonexistent_is_noop() {
    let s = pg_store().await;
    s.delete_config("_test:noop:nope").await.unwrap();
}

#[tokio::test]
async fn config_get_all() {
    let s = pg_store().await;
    let k1 = tkey("getall", "a");
    let k2 = tkey("getall", "b");
    s.set_config(&k1, "1").await.unwrap();
    s.set_config(&k2, "2").await.unwrap();
    let all = s.get_all_config().await.unwrap();
    assert_eq!(all[&k1], "1");
    assert_eq!(all[&k2], "2");
    s.delete_config(&k1).await.unwrap();
    s.delete_config(&k2).await.unwrap();
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

// ── Scan exclusions (per watch root) ──────────────────────────────

#[tokio::test]
async fn root_exclusion_prefixes_resolves_relative_entries_against_root() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root = format!("/_test/exroot/{uniq}");
    let id =
        s.add_watch_root(&root, "ex", &serde_json::json!(["Code", "archive/old"])).await.unwrap();
    let mut prefixes = s.root_exclusion_prefixes(&root).await.unwrap();
    prefixes.sort();
    assert_eq!(prefixes, vec![format!("{root}/Code"), format!("{root}/archive/old")]);
    // get_watch_root round-trips the raw relative list.
    let (path, ex) = s.get_watch_root(&id).await.unwrap().unwrap();
    assert_eq!(path, root);
    assert!(ex.contains(&"Code".to_string()));
    s.remove_watch_root(&id).await.ok();
}

#[tokio::test]
async fn prune_under_prefix_deletes_subtree_keeps_siblings() {
    // Exclusion prune: every folder at or under the prefix is deleted; a
    // sibling that only shares the prefix string is kept (boundary-safe).
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root_path = format!("/_test/prune_prefix/{uniq}");
    let root_id =
        s.add_watch_root(&root_path, "prune_prefix_root", &serde_json::json!([])).await.unwrap();

    let code = format!("{root_path}/Code");
    let inside = format!("{code}/archive/repo");
    let sibling = format!("{root_path}/Coder"); // shares the "Code" prefix string
    let code_fid = s.upsert_repo_kind(&root_id, "git", "Code", &code).await.unwrap();
    s.upsert_repo_kind(&root_id, "git", "repo", &inside).await.unwrap();
    let sib_fid = s.upsert_repo_kind(&root_id, "git", "Coder", &sibling).await.unwrap();

    let deleted = s.prune_under_prefix(&code).await.unwrap();
    assert_eq!(deleted, 2, "the prefix folder and its descendant are deleted");

    for (fid, alive, msg) in [(code_fid, 0, "prefix folder deleted"), (sib_fid, 1, "sibling kept")]
    {
        let (n,): (i64,) =
            sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.folders WHERE id=$1")
                .bind(fid)
                .fetch_one(s.pool())
                .await
                .unwrap();
        assert_eq!(n, alive, "{msg}");
    }
    // cleanup
    s.prune_under_prefix(&root_path).await.unwrap();
}

#[tokio::test]
async fn prune_empty_projects_grace_protects_fresh() {
    // A just-created empty `discovery` project must survive a grace>0 prune —
    // its folder may still be attaching in a concurrent step. (The grace=0
    // path is deliberate/global and unsafe to exercise in the shared test DB,
    // so it's covered by the exclusion handler in production, not here.)
    let s = pg_store().await;
    let fresh = s
        .create_project(&format!("_test:grace-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    s.prune_empty_projects(60).await.unwrap();
    let (alive,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.projects WHERE id=$1")
            .bind(fresh)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(alive, 1, "grace protects a fresh empty project");
    // Direct cleanup (not a grace=0 global prune, which would hit sibling tests).
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id=$1")
        .bind(fresh)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn tag_file_nodes_by_framework_kind_aggregates_symbol_kinds() {
    // G5b: a file node gets tagged with the framework kinds of the symbols it
    // contains, so `get_patterns`/`get_file_tags` return real files. A file
    // with no framework symbols stays untagged; a stale tag is cleared.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root_id = s
        .add_watch_root(&format!("/_test/tag/{uniq}"), "tag_root", &serde_json::json!([]))
        .await
        .unwrap();
    let fid = s
        .upsert_repo_kind(&root_id, "git", "repo", &format!("/_test/tag/{uniq}/repo"))
        .await
        .unwrap();

    // A .svelte file that defines a component and uses a hook.
    let widget = s
        .upsert_node(&fid, "file", "Widget.svelte", "src/Widget.svelte", None, None, None, None)
        .await
        .unwrap();
    s.upsert_node(&fid, "component", "Widget", "src/Widget.svelte", None, None, None, None)
        .await
        .unwrap();
    s.upsert_node(&fid, "hook", "effect", "src/Widget.svelte", None, None, None, None)
        .await
        .unwrap();
    // A plain file with only a function → no framework tag.
    let util = s
        .upsert_node(&fid, "file", "util.rs", "src/util.rs", None, None, None, None)
        .await
        .unwrap();
    s.upsert_node(&fid, "function", "helper", "src/util.rs", None, None, None, None).await.unwrap();

    // File-role by path convention (no symbols needed): SvelteKit routes +
    // middleware, and a Next-style middleware file.
    let page = s
        .upsert_node(
            &fid,
            "file",
            "+page.svelte",
            "src/routes/blog/+page.svelte",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let endpoint = s
        .upsert_node(
            &fid,
            "file",
            "+server.ts",
            "src/routes/api/+server.ts",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let hooks = s
        .upsert_node(&fid, "file", "hooks.server.ts", "src/hooks.server.ts", None, None, None, None)
        .await
        .unwrap();
    let mw = s
        .upsert_node(&fid, "file", "middleware.ts", "middleware.ts", None, None, None, None)
        .await
        .unwrap();

    let changed = s.tag_file_nodes_by_framework_kind(&root_id).await.unwrap();
    assert!(changed >= 1, "at least the component/hook file is tagged");

    let (widget_tags,): (Vec<String>,) =
        sqlx_core::query_as::query_as("SELECT tags FROM sensei.nodes WHERE id=$1")
            .bind(widget)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(
        widget_tags,
        vec!["component".to_string(), "hook".to_string()],
        "file tagged with its symbol kinds (sorted)"
    );
    let (util_tags,): (Vec<String>,) =
        sqlx_core::query_as::query_as("SELECT tags FROM sensei.nodes WHERE id=$1")
            .bind(util)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(util_tags.is_empty(), "a file with no framework symbols stays untagged");

    // File-role tags come from the path convention alone.
    let pool_ref = s.pool();
    let tags_of = |id: uuid::Uuid| async move {
        let (t,): (Vec<String>,) =
            sqlx_core::query_as::query_as("SELECT tags FROM sensei.nodes WHERE id=$1")
                .bind(id)
                .fetch_one(pool_ref)
                .await
                .unwrap();
        t
    };
    assert_eq!(tags_of(page).await, vec!["route".to_string()], "+page.svelte → route");
    assert_eq!(tags_of(endpoint).await, vec!["route".to_string()], "+server.ts → route");
    assert_eq!(
        tags_of(hooks).await,
        vec!["middleware".to_string()],
        "hooks.server.ts → middleware"
    );
    assert_eq!(tags_of(mw).await, vec!["middleware".to_string()], "middleware.ts → middleware");

    // Idempotent: a second run changes nothing.
    assert_eq!(s.tag_file_nodes_by_framework_kind(&root_id).await.unwrap(), 0, "no-op on re-run");

    // cleanup
    let pool = s.pool();
    sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id=$1")
        .bind(fid)
        .execute(pool)
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id=$1")
        .bind(root_id)
        .execute(pool)
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1")
        .bind(root_id)
        .execute(pool)
        .await
        .ok();
}

/// Create a unique test folder for FK tests. Uses suffix for isolation.
async fn create_test_folder(s: &PgStore, suffix: &str) -> uuid::Uuid {
    use sqlx_core::query_as::query_as;
    s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
    let abs_path = format!("/_test/{}", suffix);
    let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2) ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(suffix).bind(&abs_path).fetch_one(s.pool()).await.unwrap();
    row.0
}

/// Create a unique (project, folder) pair for FK tests that need both,
/// wiring the folder to the project. Used by the pattern tests since
/// detected_patterns is project-scoped (#82) and needs a non-null
/// project_id, while `list_patterns_by_folder` still keys on folder.
async fn create_test_project_and_folder(s: &PgStore, suffix: &str) -> (uuid::Uuid, uuid::Uuid) {
    let pid = s.create_project(&format!("_test:{}", suffix), None, None).await.unwrap();
    let fid = create_test_folder(s, suffix).await;
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(pid)
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();
    (pid, fid)
}

// ── Dōjō confidentiality: project identifiers (C5) ─────────────────

#[test]
fn repo_tokens_from_remote_parses_ssh_and_https() {
    assert_eq!(
        repo_tokens_from_remote("git@github.com:acme/acme-api.git"),
        vec!["acme-api".to_string(), "acme".to_string()]
    );
    assert_eq!(
        repo_tokens_from_remote("https://github.com/acme/acme-api"),
        vec!["acme-api".to_string(), "acme".to_string()]
    );
    // Host-like and empty segments are skipped.
    assert!(repo_tokens_from_remote("https://example.com").is_empty());
}

#[test]
fn remote_owner_slug_extracts_lowercased_owner() {
    // ssh + https, mixed case → the owner (segment before the repo), lowercased.
    assert_eq!(
        remote_owner_slug("git@github.com:Sensei-HQ/sensei.git").as_deref(),
        Some("sensei-hq")
    );
    assert_eq!(
        remote_owner_slug("https://github.com/Sensei-HQ/sensei").as_deref(),
        Some("sensei-hq")
    );
    assert_eq!(remote_owner_slug("https://gitlab.com/acme/api.git").as_deref(), Some("acme"));
    // No owner segment / unparseable → None.
    assert_eq!(remote_owner_slug("https://example.com"), None);
    assert_eq!(remote_owner_slug(""), None);
}

#[tokio::test]
async fn suggest_binding_infers_from_git_owner_then_stops_once_bound() {
    let Ok(s) = PgStore::connect_test().await else {
        return;
    };
    let suffix = format!("suggestbind_{}", uuid::Uuid::new_v4());
    let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;
    // The project's repo is owned by "Acme" (mixed case in the remote).
    sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
        .bind(fid)
        .bind(serde_json::json!([{ "name": "origin", "url": "git@github.com:Acme/widget.git" }]))
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(
        s.project_org_owners(&pid).await.unwrap(),
        vec!["acme".to_string()],
        "owner parsed + lowercased"
    );

    // No membership connected yet → no suggestion.
    assert!(crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().is_none());

    // Connect a client membership covering "acme".
    let mid = uuid::Uuid::new_v4();
    s.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/acme".into(),
        dojo_url: "http://localhost:7755/github/acme".into(),
        kind: "client".into(),
        org_slugs: vec!["acme".into()],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();

    // Now it suggests that membership, explaining which owner matched.
    let sug =
        crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().expect("a suggestion");
    assert_eq!(sug.membership_id, mid);
    assert_eq!(sug.kind, "client");
    assert_eq!(sug.matched_slug, "acme");
    assert_eq!(sug.tenant_key, "github/acme");

    // Once the project is bound, the chip no longer applies.
    assert!(s.bind_project_to_dojo(&pid, Some(&mid)).await.unwrap());
    assert!(crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().is_none());

    s.bind_project_to_dojo(&pid, None).await.unwrap();
    s.delete_dojo_membership(&mid).await.unwrap();
}

#[tokio::test]
async fn project_identifiers_gathers_names_paths_repos_and_sessions() {
    let s = pg_store().await;
    let suffix = format!("projident_{}", uuid::Uuid::new_v4());
    let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;
    // Set the client + a git remote so the parser has something to chew on.
    sqlx_core::query::query("UPDATE sensei.projects SET client = $2 WHERE id = $1")
        .bind(pid)
        .bind("Acme Corp")
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
        .bind(fid)
        .bind(serde_json::json!([{ "name": "origin", "url": "git@github.com:acme/acme-api.git" }]))
        .execute(s.pool())
        .await
        .unwrap();
    // A session for the project, with a client_session_id.
    let csid = format!("cs-{suffix}");
    s.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();

    let ids = s.project_identifiers(&pid).await.unwrap();
    assert_eq!(ids.project_name.as_deref(), Some(format!("_test:{suffix}").as_str()));
    assert_eq!(ids.client_name.as_deref(), Some("Acme Corp"));
    assert!(
        ids.repo_names.iter().any(|r| r == "acme-api"),
        "repo from remote missing: {:?}",
        ids.repo_names
    );
    assert!(
        ids.repo_names.iter().any(|r| r == "acme"),
        "owner from remote missing: {:?}",
        ids.repo_names
    );
    assert!(
        ids.folder_paths.iter().any(|p| p.contains(&suffix)),
        "folder path missing: {:?}",
        ids.folder_paths
    );
    assert!(
        ids.session_ids.iter().any(|sid| sid == &csid),
        "client_session_id missing: {:?}",
        ids.session_ids
    );
    // The observatory session UUID is also present.
    assert!(ids.session_ids.len() >= 2, "expected uuid + client_session_id: {:?}", ids.session_ids);

    // Cleanup (project delete cascades folders/sessions via FKs).
    s.delete_project(&pid).await.ok();
}

// ── PG Function tests ─────────────────────────────────────────────

#[tokio::test]
async fn rank_bm25_returns_results() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("bm25_{}", uuid::Uuid::new_v4())).await;
    s.upsert_node(
        &fid,
        "function",
        "authenticate_user",
        "src/auth.rs",
        None,
        Some("fn authenticate_user(token: &str)"),
        Some(1),
        Some(20),
    )
    .await
    .unwrap();
    s.upsert_node(
        &fid,
        "function",
        "validate_email",
        "src/validation.rs",
        None,
        Some("fn validate_email(email: &str)"),
        Some(1),
        Some(10),
    )
    .await
    .unwrap();
    let results = s.rank_bm25(&fid, "authenticate").await.unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].0, "src/auth.rs");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn rank_bm25_empty_folder() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("bm25_empty_{}", uuid::Uuid::new_v4())).await;
    let results = s.rank_bm25(&fid, "anything").await.unwrap();
    assert!(results.is_empty());
}

// ── Nodes + Edges tests ────────────────────────────────────────────

#[tokio::test]
async fn node_upsert_and_query() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("node_{}", uuid::Uuid::new_v4())).await;
    let file_id = s
        .upsert_node(&fid, "file", "main.rs", "src/main.rs", None, None, None, None)
        .await
        .unwrap();
    let fn_id = s
        .upsert_node(
            &fid,
            "function",
            "main",
            "src/main.rs",
            Some(&file_id),
            Some("fn main()"),
            Some(1),
            Some(10),
        )
        .await
        .unwrap();
    let nodes = s.get_nodes_by_folder(&fid).await.unwrap();
    assert_eq!(nodes.len(), 2);
    let by_file = s.get_nodes_by_file(&fid, "src/main.rs").await.unwrap();
    assert_eq!(by_file.len(), 2);
    s.delete_nodes_by_folder(&fid).await.unwrap();
    assert_eq!(s.get_nodes_by_folder(&fid).await.unwrap().len(), 0);
    let _ = (file_id, fn_id);
}

#[tokio::test]
async fn upsert_persists_doc_and_symbol_kinds() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("kinds_{}", uuid::Uuid::new_v4())).await;
    // Each of these failed the enum cast before the fix and was dropped.
    for (kind, name, path) in [
        ("doc", "README", "README.md"),
        ("struct", "Point", "src/geo.rs"),
        ("component", "Button", "src/Button.svelte"),
        ("hook", "useState", "src/Button.svelte"),
        ("extension", "review", "marketplace/commands/review.md"),
    ] {
        s.upsert_node(&fid, kind, name, path, None, None, Some(1), Some(2))
            .await
            .unwrap_or_else(|e| panic!("upsert {kind} failed: {e}"));
    }
    let kinds = s.count_nodes_by_kind(&fid).await.unwrap();
    for kind in ["doc", "struct", "component", "hook", "extension"] {
        assert_eq!(kinds.get(kind), Some(&1), "missing {kind} node");
    }
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn doc_nodes_are_embeddable() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("embed_{}", uuid::Uuid::new_v4())).await;
    s.upsert_node(&fid, "doc", "README", "README.md", None, None, Some(1), Some(2)).await.unwrap();
    let pending = s.nodes_without_embeddings(&fid, 100).await.unwrap();
    assert!(
        pending.iter().any(|(_, kind, name, _, _)| kind == "doc" && name == "README"),
        "doc node not returned by nodes_without_embeddings"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn semantic_search_nodes_ranks_by_cosine() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("sem_{}", uuid::Uuid::new_v4())).await;

    // Two function nodes whose *names* share no keyword with the query.
    // 384-dim (matches the vector(384) column). `alpha` points along dim 0,
    // `beta` along dim 1 — orthogonal, so the query vector's direction alone
    // decides ranking (purely semantic, no lexical overlap).
    let dim = 384usize;
    let mut e_alpha = vec![0.0f32; dim];
    e_alpha[0] = 1.0;
    let mut e_beta = vec![0.0f32; dim];
    e_beta[1] = 1.0;

    let id_alpha = s
        .upsert_node(&fid, "function", "alpha", "a.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    let id_beta = s
        .upsert_node(&fid, "function", "beta", "b.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    s.set_node_embedding(&id_alpha, &e_alpha).await.unwrap();
    s.set_node_embedding(&id_beta, &e_beta).await.unwrap();

    // Query vector leans toward alpha's direction.
    let mut query = vec![0.0f32; dim];
    query[0] = 0.9;
    query[1] = 0.1;

    let hits = s.semantic_search_nodes(&[fid], &query, &["function", "method"], 10).await.unwrap();

    let names: Vec<&str> = hits.iter().map(|(_, name, ..)| name.as_str()).collect();
    assert!(
        names.contains(&"alpha") && names.contains(&"beta"),
        "both nodes should surface, got {names:?}"
    );
    assert_eq!(
        names.first(),
        Some(&"alpha"),
        "alpha is the closest by cosine — must rank first, got {names:?}"
    );

    // A kind filter that matches neither node returns nothing.
    let none = s.semantic_search_nodes(&[fid], &query, &["class"], 10).await.unwrap();
    assert!(none.is_empty(), "kind filter should exclude functions, got {none:?}");

    // Empty inputs are cheap no-ops, never a query.
    assert!(s.semantic_search_nodes(&[], &query, &["function"], 10).await.unwrap().is_empty());
    assert!(s.semantic_search_nodes(&[fid], &[], &["function"], 10).await.unwrap().is_empty());

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn edge_insert_and_query() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("edge_{}", uuid::Uuid::new_v4())).await;
    let fn_a =
        s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
    let fn_b =
        s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();
    s.insert_edge(&fid, &fn_a, Some(&fn_b), None, None, "calls").await.unwrap();
    let callers = s.get_callers(&fn_b).await.unwrap();
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0]["caller_id"], fn_a.to_string());
    let callees = s.get_callees(&fn_a).await.unwrap();
    assert_eq!(callees.len(), 1);
    let by_kind = s.get_edges_by_kind(&fid, "calls").await.unwrap();
    assert_eq!(by_kind.len(), 1);
    s.delete_nodes_by_folder(&fid).await.unwrap(); // cascades edges
}

#[tokio::test]
async fn insert_edge_is_idempotent() {
    // D1: edges have identity — inserting the same edge twice returns the
    // SAME id and adds no second row, for both resolved and unresolved edges.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("edgeidem_{}", uuid::Uuid::new_v4())).await;
    let a =
        s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
    let b =
        s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();

    // Resolved edge: a repeated identical insert upserts to the same row.
    let e1 = s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();
    let e2 = s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();
    assert_eq!(e1, e2, "a repeated resolved edge returns the same id");
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        1,
        "no duplicate resolved edge"
    );

    // Unresolved edge: a repeated insert (same source, target_name, kind)
    // upserts to the same row (nulls-not-distinct target_file).
    let u1 = s.insert_edge(&fid, &a, None, Some("ext_fn"), None, "calls").await.unwrap();
    let u2 = s.insert_edge(&fid, &a, None, Some("ext_fn"), None, "calls").await.unwrap();
    assert_eq!(u1, u2, "a repeated unresolved edge returns the same id");
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        2,
        "one resolved (a→b) + one unresolved (a→ext_fn), no dupes"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn resolve_edge_merges_into_existing_resolved_edge() {
    // D1: promoting an unresolved edge to a target that already has a resolved
    // edge from the same (source, kind) must MERGE (delete the loser), not
    // throw a unique violation against edges_unique_resolved.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("resolvemerge_{}", uuid::Uuid::new_v4())).await;
    let a =
        s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
    let b =
        s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();

    s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap(); // resolved a→b
    let u = s.insert_edge(&fid, &a, None, Some("b"), None, "calls").await.unwrap(); // unresolved a→"b"
    // The resolved and unresolved partial indexes are DISJOINT: both edges
    // coexist (no collision on insert) until resolution merges them.
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        2,
        "resolved a→b and unresolved a→\"b\" coexist as two rows"
    );

    s.resolve_edge(&u, &b).await.unwrap(); // collides with a→b → merge

    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        1,
        "the redundant edge is merged away, not duplicated"
    );
    let exists: (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.edges WHERE id=$1)")
        .bind(u)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert!(!exists.0, "the loser unresolved edge is deleted");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_communities_for_folder_kills_stale_and_orphans() {
    // D4 invariant 5: the per-folder replace DELETEs stale community rows,
    // CLEARs every node's community_id, then writes the new set — no orphaned
    // rows, no stranded community_ids. Per-folder, sum(node_count) equals the
    // count of nodes actually carrying a community_id.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("comm_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b = s
        .upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let c = s
        .upsert_node(&fid, "function", "c", "a.rs", None, Some("()"), Some(5), Some(6))
        .await
        .unwrap();

    // Stale prior state: community 99 + nodes a & c assigned to it.
    s.upsert_community(&fid, 99, "stale", 2).await.unwrap();
    s.update_node_community(&a, 99).await.unwrap();
    s.update_node_community(&c, 99).await.unwrap();

    // Replace with a single community {1: [a, b]} — c must be orphaned out.
    s.replace_communities_for_folder(
        &fid,
        &[CommunityAssignment {
            community_id: 1,
            label: "new".into(),
            member_node_ids: vec![a, b],
            god_node_ids: vec![a],
        }],
    )
    .await
    .unwrap();

    assert_eq!(s.list_communities(&fid).await.unwrap().len(), 1, "stale community 99 is gone");
    let cid = |id: uuid::Uuid| {
        let s = &s;
        async move {
            let (v,): (Option<i32>,) =
                query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                    .bind(id)
                    .fetch_one(s.pool())
                    .await
                    .unwrap();
            v
        }
    };
    assert_eq!(cid(a).await, Some(1), "a assigned to the new community");
    assert_eq!(cid(b).await, Some(1), "b assigned to the new community");
    assert_eq!(cid(c).await, None, "c's stale community_id is cleared (orphan removed)");

    // Per-folder integrity: claimed node_count == real nodes carrying a community_id.
    let (claimed,): (i64,) = query_as(
        "SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let (real,): (i64,) = query_as(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(claimed, real, "claimed == real (invariant 5)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_communities_reruns_change_zero_rows() {
    // Same class of fix as the degree guard: community_id is deterministic for
    // an identical graph (invariant 2), so re-applying the SAME assignment must
    // rewrite 0 node rows. The old null-all-then-reset rewrote every community
    // node TWICE per DetectCommunities pass — dead-tuple churn on every re-scan.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("comm_norerun_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b = s
        .upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let assignment = vec![CommunityAssignment {
        community_id: 1,
        label: "one".into(),
        member_node_ids: vec![a, b],
        god_node_ids: vec![a],
    }];

    let first = s.replace_communities_for_folder(&fid, &assignment).await.unwrap();
    assert_eq!(first, 2, "first assignment sets community_id on both members");

    // Re-detect with the SAME (deterministic) assignment → 0 node rows change.
    let second = s.replace_communities_for_folder(&fid, &assignment).await.unwrap();
    assert_eq!(second, 0, "an unchanged re-detect rewrites 0 nodes (no dead tuples)");

    // A real change (b moves to its own community 2) still writes only what moved.
    let moved = s
        .replace_communities_for_folder(
            &fid,
            &[
                CommunityAssignment {
                    community_id: 1,
                    label: "one".into(),
                    member_node_ids: vec![a],
                    god_node_ids: vec![a],
                },
                CommunityAssignment {
                    community_id: 2,
                    label: "two".into(),
                    member_node_ids: vec![b],
                    god_node_ids: vec![b],
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(moved, 1, "only b changed community (a stays in 1)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn detect_communities_assigns_deterministic_ids_by_natural_key() {
    // D4 invariant 2: community_id is DETERMINISTIC — communities are ranked
    // 1..k by the natural key (file_path, line_start, …) of their smallest
    // member, so an identical graph always yields identical ids. Two disjoint
    // triangles ⇒ two communities; the one holding the earliest node is #1.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commdet_{}", uuid::Uuid::new_v4())).await;
    let mut n = std::collections::HashMap::new();
    for (name, line) in [("a", 10), ("b", 20), ("c", 30), ("d", 40), ("e", 50), ("f", 60)] {
        let id = s
            .upsert_node(
                &fid,
                "function",
                name,
                "a.rs",
                None,
                Some("()"),
                Some(line),
                Some(line + 1),
            )
            .await
            .unwrap();
        n.insert(name, id);
    }
    // Two disjoint triangles: {a,b,c} and {d,e,f} (resolved calls).
    for (x, y) in [("a", "b"), ("b", "c"), ("c", "a"), ("d", "e"), ("e", "f"), ("f", "d")] {
        s.insert_edge(&fid, &n[x], Some(&n[y]), None, None, "calls").await.unwrap();
    }

    let read_ids = |s: &PgStore, n: &std::collections::HashMap<&str, uuid::Uuid>| {
        let ids: Vec<uuid::Uuid> = ["a", "b", "c", "d", "e", "f"].iter().map(|k| n[*k]).collect();
        let pool = s.pool().clone();
        async move {
            let mut out = Vec::new();
            for id in ids {
                let (v,): (Option<i32>,) =
                    query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                        .bind(id)
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                out.push(v);
            }
            out
        }
    };

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
    let first: Vec<Option<i32>> = read_ids(&s, &n).await;
    // {a,b,c} share a community; {d,e,f} share another; triangle-a (earliest
    // natural key) is community 1, triangle-d is 2.
    assert_eq!(&first[0..3], &[Some(1), Some(1), Some(1)], "earliest triangle is community 1");
    assert_eq!(&first[3..6], &[Some(2), Some(2), Some(2)], "later triangle is community 2");

    // Invariant 5 after a REAL detect run (not just the hand-built replace):
    // claimed sum(node_count) == real nodes carrying a community_id.
    let (claimed,): (i64,) = query_as(
        "SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let (real,): (i64,) = query_as(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(claimed, real, "per-folder claimed == real after detect_communities (invariant 5)");

    // Re-running over the identical graph yields the identical assignment.
    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
    let second: Vec<Option<i32>> = read_ids(&s, &n).await;
    assert_eq!(first, second, "identical graph ⇒ identical community ids (deterministic)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn detect_communities_clears_stale_on_empty_folder() {
    // D4 invariant 5: running detection on a folder that has become empty
    // clears its stale community rows (the nodes.is_empty() → replace([]) path).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commempty_{}", uuid::Uuid::new_v4())).await;
    s.upsert_community(&fid, 1, "stale", 3).await.unwrap();
    assert_eq!(s.list_communities(&fid).await.unwrap().len(), 1, "seeded a stale community");

    // No nodes exist for this folder → detection must clear the stale rows.
    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
    assert!(
        s.list_communities(&fid).await.unwrap().is_empty(),
        "an empty folder's stale communities are cleared"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn community_coverage_full_singletons_inherit_file_community() {
    // D4.4: every node gets a community_id. A file's symbols with NO call/
    // import edge still land in a community via `parent_id` containment
    // (they cluster under the file), and any residual singleton inherits its
    // enclosing file community — so coverage is ~100% (invariant 5), not just
    // the nodes that happen to carry a resolved semantic edge.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commcov_{}", uuid::Uuid::new_v4())).await;
    // A file with a struct + two methods, and NO edges between any of them.
    let file = s
        .upsert_node(&fid, "file", "widget.rs", "src/widget.rs", None, None, Some(1), Some(99))
        .await
        .unwrap();
    s.upsert_node(&fid, "struct", "Widget", "src/widget.rs", Some(&file), None, Some(2), Some(2))
        .await
        .unwrap();
    s.upsert_node(
        &fid,
        "method",
        "new",
        "src/widget.rs",
        Some(&file),
        Some("() -> Self"),
        Some(3),
        Some(5),
    )
    .await
    .unwrap();
    s.upsert_node(
        &fid,
        "method",
        "render",
        "src/widget.rs",
        Some(&file),
        Some("(&self)"),
        Some(7),
        Some(20),
    )
    .await
    .unwrap();

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1")
        .bind(fid)
        .fetch_one(s.pool())
        .await
        .unwrap();
    let (covered,): (i64,) = query_as(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(total, 4, "seeded 4 nodes");
    assert_eq!(
        covered, total,
        "every node carries a community_id (singletons inherit the file community)"
    );

    // per-folder integrity still holds with the broadened coverage.
    let (claimed,): (i64,) = query_as(
        "SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(claimed, covered, "claimed == real (invariant 5)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn community_adjacency_includes_extends() {
    // D4.4: the adjacency set is broadened to calls,imports,extends,references
    // (the dead `implements` is dropped). Two classes in DIFFERENT files
    // (so `parent_id` containment does NOT group them) linked only by an
    // `extends` edge land in the SAME community — before D4b, `extends` was
    // ignored and they would be separate singletons.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commext_{}", uuid::Uuid::new_v4())).await;
    let base = s
        .upsert_node(
            &fid,
            "class",
            "Base",
            "src/base.rs",
            None,
            Some("class Base"),
            Some(1),
            Some(5),
        )
        .await
        .unwrap();
    let derived = s
        .upsert_node(
            &fid,
            "class",
            "Derived",
            "src/derived.rs",
            None,
            Some("class Derived"),
            Some(1),
            Some(5),
        )
        .await
        .unwrap();
    s.insert_edge(&fid, &derived, Some(&base), None, None, "extends").await.unwrap();

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let cid = |id: uuid::Uuid| {
        let pool = s.pool().clone();
        async move {
            let (v,): (Option<i32>,) =
                query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                    .bind(id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            v
        }
    };
    let cb = cid(base).await;
    let cd = cid(derived).await;
    assert!(cb.is_some(), "extends-linked class carries a community");
    assert_eq!(cb, cd, "extends-linked classes share a community (broadened adjacency)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn recompute_degrees_counts_incident_edges() {
    // D4.5: nodes.degree = in+out count of edges incident to the node (source,
    // plus resolved target). An edgeless node is set to 0, not left NULL.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("degree_{}", uuid::Uuid::new_v4())).await;
    let hub = s
        .upsert_node(&fid, "function", "hub", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let a = s
        .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let b = s
        .upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(5), Some(6))
        .await
        .unwrap();
    let lonely = s
        .upsert_node(&fid, "function", "lonely", "a.rs", None, Some("()"), Some(7), Some(8))
        .await
        .unwrap();
    s.insert_edge(&fid, &a, Some(&hub), None, None, "calls").await.unwrap(); // a→hub
    s.insert_edge(&fid, &b, Some(&hub), None, None, "calls").await.unwrap(); // b→hub

    s.recompute_degrees_for_folder(&fid).await.unwrap();

    let deg = |id: uuid::Uuid| {
        let pool = s.pool().clone();
        async move {
            let (d,): (Option<i32>,) = query_as("SELECT degree FROM sensei.nodes WHERE id=$1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
            d
        }
    };
    assert_eq!(deg(hub).await, Some(2), "hub is the resolved target of 2 calls");
    assert_eq!(deg(a).await, Some(1), "a is the source of 1 call");
    assert_eq!(deg(b).await, Some(1), "b is the source of 1 call");
    assert_eq!(deg(lonely).await, Some(0), "an edgeless node has degree 0, not NULL");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn recompute_degrees_reruns_change_zero_rows() {
    // Regression (bloat incident): the degree barrier runs on EVERY indexing
    // pass (build_connections). Without the `IS DISTINCT FROM` guard it rewrote
    // every node in the folder each time, so a steady-state re-scan produced a
    // full table's worth of dead tuples. Concurrent same-folder passes then
    // blocked on each other's row locks, held hours-long transactions that
    // pinned the xmin horizon, and autovacuum could never reclaim them —
    // sensei.nodes reached 99% dead / 155 GB. A re-scan with no graph change
    // MUST touch 0 rows.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("degree_norerun_{}", uuid::Uuid::new_v4())).await;
    let hub = s
        .upsert_node(&fid, "function", "hub", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let a = s
        .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    s.insert_edge(&fid, &a, Some(&hub), None, None, "calls").await.unwrap();

    // First pass sets degree on the nodes whose degree changed (NULL → value).
    let first = s.recompute_degrees_for_folder(&fid).await.unwrap();
    assert_eq!(first, 2, "first pass sets degree on the 2 incident nodes");

    // Steady state: nothing changed → the guarded UPDATE must rewrite 0 rows.
    let second = s.recompute_degrees_for_folder(&fid).await.unwrap();
    assert_eq!(second, 0, "a re-scan with no graph change rewrites 0 nodes (no dead tuples)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn god_node_ids_are_top_by_degree() {
    // D4.5: a community's god_node_ids are its highest-degree members (top-5),
    // read from nodes.degree; the hub ranks first.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("godnode_{}", uuid::Uuid::new_v4())).await;
    let hub = s
        .upsert_node(&fid, "function", "hub", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let a = s
        .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let b = s
        .upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(5), Some(6))
        .await
        .unwrap();
    let c = s
        .upsert_node(&fid, "function", "c", "a.rs", None, Some("()"), Some(7), Some(8))
        .await
        .unwrap();
    // a→hub, b→hub, c→hub, a→b (calls). Degrees: hub=3, a=2, b=2, c=1 → one
    // community {hub,a,b,c}; hub is the clear hub.
    s.insert_edge(&fid, &a, Some(&hub), None, None, "calls").await.unwrap();
    s.insert_edge(&fid, &b, Some(&hub), None, None, "calls").await.unwrap();
    s.insert_edge(&fid, &c, Some(&hub), None, None, "calls").await.unwrap();
    s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();

    s.recompute_degrees_for_folder(&fid).await.unwrap();
    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let (god,): (Vec<uuid::Uuid>,) = query_as(
            "SELECT god_node_ids FROM inference.communities WHERE folder_id=$1 ORDER BY community_id LIMIT 1")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(god.first(), Some(&hub), "the highest-degree node is the first god node");
    assert!(god.contains(&hub), "hub is a god node");
    assert!(god.len() <= 5, "at most 5 god nodes per community");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn community_description_authoritative_write_is_honest_null() {
    // D4.5 never-fabricate: the authoritative detection write leaves every
    // community's description NULL with props.source='null' — honest-empty,
    // NEVER a static template. (Model prose is stamped later, off-barrier, by
    // enrich_community_descriptions.) The Done-gate keys on
    // props.source ∈ {'insight-copy','null'}.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commdesc_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b = s
        .upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let rows: Vec<(Option<String>, serde_json::Value)> =
        query_as("SELECT description, props FROM inference.communities WHERE folder_id=$1")
            .bind(fid)
            .fetch_all(s.pool())
            .await
            .unwrap();
    assert!(!rows.is_empty(), "at least one community was written");
    for (desc, props) in &rows {
        assert_eq!(*desc, None, "description is honest-NULL without a gateway");
        let source = props.get("source").and_then(|v| v.as_str());
        assert_eq!(source, Some("null"), "props.source records the honest-empty provenance");
        assert_ne!(source, Some("template"), "never a templated description (never-fabricate)");
    }

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn graph_nodes_returns_community_and_structural_edges() {
    // 7.1: get_nodes_scoped exposes community_id, and get_edges_scoped_kinds
    // returns the full layout set calls,imports,extends — NOT just calls, and
    // NOT overlay kinds like covers.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("gscope_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b = s
        .upsert_node(&fid, "class", "B", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE sensei.nodes SET community_id=5 WHERE id=$1")
        .bind(a)
        .execute(s.pool())
        .await
        .unwrap();
    s.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();
    s.insert_edge(&fid, &a, None, Some("lib"), None, "imports").await.unwrap();
    s.insert_edge(&fid, &b, Some(&a), None, None, "extends").await.unwrap();
    s.insert_edge(&fid, &a, Some(&b), None, None, "covers").await.unwrap(); // overlay — excluded

    let nodes = s.get_nodes_scoped(&[fid]).await.unwrap();
    let a_node = nodes.iter().find(|n| n["name"] == "a").unwrap();
    assert_eq!(a_node["community_id"].as_i64(), Some(5), "get_nodes_scoped exposes community_id");

    let edges = s.get_edges_scoped_kinds(&[fid], &["calls", "imports", "extends"]).await.unwrap();
    let kinds: std::collections::HashSet<&str> =
        edges.iter().filter_map(|e| e["kind"].as_str()).collect();
    assert_eq!(edges.len(), 3, "exactly the 3 layout edges (covers excluded)");
    assert!(
        kinds.contains("calls") && kinds.contains("imports") && kinds.contains("extends"),
        "all 3 layout kinds present: {kinds:?}"
    );
    assert!(!kinds.contains("covers"), "covers (overlay) is not a layout edge");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn communities_info_uses_live_membership() {
    // 7.3: list_communities_live_scoped counts from the real nodes.community_id
    // join, NOT the denormalized communities.node_count — so a stale count
    // doesn't drive the overview.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("livecomm_{}", uuid::Uuid::new_v4())).await;
    let n1 = s
        .upsert_node(&fid, "function", "n1", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let n2 = s
        .upsert_node(&fid, "function", "n2", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let n3 = s
        .upsert_node(&fid, "function", "n3", "a.rs", None, Some("()"), Some(5), Some(6))
        .await
        .unwrap();
    // Community 1 has 2 live members, community 2 has 1 — but seed a STALE count.
    s.upsert_community(&fid, 1, "c1", 99).await.unwrap(); // stale node_count = 99
    s.upsert_community(&fid, 2, "c2", 0).await.unwrap(); // stale node_count = 0
    sqlx_core::query::query("UPDATE sensei.nodes SET community_id=1 WHERE id = ANY($1)")
        .bind(vec![n1, n2])
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE sensei.nodes SET community_id=2 WHERE id=$1")
        .bind(n3)
        .execute(s.pool())
        .await
        .unwrap();

    let live = s.list_communities_live_scoped(&[fid]).await.unwrap();
    let count_of = |label: &str| {
        live.iter().find(|c| c["label"] == label).and_then(|c| c["node_count"].as_i64())
    };
    assert_eq!(count_of("c1"), Some(2), "c1 sized by 2 LIVE members, not the stale 99");
    assert_eq!(count_of("c2"), Some(1), "c2 sized by 1 LIVE member, not the stale 0");
    // Ordered by live count desc → c1 first.
    assert_eq!(live.first().and_then(|c| c["label"].as_str()), Some("c1"));

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn upsert_node_at_same_line_keeps_id_and_renulls_embedding_on_sig_change() {
    // D3: a re-upsert at the SAME identity (line_start is part of the key)
    // keeps the id — preserving community_id and degree. The embedding is
    // PRESERVED when the signature is unchanged, and RE-NULLED (re-embed) when
    // the signature changed — signature being the only embed input that can
    // change on a same-identity conflict. A DIFFERENT line is a new identity.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("nodeid_{}", uuid::Uuid::new_v4())).await;
    let id1 = s
        .upsert_node(
            &fid,
            "function",
            "foo",
            "a.rs",
            None,
            Some("fn foo(x: i32)"),
            Some(10),
            Some(20),
        )
        .await
        .unwrap();

    // Simulate a prior enrich + embed pass on this node.
    let zeros = format!("[{}]", vec!["0"; 384].join(","));
    sqlx_core::query::query("UPDATE sensei.nodes SET community_id = 7, degree = 3, embedding = $2::vector WHERE id = $1")
            .bind(id1).bind(&zeros).execute(s.pool()).await.unwrap();

    // Re-upsert SAME line, SAME signature, only line_end grew → id kept, all preserved.
    let id2 = s
        .upsert_node(
            &fid,
            "function",
            "foo",
            "a.rs",
            None,
            Some("fn foo(x: i32)"),
            Some(10),
            Some(25),
        )
        .await
        .unwrap();
    assert_eq!(id1, id2, "a re-upsert at the same identity keeps its id");
    let (community, degree, has_emb): (Option<i32>, Option<i32>, bool) = query_as(
        "SELECT community_id, degree, embedding IS NOT NULL FROM sensei.nodes WHERE id = $1",
    )
    .bind(id1)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        (community, degree, has_emb),
        (Some(7), Some(3), true),
        "community_id/degree/embedding preserved when signature is unchanged"
    );

    // Re-upsert SAME line, CHANGED signature → id kept, community kept, embedding RE-NULLED.
    let id3 = s
        .upsert_node(
            &fid,
            "function",
            "foo",
            "a.rs",
            None,
            Some("fn foo(x: i64)"),
            Some(10),
            Some(25),
        )
        .await
        .unwrap();
    assert_eq!(id1, id3, "same identity (line) keeps the id even when signature changes");
    let (community2, has_emb2): (Option<i32>, bool) =
        query_as("SELECT community_id, embedding IS NOT NULL FROM sensei.nodes WHERE id = $1")
            .bind(id1)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(community2, Some(7), "community_id is still preserved");
    assert!(!has_emb2, "embedding is re-nulled for re-embedding when the signature changed");

    // A DIFFERENT line is a new identity ⇒ a new node (a moved symbol churns).
    let id4 = s
        .upsert_node(
            &fid,
            "function",
            "foo",
            "a.rs",
            None,
            Some("fn foo(x: i32)"),
            Some(99),
            Some(105),
        )
        .await
        .unwrap();
    assert_ne!(
        id1, id4,
        "a different line_start is a new identity (moved symbol re-mints until D5c nesting)"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A node whose fqn SHAPE changes must be adopted, not rejected.
///
/// Reproduces the production failure: `MetricSparkline.svelte` was indexed
/// with the module fqn `typescript·@sensei/desktop·lib/components/
/// MetricSparkline`; a later parse yielded no top-level defs, so the fqn's
/// language segment changed. `ON CONFLICT (folder_id, fqn)` cannot see the old
/// row, so the statement fell through to a raw INSERT and hit
/// `nodes_unique_identity` — the same file/kind/name/parent/line. That made
/// process_file fail, which withheld scan_state, which made the reconcile
/// re-drive the folder every 5 minutes forever with the folder stuck `failed`.
#[tokio::test]
async fn upsert_node_by_fqn_adopts_row_when_only_the_fqn_changed() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("adopt_{}", uuid::Uuid::new_v4())).await;

    let file_path = "src/lib/components/MetricSparkline.svelte";
    let name = "lib/components/MetricSparkline";
    // NULL line_start / parent_id on purpose — exercises the NULLS NOT
    // DISTINCT semantics the identity index uses, which is exactly the shape
    // a module container node has.
    let def = || FqnDef {
        file_path,
        signature: None,
        line_start: None,
        line_end: None,
        is_exported: false,
        parent_id: None,
    };

    // Indexed once under the original fqn.
    let first = s
        .upsert_node_by_fqn(
            &fid,
            "typescript·@sensei/desktop·lib/components/MetricSparkline",
            "module",
            name,
            Some("svelte"),
            Some(def()),
        )
        .await
        .unwrap();

    // Same structural identity, DIFFERENT fqn — this used to be a hard error.
    let second = s
        .upsert_node_by_fqn(
            &fid,
            "svelte·@sensei/desktop·lib/components/MetricSparkline",
            "module",
            name,
            Some("svelte"),
            Some(def()),
        )
        .await
        .expect("an fqn change on an existing identity must adopt the row, not fail");

    assert_eq!(
        first, second,
        "the existing node is adopted, keeping its id (and so its edges/embedding)"
    );

    // Exactly one row, now carrying the new fqn.
    let (count, fqn): (i64, Option<String>) = query_as(
        "SELECT count(*) OVER (), fqn FROM sensei.nodes
             WHERE folder_id=$1 AND file_path=$2 AND kind='module'::sensei.node_kind",
    )
    .bind(fid)
    .bind(file_path)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(count, 1, "adoption must not leave a duplicate module node behind");
    assert_eq!(
        fqn.as_deref(),
        Some("svelte·@sensei/desktop·lib/components/MetricSparkline"),
        "the adopted row is re-pointed at the new fqn"
    );
}

#[tokio::test]
async fn upsert_node_by_fqn_merges_ref_and_def() {
    // FQN get-or-create (SCIP/LSIF moniker model): a REFERENCE creates an
    // unresolved stub (resolved=false, NULL file_path); a later DEFINITION
    // with the same (folder_id, fqn) returns the SAME id, flips resolved=true
    // and fills file/line/signature; a second reference shares the one node
    // and never downgrades the resolved definition. No "unresolved" state —
    // the node exists from its first mention.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("fqn_{}", uuid::Uuid::new_v4())).await;
    let fqn = "rust·senseid·widget·Widget·new";

    // 1. Reference-first → a stub.
    let stub = s.upsert_node_by_fqn(&fid, fqn, "method", "new", Some("rust"), None).await.unwrap();
    let (resolved, fp): (bool, Option<String>) =
        query_as("SELECT resolved, file_path FROM sensei.nodes WHERE id=$1")
            .bind(stub)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(!resolved, "a reference-first node is an unresolved stub");
    assert_eq!(fp, None, "a stub has no known file");

    // 2. The definition enriches the SAME node in place.
    let def = s
        .upsert_node_by_fqn(
            &fid,
            fqn,
            "method",
            "new",
            Some("rust"),
            Some(FqnDef {
                file_path: "src/widget.rs",
                signature: Some("fn new() -> Self"),
                line_start: Some(10),
                line_end: Some(12),
                is_exported: true,
                parent_id: None,
            }),
        )
        .await
        .unwrap();
    assert_eq!(stub, def, "the definition get-or-creates the SAME node as the reference");
    let (resolved2, fp2, sig, ls, exported): (bool, Option<String>, Option<String>, Option<i32>, bool) =
            query_as("SELECT resolved, file_path, signature, line_start, is_exported FROM sensei.nodes WHERE id=$1")
            .bind(def).fetch_one(s.pool()).await.unwrap();
    assert!(resolved2, "the node is resolved once its definition is seen");
    assert_eq!(fp2.as_deref(), Some("src/widget.rs"));
    assert_eq!(sig.as_deref(), Some("fn new() -> Self"));
    assert_eq!(ls, Some(10));
    assert!(exported, "the definition's is_exported is written");

    // 3. A second reference shares the one node and does NOT downgrade it.
    let ref2 = s.upsert_node_by_fqn(&fid, fqn, "method", "new", Some("rust"), None).await.unwrap();
    assert_eq!(ref2, def, "a later reference resolves to the same node");
    let (still_resolved, still_fp): (bool, Option<String>) =
        query_as("SELECT resolved, file_path FROM sensei.nodes WHERE id=$1")
            .bind(def)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(still_resolved, "a reference must not downgrade an already-resolved node");
    assert_eq!(
        still_fp.as_deref(),
        Some("src/widget.rs"),
        "a reference must not clear the definition's file"
    );

    // Exactly one node for this fqn.
    let (n,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND fqn=$2")
        .bind(fid)
        .bind(fqn)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(n, 1, "ref + def + ref = exactly one node");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn lib_node_by_fqn() {
    // An external reference get-or-creates a first-class `lib_symbol` node:
    // resolved=true (the external symbol IS its own definition — nothing to
    // enrich), NULL file_path (no local file), grouped by package in props.
    // Stable id across repeated references.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("lib_{}", uuid::Uuid::new_v4())).await;
    let fqn = "lib·serde_json·serde_json·from_str";

    let a = s.upsert_lib_node_by_fqn(&fid, fqn, "from_str", "serde_json").await.unwrap();
    let (kind, resolved, fp, pkg): (String, bool, Option<String>, Option<String>) = query_as(
        "SELECT kind::text, resolved, file_path, props->>'package' FROM sensei.nodes WHERE id=$1",
    )
    .bind(a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(kind, "lib_symbol");
    assert!(resolved, "a lib symbol is its own definition — resolved");
    assert_eq!(fp, None, "a lib symbol has no local file");
    assert_eq!(pkg.as_deref(), Some("serde_json"), "grouped by package in props");

    // A second reference to the same external fqn shares the one node.
    let b = s.upsert_lib_node_by_fqn(&fid, fqn, "from_str", "serde_json").await.unwrap();
    assert_eq!(a, b, "repeated external references share one lib node");
    let (n,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND kind='lib_symbol'::sensei.node_kind")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(n, 1);

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn graph_nodes_and_tree_expose_fqn_and_containers() {
    // Phase 7.2: the graph/nodes projection (get_nodes_scoped) carries `fqn` +
    // `resolved` so the Atlas can key symbols by moniker, and /tree (build_tree)
    // nests the Phase-5 type/module containers (file → type → method) via
    // parent_id. These are the two projections the retrieval endpoints delegate to.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("retr_{}", uuid::Uuid::new_v4())).await;

    // file → struct container → method(fqn), nested by parent_id (Phase 5 shape).
    let file_id = s
        .upsert_node(&fid, "file", "lib.rs", "src/lib.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    let type_id = s
        .upsert_node(
            &fid,
            "struct",
            "Widget",
            "src/lib.rs",
            Some(&file_id),
            Some("struct Widget"),
            Some(2),
            Some(6),
        )
        .await
        .unwrap();
    let method_fqn = "rust·pkg·lib·Widget·render";
    let method_id = s
        .upsert_node_by_fqn(
            &fid,
            method_fqn,
            "method",
            "render",
            Some("rust"),
            Some(super::FqnDef {
                file_path: "src/lib.rs",
                signature: Some("fn render(&self)"),
                line_start: Some(3),
                line_end: Some(5),
                is_exported: true,
                parent_id: Some(&type_id),
            }),
        )
        .await
        .unwrap();

    // ── graph/nodes exposes fqn + resolved ──
    let nodes = s.get_nodes_scoped(&[fid]).await.unwrap();
    let method = nodes.iter().find(|n| n["name"] == "render").expect("method node in projection");
    assert_eq!(crate::api::util::json_uuid(&method["id"]), Some(method_id), "same method node");
    assert_eq!(
        method["fqn"].as_str(),
        Some(method_fqn),
        "get_nodes_scoped projects the node's fqn"
    );
    assert_eq!(method["resolved"].as_bool(), Some(true), "and its resolved flag");
    assert_eq!(
        method["is_test"].as_bool(),
        Some(false),
        "and its is_test flag (default false here)"
    );

    // ── /tree nests the type container → method (Phase 5 parent_id) ──
    let folders = s.get_folders_scoped(&[fid]).await.unwrap();
    let tree = crate::api::handlers::codebase::build_tree_pub(&folders, &nodes);
    let files = tree["tree"][0]["nodes"].as_array().expect("folder root nodes");
    let file = files.iter().find(|n| n["name"] == "lib.rs").expect("file node under folder");
    let widget = file["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "Widget")
        .expect("type container nested under file");
    assert_eq!(widget["kind"], "struct", "the type container carries its kind");
    let render = widget["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "render")
        .expect("method nested under the type container");
    assert_eq!(render["kind"], "method", "the method nests under the type container (Phase 5)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn two_same_name_stubs_do_not_merge() {
    // Phase 3.0: with the partial identity index (`where file_path is not null`),
    // reference stubs (file_path NULL) are governed ONLY by nodes_unique_fqn.
    // Two references with the same simple name but DIFFERENT fqns must stay two
    // distinct nodes — the false-merge this rebuild exists to kill. (Under the
    // old non-partial identity constraint these would collide on
    // (folder, NULL, kind, name, NULL, NULL).)
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("stubmerge_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .upsert_node_by_fqn(&fid, "rust·pkg·a·A·foo", "method", "foo", Some("rust"), None)
        .await
        .unwrap();
    let b = s
        .upsert_node_by_fqn(&fid, "rust·pkg·b·B·foo", "method", "foo", Some("rust"), None)
        .await
        .unwrap();
    assert_ne!(a, b, "same simple name, different fqn → two distinct stub nodes");
    let (n,): (i64,) =
        query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND resolved=false")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(n, 2, "both stubs coexist under the fqn index");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn legacy_upsert_sets_language_from_extension() {
    // Every node written via the legacy upsert_node/_ex path (all non-Rust +
    // file/section/rationale nodes, for the whole FQN transition) must carry
    // `language` derived from its file extension — otherwise the same-language
    // bare-name fallback filter (plan 0.8) has nothing to match on. Compound
    // extensions resolve too (.svelte.ts → typescript).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("lang_{}", uuid::Uuid::new_v4())).await;
    let cases = [
        ("src/a.rs", "function", Some("fn f()"), "rust"),
        ("pkg/b.py", "function", Some("def g()"), "python"),
        ("app/c.svelte.ts", "function", None, "typescript"), // compound ext
        ("docs/e.md", "doc", None, "markdown"),
    ];
    for (path, kind, sig, want) in cases {
        let id = s.upsert_node(&fid, kind, "n", path, None, sig, Some(1), Some(2)).await.unwrap();
        let (lang,): (Option<String>,) = query_as("SELECT language FROM sensei.nodes WHERE id=$1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
        assert_eq!(lang.as_deref(), Some(want), "{path} → language {want}");
    }

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn node_locations_tolerates_stub_rows() {
    // file_path is now nullable (reference stubs + lib_symbol nodes have none).
    // node_locations decodes file_path as a required String, so a stub id among
    // the requested ids must NOT error the whole fetch — the stub (no location)
    // is simply omitted while the real node still resolves.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("nodeloc_{}", uuid::Uuid::new_v4())).await;
    let real = s
        .upsert_node(&fid, "function", "real", "a.rs", None, Some("fn real()"), Some(3), Some(9))
        .await
        .unwrap();
    let stub = s
        .upsert_node_by_fqn(&fid, "rust·pkg·m·Missing·gone", "method", "gone", Some("rust"), None)
        .await
        .unwrap();

    let locs = s.node_locations(&[real, stub]).await.unwrap();
    assert_eq!(locs.len(), 1, "the stub (NULL file_path) is omitted, not an error");
    assert_eq!(locs[0].0, real, "the real node still resolves");
    assert_eq!(locs[0].2, "a.rs", "with its file_path");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn prune_file_nodes_deletes_vanished_and_unresolves_inbound() {
    // D3: a symbol that vanished from the parse is pruned; an inbound
    // cross-file edge to it is UNRESOLVED (target_id→NULL, target_name kept),
    // not cascade-deleted (invariant 3). A kept node is untouched.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("prune_{}", uuid::Uuid::new_v4())).await;
    let keep = s
        .upsert_node(&fid, "function", "keep", "a.rs", None, Some("()"), Some(1), Some(5))
        .await
        .unwrap();
    let gone = s
        .upsert_node(&fid, "function", "gone", "a.rs", None, Some("()"), Some(6), Some(9))
        .await
        .unwrap();
    let caller = s
        .upsert_node(&fid, "function", "caller", "b.rs", None, Some("()"), Some(1), Some(3))
        .await
        .unwrap();
    // A resolved inbound edge b.rs::caller → a.rs::gone, carrying target_name.
    let e = s.insert_edge(&fid, &caller, None, Some("gone"), None, "calls").await.unwrap();
    s.resolve_edge(&e, &gone).await.unwrap();

    // Re-index of a.rs keeps only `keep`.
    let pruned = s.prune_file_nodes(&fid, "a.rs", &[keep]).await.unwrap();
    assert_eq!(pruned, 1, "the vanished `gone` node is pruned");

    let (keep_exists,): (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id=$1)")
        .bind(keep)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert!(keep_exists, "the surviving node is untouched");
    let (gone_exists,): (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id=$1)")
        .bind(gone)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert!(!gone_exists, "the vanished node is deleted");
    // The inbound edge survived, unresolved (target_id NULL, target_name kept).
    let (tid, tname): (Option<uuid::Uuid>, Option<String>) =
        query_as("SELECT target_id, target_name FROM sensei.edges WHERE id = $1")
            .bind(e)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(tid, None, "inbound edge to the pruned node is unresolved, not deleted");
    assert_eq!(tname.as_deref(), Some("gone"), "target_name is kept for re-resolution");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn delete_edges_from_sources_clears_a_files_out_edges() {
    // D3 per-file reconcile: a surviving symbol's stale out-edges are cleared
    // before re-inserting the current set (they don't cascade — the node
    // lives). Only edges FROM the given sources are removed.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("outedge_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(5))
        .await
        .unwrap();
    let b = s
        .upsert_node(&fid, "function", "b", "b.rs", None, Some("()"), Some(1), Some(5))
        .await
        .unwrap();
    s.insert_edge(&fid, &a, None, Some("x"), None, "calls").await.unwrap(); // a's out-edge
    s.insert_edge(&fid, &b, None, Some("y"), None, "calls").await.unwrap(); // b's out-edge (must survive)

    let n = s.delete_edges_from_sources(&fid, &[a]).await.unwrap();
    assert_eq!(n, 1, "only a's out-edge is deleted");
    assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 1, "b's out-edge survives");
    // Empty sources is a cheap no-op.
    assert_eq!(s.delete_edges_from_sources(&fid, &[]).await.unwrap(), 0);

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_edges_of_kind_swaps_the_full_set() {
    // D2: replace_edges_of_kind removes STALE edges of a kind and inserts the
    // current set atomically — the "replaced, not appended" guarantee that
    // makes a derived kind (covers) a pure function of the current tree.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("replkind_{}", uuid::Uuid::new_v4())).await;
    let doc = s.upsert_node(&fid, "doc", "d", "d.md", None, None, None, None).await.unwrap();
    let f1 = s.upsert_node(&fid, "file", "f1", "f1.rs", None, None, None, None).await.unwrap();
    let f2 = s.upsert_node(&fid, "file", "f2", "f2.rs", None, None, None, None).await.unwrap();

    // A STALE covers edge doc→f1 (as if f1 was the covered file last scan).
    s.insert_edge(&fid, &doc, Some(&f1), None, None, "covers").await.unwrap();
    assert_eq!(s.get_edges_by_kind(&fid, "covers").await.unwrap().len(), 1);

    // Replace the covers set with {doc→f2}: the stale doc→f1 must vanish.
    s.replace_edges_of_kind(
        &fid,
        "covers",
        &[EdgeSpec { source_id: doc, target_id: Some(f2), target_name: None, target_file: None }],
    )
    .await
    .unwrap();

    let (tid,): (Option<uuid::Uuid>,) = query_as(
        "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND kind='covers'::sensei.edge_kind",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        s.get_edges_by_kind(&fid, "covers").await.unwrap().len(),
        1,
        "exactly the current set — stale edge removed, not appended"
    );
    assert_eq!(tid, Some(f2), "the surviving covers edge is the new target");

    // Replacing with an EMPTY set clears the kind for the folder.
    s.replace_edges_of_kind(&fid, "covers", &[]).await.unwrap();
    assert!(
        s.get_edges_by_kind(&fid, "covers").await.unwrap().is_empty(),
        "an empty set clears every edge of the kind"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_edges_of_kind_handles_unresolved_edges() {
    // The unresolved branch (target_id=None) — the path the per-file reconcile
    // (D3) will use. Replaces by (target_name, target_file).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("replun_{}", uuid::Uuid::new_v4())).await;
    let a =
        s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
    s.insert_edge(&fid, &a, None, Some("old"), None, "calls").await.unwrap(); // stale unresolved a→"old"

    s.replace_edges_of_kind(
        &fid,
        "calls",
        &[EdgeSpec {
            source_id: a,
            target_id: None,
            target_name: Some("new".into()),
            target_file: Some("x.rs".into()),
        }],
    )
    .await
    .unwrap();

    let (name, file): (Option<String>, Option<String>) = query_as(
            "SELECT target_name, target_file FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(
        (name.as_deref(), file.as_deref()),
        (Some("new"), Some("x.rs")),
        "unresolved edge replaced by (target_name, target_file)"
    );
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        1,
        "stale unresolved edge removed"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_edges_of_kind_is_atomic_and_rolls_back_on_failure() {
    // The "one transaction" guarantee: if an insert in the batch fails (a bad
    // source_id → FK violation), the whole replace rolls back — the OLD set is
    // intact, never half-deleted (no zero-covers window).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("replatomic_{}", uuid::Uuid::new_v4())).await;
    let doc = s.upsert_node(&fid, "doc", "d", "d.md", None, None, None, None).await.unwrap();
    let f1 = s.upsert_node(&fid, "file", "f1", "f1.rs", None, None, None, None).await.unwrap();
    s.insert_edge(&fid, &doc, Some(&f1), None, None, "covers").await.unwrap();

    // A batch whose second edge has a bogus source_id (no such node) → the
    // FK on edges.source_id fails the insert mid-batch.
    let bogus = uuid::Uuid::new_v4();
    let res = s
        .replace_edges_of_kind(
            &fid,
            "covers",
            &[
                EdgeSpec {
                    source_id: doc,
                    target_id: Some(f1),
                    target_name: None,
                    target_file: None,
                },
                EdgeSpec {
                    source_id: bogus,
                    target_id: Some(f1),
                    target_name: None,
                    target_file: None,
                },
            ],
        )
        .await;
    assert!(res.is_err(), "a bad edge fails the replace");

    assert_eq!(
        s.get_edges_by_kind(&fid, "covers").await.unwrap().len(),
        1,
        "the DELETE rolled back with the failed insert — old set intact, not half-deleted"
    );
    let (tid,): (Option<uuid::Uuid>,) = query_as(
        "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND kind='covers'::sensei.edge_kind",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(tid, Some(f1), "the surviving edge is the original (rollback)");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn insert_edge_unresolved_dedups_by_target_file() {
    // D1: the unresolved identity is (folder, source, target_name, target_file,
    // kind). Same target_name in DIFFERENT files are distinct edges; same
    // name + same file (incl. nulls-not-distinct) upserts to one row. This is
    // the whole point of the target_file column — a same-named symbol in two
    // files must not collapse to one edge.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("edgetf_{}", uuid::Uuid::new_v4())).await;
    let a =
        s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();

    let e1 = s.insert_edge(&fid, &a, None, Some("helper"), Some("x.rs"), "calls").await.unwrap();
    let e2 = s.insert_edge(&fid, &a, None, Some("helper"), Some("y.rs"), "calls").await.unwrap();
    assert_ne!(e1, e2, "same target_name in different files are distinct unresolved edges");
    assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 2, "two distinct rows");

    let e3 = s.insert_edge(&fid, &a, None, Some("helper"), Some("x.rs"), "calls").await.unwrap();
    assert_eq!(e1, e3, "same (target_name, target_file) upserts to the same row");
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        2,
        "no new row on re-insert"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn resolve_edge_second_call_is_safe() {
    // resolve_edge is idempotent: resolving the same edge to the same target
    // twice must be a safe no-op (one edge), not a unique-violation throw.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("resolve2x_{}", uuid::Uuid::new_v4())).await;
    let a =
        s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
    let b =
        s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();
    let u = s.insert_edge(&fid, &a, None, Some("b"), None, "calls").await.unwrap();

    s.resolve_edge(&u, &b).await.unwrap();
    s.resolve_edge(&u, &b).await.unwrap(); // second call — must not throw

    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        1,
        "resolving twice keeps exactly one edge"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn resolve_edge_updates_in_place_when_no_conflict() {
    // The common case: no existing resolved dup → the unresolved edge is
    // updated in place to the resolved target (not deleted).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("resolveok_{}", uuid::Uuid::new_v4())).await;
    let a =
        s.upsert_node(&fid, "function", "a", "a.rs", None, None, Some(1), Some(5)).await.unwrap();
    let b =
        s.upsert_node(&fid, "function", "b", "b.rs", None, None, Some(1), Some(5)).await.unwrap();
    let u = s.insert_edge(&fid, &a, None, Some("b"), None, "calls").await.unwrap();

    s.resolve_edge(&u, &b).await.unwrap();

    let (tid,): (Option<uuid::Uuid>,) = query_as("SELECT target_id FROM sensei.edges WHERE id=$1")
        .bind(u)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(tid, Some(b), "the edge is resolved in place to the target");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

// ── Extensions tests ───────────────────────────────────────────────

#[tokio::test]
async fn extension_create_and_list() {
    let s = pg_store().await;
    let name = format!("_test:ext_{}", uuid::Uuid::new_v4());
    let id = s
        .create_extension("skill", &name, Some("test skill"), Some("# content"), "global", "local")
        .await
        .unwrap();
    let skills = s.list_extensions_by_kind("skill").await.unwrap();
    assert!(skills.iter().any(|e| e["name"] == name));
    s.delete_extension(&id).await.unwrap();
}

#[tokio::test]
async fn extension_historize_trigger() {
    let s = pg_store().await;
    let name = format!("_test:ext_hist_{}", uuid::Uuid::new_v4());
    let id = s.create_extension("skill", &name, Some("v1"), None, "global", "local").await.unwrap();
    s.update_extension(&id, Some("v2"), None).await.unwrap();
    let history = s.get_extension_history(&id).await.unwrap();
    assert!(history.len() >= 2, "historize trigger should create INSERT + UPDATE entries");
    s.delete_extension(&id).await.unwrap();
}

// ── Folders tests ────────────────────────────────────────────────

#[tokio::test]
async fn folder_upsert_and_list() {
    let s = pg_store().await;
    let path = format!("/_test/folder_root_{}", uuid::Uuid::new_v4());
    let rid = s.add_watch_root(&path, "test_root", &serde_json::json!([])).await.unwrap();
    let fid = s
        .upsert_folder(&rid, "git", "myrepo", "myrepo", &format!("{}/myrepo", path), None, None)
        .await
        .unwrap();
    let folders = s.list_folders_by_root(&rid).await.unwrap();
    assert!(folders.iter().any(|f| f["name"] == "myrepo"));
    s.delete_folder_tree(&fid).await.unwrap();
    s.remove_watch_root(&rid).await.unwrap();
}

#[tokio::test]
async fn list_pending_folders_returns_only_non_terminal_status() {
    let s = pg_store().await;
    let root_path = format!("/_test/pending_resume_{}", uuid::Uuid::new_v4().simple());
    let rid = s.add_watch_root(&root_path, "pending_root", &serde_json::json!([])).await.unwrap();

    // Seed one folder per status. Default is 'discovered'; the rest are
    // forced with an explicit UPDATE because upsert_folder has no status
    // parameter and `mark_folder_indexed` is the only writer of `indexed`.
    for (status, suffix) in [
        ("discovered", "a"),
        ("queued", "b"),
        ("indexing", "c"),
        ("indexed", "d"),
        ("failed", "e"),
        ("deferred", "f"),
        ("archived", "g"),
    ] {
        let name = format!("repo_{}", suffix);
        let abs_path = format!("{}/{}", root_path, name);
        let fid = s.upsert_folder(&rid, "git", &name, &name, &abs_path, None, None).await.unwrap();
        s.update_folder_status(&fid, status).await.unwrap();
    }

    let rows = s.list_pending_folders().await.unwrap();
    let ours: Vec<_> = rows
        .iter()
        .filter(|r| r["abs_path"].as_str().unwrap_or("").starts_with(&root_path))
        .collect();

    // Recoverable = non-terminal. `discovered`/`queued` never started;
    // `indexing`/`failed` are a scan interrupted mid-flight or errored —
    // its in-memory task was lost on restart (D6a marks `indexing` at scan
    // start), so resume MUST re-enqueue them. `indexed`/`deferred`/`archived`
    // are terminal and never resumed.
    let statuses: std::collections::BTreeSet<&str> =
        ours.iter().map(|r| r["status"].as_str().unwrap()).collect();
    assert_eq!(
        statuses,
        std::collections::BTreeSet::from(["discovered", "queued", "indexing", "failed"]),
        "expected discovered+queued+indexing+failed, got {:?}",
        statuses
    );

    // Resume needs enough info to enqueue ProcessGitFolder: id, kind, abs_path.
    for r in &ours {
        assert!(r["id"].is_string(), "row missing id: {:?}", r);
        assert!(r["kind"].is_string(), "row missing kind: {:?}", r);
        assert!(r["abs_path"].is_string(), "row missing abs_path: {:?}", r);
    }

    // cleanup — removing the watch root cascades to folders.
    s.remove_watch_root(&rid).await.unwrap();
}

#[tokio::test]
async fn update_folder_status_round_trips() {
    // D6a: the folder-status lifecycle needs a production setter — before
    // this, only `indexed` (mark_folder_indexed) and `archived` were
    // writable, so a scan could never record that it had started.
    let s = pg_store().await;
    let root_path = format!("/_test/status_{}", uuid::Uuid::new_v4().simple());
    let rid = s.add_watch_root(&root_path, "status_root", &serde_json::json!([])).await.unwrap();
    let fid = s
        .upsert_folder(&rid, "git", "r", "r", &format!("{root_path}/r"), None, None)
        .await
        .unwrap();

    s.update_folder_status(&fid, "indexing").await.unwrap();

    let (status,): (String,) =
        sqlx_core::query_as::query_as("SELECT status::text FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(status, "indexing", "update_folder_status writes the enum value");

    s.remove_watch_root(&rid).await.unwrap();
}

#[tokio::test]
async fn get_folder_status_reads_back_status_and_is_none_for_missing() {
    // D6d: the fail-closed barrier reads folder status to decide whether to
    // mark `indexed`. A missing folder must be honest-`None`, never an error
    // or a fabricated status.
    let s = pg_store().await;
    let root_path = format!("/_test/getstatus_{}", uuid::Uuid::new_v4().simple());
    let rid = s.add_watch_root(&root_path, "getstatus_root", &serde_json::json!([])).await.unwrap();
    let fid = s
        .upsert_folder(&rid, "git", "r", "r", &format!("{root_path}/r"), None, None)
        .await
        .unwrap();

    s.update_folder_status(&fid, "failed").await.unwrap();
    assert_eq!(
        s.get_folder_status(&fid).await.unwrap().as_deref(),
        Some("failed"),
        "reads back the written status"
    );
    assert_eq!(
        s.get_folder_status(&uuid::Uuid::new_v4()).await.unwrap(),
        None,
        "a missing folder is None, not an error"
    );

    s.remove_watch_root(&rid).await.unwrap();
}

// ── Benchmark Reports tests ──────────────────────────────────────

#[tokio::test]
async fn benchmark_create_and_list() {
    let s = pg_store().await;
    let id = s
        .create_benchmark_report(
            None,
            "_test:bench",
            "strategy_a",
            Some(95.5),
            Some(1000),
            Some(5000),
        )
        .await
        .unwrap();
    let reports = s.list_benchmark_reports().await.unwrap();
    assert!(reports.iter().any(|r| r["run_name"] == "_test:bench"));
    sqlx_core::query::query("DELETE FROM sensei.benchmark_reports WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Views tests ──────────────────────────────────────────────────

#[tokio::test]
async fn repositories_view() {
    let s = pg_store().await;
    // list_repositories returns git+subtree folders
    let repos = s.list_repositories().await.unwrap();
    // Just verify it doesn't error — content depends on seeded data
    // Just verify the query succeeds — content depends on seeded data
    let _ = repos;
}

// ── Memories tests ─────────────────────────────────────────────────

#[tokio::test]
async fn memory_create_and_get() {
    let s = pg_store().await;
    let id = s
        .create_memory(
            None,
            "global",
            None,
            "decision",
            "_test:mem_create",
            "Always use TDD",
            Some("Bugs ship to prod"),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let m = s.get_memory(&id).await.unwrap().unwrap();
    assert_eq!(m["title"], "_test:mem_create");
    assert_eq!(m["scope"], "global");
    assert_eq!(m["strength"], 1.0);
    assert_eq!(m["status"], "active");
    // cleanup via historize trigger test
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn create_memory_persists_spine_slot_and_feature() {
    let s = pg_store().await;
    let pid = s
        .create_project(&format!("_test:mem_slot-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let id = s
        .create_memory(
            Some(&pid),
            "project",
            None,
            "decision",
            "t",
            "c",
            None,
            None,
            Some("decisions"),
            Some("auth"),
        )
        .await
        .unwrap();
    let row: (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT spine_slot::text, feature FROM sensei.memories WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row, (Some("decisions".into()), Some("auth".into())));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn memory_reinforce() {
    let s = pg_store().await;
    let id = s
        .create_memory(
            None,
            "global",
            None,
            "pattern",
            "_test:mem_reinforce",
            "rule",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    s.reinforce_memory(&id, 1.0).await.unwrap();
    s.reinforce_memory(&id, 1.0).await.unwrap();
    let m = s.get_memory(&id).await.unwrap().unwrap();
    assert_eq!(m["strength"], 3.0); // 1.0 + 1.0 + 1.0
    // Cap at 5.0
    s.reinforce_memory(&id, 10.0).await.unwrap();
    let m = s.get_memory(&id).await.unwrap().unwrap();
    assert_eq!(m["strength"], 5.0);
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn memory_archive() {
    let s = pg_store().await;
    let id = s
        .create_memory(
            None,
            "global",
            None,
            "question",
            "_test:mem_archive",
            "open q",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    s.archive_memory(&id).await.unwrap();
    let m = s.get_memory(&id).await.unwrap().unwrap();
    assert_eq!(m["status"], "archived");
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn memory_list_active() {
    let s = pg_store().await;
    let id1 = s
        .create_memory(
            None,
            "global",
            None,
            "decision",
            "_test:mem_list_a",
            "rule a",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let id2 = s
        .create_memory(
            None,
            "global",
            None,
            "decision",
            "_test:mem_list_b",
            "rule b",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let active = s.list_active_memories(None, Some("global")).await.unwrap();
    assert!(active.iter().any(|m| m["title"] == "_test:mem_list_a"));
    assert!(active.iter().any(|m| m["title"] == "_test:mem_list_b"));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
        .bind(&[id1, id2][..])
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Memory Examples tests ────────────────────────────────────────

#[tokio::test]
async fn memory_example_add_and_list() {
    let s = pg_store().await;
    let mid = s
        .create_memory(
            None,
            "global",
            None,
            "pattern",
            "_test:mem_ex",
            "rule",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    s.add_memory_example(&mid, "fn:auth_handler", true, Some("canonical auth")).await.unwrap();
    s.add_memory_example(&mid, "fn:inline_auth", false, Some("avoid inline")).await.unwrap();
    let examples = s.list_memory_examples(&mid).await.unwrap();
    assert_eq!(examples.len(), 2);
    assert!(examples.iter().any(|e| e["is_good"] == true));
    assert!(examples.iter().any(|e| e["is_good"] == false));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Memory Evidence tests ────────────────────────────────────────

#[tokio::test]
async fn memory_evidence_add_and_list() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("mem_ev_{}", uuid::Uuid::new_v4())).await;
    let sid = s.create_session(&fid, "test", None).await.unwrap();
    let mid = s
        .create_memory(
            None,
            "global",
            None,
            "decision",
            "_test:mem_ev",
            "rule",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    s.add_memory_evidence(&mid, Some(&sid), Some("user corrected twice")).await.unwrap();
    // A save-time source note carries no session_id (nullable).
    s.add_memory_evidence(&mid, None, Some("crates/x.rs:42")).await.unwrap();
    let evidence = s.list_memory_evidence(&mid).await.unwrap();
    assert_eq!(evidence.len(), 2);
    assert!(
        evidence.iter().any(|e| e["session_id"].is_null() && e["note"] == "crates/x.rs:42"),
        "the session-less source note round-trips with a null session_id"
    );
    assert_eq!(evidence[0]["note"], "user corrected twice");
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Memory Links tests ───────────────────────────────────────────

#[tokio::test]
async fn memory_links_parent_child() {
    let s = pg_store().await;
    let parent = s
        .create_memory(
            None,
            "global",
            None,
            "decision",
            "_test:mem_parent",
            "combined",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let child1 = s
        .create_memory(
            None,
            "global",
            None,
            "decision",
            "_test:mem_child1",
            "original 1",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let child2 = s
        .create_memory(
            None,
            "global",
            None,
            "decision",
            "_test:mem_child2",
            "original 2",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    s.link_memories(&parent, &child1).await.unwrap();
    s.link_memories(&parent, &child2).await.unwrap();
    let children = s.get_memory_children(&parent).await.unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(s.get_memory_parent(&child1).await.unwrap(), Some(parent));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
        .bind(&[parent, child1, child2][..])
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Recommendations tests ────────────────────────────────────────

#[tokio::test]
async fn recommendation_lifecycle() {
    let s = pg_store().await;
    let pid = s.create_project("_test:rec_proj", None, None).await.unwrap();
    let rid = s
        .create_recommendation(&pid, "_test:rec", "reduces corrections", "promote_pattern", "high")
        .await
        .unwrap();
    s.accept_recommendation(&rid).await.unwrap();
    s.measure_recommendation(&rid, "positive").await.unwrap();
    let recs = s.list_recommendations(&pid).await.unwrap();
    let r = recs.iter().find(|r| r["title"] == "_test:rec").unwrap();
    assert_eq!(r["status"], "accepted");
    assert_eq!(r["verdict"], "positive");
    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_project(&pid).await.unwrap();
}

// Gap 1 fix — the reject terminal is `dismissed` in the enum (not
// `rejected`), and both accept + reject must only fire from `pending`.
// Locks the enum-value contract so a future rename can't silently
// break the UI action buttons.
#[tokio::test]
async fn recommendation_reject_writes_dismissed_and_guards_at_pending() {
    let s = pg_store().await;
    let pid = s.create_project("_test:rec_reject_proj", None, None).await.unwrap();
    let rid =
        s.create_recommendation(&pid, "_test:rej", "why", "revise_rule", "low").await.unwrap();

    s.reject_recommendation(&rid).await.unwrap();
    let recs = s.list_recommendations(&pid).await.unwrap();
    let r = recs.iter().find(|r| r["title"] == "_test:rej").unwrap();
    assert_eq!(r["status"], "dismissed", "reject writes the `dismissed` enum terminal");

    // Second reject on the same rec is a no-op guarded at `pending`;
    // pg_store must return an error rather than clobber the decision.
    let err = s.reject_recommendation(&rid).await.expect_err("guard fires on already-decided");
    assert!(
        err.contains("already decided") || err.contains("not found"),
        "guard error text: {err}"
    );

    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_project(&pid).await.unwrap();
}

// ── P-A: accept → materialize a governance RULE (spec 2026-08-20) ──
#[tokio::test]
async fn accept_as_rule_materializes_a_memory_and_records_provenance() {
    let s = pg_store().await;
    let pid =
        s.create_project(&format!("_test:mat-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
    // A revise_rule rec — the canonical rule-class case ("why are you using regex").
    let rid = s
        .create_recommendation(
            &pid,
            "No bare regex in the parser",
            "regex is fragile here; use the typed lexer",
            "revise_rule",
            "high",
        )
        .await
        .unwrap();

    // Accept + materialize at gov_scope=general (namespace None) enforcement=required.
    let mat = s
        .accept_recommendation_as_rule(&rid, None, Some("required"), "general", None, None)
        .await
        .unwrap();
    assert_eq!(mat["kind"], "rule");
    assert_eq!(mat["scope"], "general");
    assert_eq!(mat["enforcement"], "required");
    let mem_id = uuid::Uuid::parse_str(mat["memory_id"].as_str().unwrap()).unwrap();

    // The memory is LIVE (active) with the rec's content + chosen enforcement + origin.
    let (title, content, status, enforcement, mtype, origin): (String, String, String, String, String, String) =
            sqlx_core::query_as::query_as(
                "SELECT title, content, status::text, enforcement::text, type::text, origin::text FROM sensei.memories WHERE id=$1"
            ).bind(mem_id).fetch_one(s.pool()).await.unwrap();
    assert_eq!(title, "No bare regex in the parser");
    assert_eq!(
        content, "regex is fragile here; use the typed lexer",
        "body defaults to the rec's why"
    );
    assert_eq!(status, "active", "an accepted rule is live immediately");
    assert_eq!(enforcement, "required");
    assert_eq!(mtype, "convention");
    assert_eq!(origin, "authored");

    // The rec is accepted + carries the materialized_ref provenance.
    let (rstatus, ref_kind, ref_mem): (String, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT status::text, materialized_ref->>'kind', materialized_ref->>'memory_id' FROM inference.recommendations WHERE id=$1"
        ).bind(rid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(rstatus, "accepted");
    assert_eq!(ref_kind.as_deref(), Some("rule"));
    assert_eq!(
        ref_mem.as_deref(),
        Some(mem_id.to_string().as_str()),
        "provenance links the exact memory"
    );

    // Idempotent guard: a second materialize on the now-accepted rec errors (no double write).
    let err = s
        .accept_recommendation_as_rule(&rid, None, None, "general", None, None)
        .await
        .expect_err("guarded at pending");
    assert!(err.contains("already decided") || err.contains("not found"), "guard: {err}");

    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id=$1")
        .bind(mem_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id=$1")
        .bind(rid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_project(&pid).await.unwrap();
}

#[tokio::test]
async fn accept_as_rule_rejects_non_rule_action_and_title_body_override() {
    let s = pg_store().await;
    let pid = s
        .create_project(&format!("_test:mat2-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();

    // Non-rule action (audit_stale) → error, no memory written (mis-route surfaced).
    let audit = s
        .create_recommendation(&pid, "_test:audit", "high rework file", "audit_stale", "low")
        .await
        .unwrap();
    let err = s
        .accept_recommendation_as_rule(&audit, None, None, "general", None, None)
        .await
        .expect_err("non-rule rejected");
    assert!(err.contains("not rule-class"), "got: {err}");

    // enrich_memory with title + body overrides (the review-before-apply edit path).
    let rid = s
        .create_recommendation(&pid, "orig title", "orig why", "enrich_memory", "medium")
        .await
        .unwrap();
    let mat = s
        .accept_recommendation_as_rule(
            &rid,
            None,
            Some("advisory"),
            "general",
            Some("edited title"),
            Some("edited body"),
        )
        .await
        .unwrap();
    let mem_id = uuid::Uuid::parse_str(mat["memory_id"].as_str().unwrap()).unwrap();
    let (title, content): (String, String) =
        sqlx_core::query_as::query_as("SELECT title, content FROM sensei.memories WHERE id=$1")
            .bind(mem_id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(title, "edited title", "title override wins");
    assert_eq!(content, "edited body", "body override wins");

    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id=$1")
        .bind(mem_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE project_id=$1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_project(&pid).await.unwrap();
}

#[tokio::test]
async fn begin_file_materialization_flips_and_returns_prompt_seed() {
    // P-B store contract: the guarded flip returns the (action_type, title, why,
    // prompt) seed a file materializer renders, and set_recommendation_materialized
    // records the file provenance. Second flip is guarded (no double write).
    let s = pg_store().await;
    let pid = s
        .create_project(&format!("_test:pbmat-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let rid = s.create_recommendation_full(
            &pid, "Establish DBD Guardian Agent", "cross-layer churn needs a review agent", None,
            "create_agent", "high", &serde_json::json!({}), None,
            Some("You are an Architectural Review Agent for dbd. Before any code is accepted, check module boundaries."),
        ).await.unwrap();

    let (action_type, title, why, prompt) =
        s.begin_file_materialization(&rid).await.unwrap().expect("pending → seed");
    assert_eq!(action_type, "create_agent");
    assert_eq!(title, "Establish DBD Guardian Agent");
    assert_eq!(why, "cross-layer churn needs a review agent");
    assert!(
        prompt.as_deref().unwrap().starts_with("You are an Architectural Review Agent"),
        "prompt seed returned"
    );

    // Record the file provenance (what the handler does after writing the file).
    let mref = serde_json::json!({ "kind": "agent", "file_path": ".claude/agents/establish-dbd-guardian-agent.md" });
    s.set_recommendation_materialized(&rid, &mref).await.unwrap();
    let (status, kind, fp): (String, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT status::text, materialized_ref->>'kind', materialized_ref->>'file_path' FROM inference.recommendations WHERE id=$1"
        ).bind(rid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(status, "accepted");
    assert_eq!(kind.as_deref(), Some("agent"));
    assert_eq!(fp.as_deref(), Some(".claude/agents/establish-dbd-guardian-agent.md"));

    // Guarded: a second flip on the accepted rec returns None (no re-materialize).
    assert!(s.begin_file_materialization(&rid).await.unwrap().is_none(), "guarded at pending");

    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id=$1")
        .bind(rid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_project(&pid).await.unwrap();
}

// ── Accept-driven pattern promotion ──────────────────────────────
// Accepting a `promote_pattern` rec advances its source pattern's
// lifecycle to `rule` (the read path renders it `adopted`). The action
// is store-owned so it stays single-call-site + unit-testable.

/// Seed a (project, folder, pattern, promote_pattern rec) fixture. The rec's
/// `based_on.patterns[0]` cites the pattern (unless `cite_pattern` is false,
/// exercising the defensive no-op path). Returns (proj, folder, pattern, rec).
async fn seed_promote_fixture(
    s: &PgStore,
    suffix: &str,
    action_type: &str,
    cite_pattern: bool,
) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let (proj_id, fid) = create_test_project_and_folder(s, suffix).await;
    let pat_id = s
        .upsert_pattern(
            &proj_id,
            Some(&fid),
            "_test:rule-candidates",
            false,
            None,
            &serde_json::json!([]),
        )
        .await
        .unwrap();
    // suggested is the seeded lifecycle default; assert the precondition so a
    // schema default change can't make the promotion test vacuously pass.
    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let seeded = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(seeded["lifecycle"], "suggested", "pattern starts at suggested");

    let based_on = if cite_pattern {
        serde_json::json!({ "patterns": [pat_id] })
    } else {
        serde_json::json!({ "patterns": [] })
    };
    let rid = s
        .create_recommendation_full(
            &proj_id,
            "_test:promote",
            "why",
            None,
            action_type,
            "medium",
            &based_on,
            None,
            None,
        )
        .await
        .unwrap();
    (proj_id, fid, pat_id, rid)
}

async fn cleanup_promote_fixture(
    s: &PgStore,
    proj_id: &uuid::Uuid,
    pat_id: &uuid::Uuid,
    rid: &uuid::Uuid,
) {
    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(pat_id)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_project(proj_id).await.unwrap();
}

/// Pure extractor: only a well-formed `patterns[0]` uuid comes back; a
/// missing key, empty array, or non-uuid is `None` (the no-op signal).
#[test]
fn based_on_first_pattern_parses_uuid_and_defends() {
    let id = uuid::Uuid::new_v4();
    let good = serde_json::json!({ "patterns": [id] }).to_string();
    assert_eq!(PgStore::based_on_first_pattern(&good), Some(id));
    assert_eq!(PgStore::based_on_first_pattern("{}"), None);
    assert_eq!(PgStore::based_on_first_pattern(r#"{"patterns":[]}"#), None);
    assert_eq!(PgStore::based_on_first_pattern(r#"{"patterns":["not-a-uuid"]}"#), None);
    assert_eq!(PgStore::based_on_first_pattern("not json"), None);
}

/// (1) Accepting a promote_pattern rec advances the cited pattern to `rule`.
#[tokio::test]
async fn accept_promote_pattern_advances_lifecycle_to_rule() {
    let s = pg_store().await;
    let suffix = format!("accept_promote_{}", uuid::Uuid::new_v4());
    let (proj_id, fid, pat_id, rid) =
        seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

    s.accept_recommendation(&rid).await.unwrap();

    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(
        p["lifecycle"], "rule",
        "accepting a promote_pattern rec advances the pattern to rule"
    );

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

/// (2) A non-promote action (e.g. write_skill) leaves the pattern untouched.
#[tokio::test]
async fn accept_non_promote_leaves_pattern_untouched() {
    let s = pg_store().await;
    let suffix = format!("accept_writeskill_{}", uuid::Uuid::new_v4());
    let (proj_id, fid, pat_id, rid) = seed_promote_fixture(&s, &suffix, "write_skill", true).await;

    s.accept_recommendation(&rid).await.unwrap();

    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(p["lifecycle"], "suggested", "a non-promote action must not advance the pattern");

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

/// (3) A promote_pattern rec with no cited pattern accepts as a no-op —
/// returns Ok, the pattern is unchanged, and nothing panics.
#[tokio::test]
async fn accept_promote_pattern_without_provenance_is_noop() {
    let s = pg_store().await;
    let suffix = format!("accept_noprov_{}", uuid::Uuid::new_v4());
    let (proj_id, fid, pat_id, rid) =
        seed_promote_fixture(&s, &suffix, "promote_pattern", false).await;

    s.accept_recommendation(&rid).await.expect("empty provenance accepts without error");

    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(p["lifecycle"], "suggested", "no cited pattern → nothing to promote");

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

/// (4) The pending-guard holds: a second accept errors, and the promotion
/// fired exactly once (the pattern is still `rule`, not re-touched into error).
#[tokio::test]
async fn accept_promote_pattern_guard_fires_promotion_once() {
    let s = pg_store().await;
    let suffix = format!("accept_guard_{}", uuid::Uuid::new_v4());
    let (proj_id, fid, pat_id, rid) =
        seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

    s.accept_recommendation(&rid).await.unwrap();
    let err = s.accept_recommendation(&rid).await.expect_err("second accept is guarded at pending");
    assert!(
        err.contains("already decided") || err.contains("not found"),
        "guard error text: {err}"
    );

    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(p["lifecycle"], "rule", "promotion fired once; the guarded re-accept is inert");

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

/// (5) Read-path: after accept, get_project_patterns surfaces the pattern
/// with kind='adopted' (pattern_kind maps lifecycle='rule' → adopted).
#[tokio::test]
async fn accept_promote_pattern_reads_back_as_adopted() {
    let s = pg_store().await;
    let suffix = format!("accept_adopted_{}", uuid::Uuid::new_v4());
    let (proj_id, _fid, pat_id, rid) =
        seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

    s.accept_recommendation(&rid).await.unwrap();

    let view = s.get_project_patterns(&proj_id).await.unwrap();
    let followed = view["followed"].as_array().expect("followed array");
    let p = followed
        .iter()
        .find(|p| p["id"] == pat_id.to_string())
        .expect("promoted pattern in followed set");
    assert_eq!(p["kind"], "adopted", "lifecycle='rule' reads back as adopted");
    assert_eq!(p["lifecycle"], "rule");

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

// ── Verdict regression → challenge the source memory ────────────────
// When an accepted rec's FTR REGRESSES after acceptance, the memory that
// spawned it (via based_on.patterns[0] → memories.source_id) is challenged
// (weakened) through the existing memory_outcome pipeline.

/// Seed a (project, folder, pattern, learned memory sourced by the pattern)
/// fixture. The memory starts at 1.0 + `strength_bump` so a single violation
/// (−0.7) challenges rather than archives it. Returns (proj, folder, pat, mem).
async fn seed_pattern_and_sourced_memory(
    s: &PgStore,
    suffix: &str,
    strength_bump: f64,
) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let (proj_id, fid) = create_test_project_and_folder(s, suffix).await;
    let pat_id = s
        .upsert_pattern(
            &proj_id,
            Some(&fid),
            "_test:regressed-rule",
            false,
            None,
            &serde_json::json!([]),
        )
        .await
        .unwrap();
    // Convention memory sourced by the pattern — mirrors the rule-candidates generator.
    let mem = InsertMemory {
        project_id: Some(proj_id),
        scope: "project".to_string(),
        scope_filter: None,
        mtype: "convention".to_string(),
        title: format!("_test:regressed-memory:{suffix}"),
        content: "always foo".to_string(),
        impact: None,
        tags: Vec::new(),
        triage_signal: None,
        status: "active".to_string(),
        namespace_id: None,
        enforcement: None,
        origin: Some("learned".to_string()),
        source_id: Some(pat_id),
        spine_slot: None,
        feature: None,
    };
    let mem_id = s.insert_memory(&mem).await.unwrap();
    if strength_bump > 0.0 {
        s.reinforce_memory(&mem_id, strength_bump).await.unwrap();
    }
    (proj_id, fid, pat_id, mem_id)
}

/// Extend the memory fixture with an accepted+regressed promote_pattern rec:
/// acted 4 days ago at baseline FTR 0.9, then ≥3 post-acceptance sessions that
/// all fail (ftr=false) so the measured current FTR is 0.0 → a negative verdict.
async fn seed_regressed_rec_with_memory(
    s: &PgStore,
    suffix: &str,
) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let (proj_id, fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(s, suffix, 2.0).await;
    let based_on = serde_json::json!({ "patterns": [pat_id] });
    let rec_id = s
        .create_recommendation_full(
            &proj_id,
            "_test:regressed-rec",
            "why",
            None,
            "promote_pattern",
            "medium",
            &based_on,
            None,
            None,
        )
        .await
        .unwrap();
    sqlx_core::query::query(
        "UPDATE inference.recommendations
                SET status = 'accepted'::sensei.recommendation_status,
                    acted_at = now() - interval '4 days',
                    baseline_ftr = 0.900
              WHERE id = $1",
    )
    .bind(rec_id)
    .execute(s.pool())
    .await
    .unwrap();
    for _ in 0..3 {
        sqlx_core::query::query(
            "INSERT INTO activity.sessions (folder_id, project_id, outcome, ftr, started_at)
                 VALUES ($1, $2, 'corrected'::sensei.session_outcome, false, now())",
        )
        .bind(fid)
        .bind(proj_id)
        .execute(s.pool())
        .await
        .unwrap();
    }
    (proj_id, pat_id, mem_id, rec_id)
}

async fn cleanup_regressed_fixture(s: &PgStore, proj_id: &uuid::Uuid) {
    // Sessions FK to the folder (persisted), not the project, so drop them
    // explicitly; delete_project cascades recs, memories(+outcomes), patterns.
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE project_id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
    s.delete_project(proj_id).await.ok();
}

async fn violated_count(s: &PgStore, mem_id: &uuid::Uuid) -> i64 {
    let row: (i64,) = query_as(
        "SELECT count(*) FROM sensei.memory_outcomes WHERE memory_id = $1 AND outcome = 'violated'",
    )
    .bind(mem_id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    row.0
}

/// Full round-trip: measuring a regressed rec flips its verdict to negative
/// AND challenges the source memory exactly once — re-measuring is inert
/// (the rec is no longer pending, so the transition never re-fires).
#[tokio::test]
async fn measure_regressed_rec_challenges_source_memory_once() {
    let s = pg_store().await;
    let suffix = format!("regress_challenge_{}", uuid::Uuid::new_v4());
    let (proj_id, _pat_id, mem_id, _rec_id) = seed_regressed_rec_with_memory(&s, &suffix).await;

    let m0 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!((m0["strength"].as_f64().unwrap() - 3.0).abs() < 1e-6, "seed strength 3.0");
    assert_eq!(m0["status"], "active", "memory starts active");

    s.measure_pending_verdicts().await.unwrap();

    let recs = s.list_recommendations(&proj_id).await.unwrap();
    let r = recs.iter().find(|r| r["title"] == "_test:regressed-rec").unwrap();
    assert_eq!(r["verdict"], "negative", "FTR dropped 0.9→0.0 → negative verdict");

    assert_eq!(
        violated_count(&s, &mem_id).await,
        1,
        "one violation recorded for the source memory"
    );
    let m1 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!(
        (m1["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6,
        "trigger dropped strength 3.0→2.3"
    );
    assert_eq!(m1["status"], "challenged", "trigger moved memory to challenged");

    // Re-measure: the rec is no longer pending → not re-measured → no second hit.
    s.measure_pending_verdicts().await.unwrap();
    assert_eq!(violated_count(&s, &mem_id).await, 1, "idempotent: no second violation on re-run");
    let m2 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!((m2["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength did not drop again");

    cleanup_regressed_fixture(&s, &proj_id).await;
}

/// Method-level idempotency: the `rec:<id>` context marker gates the write, so
/// challenging the same memory for the same rec twice records only one violation.
#[tokio::test]
async fn challenge_source_memory_for_rec_is_idempotent_per_rec() {
    let s = pg_store().await;
    let suffix = format!("challenge_idem_{}", uuid::Uuid::new_v4());
    let (proj_id, _fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(&s, &suffix, 2.0).await;
    let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
    let rec_id = uuid::Uuid::new_v4();

    assert!(
        s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "first challenge records a violation"
    );
    assert_eq!(violated_count(&s, &mem_id).await, 1);
    let m1 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!((m1["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength 3.0→2.3");
    assert_eq!(m1["status"], "challenged");

    // Same rec again → no-op, no second violation, strength unchanged.
    assert!(
        !s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "second challenge for same rec is a no-op"
    );
    assert_eq!(violated_count(&s, &mem_id).await, 1, "still exactly one violation");
    let m2 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!((m2["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength did not drop again");

    cleanup_regressed_fixture(&s, &proj_id).await;
}

#[tokio::test]
async fn reinforce_source_memory_for_rec_bumps_promotes_and_is_idempotent() {
    // The G1→G2 bridge: a positive verdict reinforces the source memory via an
    // `applied` outcome, and the memory_outcome_apply trigger promotes it up
    // the ladder. Seed strength 3.6 (active) → one applied → 4.1 (≥4.0, no
    // violations) → battle_tested. Second call for the same rec is a no-op.
    let s = pg_store().await;
    let suffix = format!("reinforce_{}", uuid::Uuid::new_v4());
    let (proj_id, _fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(&s, &suffix, 2.6).await;
    let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
    let rec_id = uuid::Uuid::new_v4();

    let (before,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT reinforced_count::bigint FROM sensei.memories WHERE id=$1",
    )
    .bind(mem_id)
    .fetch_one(s.pool())
    .await
    .unwrap();

    assert!(
        s.reinforce_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "first reinforce records an applied outcome"
    );
    let (strength, status, count): (f64, String, i64) = sqlx_core::query_as::query_as(
            "SELECT strength::float8, status::text, reinforced_count::bigint FROM sensei.memories WHERE id=$1"
        ).bind(mem_id).fetch_one(s.pool()).await.unwrap();
    assert!((strength - 4.1).abs() < 1e-6, "strength 3.6→4.1, got {strength}");
    assert_eq!(status, "battle_tested", "promoted once strength >= 4.0 with no violations");
    assert_eq!(count, before + 1, "reinforced_count bumped once");

    // Same rec again → no-op (idempotency marker), count unchanged.
    assert!(
        !s.reinforce_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "second reinforce for same rec is a no-op"
    );
    let (count2,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT reinforced_count::bigint FROM sensei.memories WHERE id=$1",
    )
    .bind(mem_id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(count2, before + 1, "reinforced_count unchanged on idempotent re-run");

    cleanup_regressed_fixture(&s, &proj_id).await;
}

/// A rec with no resolvable source memory is a clean no-op (not an error):
/// a pattern that never spawned a memory, and empty/absent provenance.
#[tokio::test]
async fn challenge_source_memory_for_rec_no_source_memory_is_noop() {
    let s = pg_store().await;
    let suffix = format!("challenge_nomem_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_project_and_folder(&s, &suffix).await;
    // Pattern with NO sourced memory.
    let pat_id = s
        .upsert_pattern(
            &proj_id,
            Some(&fid),
            "_test:orphan-rule",
            false,
            None,
            &serde_json::json!([]),
        )
        .await
        .unwrap();
    let rec_id = uuid::Uuid::new_v4();

    let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
    assert!(
        !s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "no memory sources this pattern → no-op"
    );
    // Empty / absent provenance → no-op, no panic.
    assert!(!s.challenge_source_memory_for_rec(&rec_id, r#"{"patterns":[]}"#).await.unwrap());
    assert!(!s.challenge_source_memory_for_rec(&rec_id, "{}").await.unwrap());
    assert!(
        s.memory_id_by_source(&pat_id).await.unwrap().is_none(),
        "sanity: pattern has no learned memory"
    );

    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(pat_id)
        .execute(s.pool())
        .await
        .ok();
    s.delete_project(&proj_id).await.ok();
}

// ── Gateway chains / role assignments tests ─────────────────────

/// list_chains_with_models returns every active chain, correctly
/// projects the `role` column, and JSON-aggregates member models in
/// sequence order. Seeded prod rows drive the shape assertions —
/// only a chain the test itself creates is deleted at teardown.
#[tokio::test]
async fn list_chains_with_models_returns_role_and_ordered_members() {
    let s = pg_store().await;
    let chains = s.list_chains_with_models().await.unwrap();
    assert!(!chains.is_empty(), "seed data should include at least one chain");

    // `reasoning` seeds to role=inference; `embed` to role=embedding.
    let reasoning = chains
        .iter()
        .find(|c| c["name"] == "reasoning")
        .expect("seed data should include reasoning chain");
    assert_eq!(reasoning["role"], "inference");

    let embed =
        chains.iter().find(|c| c["name"] == "embed").expect("seed data should include embed chain");
    assert_eq!(embed["role"], "embedding");

    // Utility chain — consensus-proposer — must NOT carry a role.
    let proposer = chains
        .iter()
        .find(|c| c["name"] == "consensus-proposer")
        .expect("seed data should include consensus-proposer");
    assert!(proposer["role"].is_null(), "utility chains stay unassigned");

    // Members are JSON-aggregated in `sequence_order`.
    let members = reasoning["models"].as_array().expect("models is an array");
    if members.len() >= 2 {
        let first = members[0]["sequenceOrder"].as_i64().unwrap();
        let second = members[1]["sequenceOrder"].as_i64().unwrap();
        assert!(first < second, "members are ordered by sequence_order asc");
    }
}

/// set_chain_role writes the role, clears it on None, and rejects a
/// role that another chain already owns (the unique-when-set index).
/// Runs against a scratch chain so seed rows stay intact.
#[tokio::test]
async fn set_chain_role_writes_clears_and_rejects_duplicate() {
    let s = pg_store().await;

    // Create a scratch chain with capability=reasoning so it can carry
    // a role at all.
    let scratch_name = format!("_test:chain_{}", uuid::Uuid::new_v4());
    let (scratch_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO gateway.fallback_chains (name, capability, description, is_active)
             VALUES ($1, 'reasoning'::sensei.model_capability, 'scratch', true)
             RETURNING id",
    )
    .bind(&scratch_name)
    .fetch_one(s.pool())
    .await
    .unwrap();

    // Write voice (unassigned by seed) → row now carries the role.
    s.set_chain_role(&scratch_id, Some("voice")).await.unwrap();
    let row: (Option<String>,) = sqlx_core::query_as::query_as(
        "SELECT role::text FROM gateway.fallback_chains WHERE id = $1",
    )
    .bind(scratch_id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row.0.as_deref(), Some("voice"));

    // Clear → back to null.
    s.set_chain_role(&scratch_id, None).await.unwrap();
    let row: (Option<String>,) = sqlx_core::query_as::query_as(
        "SELECT role::text FROM gateway.fallback_chains WHERE id = $1",
    )
    .bind(scratch_id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(row.0.is_none());

    // Taking a role already owned by another chain (seed: reasoning ↔
    // inference) must fail — surfaces as a duplicate-key DB error the
    // caller can map to a 409 CONFLICT.
    let err = s
        .set_chain_role(&scratch_id, Some("inference"))
        .await
        .expect_err("unique index rejects a second inference chain");
    assert!(
        err.contains("duplicate") || err.contains("unique"),
        "expected uniqueness violation, got: {err}"
    );

    // Unknown chain id → not-found error, not a silent no-op.
    let ghost = uuid::Uuid::new_v4();
    let err = s.set_chain_role(&ghost, Some("voice")).await.expect_err("missing row must error");
    assert!(err.contains("not found"), "expected not-found error, got: {err}");

    // Teardown — remove the scratch chain.
    sqlx_core::query::query("DELETE FROM gateway.fallback_chains WHERE id = $1")
        .bind(scratch_id)
        .execute(s.pool())
        .await
        .unwrap();
}

/// End-to-end chain-model editing: add → move → remove → compact.
/// Runs against a scratch chain so seed rows stay intact.
#[tokio::test]
async fn chain_model_editing_add_move_remove_compacts_sequence() {
    let s = pg_store().await;

    // Scratch chain, capability=chat so any chat model matches.
    let scratch_name = format!("_test:mchain_{}", uuid::Uuid::new_v4());
    let (chain_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO gateway.fallback_chains (name, capability, description, is_active)
             VALUES ($1, 'chat'::sensei.model_capability, 'scratch', true)
             RETURNING id",
    )
    .bind(&scratch_name)
    .fetch_one(s.pool())
    .await
    .unwrap();

    // Pick three (model, router) pairs with capability=chat.
    let pairs: Vec<(uuid::Uuid, uuid::Uuid)> = sqlx_core::query_as::query_as(
        "SELECT m.id, mir.router_id
               FROM gateway.models m
               JOIN gateway.models_in_router mir ON mir.model_id = m.id
              WHERE m.capabilities @> ARRAY['chat'::sensei.model_capability]
              LIMIT 3",
    )
    .fetch_all(s.pool())
    .await
    .unwrap();
    assert!(
        pairs.len() >= 2,
        "test needs at least 2 chat-capable (model, router) pairs; got {}",
        pairs.len()
    );

    // Add — sequence_order starts at 1 and advances.
    let (row_a, seq_a) = s.add_chain_model(&chain_id, &pairs[0].0, &pairs[0].1).await.unwrap();
    let (row_b, seq_b) = s.add_chain_model(&chain_id, &pairs[1].0, &pairs[1].1).await.unwrap();
    assert_eq!(seq_a, 1);
    assert_eq!(seq_b, 2);

    // Move A down (swap with B).
    let moved = s.move_chain_model(&chain_id, &row_a, 1).await.unwrap();
    assert!(moved, "A should swap with B");
    let (seq_a_now,): (i32,) = sqlx_core::query_as::query_as(
        "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1",
    )
    .bind(row_a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(seq_a_now, 2, "A now sits at position 2");
    let (seq_b_now,): (i32,) = sqlx_core::query_as::query_as(
        "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1",
    )
    .bind(row_b)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(seq_b_now, 1, "B moved into A's old slot");

    // Move B up past the top — boundary, no-op.
    let moved = s.move_chain_model(&chain_id, &row_b, -1).await.unwrap();
    assert!(!moved, "top boundary should return false");

    // Remove B — A should compact to sequence_order 1.
    s.remove_chain_model(&chain_id, &row_b).await.unwrap();
    let (seq_a_final,): (i32,) = sqlx_core::query_as::query_as(
        "SELECT sequence_order FROM gateway.fallback_chain_models WHERE id = $1",
    )
    .bind(row_a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(seq_a_final, 1, "A compacts after B removal");

    // Not-found errors surface, not silent no-ops.
    let ghost = uuid::Uuid::new_v4();
    let err = s.remove_chain_model(&chain_id, &ghost).await.expect_err("remove missing must error");
    assert!(err.contains("not found"), "expected not-found, got: {err}");
    let err = s.move_chain_model(&chain_id, &ghost, 1).await.expect_err("move missing must error");
    assert!(err.contains("not found"), "expected not-found, got: {err}");

    // Available list: chain has 1 model, so all others with matching
    // capability are available (excludes the row we still have).
    let available = s.list_available_models_for_chain(&chain_id).await.unwrap();
    assert!(!available.is_empty(), "at least one chat model should be available after removing B");

    // Bad direction rejected with a clear message.
    let err = s.move_chain_model(&chain_id, &row_a, 2).await.expect_err("direction 2 must reject");
    assert!(err.contains("-1") || err.contains("+1"), "expected direction hint, got: {err}");

    // Teardown.
    sqlx_core::query::query("DELETE FROM gateway.fallback_chains WHERE id = $1")
        .bind(chain_id)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Communities tests ────────────────────────────────────────────

#[tokio::test]
async fn community_upsert_and_list() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("comm_{}", uuid::Uuid::new_v4())).await;
    let cid = s.upsert_community(&fid, 1, "_test:auth_cluster", 3).await.unwrap();
    let comms = s.list_communities(&fid).await.unwrap();
    assert!(comms.iter().any(|c| c["label"] == "_test:auth_cluster" && c["node_count"] == 3));
    sqlx_core::query::query("DELETE FROM inference.communities WHERE id = $1")
        .bind(cid)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Reasoning Traces tests ───────────────────────────────────────

#[tokio::test]
async fn reasoning_trace_insert_and_get() {
    let s = pg_store().await;
    let pid = s.create_project("_test:rt_proj", None, None).await.unwrap();
    let tid = s
        .insert_reasoning_trace(
            Some(&pid),
            "pattern_emerging",
            &serde_json::json!({}),
            &["gemma4:27b".into()],
            &serde_json::json!([{"model":"gemma4","role":"proposer","content":"analyze"}]),
            &serde_json::json!({"conclusion":"adopt adapter pattern","confidence":0.9}),
        )
        .await
        .unwrap();
    let traces = s.get_reasoning_traces_by_project(&pid).await.unwrap();
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0]["consensus"]["confidence"], 0.9);
    assert_eq!(traces[0]["trigger_event"], "pattern_emerging");
    sqlx_core::query::query("DELETE FROM inference.reasoning_traces WHERE id = $1")
        .bind(tid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_project(&pid).await.unwrap();
}

// ── Folders to Watch tests ─────────────────────────────────────────

#[tokio::test]
async fn watch_root_add_and_list() {
    let s = pg_store().await;
    let path = format!("/_test/watch_{}", uuid::Uuid::new_v4());
    let id =
        s.add_watch_root(&path, "test_root", &serde_json::json!(["node_modules"])).await.unwrap();
    let roots = s.list_watch_roots().await.unwrap();
    assert!(roots.iter().any(|r| r["path"] == path));
    s.remove_watch_root(&id).await.unwrap();
}

#[tokio::test]
async fn watch_root_update_status() {
    let s = pg_store().await;
    let path = format!("/_test/watch_status_{}", uuid::Uuid::new_v4());
    let id = s.add_watch_root(&path, "test", &serde_json::json!([])).await.unwrap();
    s.update_watch_status(&id, "watching").await.unwrap();
    let roots = s.list_watch_roots().await.unwrap();
    let r = roots.iter().find(|r| r["path"] == path).unwrap();
    assert_eq!(r["status"], "watching");
    s.remove_watch_root(&id).await.unwrap();
}

// ── Scan State tests ─────────────────────────────────────────────

#[tokio::test]
async fn scan_state_upsert_and_stale() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("scan_{}", uuid::Uuid::new_v4())).await;
    s.upsert_scan_state(&fid, "src/main.rs", 1000, "hash1").await.unwrap();
    // Same mtime = not stale
    let stale = s.get_stale_files(&fid, &[("src/main.rs".into(), 1000)]).await.unwrap();
    assert!(stale.is_empty());
    // Changed mtime = stale
    let stale = s.get_stale_files(&fid, &[("src/main.rs".into(), 2000)]).await.unwrap();
    assert_eq!(stale, vec!["src/main.rs"]);
    // New file = stale
    let stale = s.get_stale_files(&fid, &[("src/new.rs".into(), 1000)]).await.unwrap();
    assert_eq!(stale, vec!["src/new.rs"]);
    s.delete_scan_state(&fid).await.unwrap();
}

// ── Services tests ───────────────────────────────────────────────

#[tokio::test]
async fn service_upsert_and_list() {
    let s = pg_store().await;
    let name = format!("_test:svc_{}", uuid::Uuid::new_v4());
    let id = s
        .upsert_service(
            &name,
            "Test MCP",
            "data",
            "mcp",
            &serde_json::json!({"url":"http://localhost"}),
        )
        .await
        .unwrap();
    let svcs = s.list_services().await.unwrap();
    assert!(svcs.iter().any(|sv| sv["name"] == name));
    s.delete_service(&name).await.unwrap();
    let _ = id;
}

// ── Snapshots tests ──────────────────────────────────────────────

#[tokio::test]
async fn snapshot_create_and_get_latest() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("snap_{}", uuid::Uuid::new_v4())).await;
    let sid = s.create_session(&fid, "snapshot test", None).await.unwrap();
    s.create_snapshot(&sid, &fid, "manual", "Step 1 done", Some("Do step 2"), &["Step 1".into()])
        .await
        .unwrap();
    s.create_snapshot(
        &sid,
        &fid,
        "checkpoint",
        "Step 2 done",
        None,
        &["Step 1".into(), "Step 2".into()],
    )
    .await
    .unwrap();
    let latest = s.get_latest_snapshot(&sid).await.unwrap().unwrap();
    assert_eq!(latest["progress_summary"], "Step 2 done");
    assert_eq!(latest["kind"], "checkpoint");
    assert_eq!(latest["completed_steps"].as_array().unwrap().len(), 2);
}

// ── Detected Patterns tests ────────────────────────────────────────

#[tokio::test]
async fn pattern_upsert_and_list() {
    let s = pg_store().await;
    let suffix = format!("pat_upsert_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_project_and_folder(&s, &suffix).await;
    let instances =
        serde_json::json!([{"file":"src/lib.rs","line":10},{"file":"src/main.rs","line":20}]);
    let pat_id = s
        .upsert_pattern(&proj_id, Some(&fid), "_test:Adapter", false, Some(0.85), &instances)
        .await
        .unwrap();
    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    assert!(patterns.iter().any(|p| p["name"] == "_test:Adapter" && p["instance_count"] == 2));
    // cleanup
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(pat_id)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn pattern_promote() {
    let s = pg_store().await;
    let suffix = format!("pat_promote_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_project_and_folder(&s, &suffix).await;
    let pat_id = s
        .upsert_pattern(&proj_id, Some(&fid), "_test:Factory", false, None, &serde_json::json!([]))
        .await
        .unwrap();
    s.promote_pattern(&pat_id, "rule").await.unwrap();
    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(p["lifecycle"], "rule");
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(pat_id)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn pattern_upsert_updates_existing() {
    let s = pg_store().await;
    let suffix = format!("pat_dup_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_project_and_folder(&s, &suffix).await;
    let id1 = s
        .upsert_pattern(
            &proj_id,
            Some(&fid),
            "_test:Singleton",
            false,
            Some(0.5),
            &serde_json::json!([{"file":"a.rs"}]),
        )
        .await
        .unwrap();
    let id2 = s
        .upsert_pattern(
            &proj_id,
            Some(&fid),
            "_test:Singleton",
            false,
            Some(0.9),
            &serde_json::json!([{"file":"a.rs"},{"file":"b.rs"}]),
        )
        .await
        .unwrap();
    assert_eq!(id1, id2); // same row updated
    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["name"] == "_test:Singleton").unwrap();
    assert_eq!(p["instance_count"], 2);
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(id1)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn pattern_upsert_merges_across_folders_in_same_project() {
    // #82: patterns are project-scoped. Two folders in the same project
    // upserting the same pattern name collapse into a single row — the
    // second upsert updates the first row's instances/folder_id locus.
    let s = pg_store().await;
    let suffix = format!("pat_project_scope_{}", uuid::Uuid::new_v4());
    let (proj_id, fid_a) = create_test_project_and_folder(&s, &suffix).await;
    let fid_b = create_test_folder(&s, &format!("{}_b", suffix)).await;
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(proj_id)
        .bind(fid_b)
        .execute(s.pool())
        .await
        .unwrap();

    let id_a = s
        .upsert_pattern(
            &proj_id,
            Some(&fid_a),
            "_test:Shared",
            false,
            None,
            &serde_json::json!([{"file":"a.rs"}]),
        )
        .await
        .unwrap();
    let id_b = s
        .upsert_pattern(
            &proj_id,
            Some(&fid_b),
            "_test:Shared",
            false,
            None,
            &serde_json::json!([{"file":"b.rs"},{"file":"b2.rs"}]),
        )
        .await
        .unwrap();
    assert_eq!(id_a, id_b, "same (project_id, name) must merge into one row");

    // The row's instances reflect the latest upsert; folder_id follows too.
    let (count, locus): (i32, uuid::Uuid) = sqlx_core::query_as::query_as(
        "SELECT instance_count, folder_id FROM inference.detected_patterns WHERE id = $1",
    )
    .bind(id_a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(count, 2);
    assert_eq!(locus, fid_b, "folder_id is the latest upsert's locus");

    // cleanup
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(id_a)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
}

// ── Project merge tests (#41) ──────────────────────────────────────

#[tokio::test]
async fn merge_projects_moves_folders_sessions_memories_and_deletes_source() {
    let s = pg_store().await;
    let suffix = format!("merge_{}", uuid::Uuid::new_v4());
    let (src, src_folder) = create_test_project_and_folder(&s, &format!("{}_src", suffix)).await;
    let (tgt, _tgt_folder) = create_test_project_and_folder(&s, &format!("{}_tgt", suffix)).await;

    // Seed a memory attributed to the source project so we can prove it
    // survives the merge (only its project_id shifts).
    let mem_id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories(project_id, scope, type, title, content, origin)
             VALUES($1, 'project'::sensei.memory_scope, 'convention'::sensei.memory_type, $2, 'body', 'user')
             RETURNING id"
        ).bind(src).bind(format!("_test:merge_memory_{}", uuid::Uuid::new_v4()))
            .fetch_one(s.pool()).await.unwrap();

    s.merge_projects(&src, &tgt).await.unwrap();

    // Source project row is gone.
    let src_exists: (bool,) =
        sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)")
            .bind(src)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(!src_exists.0, "source project should be deleted after merge");

    // The source's folder now lives under the target project.
    let (folder_project,): (Option<uuid::Uuid>,) =
        sqlx_core::query_as::query_as("SELECT project_id FROM sensei.folders WHERE id = $1")
            .bind(src_folder)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(folder_project, Some(tgt), "folder should be reassigned to target");

    // The memory survived and points at the target.
    let (mem_project,): (Option<uuid::Uuid>,) =
        sqlx_core::query_as::query_as("SELECT project_id FROM sensei.memories WHERE id = $1")
            .bind(mem_id.0)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(mem_project, Some(tgt), "user-authored memory should follow to target");

    // cleanup
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(mem_id.0)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(tgt)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn merge_projects_rejects_self_merge() {
    let s = pg_store().await;
    let (pid, _fid) =
        create_test_project_and_folder(&s, &format!("selfmerge_{}", uuid::Uuid::new_v4())).await;
    let err = s.merge_projects(&pid, &pid).await.unwrap_err();
    assert!(err.contains("must differ"), "got: {err}");
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn merge_projects_errors_on_missing_ids() {
    let s = pg_store().await;
    let ghost = uuid::Uuid::new_v4();
    let (real, _fid) =
        create_test_project_and_folder(&s, &format!("mergemiss_{}", uuid::Uuid::new_v4())).await;
    let err = s.merge_projects(&ghost, &real).await.unwrap_err();
    assert!(err.contains("expected source + target to exist"), "got: {err}");
    // The real project is untouched.
    let exists: (bool,) =
        sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)")
            .bind(real)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(exists.0);
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(real)
        .execute(s.pool())
        .await
        .ok();
}

// ── Bug 3: re-absorb a standalone root mis-scoped inside a git repo ────

#[tokio::test]
async fn heal_nested_standalone_roots_reabsorbs_and_removes_phantom() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root_path = format!("/_test/heal_nested/{uniq}");
    let root_id =
        s.add_watch_root(&root_path, "heal_nested_root", &serde_json::json!([])).await.unwrap();

    // A git repo (like the sensei monorepo) with its own project.
    let repo_abs = format!("{root_path}/repo");
    let repo_pid = s.create_project(&format!("_test:heal_repo_{uniq}"), None, None).await.unwrap();
    let repo_fid = s.upsert_repo_kind(&root_id, "git", "repo", &repo_abs).await.unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(repo_pid)
        .bind(repo_fid)
        .execute(s.pool())
        .await
        .unwrap();

    // A sub-crate INSIDE the repo, mis-scoped as its own standalone project
    // (the Bug 3 phantom). Give it a node so we can prove its nodes are dropped.
    let crate_abs = format!("{repo_abs}/crates/dojo-mind");
    let phantom_pid =
        s.create_project(&format!("_test:heal_phantom_{uniq}"), None, None).await.unwrap();
    let crate_fid =
        s.upsert_repo_kind(&root_id, "standalone", "dojo-mind", &crate_abs).await.unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(phantom_pid)
        .bind(crate_fid)
        .execute(s.pool())
        .await
        .unwrap();
    let node_id = s
        .upsert_node(&crate_fid, "struct", "DojoStore", "src/store.rs", None, None, None, None)
        .await
        .unwrap();

    // Heal.
    let healed = s.heal_nested_standalone_roots().await.unwrap();
    assert!(healed >= 1, "the nested standalone root should be re-absorbed");

    // The nested root is now a folder of the repo's project, parented to the repo.
    let (kind, pid, parent): (String, Option<uuid::Uuid>, Option<uuid::Uuid>) =
        sqlx_core::query_as::query_as(
            "SELECT kind::text, project_id, parent_id FROM sensei.folders WHERE id = $1",
        )
        .bind(crate_fid)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(kind, "folder", "mis-scoped standalone should be re-classified as a folder");
    assert_eq!(pid, Some(repo_pid), "should now belong to the enclosing repo's project");
    assert_eq!(parent, Some(repo_fid), "should be parented under the enclosing repo");

    // Its own nodes were dropped (the repo re-indexes the subtree).
    let (node_exists,): (bool,) =
        sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id = $1)")
            .bind(node_id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(!node_exists, "the mis-scoped root's own nodes should be pruned");

    // The phantom project (lived entirely inside the repo) is gone.
    let (phantom_exists,): (bool,) =
        sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)")
            .bind(phantom_pid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(!phantom_exists, "the phantom project should be merged away");

    // Idempotent for THIS test's rows: a second run leaves the phantom merged
    // away. The returned count is GLOBAL — db-gated tests share `sensei_test`
    // and other tests (e.g. the index-audit suite) may seed nested-standalone
    // rows concurrently — so assert on our own row, not the global count.
    s.heal_nested_standalone_roots().await.unwrap();
    let (phantom_gone_after_rerun,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT NOT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)",
    )
    .bind(phantom_pid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(phantom_gone_after_rerun, "re-run leaves the phantom merged away (idempotent)");

    // cleanup
    s.delete_folder_tree(&repo_fid).await.ok();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(repo_pid)
        .execute(s.pool())
        .await
        .ok();
    s.remove_watch_root(&root_id).await.ok();
}

#[tokio::test]
async fn list_indexed_files_excludes_modules_and_empties() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root_path = format!("/_test/idx_files/{uniq}");
    let root_id =
        s.add_watch_root(&root_path, "idx_files_root", &serde_json::json!([])).await.unwrap();
    let repo_abs = format!("{root_path}/repo");
    let fid = s.upsert_repo_kind(&root_id, "git", "repo", &repo_abs).await.unwrap();

    s.upsert_node(&fid, "file", "a.rs", "a.rs", None, None, None, None).await.unwrap();
    s.upsert_node(&fid, "struct", "B", "b.rs", None, None, None, None).await.unwrap();
    // A module node records an ABSOLUTE dir path — must be excluded so it never
    // pollutes the rel-path comparison in prune_vanished.
    s.upsert_node(&fid, "module", "src", &format!("{repo_abs}/src"), None, None, None, None)
        .await
        .unwrap();

    let mut files = s.list_indexed_files(&fid).await.unwrap();
    files.sort();
    assert_eq!(
        files,
        vec!["a.rs".to_string(), "b.rs".to_string()],
        "only real (rel) file paths, no module"
    );

    s.delete_folder_tree(&fid).await.ok();
    s.remove_watch_root(&root_id).await.ok();
}

// ── Activity pruner tests (#74) ────────────────────────────────────

#[tokio::test]
async fn prune_activity_keeps_unanalyzed_sessions_even_when_old() {
    let s = pg_store().await;
    let suffix = format!("prune_keep_unanalyzed_{}", uuid::Uuid::new_v4());
    let (_pid, fid) = create_test_project_and_folder(&s, &suffix).await;
    let csid = format!("{}-csid", suffix);
    let sid = s.record_session_event(&csid, &fid, None, "claude", true).await.unwrap();
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
    let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;
    let csid = format!("{}-csid", suffix);
    let sid = s.record_session_event(&csid, &fid, None, "claude", true).await.unwrap();
    // Age to 40 days: past the 30-day retention window but INSIDE the 60-day
    // backstop, so the ONLY thing that makes it prune-eligible is the
    // capture path — its day must already exist in sensei.project_metrics.
    // (Previously aged 90 days, which pruned unconditionally; now the test
    // exercises capture-before-reclaim directly.)
    // Repo-grain: the capture guard keys on the session's repository, so give the
    // folder a repository and anchor the session to it via repo_folder_id.
    let (repo_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'prune-del') RETURNING id",
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
    // Seed a covering daily project_metrics row for the session's day (its OWN
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
    s.upsert_project_metric_repo(
        &ftr_id.0,
        &repo_id,
        "user",
        None,
        None,
        day40.0,
        "daily",
        1.0,
        &serde_json::json!({}),
        "measured",
    )
    .await
    .unwrap();
    // Seed a child transcript_turn keyed on client_session_id (no FK).
    sqlx_core::query::query(
            // `family` is NOT NULL — TranscriptAdapter::family() returns &'static str,
            // so the ingest path always supplies one and a fixture must too.
            "INSERT INTO activity.transcript_turns(session_id, source, family, turn_index, assistant_text)
             VALUES ($1, 'claude_code', 'claude', 0, 'hello')"
        ).bind(&csid).execute(s.pool()).await.unwrap();
    // Seed a hook event under the same client_session_id.
    s.insert_hook_event(
        &csid,
        "claude",
        "UserPromptSubmit",
        None,
        None,
        1000,
        None,
        &serde_json::json!({"prompt": "hi"}),
    )
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
/// row, and `metrics.capture_source='session'` — NOT the project, NOT the
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
    let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;

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
        let sid = s.record_session_event(&csid, fid, None, "claude", true).await.unwrap();
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
    s.upsert_project_metric_repo(
        &ftr_id.0,
        &repo_a,
        "user",
        None,
        None,
        day_ago(&s, 40).await,
        "daily",
        1.0,
        &serde_json::json!({}),
        "measured",
    )
    .await
    .unwrap();

    // (b) uncaptured on repo_a; a DECOY scope=user session metric for the SAME day
    //     lives on repo_b → KEPT (the guard must match the session's repository).
    let uncaptured = aged_repo_session(&s, &fid, &suffix, "uncaptured", 45).await;
    s.upsert_project_metric_repo(
        &ftr_id.0,
        &repo_b,
        "user",
        None,
        None,
        day_ago(&s, 45).await,
        "daily",
        1.0,
        &serde_json::json!({}),
        "measured",
    )
    .await
    .unwrap();

    // (c) on repo_a/day50, TWO non-authorizing rows: a snapshot + DAY-cadence
    //     rework_density (scope=user) and a session-source ftr at scope=repo.
    //     Neither authorizes reclaim → KEPT. Fails if the guard keys on
    //     cadence='day' (the rework_density row would capture) or drops the
    //     scope='user' filter (the scope=repo ftr row would capture).
    let wrong_signal = aged_repo_session(&s, &fid, &suffix, "wrongsignal", 50).await;
    s.upsert_project_metric_repo(
        &rework_id.0,
        &repo_a,
        "user",
        None,
        None,
        day_ago(&s, 50).await,
        "daily",
        0.2,
        &serde_json::json!({}),
        "measured",
    )
    .await
    .unwrap();
    s.upsert_project_metric_repo(
        &ftr_id.0,
        &repo_a,
        "repo",
        None,
        None,
        day_ago(&s, 50).await,
        "daily",
        1.0,
        &serde_json::json!({}),
        "measured",
    )
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
    // uniqueness must not collide across runs of the shared test DB). project_metrics
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
    s.insert_hook_event(
        &orphan_csid,
        "claude",
        "PostToolUse",
        Some("Read"),
        None,
        old_ts,
        None,
        &serde_json::json!({}),
    )
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
    s.insert_hook_event(
        &orphan_csid,
        "claude",
        "PostToolUse",
        Some("Read"),
        None,
        old_ts,
        None,
        &serde_json::json!({}),
    )
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

// ── Corrections aggregation tests ──────────────────────────────────

#[tokio::test]
async fn correction_upsert_is_idempotent_by_signature() {
    // Prunes by keep-set, which deletes every OTHER test's corrections.
    let _serialised = crate::tasks::test_support::CORRECTIONS_TABLE_LOCK.enter();
    let s = pg_store().await;
    let p = uuid::Uuid::new_v4();
    let sig = format!("corr-test-{}", uuid::Uuid::new_v4());
    let row = crate::corrections::CorrectionRow {
        signature: sig.clone(),
        text: "Use $state for reactive locals".into(),
        suggestion: Some("Reinforce the svelte5 memory".into()),
        count: 3,
        project_ids: vec![p],
        last_seen: chrono::Utc::now(),
        memory_id: None,
        instances: serde_json::json!([{"session_id": "s1", "ts": 1, "prompt": "use $state"}]),
    };
    let id1 = s.upsert_correction(&row).await.unwrap();
    let mut row2 = row.clone();
    row2.count = 4;
    let id2 = s.upsert_correction(&row2).await.unwrap();
    assert_eq!(id1, id2, "same signature updates the same row");

    let global = s.list_corrections().await.unwrap();
    let found = global["corrections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id1.to_string())
        .unwrap()
        .clone();
    assert_eq!(found["count"], 4);
    assert_eq!(found["text"], "Use $state for reactive locals");

    // the project-scoped read exercises the `$1 = ANY(project_ids)` filter.
    let scoped = s.list_corrections_for_project(&p).await.unwrap();
    assert!(
        scoped["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
        "per-project read returns a correction tagged with that project"
    );

    // prune keeping our signature → the row survives.
    s.delete_corrections_not_in(std::slice::from_ref(&sig)).await.unwrap();
    let kept = s.list_corrections().await.unwrap();
    assert!(
        kept["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
        "a kept signature survives the prune"
    );

    // prune excluding our signature → the row is deleted (also leaves the
    // test DB clean — this clears the derived corrections table).
    s.delete_corrections_not_in(&["corr-nope".to_string()]).await.unwrap();
    let after = s.list_corrections().await.unwrap();
    assert!(
        !after["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
        "a signature not in the keep set is pruned"
    );
}

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
async fn upsert_project_dependency_is_idempotent_and_stores_all_columns() {
    // 1a Step 5: project → project edges must be idempotent on the
    // composite PK (from_project, to_project, from_folder, source_manifest)
    // and must preserve source_protocol and resolved_target across upserts.
    let s = pg_store().await;
    let from_pid =
        s.ensure_test_project(&format!("dep-from-{}", uuid::Uuid::new_v4())).await.unwrap();
    let to_pid = s.ensure_test_project(&format!("dep-to-{}", uuid::Uuid::new_v4())).await.unwrap();
    let from_fid = create_test_folder(&s, &format!("pd-{}", uuid::Uuid::new_v4())).await;

    // First upsert
    s.upsert_project_dependency(
        &from_pid,
        &to_pid,
        &from_fid,
        "link",
        "package.json",
        Some("../actions"),
    )
    .await
    .unwrap();
    // Repeat with a different resolved_target — same PK, so this must
    // update in place (last-writer wins on non-key columns).
    s.upsert_project_dependency(
        &from_pid,
        &to_pid,
        &from_fid,
        "link",
        "package.json",
        Some("../actions-renamed"),
    )
    .await
    .unwrap();

    use sqlx_core::query_as::query_as;
    let rows: Vec<(String, Option<String>)> = query_as(
        "SELECT source_protocol, resolved_target
               FROM sensei.project_dependencies
              WHERE from_project_id = $1 AND to_project_id = $2 AND from_folder_id = $3",
    )
    .bind(from_pid)
    .bind(to_pid)
    .bind(from_fid)
    .fetch_all(s.pool())
    .await
    .unwrap();

    assert_eq!(rows.len(), 1, "composite PK must dedupe two upserts");
    assert_eq!(rows[0].0, "link", "protocol preserved");
    assert_eq!(rows[0].1.as_deref(), Some("../actions-renamed"), "target updated in place");

    // Cleanup
    sqlx_core::query::query("DELETE FROM sensei.project_dependencies WHERE from_folder_id = $1")
        .bind(from_fid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_project(&from_pid).await.ok();
    s.delete_project(&to_pid).await.ok();
}

#[tokio::test]
async fn version_conflicts_view_flags_multi_version_pins_and_excludes_local() {
    // 1a Step 7-8: two folders in the same project pin the same library
    // at DIFFERENT versions → surfaces as a conflict. A third row tagged
    // local_source (as if declared via link:) with a different version
    // must NOT contribute to the conflict.
    let s = pg_store().await;
    let suffix = uuid::Uuid::new_v4();
    let pid = s.ensure_test_project(&format!("vc-{suffix}")).await.unwrap();
    let lib = s
        .upsert_library(&format!("_test:vc-lib-{suffix}"), "npm", Some("1.0"), None, None, None)
        .await
        .unwrap();

    // Two folders in the same project, different versions.
    let fid_a = create_test_folder(&s, &format!("vc-a-{suffix}")).await;
    let fid_b = create_test_folder(&s, &format!("vc-b-{suffix}")).await;
    // Attach folders to the project.
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id IN ($2, $3)")
        .bind(pid)
        .bind(fid_a)
        .bind(fid_b)
        .execute(s.pool())
        .await
        .unwrap();

    s.upsert_referenced_library(&fid_a, &lib, Some("1.2.0"), None).await.unwrap();
    s.upsert_referenced_library(&fid_b, &lib, Some("1.3.0"), None).await.unwrap();

    // Third folder pins a local-source variant. This must be excluded.
    let fid_local = create_test_folder(&s, &format!("vc-local-{suffix}")).await;
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(pid)
        .bind(fid_local)
        .execute(s.pool())
        .await
        .unwrap();
    s.upsert_referenced_library(
        &fid_local,
        &lib,
        Some("workspace-42"),
        Some(serde_json::json!({"local_source": "../lib"})),
    )
    .await
    .unwrap();

    let rows = s.list_project_library_version_conflicts(&pid).await.unwrap();
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
    // project (folders cascade because project_id set null).
    s.delete_library(&lib).await.unwrap();
    s.delete_project(&pid).await.ok();
}

#[tokio::test]
async fn list_project_dependencies_joins_target_name_and_folder() {
    // 1a Step 6: the list endpoint returns each outgoing edge with the
    // TARGET project's name and the source folder's name joined in.
    let s = pg_store().await;
    let suffix = uuid::Uuid::new_v4();
    let from_pid = s.ensure_test_project(&format!("lpd-from-{suffix}")).await.unwrap();
    let to_pid = s.ensure_test_project(&format!("lpd-to-{suffix}")).await.unwrap();
    let from_fid = create_test_folder(&s, &format!("lpd-fid-{suffix}")).await;

    s.upsert_project_dependency(
        &from_pid,
        &to_pid,
        &from_fid,
        "link",
        "package.json",
        Some("../actions"),
    )
    .await
    .unwrap();

    let deps = s.list_project_dependencies(&from_pid).await.unwrap();

    assert_eq!(deps.len(), 1);
    let d = &deps[0];
    assert_eq!(d["to_project_id"].as_str().unwrap(), to_pid.to_string());
    assert!(
        d["to_project_name"].as_str().unwrap().starts_with("_test:lpd-to-"),
        "target project name must be joined in"
    );
    assert!(
        d["from_folder"].as_str().unwrap().starts_with("lpd-fid-"),
        "source folder name must be joined in"
    );
    assert_eq!(d["source_protocol"], "link");
    assert_eq!(d["source_manifest"], "package.json");
    assert_eq!(d["resolved_target"], "../actions");

    // Reverse direction returns empty
    let none = s.list_project_dependencies(&to_pid).await.unwrap();
    assert!(none.is_empty(), "target project has no outgoing edges");

    sqlx_core::query::query("DELETE FROM sensei.project_dependencies WHERE from_folder_id = $1")
        .bind(from_fid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_project(&from_pid).await.ok();
    s.delete_project(&to_pid).await.ok();
}

#[tokio::test]
async fn upsert_project_dependency_rejects_self_edges() {
    // 1a Step 5: DDL check constraint (from_project_id <> to_project_id)
    // must reject self-edges at the write path.
    let s = pg_store().await;
    let pid = s.ensure_test_project(&format!("self-{}", uuid::Uuid::new_v4())).await.unwrap();
    let fid = create_test_folder(&s, &format!("self-fid-{}", uuid::Uuid::new_v4())).await;

    let err = s.upsert_project_dependency(&pid, &pid, &fid, "path", "Cargo.toml", Some(".")).await;

    assert!(err.is_err(), "self-edge must be rejected");
    assert!(err.unwrap_err().contains("check"), "err message must reference the check constraint");

    s.delete_project(&pid).await.ok();
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
async fn project_library_promotion_shows_in_resolved_and_is_idempotent() {
    // #30: referenced_libraries (folder-grained) must roll up to
    // project_libraries so detected libs — incl. scoped @rokkit/* — show in
    // project_libraries_resolved (the Projects screen). Was never populated.
    let s = pg_store().await;
    let pid = s.ensure_test_project("proj-lib-promo").await.unwrap();
    let lib =
        s.upsert_library("_test:@rokkit/core", "npm", Some("1.2"), None, None, None).await.unwrap();
    // Promote twice — must be idempotent (no error, no duplicate row).
    s.upsert_project_library(&lib, &pid).await.unwrap();
    s.upsert_project_library(&lib, &pid).await.unwrap();
    let libs = s.get_project_libraries(&pid).await.unwrap();
    let hits = libs.iter().filter(|l| l["name"] == "_test:@rokkit/core").count();
    assert_eq!(
        hits, 1,
        "promoted scoped lib should appear exactly once in resolved view; got {libs:?}"
    );
    s.delete_library(&lib).await.unwrap(); // FK CASCADE removes the project_libraries row
}

#[tokio::test]
async fn ensure_test_project_is_namespaced_and_idempotent() {
    // #34: test fixtures must not accrete a new row per run, nor look like
    // real projects. Reuse one `_test:`-namespaced row per name.
    let s = pg_store().await;
    let a = s.ensure_test_project("dup-check").await.unwrap();
    let b = s.ensure_test_project("dup-check").await.unwrap();
    assert_eq!(a, b, "repeated ensure_test_project must reuse one row, not create a new one");
    let proj = s.get_project(&a).await.unwrap().unwrap();
    assert_eq!(proj["name"], "_test:dup-check", "test projects must be _test:-namespaced");
    s.delete_project(&a).await.ok();
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
async fn session_row_folder(s: &PgStore, client_session_id: &str) -> Option<uuid::Uuid> {
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
async fn clear_test_session(s: &PgStore, sid: &str) {
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
/// leaving `heal_duplicate_name_projects` ungated left its two tests racing
/// against every other test's setup — a project is a folderless duplicate for
/// the instant between being created and having its folder attached, which is
/// exactly the state the sweep is entitled to prune.
///
/// Blocking on purpose — see [`crate::tasks::test_support::TestGate`] for why
/// an async mutex loses wakers across per-test runtimes.
static REPAIR_SWEEP_GATE: crate::tasks::test_support::TestGate =
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
    s.insert_hook_event(
        sess,
        "claude",
        "PreToolUse",
        None,
        Some("/_test/repair-old/src"),
        1_700_000_000,
        None,
        &serde_json::json!({}),
    )
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
    s.insert_hook_event(
        sess,
        "claude",
        "PreToolUse",
        None,
        Some("/_test/shadow-parent"),
        1_700_000_100,
        None,
        &serde_json::json!({}),
    )
    .await
    .unwrap();
    s.insert_hook_event(
        sess,
        "claude",
        "PreToolUse",
        None,
        Some("/_test/shadow-parent/sub/x"),
        1_700_000_200,
        None,
        &serde_json::json!({}),
    )
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

// ── resolve_repo_anchor / sensei.repo_anchor_for (spec 2026-08-18) ────────
/// Create an anchor-test folder at `abs_path` with an explicit kind + optional project.
async fn mk_anchor_folder(
    s: &PgStore,
    abs_path: &str,
    kind: &str,
    project_id: Option<uuid::Uuid>,
) -> uuid::Uuid {
    use sqlx_core::query_as::query_as;
    s.execute_raw("INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING").await.unwrap();
    let name = abs_path.rsplit('/').next().unwrap_or(abs_path);
    let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, project_id) \
             VALUES('00000000-0000-0000-0000-000000000001', $1::sensei.folder_kind, $2, $2, $3, $4) \
             ON CONFLICT(abs_path) DO UPDATE SET kind = EXCLUDED.kind, project_id = EXCLUDED.project_id RETURNING id"
        ).bind(kind).bind(name).bind(abs_path).bind(project_id).fetch_one(s.pool()).await.unwrap();
    row.0
}

#[tokio::test]
async fn repo_anchor_repo_subdirs_and_plain_folders() {
    // 1,2,5: repo self; deep subdir → repo; under a plain `folder` row → repo (never the folder).
    let s = pg_store().await;
    let b = "/_test/anchor_a";
    let repo = mk_anchor_folder(&s, &format!("{b}/repo"), "git", None).await;
    mk_anchor_folder(&s, &format!("{b}/repo/src"), "folder", None).await; // must be invisible
    assert_eq!(
        s.resolve_repo_anchor(&format!("{b}/repo")).await.unwrap().unwrap().repo_folder_id,
        repo,
        "cwd == repo abs_path"
    );
    let a = s.resolve_repo_anchor(&format!("{b}/repo/src/deep/x.ts")).await.unwrap().unwrap();
    assert_eq!(
        a.repo_folder_id, repo,
        "deep cwd under a plain folder resolves to the repo, never the folder"
    );
    assert_eq!(a.matched_via, "live");
}

#[tokio::test]
async fn repo_anchor_rolls_monorepo_member_to_git_root() {
    // 3 (E6/D6): a cwd inside a workspace_member rolls up to the enclosing git root.
    let s = pg_store().await;
    let b = "/_test/anchor_mono";
    let root = mk_anchor_folder(&s, &format!("{b}/mono"), "git", None).await;
    mk_anchor_folder(&s, &format!("{b}/mono/packages/pkg"), "workspace_member", None).await;
    let a = s.resolve_repo_anchor(&format!("{b}/mono/packages/pkg/lib.ts")).await.unwrap().unwrap();
    assert_eq!(a.repo_folder_id, root, "monorepo member rolls up to the git root, not the member");
}

#[tokio::test]
async fn repo_anchor_prefers_deepest_subtree() {
    // 4 (E7): a subtree nested in a git root wins for cwds inside it (deepest anchor).
    let s = pg_store().await;
    let b = "/_test/anchor_sub";
    mk_anchor_folder(&s, &format!("{b}/outer"), "git", None).await;
    let sub = mk_anchor_folder(&s, &format!("{b}/outer/vendor/sub"), "subtree", None).await;
    let a = s.resolve_repo_anchor(&format!("{b}/outer/vendor/sub/y.rs")).await.unwrap().unwrap();
    assert_eq!(a.repo_folder_id, sub, "deepest anchor (the subtree) beats the outer git root");
}

#[tokio::test]
async fn repo_anchor_standalone_project_root_anchors_but_bare_does_not() {
    // D1/E8: a standalone dir anchors only when it is a tracked project root (project_id set).
    let s = pg_store().await;
    let b = "/_test/anchor_std";
    let pid = s.create_project("_test:anchor_std", None, None).await.unwrap();
    let proj = mk_anchor_folder(&s, &format!("{b}/proj"), "standalone", Some(pid)).await;
    mk_anchor_folder(&s, &format!("{b}/bare"), "standalone", None).await; // not a project root
    assert_eq!(
        s.resolve_repo_anchor(&format!("{b}/proj/file")).await.unwrap().unwrap().repo_folder_id,
        proj,
        "a standalone project root anchors"
    );
    assert!(
        s.resolve_repo_anchor(&format!("{b}/bare/file")).await.unwrap().is_none(),
        "a project-less standalone dir is NOT an anchor"
    );
}

#[tokio::test]
async fn repo_anchor_follows_alias_after_move() {
    // 6 (E3/E4): a path recorded under a repo's OLD (moved) location resolves to the current repo.
    let s = pg_store().await;
    let b = "/_test/anchor_alias";
    let repo = mk_anchor_folder(&s, &format!("{b}/new-home/repo"), "git", None).await;
    s.add_folder_path_alias(&format!("{b}/old-home/repo"), &repo, "detected").await.unwrap();
    let a = s.resolve_repo_anchor(&format!("{b}/old-home/repo/src/x.ts")).await.unwrap().unwrap();
    assert_eq!(a.repo_folder_id, repo, "old path resolves to the current repo via alias");
    assert_eq!(a.matched_via, "alias");
}

#[tokio::test]
async fn repo_anchor_none_for_foreign_and_container() {
    // 7,8 (E5): a foreign cwd and a container dir above repos both resolve to None (no phantom).
    let s = pg_store().await;
    let b = "/_test/anchor_none";
    mk_anchor_folder(&s, &format!("{b}/holder/repo"), "git", None).await;
    assert!(
        s.resolve_repo_anchor("/tmp/_test_foreign_zzz").await.unwrap().is_none(),
        "a foreign cwd never fabricates a repo"
    );
    assert!(
        s.resolve_repo_anchor(&format!("{b}/holder")).await.unwrap().is_none(),
        "a container dir above a repo is not itself an anchor"
    );
}

#[tokio::test]
async fn repo_anchor_live_beats_equal_length_alias() {
    // 9: determinism — when a live abs_path and an alias of equal length both match, live wins.
    let s = pg_store().await;
    let b = "/_test/anchor_dup";
    let live = mk_anchor_folder(&s, &format!("{b}/x"), "git", None).await;
    let other = mk_anchor_folder(&s, &format!("{b}/other"), "git", None).await;
    s.add_folder_path_alias(&format!("{b}/x"), &other, "rename").await.unwrap(); // alias collides with a live path
    let a = s.resolve_repo_anchor(&format!("{b}/x")).await.unwrap().unwrap();
    assert_eq!(a.repo_folder_id, live, "a live abs_path beats an equal-length alias");
    assert_eq!(a.matched_via, "live");
}

#[tokio::test]
async fn session_anchors_to_repo_not_the_cwd_subfolder() {
    // 17: a hook event whose cwd is deep in a repo anchors the session to the REPO
    // (repo_folder_id), while folder_id keeps the raw cwd folder for provenance.
    let s = pg_store().await;
    let b = "/_test/p1_anchor";
    let repo = mk_anchor_folder(&s, &format!("{b}/repo"), "git", None).await;
    let sub = mk_anchor_folder(&s, &format!("{b}/repo/src"), "folder", None).await;
    let sess = "_test-p1-anchor";
    clear_test_session(&s, sess).await;
    s.record_session_event(sess, &sub, None, "claude", false).await.unwrap();
    let (folder_id, repo_folder_id): (Option<uuid::Uuid>, Option<uuid::Uuid>) =
        sqlx_core::query_as::query_as(
            "SELECT folder_id, repo_folder_id FROM activity.sessions WHERE client_session_id = $1",
        )
        .bind(sess)
        .fetch_one(&s.pool)
        .await
        .unwrap();
    assert_eq!(folder_id, Some(sub), "folder_id keeps the raw cwd folder (provenance)");
    assert_eq!(
        repo_folder_id,
        Some(repo),
        "repo_folder_id anchors to the enclosing repo, not the subfolder"
    );
}

#[tokio::test]
async fn folder_prune_nulls_folder_id_but_session_and_repo_survive() {
    // 18 (I2): pruning the raw cwd folder SET-NULLs folder_id but never deletes the
    // session; the repo anchor survives.
    let s = pg_store().await;
    let b = "/_test/p1_prune";
    let repo = mk_anchor_folder(&s, &format!("{b}/repo"), "git", None).await;
    let sub = mk_anchor_folder(&s, &format!("{b}/repo/src"), "folder", None).await;
    let sess = "_test-p1-prune";
    clear_test_session(&s, sess).await;
    s.record_session_event(sess, &sub, None, "claude", false).await.unwrap();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(sub)
        .execute(&s.pool)
        .await
        .unwrap();
    let row: Option<(Option<uuid::Uuid>, Option<uuid::Uuid>)> = sqlx_core::query_as::query_as(
        "SELECT folder_id, repo_folder_id FROM activity.sessions WHERE client_session_id = $1",
    )
    .bind(sess)
    .fetch_optional(&s.pool)
    .await
    .unwrap();
    let (folder_id, repo_folder_id) =
        row.expect("session survives the folder prune (no cascade delete)");
    assert_eq!(folder_id, None, "the pruned raw folder SET-NULLs folder_id");
    assert_eq!(repo_folder_id, Some(repo), "the repo anchor survives the prune");
}

async fn set_folder_remotes(s: &PgStore, id: &uuid::Uuid, urls: &[&str]) {
    let json = serde_json::Value::Array(
        urls.iter().map(|u| serde_json::json!({"name": "origin", "url": u})).collect(),
    );
    sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
        .bind(id)
        .bind(&json)
        .execute(&s.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn find_live_root_by_remote_matches_only_a_live_path_sharing_a_url() {
    let s = pg_store().await;
    let url = "git@github.com:sensei-hq/remote-probe.git";
    let live = create_test_folder(&s, "remote-live").await; // /_test/remote-live
    set_folder_remotes(&s, &live, &[url]).await;
    let live_abs = vec!["/_test/remote-live".to_string()];

    assert_eq!(
        s.find_live_root_by_remote(&[url.to_string()], &live_abs).await.unwrap(),
        Some(live),
        "a live root sharing the git remote is the remap target"
    );
    assert_eq!(
        s.find_live_root_by_remote(&["git@github.com:other/x.git".to_string()], &live_abs)
            .await
            .unwrap(),
        None,
        "a non-matching remote finds nothing"
    );
    assert_eq!(
        s.find_live_root_by_remote(&[], &live_abs).await.unwrap(),
        None,
        "no remote to match on → None (a remote-less folder can't be remapped)"
    );
    assert_eq!(
        s.find_live_root_by_remote(&[url.to_string()], &["/_test/not-live".to_string()])
            .await
            .unwrap(),
        None,
        "the matching folder is not in the live set → not a remap target"
    );
}

#[tokio::test]
async fn folder_has_sessions_reflects_attached_history() {
    let s = pg_store().await;
    let sess = "_test-hasshist-session";
    clear_test_session(&s, sess).await;
    let fid = create_test_folder(&s, "hashist").await;
    assert!(!s.folder_has_sessions(&fid).await.unwrap(), "no sessions yet");
    s.record_session_event(sess, &fid, None, "claude", true).await.unwrap();
    assert!(s.folder_has_sessions(&fid).await.unwrap(), "session attached → has history");
    // The archive gate keys on the durable anchor too (P2): null the raw folder (as a
    // prune would) and the repo still reports history via repo_folder_id — so a
    // history-bearing repo is archived, never hard-deleted.
    sqlx_core::query::query(
        "UPDATE activity.sessions SET folder_id = NULL WHERE client_session_id = $1",
    )
    .bind(sess)
    .execute(&s.pool)
    .await
    .unwrap();
    assert!(
        s.folder_has_sessions(&fid).await.unwrap(),
        "history still detected via repo_folder_id after the raw folder is pruned"
    );
}

#[tokio::test]
async fn remap_folder_moves_sessions_aliases_old_path_and_drops_old_row() {
    let s = pg_store().await;
    let sess = "_test-remap-session";
    clear_test_session(&s, sess).await;
    let old = create_test_folder(&s, "remap-old").await; // /_test/remap-old
    let new = create_test_folder(&s, "remap-new").await; // /_test/remap-new
    s.record_session_event(sess, &old, None, "claude", true).await.unwrap();

    s.remap_folder(&old, "/_test/remap-old", &new).await.unwrap();

    assert_eq!(session_row_folder(&s, sess).await, Some(new), "history moved onto the new folder");
    let (repo_anchor,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
        "SELECT repo_folder_id FROM activity.sessions WHERE client_session_id = $1",
    )
    .bind(sess)
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert_eq!(
        repo_anchor,
        Some(new),
        "the repo anchor is repointed to the moved folder, not nulled by the old-row delete (P2)"
    );
    assert_eq!(
        s.find_folder_for_path("/_test/remap-old").await.unwrap().map(|(f, _)| f),
        Some(new),
        "the old path now aliases forward to the new folder"
    );
    let old_still_there: Option<(uuid::Uuid,)> =
        sqlx_core::query_as::query_as("SELECT id FROM sensei.folders WHERE id = $1")
            .bind(old)
            .fetch_optional(&s.pool)
            .await
            .unwrap();
    assert!(old_still_there.is_none(), "the old husk row is dropped");
}

#[tokio::test]
async fn archive_folder_sets_archived_status() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "to-archive").await;
    s.archive_folder(&fid).await.unwrap();
    let (status,): (String,) =
        sqlx_core::query_as::query_as("SELECT status::text FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .fetch_one(&s.pool)
            .await
            .unwrap();
    assert_eq!(status, "archived", "the vanished history-bearing root is retained as archived");
}

#[tokio::test]
async fn update_folder_remotes_populates_and_is_matchable() {
    let s = pg_store().await;
    let url = "git@github.com:sensei-hq/populate-probe.git";
    let fid = create_test_folder(&s, "populate-remotes").await; // /_test/populate-remotes
    s.update_folder_remotes(&fid, &serde_json::json!([{"name":"origin","url":url}])).await.unwrap();
    // Round-trips into remote_urls AND is now findable as a live-root remote match.
    assert_eq!(
        s.find_live_root_by_remote(&[url.to_string()], &["/_test/populate-remotes".to_string()])
            .await
            .unwrap(),
        Some(fid),
        "the written remote is what makes auto-remap able to fire"
    );
}

#[tokio::test]
async fn replace_library_capabilities_is_manifest_authoritative() {
    use crate::libraries::manifest::{ProvidedAgent, ProvidedSkill};
    let s = pg_store().await;
    let lib = format!("_testlib_{}", uuid::Uuid::new_v4());
    let lid = s.upsert_library(&lib, "npm", Some(">=1.0"), None, None, None).await.unwrap();
    let sk = |n: &str, f: &str| ProvidedSkill {
        name: n.into(),
        focus: f.into(),
        path: Some(format!("p/{n}.md")),
        body: Some(format!("# {n}")),
    };
    let ag = ProvidedAgent {
        name: "rev".into(),
        focus: "review".into(),
        path: Some("a/rev.md".into()),
        body: Some("# rev".into()),
    };

    let (ns, na) = s
        .replace_library_capabilities(
            &lid,
            "manifest",
            Some(">=1.0"),
            &[sk("styling", "styling"), sk("a11y", "accessibility")],
            &[ag],
        )
        .await
        .unwrap();
    assert_eq!((ns, na), (2, 1));
    assert_eq!(s.list_library_skills(&lib).await.unwrap().len(), 2);
    assert_eq!(s.list_library_agents(&lib).await.unwrap().len(), 1);
    assert!(s.get_library_skill(&lib, "styling").await.unwrap().is_some(), "focus lookup finds it");
    assert!(
        s.get_library_skill(&lib, "nope").await.unwrap().is_none(),
        "genuine miss → None (not an error)"
    );

    // Re-ingest a manifest that now declares only 1 skill → the removed one disappears.
    let (ns2, _) = s
        .replace_library_capabilities(
            &lid,
            "manifest",
            Some(">=1.0"),
            &[sk("styling", "styling")],
            &[],
        )
        .await
        .unwrap();
    assert_eq!(ns2, 1);
    assert_eq!(
        s.list_library_skills(&lib).await.unwrap().len(),
        1,
        "the dropped skill is gone (manifest-authoritative)"
    );
    assert_eq!(s.list_library_agents(&lib).await.unwrap().len(), 0);

    // A path/body-less entry is not persisted (no fabricated body).
    let bodyless = ProvidedSkill { name: "x".into(), focus: "x".into(), path: None, body: None };
    let (ns3, _) = s
        .replace_library_capabilities(
            &lid,
            "manifest",
            None,
            &[sk("styling", "styling"), bodyless],
            &[],
        )
        .await
        .unwrap();
    assert_eq!(ns3, 1, "the body-less entry is skipped");
}

#[tokio::test]
async fn list_project_library_capabilities_suggests_from_a_projects_deps() {
    use crate::libraries::manifest::ProvidedSkill;
    let s = pg_store().await;
    let pid =
        s.create_project(&format!("_libcap_{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
    let lib = format!("_libcapdep_{}", uuid::Uuid::new_v4());
    let lid = s.upsert_library(&lib, "npm", Some("1"), None, None, None).await.unwrap();
    s.replace_library_capabilities(
        &lid,
        "manifest",
        Some("1"),
        &[ProvidedSkill {
            name: "semantic-styles-rokkit".into(),
            focus: "styling".into(),
            path: Some("p".into()),
            body: Some("b".into()),
        }],
        &[],
    )
    .await
    .unwrap();
    // The project depends on the library.
    s.execute_raw(&format!(
            "INSERT INTO sensei.project_libraries(library_id, project_id, enabled) VALUES('{lid}','{pid}',true) ON CONFLICT DO NOTHING"
        )).await.unwrap();

    let caps = s.list_project_library_capabilities(&pid).await.unwrap();
    let skills = caps["suggested_skills"].as_array().unwrap();
    assert!(
        skills
            .iter()
            .any(|x| x["name"] == "semantic-styles-rokkit" && x["library"] == lib.as_str()),
        "the project's dependency contributes its skill: {caps:?}"
    );
}

#[tokio::test]
async fn folder_id_by_abs_path_is_exact_and_never_follows_aliases() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "exact-path").await; // /_test/exact-path
    s.add_folder_path_alias("/_test/exact-old", &fid, "rename").await.unwrap();
    assert_eq!(
        s.folder_id_by_abs_path("/_test/exact-path").await.unwrap(),
        Some(fid),
        "exact abs_path resolves to the folder"
    );
    assert_eq!(
        s.folder_id_by_abs_path("/_test/exact-old").await.unwrap(),
        None,
        "an aliased path is NOT a real folder row — exact lookup returns None"
    );
    assert_eq!(s.folder_id_by_abs_path("/_test/never").await.unwrap(), None);
}

#[tokio::test]
async fn folder_path_alias_resolves_old_paths_after_a_rename() {
    // A renamed repo: the folder now lives at the new abs_path, and its OLD
    // path is registered as an alias. Transcripts/hooks recorded under the old
    // path (and its subdirs) must still resolve to the folder + project.
    let s = pg_store().await;
    let fid = create_test_folder(&s, "alias-new").await; // abs_path /_test/alias-new
    let old = "/_test/alias-old";
    s.add_folder_path_alias(old, &fid, "rename").await.unwrap();
    // exact-match resolver (transcript synthesis) resolves the old path via alias.
    assert_eq!(
        s.get_folder_ids_by_path(old).await.unwrap().map(|(id, _)| id),
        Some(fid),
        "old exact path resolves via alias"
    );
    // ancestor resolver (hooks / synth fallback) resolves an old SUBDIR via alias.
    assert_eq!(
        s.find_folder_for_path("/_test/alias-old/docs/mockups").await.unwrap().map(|(id, _)| id),
        Some(fid),
        "old subdir resolves to the folder via the alias ancestor"
    );
    // the current path still resolves (live abs_path unaffected).
    assert_eq!(
        s.get_folder_ids_by_path("/_test/alias-new").await.unwrap().map(|(id, _)| id),
        Some(fid),
        "current path still resolves"
    );
    // idempotent re-register.
    s.add_folder_path_alias(old, &fid, "detected").await.unwrap();
    assert_eq!(s.get_folder_ids_by_path(old).await.unwrap().map(|(id, _)| id), Some(fid));
}

#[tokio::test]
async fn repo_root_for_path_resolves_nearest_git_ancestor_skipping_members() {
    // The watcher resolver: a change under a repo → the repo ROOT (git/
    // standalone), skipping structural workspace_member subdirs so the
    // one-owner repo wins.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root = format!("/_test/reporoot/{uniq}");
    let root_id = s.add_watch_root(&root, "rr", &serde_json::json!([])).await.unwrap();
    let repo = format!("{root}/mono");
    let repo_fid = s.upsert_repo_kind(&root_id, "git", "mono", &repo).await.unwrap();
    // A workspace member subdir (not an index owner) — must be skipped.
    let member = format!("{repo}/packages/chart");
    s.upsert_subfolder(&root_id, "chart", "mono/packages/chart", &member, Some(&repo_fid), None)
        .await
        .ok();

    let got = s.repo_root_for_path(&format!("{member}/src/x.ts")).await.unwrap();
    assert_eq!(
        got.map(|(p, _)| p),
        Some(repo.clone()),
        "deep file resolves to the git repo root, not the member subdir"
    );
    assert_eq!(
        s.repo_root_for_path(&repo).await.unwrap().map(|(p, _)| p),
        Some(repo),
        "exact repo path resolves too"
    );
    assert_eq!(
        s.repo_root_for_path(&format!("/_test/nope-{uniq}/x")).await.unwrap(),
        None,
        "path under no repo → None"
    );

    let pool = s.pool();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id=$1")
        .bind(root_id)
        .execute(pool)
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1")
        .bind(root_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn scope_repo_roots_returns_repo_roots_not_structural_subfolders() {
    // Content grep walks repo ROOTS (git/subtree/standalone), never the
    // structural `folder` subdirs the index also tracks.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root = format!("/_test/scoperoots/{uniq}");
    let root_id = s.add_watch_root(&root, "sr", &serde_json::json!([])).await.unwrap();
    let repo = format!("{root}/app");
    let repo_fid = s.upsert_repo_kind(&root_id, "git", "app", &repo).await.unwrap();
    // A structural subfolder (kind='folder') under the repo — NOT a repo root.
    let comp = format!("{repo}/src/lib");
    let comp_fid = s
        .upsert_subfolder(&root_id, "lib", "app/src/lib", &comp, Some(&repo_fid), None)
        .await
        .unwrap();

    let roots = s.scope_repo_roots(&[repo_fid, comp_fid]).await.unwrap();
    assert!(roots.contains(&repo), "the git repo root is returned");
    assert!(!roots.contains(&comp), "a structural (kind='folder') subdir is not a repo root");
    assert!(s.scope_repo_roots(&[]).await.unwrap().is_empty(), "empty scope → no roots");

    let pool = s.pool();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id=$1")
        .bind(root_id)
        .execute(pool)
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1")
        .bind(root_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn record_symbol_names_is_monotonic_history() {
    // The symbol-history registry backing doc-drift: a current symbol is
    // recorded, and a REMOVED symbol stays recorded (monotonic) so a later
    // scan can still tell it was once real (→ its stale doc refs are drift).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("symhist_{}", uuid::Uuid::new_v4())).await;
    let uniq = format!("SymHist_{}", uuid::Uuid::new_v4().simple());

    let nid =
        s.upsert_node(&fid, "function", &uniq, "x.rs", None, None, Some(1), Some(2)).await.unwrap();
    s.record_symbol_names().await.unwrap();
    let present: Option<(String,)> =
        sqlx_core::query_as::query_as("SELECT name FROM sensei.symbol_names WHERE name = $1")
            .bind(&uniq)
            .fetch_optional(s.pool())
            .await
            .unwrap();
    assert!(present.is_some(), "a current symbol name is recorded");

    // Remove the symbol and re-record — the name must persist.
    sqlx_core::query::query("DELETE FROM sensei.nodes WHERE id = $1")
        .bind(nid)
        .execute(s.pool())
        .await
        .unwrap();
    s.record_symbol_names().await.unwrap();
    let still: Option<(String,)> =
        sqlx_core::query_as::query_as("SELECT name FROM sensei.symbol_names WHERE name = $1")
            .bind(&uniq)
            .fetch_optional(s.pool())
            .await
            .unwrap();
    assert!(still.is_some(), "a removed symbol stays in the registry (monotonic history)");

    sqlx_core::query::query("DELETE FROM sensei.symbol_names WHERE name = $1")
        .bind(&uniq)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(fid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn get_project_commands_marks_and_ranks_the_preferred_tool() {
    // G10: when several commands share a category, the user's dojo_preference
    // marks one preferred and ranks it first.
    let s = pg_store().await;
    let pid =
        s.create_project(&format!("_test:g10-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
    let fid = create_test_folder(&s, &format!("g10_{}", uuid::Uuid::new_v4())).await;
    s.set_folder_project(&fid, &pid, "primary", None).await.unwrap();
    // Two `test` commands; alphabetical order is jest, then vitest.
    sqlx_core::query::query(
            "INSERT INTO sensei.project_commands (folder_id, raw_name, command_line, category, ecosystem)
             VALUES ($1, 'jest', 'jest', 'test', 'npm'), ($1, 'vitest', 'vitest run', 'test', 'npm')",
        ).bind(fid).execute(s.pool()).await.unwrap();
    // Clean slate for the shared preference row.
    sqlx_core::query::query(
        "DELETE FROM sensei.dojo_preferences WHERE scope='user' AND capability='test'",
    )
    .execute(s.pool())
    .await
    .ok();

    // No preference → alphabetical, nothing marked preferred.
    let before = s.get_project_commands(&pid, Some("test")).await.unwrap();
    assert_eq!(before.len(), 2);
    assert_eq!(before[0]["raw_name"], "jest");
    assert!(
        before.iter().all(|c| c["preferred"] == serde_json::json!(false)),
        "no preference → none preferred"
    );

    // Prefer vitest → it is marked and ranked first (ahead of the alphabetically-first jest).
    s.upsert_command_preference("user", "test", "vitest", None).await.unwrap();
    let after = s.get_project_commands(&pid, Some("test")).await.unwrap();
    assert_eq!(after[0]["raw_name"], "vitest", "preferred ranked first");
    assert_eq!(after[0]["preferred"], serde_json::json!(true));
    assert_eq!(after[1]["raw_name"], "jest");
    assert_eq!(
        after[1]["preferred"],
        serde_json::json!(false),
        "the non-preferred sibling isn't marked"
    );

    let pool = s.pool();
    sqlx_core::query::query("DELETE FROM sensei.project_commands WHERE folder_id=$1")
        .bind(fid)
        .execute(pool)
        .await
        .ok();
    sqlx_core::query::query(
        "DELETE FROM sensei.dojo_preferences WHERE scope='user' AND capability='test'",
    )
    .execute(pool)
    .await
    .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id=$1")
        .bind(fid)
        .execute(pool)
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id=$1")
        .bind(pid)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn record_session_event_folds_into_one_row_and_completes() {
    // #31: every hook event of a session folds into one row keyed by the
    // assistant session id; Stop/SessionEnd marks it completed.
    let s = pg_store().await;
    let fid = create_test_folder(&s, "sess-record").await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    let id1 = s.record_session_event(&sid, &fid, None, "claude", false).await.unwrap();
    let id2 = s.record_session_event(&sid, &fid, None, "claude", false).await.unwrap();
    assert_eq!(id1, id2, "same client_session_id must fold into one session row");
    assert!(
        s.get_session(&id1).await.unwrap().unwrap()["completed_at"].is_null(),
        "not completed before an end event"
    );
    let id3 = s.record_session_event(&sid, &fid, None, "claude", true).await.unwrap();
    assert_eq!(id3, id1, "end event updates the same row");
    assert!(
        !s.get_session(&id1).await.unwrap().unwrap()["completed_at"].is_null(),
        "Stop/SessionEnd sets completed_at"
    );
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(id1)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn list_all_sessions_joins_project_and_uses_camelcase_times() {
    // #61: the observatory reads project name + startedAt/completedAt. The
    // returned row must carry the joined project NAME (not a bare folder
    // uuid) under camelCase timestamp keys, with completedAt set once the
    // session ends — otherwise every displayed column renders blank.
    let s = pg_store().await;
    let proj_name = format!("_test:obs-{}", uuid::Uuid::new_v4());
    let pid = s.create_project(&proj_name, None, None).await.unwrap();
    let fid = create_test_folder(&s, "obs-sess").await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    let session_id = s.record_session_event(&sid, &fid, Some(&pid), "claude", false).await.unwrap();
    s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();

    let all = s.list_all_sessions(500, None, None).await.unwrap();
    let row = all
        .iter()
        .find(|r| r["id"].as_str() == Some(session_id.to_string().as_str()))
        .expect("our session is listed");

    assert_eq!(
        row["project"],
        serde_json::json!(proj_name),
        "project name is joined, not a folder uuid"
    );
    assert!(row["startedAt"].as_str().is_some(), "startedAt present (camelCase)");
    assert!(row.get("started_at").is_none(), "no stale snake_case started_at key");
    assert!(row["completedAt"].as_str().is_some(), "completedAt set after the end event");
    assert!(row.get("folder_id").is_none(), "folder_id no longer leaks in place of the project");

    // Token + duration + model surfaced (camelCase): a session with captured
    // usage returns real numbers; a session with none returns null (never a 0).
    sqlx_core::query::query(
            "UPDATE activity.sessions SET tokens_in=1500, tokens_out=400, \
                 duration=make_interval(secs => 1800), provider='anthropic', model='claude-opus-4-8' WHERE id=$1",
        ).bind(session_id).execute(s.pool()).await.unwrap();
    let all2 = s.list_all_sessions(500, None, None).await.unwrap();
    let row2 =
        all2.iter().find(|r| r["id"].as_str() == Some(session_id.to_string().as_str())).unwrap();
    assert_eq!(row2["tokensIn"].as_i64(), Some(1500), "tokensIn surfaced (camelCase)");
    assert_eq!(row2["tokensOut"].as_i64(), Some(400), "tokensOut surfaced");
    assert_eq!(
        row2["durationSecs"].as_f64(),
        Some(1800.0),
        "durationSecs = gap-aware active seconds"
    );
    assert_eq!(row2["model"].as_str(), Some("claude-opus-4-8"), "model surfaced");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(session_id)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn process_analysis_selects_unscored_then_saves_judgments_evidence_watermark() {
    // The process-quality store contract (spec 2026-08-20): a measurable
    // session with a client_session_id + NULL process_analyzed_at is selected;
    // save_session_process merges props.process, writes evidence rows keyed on
    // the client id, stamps the watermark; and a re-save overwrites in place.
    let s = pg_store().await;
    let suffix = format!("proc_{}", uuid::Uuid::new_v4());
    let (pid, fid) = create_test_project_and_folder(&s, &suffix).await;
    let csid = format!("{suffix}-csid");
    let sid = s.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();
    // Make it measurable (outcome not null) — the selection base.
    sqlx_core::query::query(
            "UPDATE activity.sessions SET outcome='completed'::sensei.session_outcome, ftr=true WHERE id=$1"
        ).bind(sid).execute(s.pool()).await.unwrap();

    // Selected while un-scored.
    let due = s.sessions_needing_process_analysis(&pid, 50).await.unwrap();
    assert!(
        due.iter().any(|(id, c)| *id == sid && c == &csid),
        "un-scored measurable session is selected"
    );

    // Save a judgment + one evidence row.
    let judgments = serde_json::json!({
        "spec_depth": {"score": 4, "evidence": [{"turn": 1, "quote": "the plan is X", "kind": "plan"}], "note": "clear"},
        "spec_deviation": {"score": null, "note": "no deviation"},
        "refuted_findings": {"score": null},
        "incomplete_analysis_llm": {"score": null},
    });
    let evidence =
        vec![("spec_depth".to_string(), 1, "the plan is X".to_string(), Some("plan".to_string()))];
    s.save_session_process(&sid, &csid, &judgments, &evidence).await.unwrap();

    // props.process merged + watermark stamped.
    let (proc, stamped): (serde_json::Value, bool) = sqlx_core::query_as::query_as(
            "SELECT props->'process', process_analyzed_at IS NOT NULL FROM activity.sessions WHERE id=$1"
        ).bind(sid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(
        proc["spec_depth"]["score"].as_f64(),
        Some(4.0),
        "judgment stored under props.process"
    );
    assert!(stamped, "process_analyzed_at watermark stamped");

    // Evidence row keyed on the client id.
    let (ev_n,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM activity.session_process_evidence WHERE session_id=$1 AND signal='spec_depth' AND turn_index=1"
        ).bind(&csid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(ev_n, 1, "one grounded evidence row written");

    // No longer selected (watermarked).
    let due2 = s.sessions_needing_process_analysis(&pid, 50).await.unwrap();
    assert!(!due2.iter().any(|(id, _)| *id == sid), "scored session no longer selected");

    // Re-save overwrites evidence in place (no duplicates).
    s.save_session_process(&sid, &csid, &judgments, &evidence).await.unwrap();
    let (ev_after,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM activity.session_process_evidence WHERE session_id=$1",
    )
    .bind(&csid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(ev_after, 1, "re-save replaces evidence in place (idempotent), not appends");

    // cleanup
    sqlx_core::query::query("DELETE FROM activity.session_process_evidence WHERE session_id=$1")
        .bind(&csid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id=$1")
        .bind(sid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id=$1")
        .bind(fid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id=$1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn set_session_summary_refreshes_when_changed() {
    // The retro producer REFRESHES a session's summary when it changes — so a
    // re-derivation (the transcript backfill) corrects a now-stale line, e.g.
    // an outcome that flipped abandoned → completed. An identical write is a
    // guarded no-op.
    let s = pg_store().await;
    let pid =
        s.create_project(&format!("_test:sum-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
    let fid = create_test_folder(&s, &format!("sum-{}", uuid::Uuid::new_v4())).await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    let session_id = s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();

    // Fresh session → summary NULL → the write persists (1 row).
    let n1 =
        s.set_session_summary(&session_id, "touched 2 files; outcome abandoned").await.unwrap();
    assert_eq!(n1, 1, "first write persists");

    // A CHANGED summary is refreshed (the backfill correcting a stale line).
    let n2 =
        s.set_session_summary(&session_id, "touched 2 files; outcome completed").await.unwrap();
    assert_eq!(n2, 1, "a changed summary is refreshed, not preserved");
    let cur: (Option<String>,) =
        sqlx_core::query_as::query_as("SELECT summary FROM activity.sessions WHERE id = $1")
            .bind(session_id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(
        cur.0.as_deref(),
        Some("touched 2 files; outcome completed"),
        "stale summary corrected"
    );

    // An identical write is a guarded no-op (0 rows).
    let n3 =
        s.set_session_summary(&session_id, "touched 2 files; outcome completed").await.unwrap();
    assert_eq!(n3, 0, "unchanged summary is a no-op");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(session_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn get_project_repos_excludes_subfolder_tree() {
    // #62: a single-repo project with subfolders must list only its repo
    // root(s), never the kind='folder' subfolder tree — else the UI shows it
    // as a multi-repo project with every folder as a repo.
    let s = pg_store().await;
    let pid = s
        .create_project(&format!("_test:repos-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
    let git_abs = format!("/_test/repos-git-{}", uuid::Uuid::new_v4());
    let sub_abs = format!("/_test/repos-sub-{}", uuid::Uuid::new_v4());
    let mem_abs = format!("/_test/repos-mem-{}", uuid::Uuid::new_v4());
    sqlx_core::query::query(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, project_id) VALUES
               ('00000000-0000-0000-0000-000000000001','git'::sensei.folder_kind,'the-repo','the-repo',$1,$3),
               ('00000000-0000-0000-0000-000000000001','folder'::sensei.folder_kind,'subdir','subdir',$2,$3),
               ('00000000-0000-0000-0000-000000000001','workspace_member'::sensei.folder_kind,'member','member',$4,$3)"
        ).bind(&git_abs).bind(&sub_abs).bind(pid).bind(&mem_abs).execute(s.pool()).await.unwrap();

    let repos = s.get_project_repos(&pid).await.unwrap();
    let kinds: Vec<String> =
        repos.iter().filter_map(|r| r["kind"].as_str().map(str::to_string)).collect();
    assert!(kinds.iter().any(|k| k == "git"), "the repo root is listed: {kinds:?}");
    assert!(!kinds.iter().any(|k| k == "folder"), "kind=folder subfolders excluded: {kinds:?}");
    // D5a: monorepo members are the structural tree, NOT separate repos — else
    // a monorepo with N members regresses to an N+1-repo project (#62).
    assert!(
        !kinds.iter().any(|k| k == "workspace_member"),
        "kind=workspace_member excluded from repos: {kinds:?}"
    );

    sqlx_core::query::query("DELETE FROM sensei.folders WHERE project_id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn upsert_subfolder_kind_relabels_structural_but_preserves_root() {
    // D5a: upsert_subfolder_kind relabels between the two STRUCTURAL kinds
    // (folder ↔ workspace_member) on conflict, but NEVER reclassifies a path
    // that is actually a nested project ROOT (git/standalone/subtree).
    let s = pg_store().await;
    s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
    let rid = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let kind_at = |s: &PgStore, abs: String| {
        let pool = s.pool().clone();
        async move {
            let (k,): (String,) =
                query_as("SELECT kind::text FROM sensei.folders WHERE abs_path=$1")
                    .bind(&abs)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            k
        }
    };

    // A plain structural folder → relabel to workspace_member on re-upsert.
    let a = format!("/_test/sfk-a-{}", uuid::Uuid::new_v4());
    s.upsert_subfolder(&rid, "a", "a", &a, None, None).await.unwrap();
    assert_eq!(kind_at(&s, a.clone()).await, "folder", "first upsert is a plain folder");
    s.upsert_subfolder_kind(&rid, "workspace_member", "a", "a", &a, None, None).await.unwrap();
    assert_eq!(
        kind_at(&s, a.clone()).await,
        "workspace_member",
        "relabelled folder → workspace_member"
    );

    // A nested project root (subtree) must NOT be reclassified by a member upsert.
    let b = format!("/_test/sfk-b-{}", uuid::Uuid::new_v4());
    s.upsert_repo_kind(&rid, "subtree", "b", &b).await.unwrap();
    s.upsert_subfolder_kind(&rid, "workspace_member", "b", "b", &b, None, None).await.unwrap();
    assert_eq!(
        kind_at(&s, b.clone()).await,
        "subtree",
        "a nested root is preserved, never reclassified"
    );

    sqlx_core::query::query("DELETE FROM sensei.folders WHERE abs_path IN ($1,$2)")
        .bind(&a)
        .bind(&b)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn projects_with_session_activity_reports_the_project() {
    // #67: the scheduler reads (project_id, latest activity) to decide what
    // to re-analyze. A project with attributed sessions must appear.
    let s = pg_store().await;
    let pid =
        s.create_project(&format!("_test:act-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
    let fid = create_test_folder(&s, &format!("act-{}", uuid::Uuid::new_v4())).await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();

    let activity = s.get_projects_with_session_activity().await.unwrap();
    let row =
        activity.iter().find(|(p, _)| *p == pid).expect("project appears in session-activity");
    assert!(row.1.timestamp() > 0, "carries a real latest-activity timestamp");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE project_id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn project_ftr_and_quality_decode_numeric_metrics() {
    // Regression: the headline (Σ props / value) and the daily AVG(...) trend /
    // ftr_7d / avg_duration_ms are all NUMERIC; without ::float8 casts sqlx
    // fails to decode into f64 and the endpoint 500s (masked by the client's
    // default-on-error). The project must have BOTH a stored `ftr` row (so the
    // headline decodes a real number, not a short-circuiting NULL) AND an
    // analyzed session in the window (so the inline trend decodes a numeric row).
    let s = pg_store().await;
    let pid =
        s.create_project(&format!("_test:ftr-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
    let fid = create_test_folder(&s, &format!("ftr-{}", uuid::Uuid::new_v4())).await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    let session_id = s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();
    s.update_session_metrics(
        &session_id,
        3,
        0,
        "completed",
        true,
        1000,
        None,
        &serde_json::json!({}),
    )
    .await
    .unwrap();
    // Stored daily ftr row in the 14d window → the headline decodes a real value.
    let (ftr_mid,): (uuid::Uuid,) = query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
        .fetch_one(s.pool())
        .await
        .unwrap();
    // Repo-grain (_v2): the shared `ftr` metric_id is unique per (metric, repo,
    // user, day), so seed a per-test repository — a NULL-repository row would
    // collide with a sibling project's `ftr` row on the same day.
    let (rid,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftr-decode') RETURNING id",
    )
    .bind(format!("test/ftr-decode-{}", uuid::Uuid::new_v4()))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_project(&s, &rid, &pid, "ftr-decode").await;
    s.upsert_project_metric_repo(
        &ftr_mid,
        &rid,
        "user",
        None,
        None,
        chrono::Utc::now().date_naive(),
        "daily",
        1.0,
        &serde_json::json!({"numerator": 1, "denominator": 1}),
        "measured",
    )
    .await
    .unwrap();

    let ftr = s.get_project_ftr(&pid).await.expect("get_project_ftr decodes numeric metrics");
    assert!(ftr["ftr14d"].as_f64().is_some(), "ftr14d decodes a real number from the stored row");
    assert!(
        ftr["ftrTrend"].as_array().is_some_and(|a| !a.is_empty()),
        "daily trend decodes a numeric row"
    );
    s.get_quality_signals(&pid).await.expect("get_quality_signals decodes numeric metrics");
    s.get_tool_usage_stats().await.expect("get_tool_usage_stats decodes numeric avg_duration_ms");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE project_id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn library_upsert_updates() {
    let s = pg_store().await;
    let id1 = s.upsert_library("_test:react", "npm", Some("18"), None, None, None).await.unwrap();
    let id2 = s
        .upsert_library("_test:react", "npm", Some("19"), Some("UI library"), None, None)
        .await
        .unwrap();
    assert_eq!(id1, id2);
    let lib = s.get_library(&id1).await.unwrap().unwrap();
    assert_eq!(lib["version"], "19");
    assert_eq!(lib["description"], "UI library");
    s.delete_library(&id1).await.unwrap();
}

#[tokio::test]
async fn library_list() {
    let s = pg_store().await;
    let id1 = s.upsert_library("_test:lib_a", "npm", None, None, None, None).await.unwrap();
    let id2 = s.upsert_library("_test:lib_b", "cargo", None, None, None, None).await.unwrap();
    let all = s.list_libraries().await.unwrap();
    assert!(all.iter().any(|l| l["name"] == "_test:lib_a"));
    assert!(all.iter().any(|l| l["name"] == "_test:lib_b"));
    s.delete_library(&id1).await.unwrap();
    s.delete_library(&id2).await.unwrap();
}

#[tokio::test]
async fn library_delete() {
    let s = pg_store().await;
    let id = s.upsert_library("_test:deleteme", "npm", None, None, None, None).await.unwrap();
    s.delete_library(&id).await.unwrap();
    assert!(s.get_library(&id).await.unwrap().is_none());
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

#[tokio::test]
async fn session_complete() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "sess_complete").await;
    let sid = s.create_session(&fid, "add feature", None).await.unwrap();
    s.complete_session(&sid, "completed", true, 5, 0, Some("shipped it"), Some(1200), Some(3400))
        .await
        .unwrap();
    let sess = s.get_session(&sid).await.unwrap().unwrap();
    assert_eq!(sess["outcome"], "completed");
    assert_eq!(sess["ftr"], true);
    assert_eq!(sess["turns"], 5);
    assert!(sess["completed_at"].as_str().is_some());
    // summary + tokens actually PERSIST (were previously advertised-but-dropped).
    let meta: (Option<String>, Option<i32>, Option<i32>) = sqlx_core::query_as::query_as(
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
        .insert_hook_event(
            &session_id,
            "claude",
            "PreToolUse",
            Some("Read"),
            Some("/tmp/test"),
            chrono::Utc::now().timestamp_millis(),
            None,
            &payload,
        )
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
        .insert_hook_event(
            &session_id,
            "claude",
            "PostToolUse",
            Some("Bash"),
            None,
            chrono::Utc::now().timestamp_millis(),
            Some(true),
            &payload,
        )
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
        .insert_hook_event(
            &session_id,
            "claude",
            "SessionStart",
            None,
            Some("/home/user/project"),
            chrono::Utc::now().timestamp_millis(),
            None,
            &payload,
        )
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
        .insert_hook_event(
            &session_id,
            "cursor",
            "SessionStart",
            None,
            Some("/home/user/project"),
            chrono::Utc::now().timestamp_millis(),
            None,
            &payload,
        )
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
    s.insert_hook_event(
        &pending_sid,
        "claude",
        "PostToolUse",
        Some("Read"),
        None,
        now,
        Some(true),
        &serde_json::json!({"tool_response": "x"}),
    )
    .await
    .unwrap();

    // (b) in-window PostToolUse that already carries a verdict row → excluded.
    let classified_sid = format!("_test-unclassified-classified-{}", uuid::Uuid::new_v4());
    let ev_id = s
        .insert_hook_event(
            &classified_sid,
            "claude",
            "PostToolUse",
            Some("Read"),
            None,
            now,
            Some(true),
            &serde_json::json!({"tool_response": "y"}),
        )
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
    s.insert_hook_event(
        &old_sid,
        "claude",
        "PostToolUse",
        Some("Read"),
        None,
        now - 30 * day_ms,
        Some(true),
        &serde_json::json!({"tool_response": "z"}),
    )
    .await
    .unwrap();

    let pending = s.unclassified_verdict_sessions(HEALTH_VERDICT_WINDOW_DAYS).await.unwrap();
    assert!(pending.contains(&pending_sid), "in-window unclassified session is pending");
    assert!(!pending.contains(&classified_sid), "already-classified session excluded");
    assert!(!pending.contains(&old_sid), "out-of-window session excluded");
}

// ── Projects tests ────────────────────────────────────────────────

#[tokio::test]
async fn project_create_and_get() {
    let s = pg_store().await;
    let id = s.create_project("_test:proj:create", Some("desc"), Some("client")).await.unwrap();
    let p = s.get_project(&id).await.unwrap().unwrap();
    assert_eq!(p["name"], "_test:proj:create");
    assert_eq!(p["description"], "desc");
    assert_eq!(p["client"], "client");
    assert_eq!(p["maturity"], "discovery"); // default
    s.delete_project(&id).await.unwrap();
}

#[tokio::test]
async fn project_list() {
    let s = pg_store().await;
    let id1 = s.create_project("_test:proj:list_a", None, None).await.unwrap();
    let id2 = s.create_project("_test:proj:list_b", None, None).await.unwrap();
    let all = s.list_projects().await.unwrap();
    let names: Vec<&str> = all.iter().filter_map(|p| p["name"].as_str()).collect();
    assert!(names.contains(&"_test:proj:list_a"));
    assert!(names.contains(&"_test:proj:list_b"));
    s.delete_project(&id1).await.unwrap();
    s.delete_project(&id2).await.unwrap();
}

#[tokio::test]
async fn list_projects_under_filters_by_folder_path_boundary() {
    // find_projects (MCP) needs a folder-scoped view: only projects whose
    // folders live under a given path. This pins the SQL boundary rule —
    // exact match + child match, but never a sibling that merely shares the
    // textual prefix (`/x` must not catch `/x-other`).
    let s = pg_store().await;
    let short = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
    let base = format!("/tmp/_test-fpu-{short}");
    let under = format!("{base}/x");
    let root =
        s.add_watch_root(&base, &format!("fpu-{short}"), &serde_json::json!([])).await.unwrap();

    // A: folder strictly beneath `under`.
    let a = s.ensure_test_project(&format!("fpu-a-{short}")).await.unwrap();
    s.upsert_folder(&root, "git", "a", "x/a", &format!("{under}/a"), None, Some(&a)).await.unwrap();
    // B: folder exactly equal to `under` (boundary: abs_path == under).
    let b = s.ensure_test_project(&format!("fpu-b-{short}")).await.unwrap();
    s.upsert_folder(&root, "git", "b", "x", &under, None, Some(&b)).await.unwrap();
    // C: folder elsewhere under base but outside `under`.
    let c = s.ensure_test_project(&format!("fpu-c-{short}")).await.unwrap();
    s.upsert_folder(&root, "git", "c", "elsewhere", &format!("{base}/elsewhere"), None, Some(&c))
        .await
        .unwrap();
    // D: sibling sharing the `under` prefix textually but across a path boundary.
    let d = s.ensure_test_project(&format!("fpu-d-{short}")).await.unwrap();
    s.upsert_folder(&root, "git", "d", "x-other", &format!("{under}-other/z"), None, Some(&d))
        .await
        .unwrap();

    let scoped: Vec<String> = s
        .list_projects_under(Some(&under))
        .await
        .unwrap()
        .iter()
        .filter_map(|p| p["id"].as_str().map(str::to_string))
        .collect();
    let has = |id: &uuid::Uuid| scoped.contains(&id.to_string());
    assert!(has(&a), "folder strictly under `under` must match");
    assert!(has(&b), "folder equal to `under` must match (boundary)");
    assert!(!has(&c), "folder outside `under` must NOT match");
    assert!(!has(&d), "sibling `{under}-other` must NOT match (path boundary, not raw prefix)");

    // No-filter returns everything — all four present.
    let all: Vec<String> = s
        .list_projects_under(None)
        .await
        .unwrap()
        .iter()
        .filter_map(|p| p["id"].as_str().map(str::to_string))
        .collect();
    for id in [&a, &b, &c, &d] {
        assert!(all.contains(&id.to_string()), "no-filter list must include every project");
    }
    // list_projects() (public no-arg) is equivalent to the None filter.
    let plain: Vec<String> = s
        .list_projects()
        .await
        .unwrap()
        .iter()
        .filter_map(|p| p["id"].as_str().map(str::to_string))
        .collect();
    assert!(
        plain.contains(&a.to_string()) && plain.contains(&c.to_string()),
        "list_projects() must stay unfiltered"
    );

    for id in [a, b, c, d] {
        s.delete_project(&id).await.unwrap();
    }
}

#[tokio::test]
async fn list_root_folders_excludes_nested_folder_descendants() {
    // find_projects (`?under=`) must return the COMPACT folder set — repo
    // roots only. The hundreds of nested `kind:'folder'` descendants are the
    // MCP token-cap bloat; list_root_folders_by_project drops them while
    // list_folders_by_project (app path) keeps the whole tree.
    let s = pg_store().await;
    let short = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
    let base = format!("/tmp/_test-rootf-{short}");
    let root =
        s.add_watch_root(&base, &format!("rootf-{short}"), &serde_json::json!([])).await.unwrap();
    let p = s.ensure_test_project(&format!("rootf-{short}")).await.unwrap();

    // One git repo root …
    s.upsert_folder(&root, "git", "repo", "repo", &format!("{base}/repo"), None, Some(&p))
        .await
        .unwrap();
    // … plus one standalone root …
    s.upsert_folder(&root, "standalone", "lib", "lib", &format!("{base}/lib"), None, Some(&p))
        .await
        .unwrap();
    // … plus many nested `kind:'folder'` descendants (the bloat).
    for i in 0..30 {
        s.upsert_folder(
            &root,
            "folder",
            &format!("d{i}"),
            &format!("repo/src/d{i}"),
            &format!("{base}/repo/src/d{i}"),
            None,
            Some(&p),
        )
        .await
        .unwrap();
    }

    let all = s.list_folders_by_project(&p).await.unwrap();
    assert_eq!(all.len(), 32, "full list keeps roots + all descendants");

    let roots = s.list_root_folders_by_project(&p).await.unwrap();
    assert_eq!(roots.len(), 2, "root list is repo roots only");
    assert!(
        roots.iter().all(|f| matches!(f["kind"].as_str(), Some("git") | Some("standalone"))),
        "root list must contain no `kind:'folder'` descendants",
    );
    // The repo root's abs_path (what cwd→project resolution needs) survives.
    assert!(roots.iter().any(|f| f["abs_path"] == format!("{base}/repo")));

    s.delete_project(&p).await.unwrap();
}

#[tokio::test]
async fn project_update() {
    let s = pg_store().await;
    let id = s.create_project("_test:proj:update", None, None).await.unwrap();
    s.update_project(
        &id,
        &ProjectPatch { name: Some("renamed"), maturity: Some("active"), ..Default::default() },
    )
    .await
    .unwrap();
    let p = s.get_project(&id).await.unwrap().unwrap();
    assert_eq!(p["name"], "renamed");
    assert_eq!(p["maturity"], "active");
    s.delete_project(&id).await.unwrap();
}

#[tokio::test]
async fn project_update_persists_widened_identity_fields() {
    // The About-edit form PUTs goal/icon/stack/links/client/preferred_acp
    // alongside name/maturity; all must round-trip through update_project.
    let s = pg_store().await;
    let id = s.create_project("_test:proj:widen", None, None).await.unwrap();
    let icon =
        serde_json::json!({"kind":"kanji","value":"識","bg":"var(--shu-soft)","fg":"var(--shu)"});
    let stack = serde_json::json!({"languages":["rust"],"frameworks":["axum"]});
    let links =
        serde_json::json!([{"id":"1","kind":"docs","label":"Docs","url":"https://example.com"}]);
    s.update_project(
        &id,
        &ProjectPatch {
            goal: Some("teach sensei"),
            client: Some("acme"),
            preferred_acp: Some("zed"),
            maturity: Some("active"),
            icon: Some(&icon),
            stack: Some(&stack),
            links: Some(&links),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let p = s.get_project(&id).await.unwrap().unwrap();
    assert_eq!(p["goal"], "teach sensei");
    assert_eq!(p["client"], "acme");
    assert_eq!(p["maturity"], "active");
    assert_eq!(p["icon"], icon, "icon jsonb must persist verbatim");
    assert_eq!(p["stack"], stack, "stack jsonb must persist verbatim");
    assert_eq!(p["links"], links, "links jsonb must persist verbatim");

    // preferred_acp isn't in get_project's projection — read it directly.
    let (acp,): (Option<String>,) =
        query_as("SELECT preferred_acp FROM sensei.projects WHERE id = $1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(acp.as_deref(), Some("zed"), "preferred_acp text must persist");

    s.delete_project(&id).await.unwrap();
}

#[tokio::test]
async fn set_project_icon_round_trips() {
    // The inferred-icon write path persists the jsonb verbatim, overwriting
    // the '{}' default the row was created with.
    let s = pg_store().await;
    let id = s.create_project("_test:proj:icon", None, None).await.unwrap();
    let icon = serde_json::json!({"kind":"kanji","value":"鉄","source":"kanji_map"});
    s.set_project_icon(&id, &icon).await.unwrap();
    let p = s.get_project(&id).await.unwrap().unwrap();
    assert_eq!(p["icon"], icon, "inferred icon jsonb must persist verbatim");
    s.delete_project(&id).await.unwrap();
}

#[tokio::test]
async fn project_update_rejects_unknown_maturity() {
    // maturity is the sensei.project_maturity enum — an unknown value must
    // be rejected (Err → 400 at the HTTP layer), never a raw cast 500, and
    // the row must be left untouched.
    let s = pg_store().await;
    let id = s.create_project("_test:proj:badmaturity", None, None).await.unwrap();
    let res = s
        .update_project(&id, &ProjectPatch { maturity: Some("spike"), ..Default::default() })
        .await;
    assert!(res.is_err(), "unknown maturity must be rejected");
    let p = s.get_project(&id).await.unwrap().unwrap();
    assert_eq!(p["maturity"], "discovery", "rejected update must not mutate the row");
    s.delete_project(&id).await.unwrap();
}

#[tokio::test]
async fn project_update_omitted_fields_unchanged() {
    // Partial-update (COALESCE) semantics: a patch that only sets `name`
    // must leave goal/client/description/maturity exactly as they were.
    let s = pg_store().await;
    let id = s
        .create_project("_test:proj:partial", Some("orig desc"), Some("orig client"))
        .await
        .unwrap();
    s.update_project(
        &id,
        &ProjectPatch { goal: Some("g1"), maturity: Some("active"), ..Default::default() },
    )
    .await
    .unwrap();
    s.update_project(&id, &ProjectPatch { name: Some("renamed2"), ..Default::default() })
        .await
        .unwrap();

    let p = s.get_project(&id).await.unwrap().unwrap();
    assert_eq!(p["name"], "renamed2");
    assert_eq!(p["goal"], "g1", "omitted goal must be unchanged");
    assert_eq!(p["client"], "orig client", "omitted client must be unchanged");
    assert_eq!(p["description"], "orig desc", "omitted description must be unchanged");
    assert_eq!(p["maturity"], "active", "omitted maturity must be unchanged");
    s.delete_project(&id).await.unwrap();
}

#[tokio::test]
async fn project_delete() {
    let s = pg_store().await;
    let id = s.create_project("_test:proj:delete", None, None).await.unwrap();
    s.delete_project(&id).await.unwrap();
    assert!(s.get_project(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn project_get_nonexistent() {
    let s = pg_store().await;
    let fake = uuid::Uuid::new_v4();
    assert!(s.get_project(&fake).await.unwrap().is_none());
}

// ── Name-duplicate phantom-project guard (creation + heal) ─────────

/// Guard A: the scan-time creation path is get-or-adopt, not
/// select-then-insert — a repeat call for the same name ADOPTS the existing
/// row instead of minting a second (the mechanism that produced the 0-folder
/// phantom).
#[tokio::test]
async fn get_or_create_project_by_name_is_idempotent() {
    let Ok(s) = PgStore::connect_test().await else {
        return;
    };
    let name = format!("_test:dupname:{}", uuid::Uuid::new_v4());

    let (id1, created1) = s.get_or_create_project_by_name(&name).await.unwrap();
    assert!(created1, "first call should mint the project");
    let (id2, created2) = s.get_or_create_project_by_name(&name).await.unwrap();
    assert!(!created2, "second call should adopt, not create");
    assert_eq!(id1, id2, "same name must resolve to the same project id");

    let (count,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.projects WHERE name = $1")
            .bind(&name)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(count, 1, "exactly one project row for the name");

    s.delete_project(&id1).await.ok();
}

/// Guard A: when a folder-bearing project of the name already exists, the
/// creation path adopts THAT one (not a fresh row) — no second "sensei".
#[tokio::test]
async fn get_or_create_adopts_folder_bearing_project_no_duplicate() {
    let Ok(s) = PgStore::connect_test().await else {
        return;
    };
    let name = format!("_test:dupname:{}", uuid::Uuid::new_v4());

    let keep = s.create_project(&name, None, None).await.unwrap();
    let fid = create_test_folder(&s, &format!("dupname-{}", uuid::Uuid::new_v4())).await;
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(keep)
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();

    let (resolved, created) = s.get_or_create_project_by_name(&name).await.unwrap();
    assert!(!created, "should adopt the existing folder-bearing project");
    assert_eq!(resolved, keep, "must resolve to the folder-bearing project");

    let (count,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.projects WHERE name = $1")
            .bind(&name)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(count, 1, "exactly one project row for the name");

    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(fid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_project(&keep).await.ok();
}

/// Guard B: a 0-folder discovery phantom sharing its name with a
/// folder-bearing project is pruned (merged into the survivor); its FK rows
/// (here a session) are reassigned, never orphaned; a re-run is a no-op.
#[tokio::test]
async fn heal_duplicate_name_projects_prunes_empty_dupe_idempotently() {
    // heal_duplicate_name_projects is a GLOBAL sweep — it prunes every
    // folderless duplicate in the table, not just this test's. Ungated, a
    // concurrent test's sweep runs inside this one's setup window.
    let _gate = REPAIR_SWEEP_GATE.enter();
    let Ok(s) = PgStore::connect_test().await else {
        return;
    };
    let name = format!("_test:dupheal:{}", uuid::Uuid::new_v4());

    // Survivor: folder-bearing project.
    let keep = s.create_project(&name, None, None).await.unwrap();
    let fid = create_test_folder(&s, &format!("dupheal-{}", uuid::Uuid::new_v4())).await;
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(keep)
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();

    // Phantom: second same-name project, 0 folders, maturity=discovery (default),
    // carrying a session so we can prove the heal reassigns FK rows.
    let phantom = s.create_project(&name, None, None).await.unwrap();
    let (sess,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions(folder_id, project_id, task) VALUES($1, $2, $3) RETURNING id"
        ).bind(fid).bind(phantom).bind("_test:dupheal-session").fetch_one(s.pool()).await.unwrap();

    s.heal_duplicate_name_projects().await.unwrap();

    // Phantom gone, survivor stays.
    let (phantom_exists,): (bool,) =
        sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.projects WHERE id = $1)")
            .bind(phantom)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(!phantom_exists, "empty phantom should be pruned");
    assert!(s.get_project(&keep).await.unwrap().is_some(), "folder-bearing survivor must remain");

    // Survivor still owns its folder; the phantom's session followed it.
    let (folder_project,): (Option<uuid::Uuid>,) =
        sqlx_core::query_as::query_as("SELECT project_id FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(folder_project, Some(keep), "survivor keeps its folder");
    let (sess_project,): (Option<uuid::Uuid>,) =
        sqlx_core::query_as::query_as("SELECT project_id FROM activity.sessions WHERE id = $1")
            .bind(sess)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(
        sess_project,
        Some(keep),
        "phantom's session must be reassigned to the survivor, not orphaned"
    );

    // Idempotent: after the heal exactly one project remains for the name and
    // a re-run leaves it untouched.
    s.heal_duplicate_name_projects().await.unwrap();
    let (count,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.projects WHERE name = $1")
            .bind(&name)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(count, 1, "exactly one project remains for the name; re-run is a no-op");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(sess)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(fid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_project(&keep).await.ok();
}

/// Guard B negative: two DIFFERENT repos (different paths) that share a name
/// are BOTH folder-bearing — legitimately distinct projects (identity =
/// path, not name) — and must NOT be merged.
#[tokio::test]
async fn heal_leaves_two_folder_bearing_same_name_projects() {
    // Gated for the same reason as the sweep tests above, and this one is the
    // proof: it failed once in CI with "second folder-bearing project must
    // survive". The window is between creating the second project and attaching
    // its folder — at that instant the project IS a folderless duplicate, so
    // another test's global sweep legitimately prunes it, and the assert then
    // fails describing a bug that does not exist.
    let _gate = REPAIR_SWEEP_GATE.enter();
    let Ok(s) = PgStore::connect_test().await else {
        return;
    };
    let name = format!("_test:dupneg:{}", uuid::Uuid::new_v4());

    let a = s.create_project(&name, None, None).await.unwrap();
    let fa = create_test_folder(&s, &format!("dupneg-a-{}", uuid::Uuid::new_v4())).await;
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(a)
        .bind(fa)
        .execute(s.pool())
        .await
        .unwrap();

    let b = s.create_project(&name, None, None).await.unwrap();
    let fb = create_test_folder(&s, &format!("dupneg-b-{}", uuid::Uuid::new_v4())).await;
    sqlx_core::query::query("UPDATE sensei.folders SET project_id = $1 WHERE id = $2")
        .bind(b)
        .bind(fb)
        .execute(s.pool())
        .await
        .unwrap();

    s.heal_duplicate_name_projects().await.unwrap();

    assert!(
        s.get_project(&a).await.unwrap().is_some(),
        "first folder-bearing project must survive"
    );
    assert!(
        s.get_project(&b).await.unwrap().is_some(),
        "second folder-bearing project must survive"
    );
    let (count,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.projects WHERE name = $1")
            .bind(&name)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(count, 2, "two folder-bearing same-name projects must both survive");

    for (p, f) in [(a, fa), (b, fb)] {
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(f)
            .execute(s.pool())
            .await
            .ok();
        s.delete_project(&p).await.ok();
    }
}

// ── Index Errors tests ───────────────────────────────────────────

#[tokio::test]
async fn idx_err_log_and_get() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "idx_err_log").await;
    s.clear_index_errors(&fid).await.unwrap(); // ensure clean
    s.log_index_error(&fid, "src/bad.ts", "SyntaxError", Some("typescript"), None).await.unwrap();
    s.log_index_error(&fid, "src/x.py", "IndentError", Some("python"), Some("parse"))
        .await
        .unwrap();
    let errors = s.get_index_errors(Some(&fid)).await.unwrap();
    assert_eq!(errors.len(), 2);
    s.clear_index_errors(&fid).await.unwrap();
}

#[tokio::test]
async fn idx_err_clear() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "idx_err_clear").await;
    s.clear_index_errors(&fid).await.unwrap();
    s.log_index_error(&fid, "a.rs", "err", Some("rust"), None).await.unwrap();
    s.clear_index_errors(&fid).await.unwrap();
    assert_eq!(s.get_index_errors(Some(&fid)).await.unwrap().len(), 0);
}

#[tokio::test]
async fn idx_err_empty() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "idx_err_empty").await;
    s.clear_index_errors(&fid).await.unwrap();
    assert_eq!(s.get_index_errors(Some(&fid)).await.unwrap().len(), 0);
}

// ── Workflow State tests ────────────────────────────────────────────

#[tokio::test]
async fn wf_upsert_and_get() {
    let s = pg_store().await;
    let p = "_test:wf:upsert";
    s.delete_workflow_state(p).await.unwrap();
    assert!(s.get_workflow_state(p).await.unwrap().is_none());
    s.upsert_workflow_state(p, Some("ideate"), None, None, None, None, None).await.unwrap();
    let state = s.get_workflow_state(p).await.unwrap().unwrap();
    assert_eq!(state["active_phase"], "ideate");
    assert!(state["active_task"].is_null());
    s.delete_workflow_state(p).await.unwrap();
}

#[tokio::test]
async fn wf_partial_update_preserves() {
    let s = pg_store().await;
    let p = "_test:wf:partial";
    s.delete_workflow_state(p).await.unwrap();
    s.upsert_workflow_state(
        p,
        Some("build"),
        Some("plan.md"),
        Some("task 1"),
        Some(42),
        None,
        Some("hash123"),
    )
    .await
    .unwrap();
    s.upsert_workflow_state(p, Some("validate"), None, None, None, None, None).await.unwrap();
    let state = s.get_workflow_state(p).await.unwrap().unwrap();
    assert_eq!(state["active_phase"], "validate");
    assert_eq!(state["active_plan"], "plan.md");
    assert_eq!(state["active_task"], "task 1");
    assert_eq!(state["active_issue"], 42);
    s.delete_workflow_state(p).await.unwrap();
}

#[tokio::test]
async fn wf_nonexistent_returns_none() {
    let s = pg_store().await;
    assert!(s.get_workflow_state("_test:wf:none").await.unwrap().is_none());
}

#[tokio::test]
async fn wf_delete() {
    let s = pg_store().await;
    let p = "_test:wf:delete";
    s.upsert_workflow_state(p, Some("ideate"), None, None, None, None, None).await.unwrap();
    s.delete_workflow_state(p).await.unwrap();
    assert!(s.get_workflow_state(p).await.unwrap().is_none());
}

// ── Tags tests ────────────────────────────────────────────────────

#[tokio::test]
async fn tag_add_and_list() {
    let s = pg_store().await;
    let tag = "_test:tag_add:rust";
    s.add_tag(tag, Some("stack")).await.unwrap();
    let tags = s.list_tags().await.unwrap();
    assert!(tags.iter().any(|(t, c)| t == tag && c.as_deref() == Some("stack")));
    s.remove_tag(tag).await.unwrap();
}

#[tokio::test]
async fn tag_add_without_category() {
    let s = pg_store().await;
    let tag = "_test:tag_nocat:misc";
    s.add_tag(tag, None).await.unwrap();
    let tags = s.list_tags().await.unwrap();
    assert!(tags.iter().any(|(t, c)| t == tag && c.is_none()));
    s.remove_tag(tag).await.unwrap();
}

#[tokio::test]
async fn tag_add_duplicate_is_upsert() {
    let s = pg_store().await;
    let tag = "_test:tag_dup:ts";
    s.add_tag(tag, Some("stack")).await.unwrap();
    s.add_tag(tag, Some("language")).await.unwrap(); // update category
    let tags = s.list_tags().await.unwrap();
    let found: Vec<_> = tags.iter().filter(|(t, _)| t == tag).collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.as_deref(), Some("language"));
    s.remove_tag(tag).await.unwrap();
}

#[tokio::test]
async fn tag_remove() {
    let s = pg_store().await;
    let tag = "_test:tag_rm:go";
    s.add_tag(tag, Some("stack")).await.unwrap();
    s.remove_tag(tag).await.unwrap();
    let tags = s.list_tags().await.unwrap();
    assert!(!tags.iter().any(|(t, _)| t == tag));
}

#[tokio::test]
async fn tag_remove_nonexistent_is_noop() {
    let s = pg_store().await;
    s.remove_tag("_test:tag_rm_noop:xyz").await.unwrap();
}

#[tokio::test]
async fn tag_list_by_category() {
    let s = pg_store().await;
    let t1 = "_test:tag_cat:rust";
    let t2 = "_test:tag_cat:ts";
    let t3 = "_test:tag_cat:active";
    s.add_tag(t1, Some("stack")).await.unwrap();
    s.add_tag(t2, Some("stack")).await.unwrap();
    s.add_tag(t3, Some("status")).await.unwrap();
    let stack_tags = s.list_tags_by_category("stack").await.unwrap();
    assert!(stack_tags.contains(&t1.to_string()));
    assert!(stack_tags.contains(&t2.to_string()));
    assert!(!stack_tags.contains(&t3.to_string()));
    s.remove_tag(t1).await.unwrap();
    s.remove_tag(t2).await.unwrap();
    s.remove_tag(t3).await.unwrap();
}

// ── Schema tests ─────────────────────────────────────────────────

#[tokio::test]
async fn memories_table_exists() {
    let store = PgStore::connect_test().await.unwrap();
    let row: (bool,) = query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = 'sensei' AND table_name = 'memories')"
        )
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(row.0, "sensei.memories table must exist — run `dbd apply` first");
}

// ── Knowledge Sources tests ───────────────────────────────────────

#[tokio::test]
async fn knowledge_source_crud_roundtrip() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let id = pg
        .create_knowledge_source(&NewKnowledgeSource {
            kind: "hive_mind".into(),
            name: "Org Dōjō".into(),
            url: "https://dojo.example".into(),
            namespace_id: None,
            credential_ref: "dojo-test".into(),
            direction: "both".into(),
        })
        .await
        .unwrap();

    let all = pg.list_knowledge_sources().await.unwrap();
    assert!(all.iter().any(|s| s.id == id && s.last_seq == 0 && s.enabled));

    pg.set_source_cursor(&id, 42).await.unwrap();
    let one = pg.get_knowledge_source(&id).await.unwrap().unwrap();
    assert_eq!(one.last_seq, 42);
    assert_eq!(one.direction, "both");

    assert!(pg.delete_knowledge_source(&id).await.unwrap());
    assert!(pg.get_knowledge_source(&id).await.unwrap().is_none());
}

// ── Dōjō connections tests ────────────────────────────────────────

#[tokio::test]
async fn dojo_membership_crud_and_project_binding_roundtrip() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    // Service-assigned membership id (the local PK; projects.dojo_id → this).
    let mid = uuid::Uuid::new_v4();
    pg.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/acme".into(),
        dojo_url: "http://localhost:7755/github/acme".into(),
        kind: "client".into(),
        org_slugs: vec!["acme".into(), "acme-labs".into()],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()),
        sync_status: "authenticating".into(),
    })
    .await
    .unwrap();

    // Present in the list with sane defaults + the org_slugs roundtrip.
    let all = pg.list_dojo_memberships().await.unwrap();
    let row = all.iter().find(|m| m.id == mid).expect("membership listed");
    assert_eq!(row.kind, "client");
    assert_eq!(row.org_slugs, vec!["acme".to_string(), "acme-labs".to_string()]);
    assert_eq!(row.last_seq, 0);
    assert!(row.enabled);
    assert!(row.last_heartbeat_at.is_none());

    // org-tagging edit: replace the covered org slugs.
    assert!(pg.set_dojo_membership_orgs(&mid, &["acme".into(), "acme-corp".into()]).await.unwrap());
    assert_eq!(
        pg.get_dojo_membership(&mid).await.unwrap().unwrap().org_slugs,
        vec!["acme".to_string(), "acme-corp".to_string()]
    );
    assert!(
        !pg.set_dojo_membership_orgs(&uuid::Uuid::new_v4(), &[]).await.unwrap(),
        "unknown id → false"
    );

    // sync-status update.
    assert!(pg.set_dojo_sync_status(&mid, "healthy").await.unwrap());
    assert_eq!(pg.get_dojo_membership(&mid).await.unwrap().unwrap().sync_status, "healthy");

    // Bind a project → projects.dojo_id → appears in the bound-projects strip.
    let proj = pg.create_project("_test:dojo:bind", None, None).await.unwrap();
    assert!(pg.bind_project_to_dojo(&proj, Some(&mid)).await.unwrap());
    let bound = pg.projects_bound_to_dojo(&mid).await.unwrap();
    assert!(bound.iter().any(|(id, _)| *id == proj), "bound project surfaces");

    // Unbind + cleanup.
    assert!(pg.bind_project_to_dojo(&proj, None).await.unwrap());
    assert!(pg.projects_bound_to_dojo(&mid).await.unwrap().is_empty());
    pg.delete_project(&proj).await.unwrap();

    assert!(pg.delete_dojo_membership(&mid).await.unwrap());
    assert!(pg.get_dojo_membership(&mid).await.unwrap().is_none());
}

#[tokio::test]
async fn collective_preferences_defaults_and_upsert_roundtrip() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    use crate::collective::preferences::{self, CollectivePreferences};
    let _guard = preferences::test_lock().enter();

    // Clean slate — the singleton row may linger from a prior run.
    sqlx_core::query::query("DELETE FROM sensei.collective_preferences")
        .execute(pg.pool())
        .await
        .unwrap();

    // Defaults-when-empty: no row → conservative defaults, updated_at None.
    assert!(pg.get_collective_preferences().await.unwrap().is_none());
    let defaults = preferences::get(&pg).await.unwrap();
    assert_eq!(defaults.destination, "none");
    assert_eq!(defaults.cadence, "manual");
    assert_eq!(defaults.attribution_default, "anonymous");
    assert_eq!(defaults.updated_at, None);

    // Upsert a validated body, then read it back.
    let body = serde_json::json!({
        "destination": "both", "cadence": "daily", "attribution_default": "named",
        "categories": { "memory": false, "guard": false }
    });
    let saved =
        preferences::set(&pg, CollectivePreferences::from_request(&body).unwrap()).await.unwrap();
    assert!(saved.updated_at.is_some(), "upsert assigns updated_at");

    let got = preferences::get(&pg).await.unwrap();
    assert_eq!(got.destination, "both");
    assert_eq!(got.cadence, "daily");
    assert_eq!(got.attribution_default, "named");
    assert_eq!(got.categories.get("memory"), Some(&false));
    assert_eq!(got.categories.get("guard"), Some(&false));
    assert_eq!(got.categories.get("pattern"), Some(&true));
    assert!(got.updated_at.is_some());

    // Re-upsert → still exactly one row (singleton), values fully replaced.
    let body2 = serde_json::json!({ "destination": "global", "cadence": "weekly" });
    preferences::set(&pg, CollectivePreferences::from_request(&body2).unwrap()).await.unwrap();
    let (n,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.collective_preferences")
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(n, 1, "singleton table holds exactly one row after re-upsert");
    let got2 = preferences::get(&pg).await.unwrap();
    assert_eq!(got2.destination, "global");
    assert_eq!(
        got2.categories.get("memory"),
        Some(&true),
        "full replace resets toggles to default"
    );

    // Cleanup.
    sqlx_core::query::query("DELETE FROM sensei.collective_preferences")
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn upsert_stance_default_and_scoped_roundtrip() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    // Unique key so parallel tests / lingering rows never collide.
    let user = format!("stance-upsert-{}@test.local", uuid::Uuid::new_v4());

    // 1. Default row (namespace_id NULL): insert, then re-resolve returns it
    //    as the "default" source.
    let ua = pg.upsert_stance(&user, None, "run_freely", "private", "quorum").await.unwrap();
    assert!(!ua.is_empty(), "upsert returns an updated_at");
    let r = pg.resolve_stance(&user, None).await.unwrap();
    assert_eq!(
        (r.autonomy.as_str(), r.sharing.as_str(), r.review.as_str(), r.source.as_str()),
        ("run_freely", "private", "quorum", "default")
    );

    // 2. Re-upsert the default (same partial-index conflict target): updates in
    //    place, no duplicate default row.
    pg.upsert_stance(&user, None, "ask_always", "patterns", "me_alone").await.unwrap();
    let r = pg.resolve_stance(&user, None).await.unwrap();
    assert_eq!(
        (r.autonomy.as_str(), r.review.as_str()),
        ("ask_always", "me_alone"),
        "default row updated"
    );
    let (n,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM sensei.stances WHERE user_key = $1 AND namespace_id IS NULL",
    )
    .bind(&user)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "exactly one default row after re-upsert");

    // 3. Scoped row: seed a throwaway namespace, upsert against it (composite
    //    conflict target), read it back, then re-upsert to prove update. The
    //    `project` scope may be absent in an unseeded test DB — seed it idempotently.
    sqlx_core::query::query(
        "INSERT INTO sensei.scopes (key, name, level) VALUES ('project', 'Project', 60)
             ON CONFLICT (key) DO NOTHING",
    )
    .execute(pg.pool())
    .await
    .unwrap();
    let ns = uuid::Uuid::new_v4();
    sqlx_core::query::query(
        "INSERT INTO sensei.namespaces (id, scope_key, name, slug, level)
             VALUES ($1, 'project', 'stance-test', $2, 60)",
    )
    .bind(ns)
    .bind(format!("stance-test-{ns}"))
    .execute(pg.pool())
    .await
    .unwrap();

    pg.upsert_stance(&user, Some(&ns), "ask_on_risky", "derived", "two_maintainers").await.unwrap();
    let (au, sh, rv): (String, String, String) = sqlx_core::query_as::query_as(
        "SELECT autonomy::text, sharing::text, review::text FROM sensei.stances
             WHERE user_key = $1 AND namespace_id = $2",
    )
    .bind(&user)
    .bind(ns)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    assert_eq!(
        (au.as_str(), sh.as_str(), rv.as_str()),
        ("ask_on_risky", "derived", "two_maintainers")
    );

    pg.upsert_stance(&user, Some(&ns), "run_freely", "patterns", "quorum").await.unwrap();
    let (au2,): (String,) = sqlx_core::query_as::query_as(
        "SELECT autonomy::text FROM sensei.stances WHERE user_key = $1 AND namespace_id = $2",
    )
    .bind(&user)
    .bind(ns)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    assert_eq!(au2, "run_freely", "scoped row updated via composite conflict target");
    let (n,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.stances WHERE user_key = $1")
            .bind(&user)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(n, 2, "one default + one scoped row for the user");

    // Cleanup (stances cascade off the namespace delete).
    sqlx_core::query::query("DELETE FROM sensei.stances WHERE user_key = $1")
        .bind(&user)
        .execute(pg.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1")
        .bind(ns)
        .execute(pg.pool())
        .await
        .ok();
}

#[tokio::test]
async fn dojo_outbox_and_batch_items_roundtrip() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };

    // A project + a memory to share + an APPROVED batch containing it.
    let proj = pg.create_project("_test:dojo:outbox", None, None).await.unwrap();
    let mem = pg
        .insert_memory(&InsertMemory {
            project_id: Some(proj),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "prefer migration tools".into(),
            content: "Use a dedicated migration tool over hand-rolled SQL.".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: Some("learned".into()),
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let batch = pg.create_memory_share_batch(&proj, &[mem], None).await.unwrap();
    pg.set_memory_share_batch_status(&batch, "approved", None).await.unwrap();

    // batch_share_items: approved batch, one member, body = content.
    let (bp, status, items) = pg.batch_share_items(&batch).await.unwrap().expect("batch loads");
    assert_eq!(bp, proj);
    assert_eq!(status, "approved");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].memory_id, mem);
    assert!(items[0].body.contains("migration tool"));
    assert_eq!(items[0].memory_type, "convention");

    // An unbound project → no routing anchor.
    assert!(pg.project_bound_membership(&proj).await.unwrap().is_none());

    // While the member has no `sent` outbox row, an unsent approved batch exists.
    assert!(pg.next_unsent_approved_batch().await.unwrap().is_some());

    // A destination membership + the outbox dedup ledger.
    let mid = uuid::Uuid::new_v4();
    pg.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/acme".into(),
        dojo_url: "http://localhost:7755/github/acme".into(),
        kind: "client".into(),
        org_slugs: vec![],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();

    assert!(!pg.outbox_already_sent(&mid, "sig-1").await.unwrap());
    pg.outbox_mark_sent(&mid, Some(&batch), Some(&mem), "sig-1", 5, "remote-1").await.unwrap();
    assert!(pg.outbox_already_sent(&mid, "sig-1").await.unwrap());
    // A different signature is independent.
    assert!(!pg.outbox_already_sent(&mid, "sig-2").await.unwrap());
    // A late held/queued signal must NOT downgrade an already-sent row.
    pg.outbox_mark_state(&mid, Some(&batch), Some(&mem), "sig-1", "queued").await.unwrap();
    assert!(
        pg.outbox_already_sent(&mid, "sig-1").await.unwrap(),
        "sent row must survive a late queued mark"
    );
    // A held record for a fresh signature.
    pg.outbox_mark_state(&mid, Some(&batch), Some(&mem), "sig-3", "held").await.unwrap();
    assert!(!pg.outbox_already_sent(&mid, "sig-3").await.unwrap());

    // Cleanup: membership delete cascades its outbox rows; then batch/memory/project.
    assert!(pg.delete_dojo_membership(&mid).await.unwrap());
    pg.delete_project(&proj).await.unwrap();
}

#[tokio::test]
async fn dojo_inbox_upsert_apply_and_state_roundtrip() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };

    // A source membership (inbox rows cascade with it on delete).
    let mid = uuid::Uuid::new_v4();
    pg.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/acme".into(),
        dojo_url: "http://localhost:7755/github/acme".into(),
        kind: "community".into(),
        org_slugs: vec![],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        credential_ref: format!("dojo-{}", uuid::Uuid::new_v4()),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();

    let attribution = dojo_protocol::Attribution {
        mode: dojo_protocol::AttributionMode::Anonymous,
        author: None,
        org: None,
        anonymous_id: Some("anon-1".into()),
    };
    let row = |sig: &str, title: &str| crate::collective::inbox::InboxRow {
        membership_id: mid,
        artifact_seq: 3,
        signature: sig.into(),
        remote_id: "art-x".into(),
        kind: "principle".into(),
        title: title.into(),
        body: "keep units testable".into(),
        scope: dojo_protocol::ArtifactScope::default(),
        attribution: attribution.clone(),
    };

    // Upsert is idempotent by (membership, signature): first inserts, re-pull skips.
    assert!(pg.upsert_dojo_inbox(&row("inbox-sig-1", "prefer small fns")).await.unwrap());
    assert!(
        !pg.upsert_dojo_inbox(&row("inbox-sig-1", "prefer small fns")).await.unwrap(),
        "re-pull dedups"
    );
    pg.upsert_dojo_inbox(&row("inbox-sig-2", "write tests first")).await.unwrap();

    let items = pg.list_dojo_inbox(true).await.unwrap();
    let item1 = items
        .iter()
        .find(|i| i.artifact_signature == "inbox-sig-1")
        .expect("row 1 present")
        .clone();
    let id2 =
        items.iter().find(|i| i.artifact_signature == "inbox-sig-2").expect("row 2 present").id;
    assert_eq!(item1.kind, "principle");
    assert_eq!(item1.state, "pending");
    assert_eq!(item1.attribution.anonymous_id.as_deref(), Some("anon-1"));

    // resolve_project_by_name — the scope-match lookup.
    let proj = pg.create_project("_test:dojo:inbox", None, None).await.unwrap();
    assert_eq!(pg.resolve_project_by_name("_test:dojo:inbox".into()).await.unwrap(), Some(proj));
    assert!(
        pg.resolve_project_by_name(format!("nope-{}", uuid::Uuid::new_v4()))
            .await
            .unwrap()
            .is_none()
    );

    // Land item1 as a global origin='dojo' memory; the row flips to applied.
    let mem_input = InsertMemory {
        project_id: None,
        scope: "global".into(),
        scope_filter: None,
        mtype: "convention".into(),
        title: item1.title.clone(),
        content: item1.body.clone(),
        impact: None,
        tags: vec!["dojo".into()],
        triage_signal: None,
        status: "active".into(),
        namespace_id: None,
        enforcement: Some("recommended".into()),
        origin: Some("dojo".into()),
        source_id: None,
        spine_slot: None,
        feature: None,
    };
    let memory_id = pg.land_dojo_inbox_memory(item1.id, &mem_input).await.unwrap();
    let (origin,): (String,) =
        sqlx_core::query_as::query_as("SELECT origin FROM sensei.memories WHERE id = $1")
            .bind(memory_id)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(origin, "dojo");
    let applied = pg.get_dojo_inbox(item1.id).await.unwrap().unwrap();
    assert_eq!(applied.state, "applied");
    assert_eq!(applied.applied_memory_id, Some(memory_id));

    // mute hides from the default list; pin floats to the top; unknown → false.
    assert!(pg.set_dojo_inbox_state(id2, "muted").await.unwrap());
    assert!(
        pg.list_dojo_inbox(false).await.unwrap().iter().all(|i| i.id != id2),
        "muted hidden by default"
    );
    assert!(
        pg.list_dojo_inbox(true).await.unwrap().iter().any(|i| i.id == id2),
        "include_muted surfaces it"
    );
    assert!(pg.set_dojo_inbox_state(id2, "pinned").await.unwrap());
    assert_eq!(pg.list_dojo_inbox(false).await.unwrap()[0].id, id2, "pinned floats to the top");
    assert!(
        !pg.set_dojo_inbox_state(uuid::Uuid::new_v4(), "muted").await.unwrap(),
        "unknown id → false"
    );

    // Cursor advance lives on the membership's last_seq.
    pg.set_dojo_pull_cursor(mid, 42).await.unwrap();
    assert_eq!(pg.get_dojo_membership(&mid).await.unwrap().unwrap().last_seq, 42);

    // Cleanup: membership delete cascades inbox rows; then memory + project.
    assert!(pg.delete_dojo_membership(&mid).await.unwrap());
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(memory_id)
        .execute(pg.pool())
        .await
        .unwrap();
    pg.delete_project(&proj).await.unwrap();
}

// ── scope_folder_ids tests (#60) ─────────────────────────────────

/// Build an isolated project + root folder + child subfolder for scope tests.
async fn setup_scope_test(s: &PgStore, suffix: &str) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let proj_name = format!("_test:scope:{}", suffix);
    let proj_id = s.create_project(&proj_name, None, None).await.unwrap();

    // Root folder: upsert into folders_to_watch first (foreign-key for root_id).
    let watch_path = format!("/_test/scope_{}", suffix);
    let watch_id = s
        .add_watch_root(&watch_path, &format!("scope_root_{}", suffix), &serde_json::json!([]))
        .await
        .unwrap();

    // Root repo folder (kind='git', owns root_id = watch_id).
    let root_abs = format!("/_test/scope_{}/root", suffix);
    let root_name = format!("scope_root_{}", suffix);
    let root_id = s.upsert_repo(&watch_id, &root_name, &root_abs).await.unwrap();
    s.set_folder_project(&root_id, &proj_id, "main", None).await.unwrap();

    // Child subfolder (kind='folder', parent = root, project = proj_id).
    let child_abs = format!("/_test/scope_{}/root/child", suffix);
    let child_name = format!("scope_child_{}", suffix);
    let child_id = s
        .upsert_subfolder(
            &watch_id,
            &child_name,
            &child_name,
            &child_abs,
            Some(&root_id),
            Some(&proj_id),
        )
        .await
        .unwrap();

    (proj_id, root_id, child_id)
}

#[tokio::test]
async fn scope_folder_ids_by_project_name_returns_all_folders() {
    let s = pg_store().await;
    let uid = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (proj_id, root_id, child_id) = setup_scope_test(&s, &uid).await;
    let proj_name = format!("_test:scope:{}", uid);

    let ids = s.scope_folder_ids(&proj_name).await.unwrap();
    assert!(ids.contains(&root_id), "root folder must be in scope ids; got {:?}", ids);
    assert!(ids.contains(&child_id), "child folder must be in scope ids; got {:?}", ids);

    // Also test by UUID string.
    let by_uuid = s.scope_folder_ids(&proj_id.to_string()).await.unwrap();
    assert!(by_uuid.contains(&child_id), "UUID lookup must find child; got {:?}", by_uuid);

    // Nonexistent ident returns empty.
    let empty = s.scope_folder_ids("nonexistent-xyz-scope-test-noop").await.unwrap();
    assert!(empty.is_empty(), "nonexistent must be empty; got {:?}", empty);

    // Cleanup.
    s.delete_nodes_by_folder(&root_id).await.unwrap();
    s.delete_nodes_by_folder(&child_id).await.unwrap();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = ANY($1::uuid[])")
        .bind(vec![child_id, root_id])
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_project(&proj_id).await.unwrap();
}

// ── project-scoped query variants tests (#60) ─────────────────────

#[tokio::test]
async fn scoped_search_and_count_across_child_folder() {
    let s = pg_store().await;
    let uid = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (proj_id, root_id, child_id) = setup_scope_test(&s, &uid).await;
    let proj_name = format!("_test:scope:{}", uid);

    // Insert a function node in the CHILD folder.
    let fn_id = s
        .upsert_node(
            &child_id,
            "function",
            "widget_builder",
            "src/widget.rs",
            None,
            Some("fn widget_builder()"),
            Some(1),
            Some(10),
        )
        .await
        .unwrap();
    // Insert a callee node (target) in child folder.
    let tgt_id = s
        .upsert_node(
            &child_id,
            "function",
            "render_widget",
            "src/widget.rs",
            None,
            Some("fn render_widget()"),
            Some(12),
            Some(20),
        )
        .await
        .unwrap();
    // Insert resolved edge: widget_builder calls render_widget.
    s.insert_edge(&child_id, &fn_id, Some(&tgt_id), Some("render_widget"), None, "calls")
        .await
        .unwrap();

    // Resolve scope.
    let ids = s.scope_folder_ids(&proj_name).await.unwrap();
    assert!(!ids.is_empty());

    // search_functions_scoped must find widget_builder.
    let fns = s.search_functions_scoped(&ids, "widget_builder").await.unwrap();
    assert!(
        fns.iter().any(|f| f["name"] == "widget_builder"),
        "expected widget_builder in {:?}",
        fns
    );

    // count_nodes_by_kind_scoped must report at least 2 functions.
    let counts = s.count_nodes_by_kind_scoped(&ids).await.unwrap();
    let fn_count = counts.get("function").copied().unwrap_or(0);
    assert!(fn_count >= 2, "expected >=2 function nodes, got {:?}", counts);

    // get_nodes_scoped must include child nodes.
    let nodes = s.get_nodes_scoped(&ids).await.unwrap();
    assert!(
        nodes.iter().any(|n| n["name"] == "widget_builder"),
        "nodes_scoped missing widget_builder"
    );

    // get_edges_scoped must return the calls edge.
    let edges = s.get_edges_scoped(&ids, "calls").await.unwrap();
    assert!(!edges.is_empty(), "expected >=1 calls edge in scoped result");

    // get_callers_by_name with project name: render_widget is called by widget_builder.
    let callers = s.get_callers_by_name(&proj_name, "render_widget").await.unwrap();
    assert!(
        callers.iter().any(|c| c["name"] == "widget_builder"),
        "expected widget_builder as caller of render_widget; got {:?}",
        callers
    );

    // get_callees_by_name with project name: widget_builder calls render_widget.
    let callees = s.get_callees_by_name(&proj_name, "widget_builder").await.unwrap();
    assert!(
        callees.iter().any(|c| c["name"] == "render_widget"),
        "expected render_widget as callee of widget_builder; got {:?}",
        callees
    );

    // Cleanup.
    s.delete_nodes_by_folder(&child_id).await.unwrap(); // cascades edges
    s.delete_nodes_by_folder(&root_id).await.unwrap();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = ANY($1::uuid[])")
        .bind(vec![child_id, root_id])
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_project(&proj_id).await.unwrap();
    let _ = (fn_id, tgt_id);
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
        pg.insert_log(
            level,
            &running_on,
            Some(module),
            &ts.to_rfc3339(),
            &format!("{marker} {level} message"),
            &serde_json::json!({}),
            &None,
            &None,
        )
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
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
    seed_logs(&pg, &marker).await;

    // Scope to this run via the `source` (running_on) filter is not enough
    // (two distinct sources), so fetch broadly and filter in-memory.
    let all = pg.query_logs(None, None, None, None, 1000).await.unwrap();
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
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
    seed_logs(&pg, &marker).await;

    // level filter → only the warn row.
    let warns =
        pg.query_logs(Some("warn"), Some(&format!("{marker}-a")), None, None, 1000).await.unwrap();
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0]["level"], "warn");

    // source (running_on) filter → the two `-a` rows only.
    let a_rows = pg.query_logs(None, Some(&format!("{marker}-a")), None, None, 1000).await.unwrap();
    assert_eq!(a_rows.len(), 2);
    assert!(a_rows.iter().all(|r| r["source"] == format!("{marker}-a")));

    // module column filter → only the analyzer row.
    let ana = pg.query_logs(None, None, Some("analyzer"), None, 1000).await.unwrap();
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
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
    seed_logs(&pg, &marker).await;

    // Cutoff at 1h ago excludes the two rows at base(-2h) and base+30m(-90m),
    // keeping only the base+90m(-30m) analyzer/error row.
    let since = chrono::Utc::now() - chrono::Duration::hours(1);
    let recent = pg.query_logs(None, None, None, Some(since), 1000).await.unwrap();
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
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let marker = format!("_test:logs:{}", uuid::Uuid::new_v4());
    seed_logs(&pg, &marker).await;

    // Scope to this run's sources so the global limit is deterministic.
    let a = pg.query_logs(None, Some(&format!("{marker}-a")), None, None, 1).await.unwrap();
    assert_eq!(a.len(), 1, "limit=1 returns exactly one of the two -a rows");
    // Newest -a row is the warn/watcher one.
    assert_eq!(a[0]["level"], "warn");

    cleanup_logs(&pg, &marker).await;
}

#[tokio::test]
async fn query_logs_empty_result_is_empty_array() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    // A source that never exists → empty Vec, not an error.
    let none =
        pg.query_logs(None, Some("_test:logs:does-not-exist"), None, None, 200).await.unwrap();
    assert!(none.is_empty());
}

// ── Metrics: value store + active registry (Phase 3) ──────────────────

/// Seed a `sensei.metrics` registry row for a test. Dates are relative to the
/// DB's `current_date` (via `current_date + <offset> days`) so the active-window
/// tests don't flake at a local midnight boundary. `until_offset = None` leaves
/// `effective_until` NULL (never retired). `name` is set to `key` so facet
/// assertions have a known value.
async fn seed_metric(
    s: &PgStore,
    key: &str,
    task_name: &str,
    from_offset: i32,
    until_offset: Option<i32>,
) -> uuid::Uuid {
    let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.metrics
                (key, name, description, family, type, direction, purpose, how_to_read, formula,
                 task_name, effective_from, effective_until)
             VALUES ($1, $1, 'test metric', 'quality'::sensei.metric_family, 'ratio'::sensei.metric_type,
                     'higher_better'::sensei.metric_direction, 'test purpose', 'test how', 'test formula',
                     $2, current_date + $3::int, current_date + $4::int)
             RETURNING id",
        )
        .bind(key).bind(task_name).bind(from_offset).bind(until_offset)
        .fetch_one(s.pool()).await.unwrap();
    row.0
}

#[tokio::test]
async fn upsert_project_metric_is_idempotent() {
    // Two upserts of the same identity (metric x project x null folder x null
    // session x date x daily) collapse to ONE row: the second updates value,
    // props, source and bumps modified_at rather than duplicating.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_project(&format!("_test:pm-idem:{uniq}"), None, None).await.unwrap();
    let rid =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &uuid::Uuid::new_v4()).await;
    let mid = seed_metric(&s, &format!("_test:pm-idem:{uniq}:ftr"), "ComputeFtr", 0, None).await;
    let day = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

    let id1 = s
        .upsert_project_metric(
            &mid,
            &rid,
            day,
            "daily",
            0.5,
            &serde_json::json!({"numerator": 1, "denominator": 2}),
            "measured",
        )
        .await
        .unwrap();

    // Backdate modified_at so the second upsert's bump is observable.
    sqlx_core::query::query(
        "UPDATE sensei.project_metrics SET modified_at = now() - interval '1 hour' WHERE id = $1",
    )
    .bind(id1)
    .execute(s.pool())
    .await
    .unwrap();
    let (before,): (chrono::DateTime<chrono::Utc>,) =
        query_as("SELECT modified_at FROM sensei.project_metrics WHERE id = $1")
            .bind(id1)
            .fetch_one(s.pool())
            .await
            .unwrap();

    let id2 = s
        .upsert_project_metric(
            &mid,
            &rid,
            day,
            "daily",
            0.75,
            &serde_json::json!({"numerator": 3, "denominator": 4}),
            "estimated",
        )
        .await
        .unwrap();
    assert_eq!(id1, id2, "same identity upserts the same row (no duplicate)");

    let (n,): (i64,) = query_as(
        "SELECT count(*) FROM sensei.project_metrics
              WHERE metric_id = $1 AND project_id = $2 AND computed_on = $3 AND grain = 'daily'",
    )
    .bind(mid)
    .bind(pid)
    .bind(day)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "one row per identity — the second upsert updated in place");

    let (value, props, source, after): (f64, serde_json::Value, String, chrono::DateTime<chrono::Utc>) =
            query_as("SELECT value::float8, props, source::text, modified_at FROM sensei.project_metrics WHERE id = $1")
                .bind(id1).fetch_one(s.pool()).await.unwrap();
    assert_eq!(value, 0.75, "value updated to the second upsert's");
    assert_eq!(props, serde_json::json!({"numerator": 3, "denominator": 4}), "props updated");
    assert_eq!(source, "estimated", "source updated");
    assert!(after > before, "modified_at bumped past the backdated value");

    // cleanup — project_metrics rows cascade from the metric + project.
    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn active_metrics_excludes_retired_and_future() {
    // active_metrics() returns only rows live on current_date: the retired
    // (past effective_until) and not-yet-effective (future effective_from) rows
    // are excluded. Assertions are key-specific so the pre-seeded registry and
    // concurrent tests don't interfere.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let active_key = format!("_test:active:{uniq}");
    let retired_key = format!("_test:retired:{uniq}");
    let future_key = format!("_test:future:{uniq}");
    let today_retire_key = format!("_test:today-retire:{uniq}");
    let active_task = format!("ComputeActive_{uniq}");
    let retired_task = format!("ComputeRetired_{uniq}");
    let future_task = format!("ComputeFuture_{uniq}");
    let today_retire_task = format!("ComputeTodayRetire_{uniq}");
    seed_metric(&s, &active_key, &active_task, 0, None).await; // from today, no end
    seed_metric(&s, &retired_key, &retired_task, -10, Some(-1)).await; // ended yesterday
    seed_metric(&s, &future_key, &future_task, 1, None).await; // effective tomorrow
    // Retired EFFECTIVE TODAY: effective_until = current_date. The window is
    // half-open [from, until), so `until > current_date` is false today — this
    // row must already be inactive (locks the exclusive-upper-bound boundary).
    seed_metric(&s, &today_retire_key, &today_retire_task, -10, Some(0)).await;

    let metrics = s.active_metrics().await.unwrap();
    let keys: Vec<&str> = metrics.iter().map(|m| m.key.as_str()).collect();
    assert!(keys.contains(&active_key.as_str()), "active metric is returned");
    assert!(!keys.contains(&retired_key.as_str()), "retired metric is excluded");
    assert!(!keys.contains(&future_key.as_str()), "not-yet-effective metric is excluded");
    assert!(
        !keys.contains(&today_retire_key.as_str()),
        "a metric retired effective today is excluded (effective_until is exclusive)"
    );

    let tasks = s.active_task_names().await.unwrap();
    assert!(tasks.contains(&active_task), "active metric's task_name is present");
    assert!(!tasks.contains(&retired_task), "retired metric's task_name is absent");
    assert!(!tasks.contains(&future_task), "future metric's task_name is absent");
    assert!(
        !tasks.contains(&today_retire_task),
        "task_name of a metric retired effective today is not scheduled"
    );

    // The mapped Metric carries the facets/knobs.
    let active = metrics.iter().find(|m| m.key == active_key).unwrap();
    assert_eq!(active.family, "quality");
    assert_eq!(active.metric_type, "ratio");
    assert_eq!(active.direction, "higher_better");
    assert_eq!(active.weight, 1.0, "numeric weight defaults to 1");
    assert!(active.effective_until.is_none(), "active metric has no end date");

    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE key IN ($1, $2, $3, $4)")
        .bind(&active_key)
        .bind(&retired_key)
        .bind(&future_key)
        .bind(&today_retire_key)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn active_metric_ids_maps_active_keys_for_task_name_only() {
    // active_metric_ids(task) returns key→id for ONLY the active metrics whose
    // task_name matches: a same-task retired metric and a different-task metric
    // are both absent, so a compute handler's `ids.get(key)` skips them.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let task = format!("ComputeSO_{uniq}"); // unique → no pre-seeded rows share it
    let other_task = format!("ComputeOther_{uniq}");
    let k_a = format!("_test:ami:{uniq}:a");
    let k_b = format!("_test:ami:{uniq}:b");
    let k_other = format!("_test:ami:{uniq}:other");
    let k_retired = format!("_test:ami:{uniq}:retired");
    let id_a = seed_metric(&s, &k_a, &task, 0, None).await; // active, our task
    let id_b = seed_metric(&s, &k_b, &task, 0, None).await; // active, our task
    seed_metric(&s, &k_other, &other_task, 0, None).await; // active, DIFFERENT task
    seed_metric(&s, &k_retired, &task, -10, Some(-1)).await; // our task but RETIRED

    let ids = s.active_metric_ids(&task).await.unwrap();
    assert_eq!(ids.len(), 2, "only this task's two ACTIVE keys (task_name is unique to this test)");
    assert_eq!(ids.get(&k_a).copied(), Some(id_a), "active key → its metric_id");
    assert_eq!(ids.get(&k_b).copied(), Some(id_b));
    assert!(!ids.contains_key(&k_other), "a key with a different task_name is excluded");
    assert!(!ids.contains_key(&k_retired), "a retired (inactive) key is excluded (never computed)");

    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE key IN ($1, $2, $3, $4)")
        .bind(&k_a)
        .bind(&k_b)
        .bind(&k_other)
        .bind(&k_retired)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn resolve_folder_from_path_uses_aliases() {
    // A live folders.abs_path resolves; a folder_path_aliases OLD path resolves
    // to the CURRENT folder + project; an unknown path is an honest None.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, fid) = create_test_project_and_folder(&s, &format!("resolve-{uniq}")).await;
    let abs_path = format!("/_test/resolve-{uniq}"); // create_test_folder sets abs_path = /_test/{suffix}
    let alias = format!("/_test/old-resolve-{uniq}");
    s.add_folder_path_alias(&alias, &fid, "rename").await.unwrap();

    assert_eq!(
        s.resolve_folder_by_path(&abs_path).await.unwrap(),
        Some((fid, pid)),
        "a live folders.abs_path resolves to (folder_id, project_id)"
    );
    assert_eq!(
        s.resolve_folder_by_path(&alias).await.unwrap(),
        Some((fid, pid)),
        "a folder_path_aliases old path resolves to the current folder + project"
    );
    assert_eq!(
        s.resolve_folder_by_path(&format!("/_test/unknown-{uniq}")).await.unwrap(),
        None,
        "an unknown path resolves to None (never fabricated)"
    );

    // A folder with NO project attached (folders.project_id null — a real,
    // reachable state: create_test_folder does not wire a project) resolves to
    // None. This pins the never-fabricate contract: the impl must NOT invent a
    // project id (e.g. from the folder id) when the folder has no project.
    let noproj_fid = create_test_folder(&s, &format!("noproj-{uniq}")).await;
    let noproj_path = format!("/_test/noproj-{uniq}");
    assert_eq!(
        s.resolve_folder_by_path(&noproj_path).await.unwrap(),
        None,
        "a folder without a project resolves to None (never a fabricated project id)"
    );

    // cleanup — the alias cascades on folder delete.
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(noproj_fid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn get_project_metrics_reads_views() {
    // After upserting two daily rows on different dates, get_project_metrics
    // returns the LATEST-per-metric value + props with the catalog facets
    // (name/type/unit/direction/purpose/how_to_read) joined from sensei.metrics.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_project(&format!("_test:gpm:{uniq}"), None, None).await.unwrap();
    let rid =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &uuid::Uuid::new_v4()).await;
    let key = format!("_test:gpm:{uniq}:cov");
    let mid = seed_metric(&s, &key, "ComputeCoverage", 0, None).await;
    let d1 = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let d2 = chrono::NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(); // later => latest

    s.upsert_project_metric(
        &mid,
        &rid,
        d1,
        "daily",
        0.5,
        &serde_json::json!({"numerator": 1, "denominator": 2}),
        "measured",
    )
    .await
    .unwrap();
    s.upsert_project_metric(
        &mid,
        &rid,
        d2,
        "daily",
        0.75,
        &serde_json::json!({"numerator": 3, "denominator": 4}),
        "measured",
    )
    .await
    .unwrap();

    let rows = s.get_project_metrics(&pid).await.unwrap();
    let row = rows.iter().find(|r| r.metric == key).expect("our metric is present");
    assert_eq!(row.date, d2, "latest date per metric wins");
    assert_eq!(row.value, 0.75, "latest value");
    assert_eq!(
        row.props,
        serde_json::json!({"numerator": 3, "denominator": 4}),
        "props from the latest row"
    );
    assert_eq!(row.name, key, "name facet joined from sensei.metrics (seed sets name = key)");
    assert_eq!(row.metric_type, "ratio", "type facet");
    assert_eq!(row.direction, "higher_better", "direction facet");
    assert_eq!(row.purpose, "test purpose", "purpose facet");
    assert_eq!(row.how_to_read, "test how", "how_to_read facet");
    assert!(row.unit.is_none(), "seed leaves unit null");

    // cleanup — project_metrics rows cascade from the metric + project.
    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn get_project_metrics_excludes_a_retired_metric() {
    // A retired metric (past `effective_until`, e.g. project_health) keeps its
    // durable project_metrics rows — retirement is "in place, never hand-delete
    // a row" — so the values read MUST exclude it by the active window, or its
    // stale rows keep rendering as a signal card.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_project(&format!("_test:gpm-ret:{uniq}"), None, None).await.unwrap();
    let rid =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &uuid::Uuid::new_v4()).await;
    let active_key = format!("_test:gpm-ret:{uniq}:active");
    let retired_key = format!("_test:gpm-ret:{uniq}:retired");
    let active_mid = seed_metric(&s, &active_key, "ComputeActive", 0, None).await; // active, no end
    let retired_mid = seed_metric(&s, &retired_key, "ComputeRetired", -10, Some(-1)).await; // ended yesterday
    let d = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    s.upsert_project_metric(
        &active_mid,
        &rid,
        d,
        "daily",
        0.5,
        &serde_json::json!({"numerator": 1, "denominator": 2}),
        "measured",
    )
    .await
    .unwrap();
    // The retired metric HAS a durable row — it just must not be read as active.
    s.upsert_project_metric(
        &retired_mid,
        &rid,
        d,
        "daily",
        0.9,
        &serde_json::json!({"numerator": 9, "denominator": 10}),
        "measured",
    )
    .await
    .unwrap();

    let keys: Vec<String> =
        s.get_project_metrics(&pid).await.unwrap().into_iter().map(|r| r.metric).collect();
    assert!(keys.contains(&active_key), "the active metric is returned");
    assert!(
        !keys.contains(&retired_key),
        "the retired metric is excluded from the values read — its stale rows must not render",
    );

    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = ANY($1)")
        .bind(vec![active_mid, retired_mid])
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// The datapoint→sessions drill-down returns ONLY that day's MEASURABLE sessions
/// for the project, scoped through the folder-join (`sensei.folders.project_id`),
/// newest-first, carrying the structural one-liner fields + summary. Guards the
/// three exclusions the old un-scoped digest got wrong: a different day, an
/// in-flight (`outcome IS NULL`) session, and a session whose FOLDER belongs to
/// another project (even when its own `project_id` column points here — proving
/// the scope is the folder-join, not `sessions.project_id`).
#[tokio::test]
async fn get_project_sessions_for_day_scopes_day_and_measurable() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let other = uuid::Uuid::new_v4();
    let (pid, fid) = crate::tasks::test_support::seed_metrics_project_folder(&s, &uniq).await;
    let (other_pid, other_fid) =
        crate::tasks::test_support::seed_metrics_project_folder(&s, &other).await;

    let day = chrono::NaiveDate::from_ymd_opt(2020, 6, 15).unwrap();
    let utc =
        |ts: &str| chrono::DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&chrono::Utc);
    let noon = utc("2020-06-15T12:00:00Z"); // in-day, earlier
    let evening = utc("2020-06-15T20:00:00Z"); // in-day, later → newest-first
    let prev = utc("2020-06-14T12:00:00Z"); // different day → excluded

    #[allow(clippy::too_many_arguments)]
    async fn ins(
        s: &PgStore,
        fid: uuid::Uuid,
        pid: uuid::Uuid,
        cs: &str,
        task: &str,
        outcome: Option<&str>,
        ftr: Option<bool>,
        turns: i32,
        corr: i32,
        summary: Option<&str>,
        at: chrono::DateTime<chrono::Utc>,
    ) {
        sqlx_core::query::query(
                "INSERT INTO activity.sessions
                    (folder_id, project_id, client_session_id, task, outcome, ftr, turns, corrections, summary, started_at)
                 VALUES ($1, $2, $3, $4, $5::sensei.session_outcome, $6, $7, $8, $9, $10)")
                .bind(fid).bind(pid).bind(cs).bind(task).bind(outcome).bind(ftr)
                .bind(turns).bind(corr).bind(summary).bind(at)
                .execute(s.pool()).await.unwrap();
    }

    // `client_session_id` is GLOBALLY unique (sessions_client_session_id_uniq),
    // so scope the ids to this run's uniq — otherwise a re-run collides.
    let sid = |n: &str| format!("cs-{n}-{}", uniq.simple());
    // The two measurable, in-day, in-project sessions we expect back.
    ins(
        &s,
        fid,
        pid,
        &sid("noon"),
        "task-a",
        Some("completed"),
        Some(true),
        3,
        0,
        Some("sum-a"),
        noon,
    )
    .await;
    ins(&s, fid, pid, &sid("eve"), "task-b", Some("corrected"), Some(false), 5, 2, None, evening)
        .await;
    // Exclusions:
    ins(&s, fid, pid, &sid("inflight"), "task-x", None, None, 0, 0, None, noon).await; // not measurable
    ins(&s, fid, pid, &sid("prevday"), "task-y", Some("completed"), Some(true), 1, 0, None, prev)
        .await; // other day
    // Folder in ANOTHER project, but its own project_id column points at pid —
    // folder-join scope must still exclude it.
    ins(
        &s,
        other_fid,
        pid,
        &sid("otherproj"),
        "task-z",
        Some("completed"),
        Some(true),
        1,
        0,
        None,
        noon,
    )
    .await;

    let rows = s.get_project_sessions_for_day(&pid, day).await.unwrap();
    assert_eq!(rows.len(), 2, "only the two measurable, in-day, in-project (folder-join) sessions");
    // Newest-first: evening then noon.
    assert_eq!(
        rows[0]["client_session_id"].as_str(),
        Some(sid("eve").as_str()),
        "ordered newest-first"
    );
    assert_eq!(rows[1]["client_session_id"].as_str(), Some(sid("noon").as_str()));
    // Structural one-liner fields + summary on the corrected (evening) session.
    let eve = &rows[0];
    assert_eq!(eve["outcome"], "corrected");
    assert_eq!(eve["ftr"], serde_json::json!(false));
    assert_eq!(eve["turns"], 5);
    assert_eq!(eve["corrections"], 2);
    assert_eq!(eve["task"], "task-b");
    assert!(eve["summary"].is_null(), "backfilled/absent summary is honest-null");
    assert!(eve["started_at"].as_str().is_some(), "started_at is an rfc3339 string");
    assert_eq!(rows[1]["summary"], "sum-a", "the existing summary column is returned when present");

    // Honest-empty: a day with no measurable session → [] (not a failure).
    let empty = s
        .get_project_sessions_for_day(&pid, chrono::NaiveDate::from_ymd_opt(2019, 1, 1).unwrap())
        .await
        .unwrap();
    assert!(empty.is_empty(), "no sessions that day → honest-empty list");

    // cleanup — sessions FK to the folder; drop them, then the projects (folders cascade).
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE folder_id = ANY($1)")
        .bind(vec![fid, other_fid])
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = ANY($1)")
        .bind(vec![pid, other_pid])
        .execute(s.pool())
        .await
        .unwrap();
}

/// Phase 8.1: both FTR getters read `project_metrics` (via
/// `project_metric_daily`), NOT the retired `sensei.ftr_daily` /
/// `sensei.project_ftr_metrics` views. Seeds daily `ftr` rows across the 14d,
/// 7d, and prior-14d windows and asserts the re-sourced values + unchanged
/// response shape.
#[tokio::test]
async fn ftr_getters_read_project_metrics() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_project(&format!("_test:ftrget:{uniq}"), None, None).await.unwrap();
    // The REAL registry ftr metric — the getters filter `metric = 'ftr'`.
    let (ftr_mid,): (uuid::Uuid,) =
        sqlx_core::query_as::query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
            .fetch_one(s.pool())
            .await
            .expect("ftr metric seeded in registry");

    let today = chrono::Utc::now().date_naive();
    let d_recent = today - chrono::Duration::days(3); // 7d + 14d window
    let d_prev = today - chrono::Duration::days(20); // prior-14d window only
    // Repo-grain (_v2): seed a per-test repository so these shared-`ftr` rows key
    // on (ftr, repo, user, day) and can't collide with a sibling project's rows.
    let (rid,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftrget') RETURNING id",
    )
    .bind(format!("test/ftrget-{uniq}"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_project(&s, &rid, &pid, "ftrget").await;
    // day A (today):     3/4 = 0.75
    s.upsert_project_metric_repo(
        &ftr_mid,
        &rid,
        "user",
        None,
        None,
        today,
        "daily",
        0.75,
        &serde_json::json!({"numerator": 3, "denominator": 4, "correction_count": 1}),
        "measured",
    )
    .await
    .unwrap();
    // day B (today-3):   1/2 = 0.50
    s.upsert_project_metric_repo(
        &ftr_mid,
        &rid,
        "user",
        None,
        None,
        d_recent,
        "daily",
        0.5,
        &serde_json::json!({"numerator": 1, "denominator": 2, "correction_count": 2}),
        "measured",
    )
    .await
    .unwrap();
    // day C (today-20):  1/2 = 0.50 — prior-14d window, excluded from 14d/7d
    s.upsert_project_metric_repo(
        &ftr_mid,
        &rid,
        "user",
        None,
        None,
        d_prev,
        "daily",
        0.5,
        &serde_json::json!({"numerator": 1, "denominator": 2, "correction_count": 3}),
        "measured",
    )
    .await
    .unwrap();

    // ── get_ftr_daily (per-project): value → ftr_rate, props.denominator →
    //    session_count; day C (older than 14d) excluded. Shape unchanged.
    let daily = s.get_ftr_daily(Some(&pid), 14).await.unwrap();
    assert_eq!(daily.len(), 2, "only the two rows inside the 14d window (day C excluded)");
    let a = daily
        .iter()
        .find(|r| r["day"].as_str() == Some(today.to_string().as_str()))
        .expect("today row");
    assert_eq!(
        a.as_object().unwrap().keys().collect::<Vec<_>>(),
        vec!["day", "ftr_rate", "session_count"],
        "exact response shape preserved"
    );
    assert!((a["ftr_rate"].as_f64().unwrap() - 0.75).abs() < 1e-9, "ftr_rate = stored value");
    assert_eq!(a["session_count"].as_i64(), Some(4), "session_count = props.denominator");
    let b = daily
        .iter()
        .find(|r| r["day"].as_str() == Some(d_recent.to_string().as_str()))
        .expect("today-3 row");
    assert!((b["ftr_rate"].as_f64().unwrap() - 0.5).abs() < 1e-9);
    assert_eq!(b["session_count"].as_i64(), Some(2));

    // ── get_ftr_daily (holistic): sums denominators across projects for the
    //    day; our contribution is a safe lower bound (other test data may add).
    let holistic = s.get_ftr_daily(None, 14).await.unwrap();
    let ht = holistic
        .iter()
        .find(|r| r["day"].as_str() == Some(today.to_string().as_str()))
        .expect("today holistic row");
    assert!(ht["ftr_rate"].as_f64().is_some(), "holistic ftr_rate present");
    assert!(
        ht["session_count"].as_i64().unwrap() >= 4,
        "holistic session_count sums denominators (>= our 4)"
    );

    // ── get_project_ftr headline: Σnum/Σden per window; shape unchanged.
    let ftr = s.get_project_ftr(&pid).await.unwrap();
    assert_eq!(
        ftr.as_object().unwrap().keys().collect::<Vec<_>>(),
        vec!["ftr14d", "ftr14dPrev", "ftrTrend", "sessions7d"],
        "exact response shape preserved"
    );
    assert!(
        (ftr["ftr14d"].as_f64().unwrap() - (4.0 / 6.0)).abs() < 1e-9,
        "ftr14d = Σnum/Σden over 14d = (3+1)/(4+2)"
    );
    assert!(
        (ftr["ftr14dPrev"].as_f64().unwrap() - 0.5).abs() < 1e-9,
        "ftr14dPrev = Σnum/Σden over prior-14d window (day C only) = 1/2"
    );
    assert_eq!(ftr["sessions7d"].as_i64(), Some(6), "sessions7d = Σdenominator over 7d = 4+2");
    assert!(
        ftr["ftrTrend"].as_array().is_some(),
        "ftrTrend is an array (trend reads sessions, not the store)"
    );

    // ── shared rate helper agrees with the headline.
    assert!(
        (s.get_project_ftr_rate(&pid).await.unwrap().unwrap() - (4.0 / 6.0)).abs() < 1e-9,
        "get_project_ftr_rate == ftr14d"
    );

    // cleanup — project_metrics rows cascade from the project (ftr metric kept).
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// FIX 1 (DB-free): the shared headline builder serializes an absent 14d /
/// prior-14d FTR as JSON `null` — NEVER a fabricated `0.0`. Covers BOTH
/// `get_project_ftr` and `get_holistic_ftr`, which share this builder.
/// Mutation guard: reverting the builder to `.unwrap_or(0.0)` fails this.
#[test]
fn ftr_headline_json_absent_serializes_null_not_zero() {
    let absent = PgStore::ftr_headline_json(None, None, vec![], 0);
    assert!(absent["ftr14d"].is_null(), "absent ftr14d → JSON null, not 0.0");
    assert!(absent["ftr14dPrev"].is_null(), "absent ftr14dPrev → JSON null, not 0.0");
    assert_eq!(absent["sessions7d"].as_i64(), Some(0), "sessions7d is an honest count");
    let present = PgStore::ftr_headline_json(Some(0.5), Some(0.25), vec![0.5], 3);
    assert_eq!(
        present["ftr14d"].as_f64(),
        Some(0.5),
        "a present value still serializes as a number"
    );
    assert_eq!(present["ftr14dPrev"].as_f64(), Some(0.25));
}

/// FIX 1 (end-to-end): a project with zero stored `ftr` rows reports honest
/// `null` for the headline through `get_project_ftr` — never a fabricated 0%.
#[tokio::test]
async fn get_project_ftr_absent_is_null_not_zero() {
    let s = pg_store().await;
    let pid = s
        .create_project(&format!("_test:ftrnull:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let ftr = s.get_project_ftr(&pid).await.unwrap();
    assert!(ftr["ftr14d"].is_null(), "no ftr rows → ftr14d is null, NOT 0.0");
    assert!(ftr["ftr14dPrev"].is_null(), "no ftr rows → ftr14dPrev is null, NOT 0.0");
    assert_eq!(ftr["sessions7d"].as_i64(), Some(0), "sessions7d is an honest 0 (a count)");
    assert!(ftr["ftrTrend"].as_array().is_some_and(|a| a.is_empty()), "no sessions → empty trend");
    assert_eq!(s.get_project_ftr_rate(&pid).await.unwrap(), None, "rate helper is None on no data");
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// FIX 5 (window mutation guard): `ftr14d` must reach back the full 14 days.
/// The only stored row is 10 days old — inside 14d, outside 7d — so `ftr14d`
/// is 1.0 while `sessions7d` (7d) excludes it. Narrowing the 14d window to 7d
/// would make `ftr14d` null, failing the `.expect` below.
#[tokio::test]
async fn ftr14d_window_reaches_the_8_to_13_day_band() {
    let s = pg_store().await;
    let pid = s
        .create_project(&format!("_test:ftrwin:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let (ftr_mid,): (uuid::Uuid,) = query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
        .fetch_one(s.pool())
        .await
        .unwrap();
    let d10 = chrono::Utc::now().date_naive() - chrono::Duration::days(10); // 8–13d band
    // Repo-grain (_v2): a per-test repository so the shared `ftr` row keys on
    // (ftr, repo, user, day) and can't collide with a sibling project's row.
    let (rid,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftrwin') RETURNING id",
    )
    .bind(format!("test/ftrwin-{}", uuid::Uuid::new_v4()))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_project(&s, &rid, &pid, "ftrwin").await;
    s.upsert_project_metric_repo(
        &ftr_mid,
        &rid,
        "user",
        None,
        None,
        d10,
        "daily",
        1.0,
        &serde_json::json!({"numerator": 2, "denominator": 2}),
        "measured",
    )
    .await
    .unwrap();

    let ftr = s.get_project_ftr(&pid).await.unwrap();
    assert!(
        (ftr["ftr14d"].as_f64().expect("ftr14d includes the 10-day-old row (14d window)") - 1.0)
            .abs()
            < 1e-9,
        "only row is 10d old → ftr14d = 1.0; a 7d-narrowed window would make this null"
    );
    assert_eq!(
        ftr["sessions7d"].as_i64(),
        Some(0),
        "sessions7d (7d window) excludes the 10-day-old row — proves 14d ≠ 7d"
    );
    assert_eq!(
        s.get_project_ftr_rate(&pid).await.unwrap(),
        Some(1.0),
        "rate helper (14d) includes it too"
    );

    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// FIX 3: the holistic (no-project) `get_ftr_daily` branch POOLS Σnum/Σden per
/// day — it must NOT average per-project daily rates (the `project_metrics`
/// ratio invariant). Two projects on one day with unequal denominators make
/// pooled ≠ avg-of-rates; the getter must match the pooled value.
#[tokio::test]
async fn holistic_ftr_daily_pools_not_average_of_rates() {
    let s = pg_store().await;
    let (ftr_mid,): (uuid::Uuid,) = query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
        .fetch_one(s.pool())
        .await
        .unwrap();
    // A day off the busy 'today' (compute-writing tests seed today) but inside 14d.
    let day = chrono::Utc::now().date_naive() - chrono::Duration::days(6);
    let p1 = s
        .create_project(&format!("_test:ftrpool1:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let p2 = s
        .create_project(&format!("_test:ftrpool2:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    // Repo-grain (_v2): each project needs its OWN repository — under the repo-grain
    // identity a NULL-repository `(ftr, day)` row is shared, so both projects would
    // collapse onto ONE row and the pooled Σnum/Σden could never be observed.
    let (r1,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftrpool1') RETURNING id",
    )
    .bind(format!("test/ftrpool1-{}", uuid::Uuid::new_v4()))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_project(&s, &r1, &p1, "ftrpool1").await;
    let (r2,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftrpool2') RETURNING id",
    )
    .bind(format!("test/ftrpool2-{}", uuid::Uuid::new_v4()))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_project(&s, &r2, &p2, "ftrpool2").await;
    // P1: 1/1 = 1.0 ; P2: 0/3 = 0.0 → avg-of-rates 0.5, pooled 1/4 = 0.25.
    s.upsert_project_metric_repo(
        &ftr_mid,
        &r1,
        "user",
        None,
        None,
        day,
        "daily",
        1.0,
        &serde_json::json!({"numerator": 1, "denominator": 1}),
        "measured",
    )
    .await
    .unwrap();
    s.upsert_project_metric_repo(
        &ftr_mid,
        &r2,
        "user",
        None,
        None,
        day,
        "daily",
        0.0,
        &serde_json::json!({"numerator": 0, "denominator": 3}),
        "measured",
    )
    .await
    .unwrap();

    let holistic = s.get_ftr_daily(None, 14).await.unwrap();
    let row = holistic
        .iter()
        .find(|r| r["day"].as_str() == Some(day.to_string().as_str()))
        .expect("holistic row for the seeded day");

    // Compare to the DIRECT pooled + avg over whatever exists globally for that
    // day (robust to other rows), and assert the getter matches POOLED, not avg.
    let (sum_num, sum_den, avg_rate): (f64, i64, f64) = query_as(
            "SELECT SUM((props->>'numerator')::float8), SUM((props->>'denominator')::int8)::int8, AVG(value)::float8 \
               FROM sensei.project_metric_daily WHERE metric = 'ftr' AND date = $1",
        ).bind(day).fetch_one(s.pool()).await.unwrap();
    let pooled = sum_num / sum_den as f64;
    assert!(
        (row["ftr_rate"].as_f64().unwrap() - pooled).abs() < 1e-9,
        "holistic ftr_rate is pooled Σnum/Σden, not an average of per-project rates"
    );
    assert_eq!(
        row["session_count"].as_i64(),
        Some(sum_den),
        "holistic session_count is Σdenominator"
    );
    assert!(
        (pooled - avg_rate).abs() > 1e-9,
        "seed makes pooled ({pooled}) differ from avg-of-rates ({avg_rate}) — so the check above is a real discriminator"
    );

    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = ANY($1)")
        .bind(vec![p1, p2])
        .execute(s.pool())
        .await
        .unwrap();
}

// ── P-A.2: canonical sensei.repositories schema invariants ───────────
/// The repo-grain foundation: repositories are keyed on a normalized remote
/// (unique repo_key), a remote-less repo is NULL (multiple coexist), and a
/// folder's repository_id is SET NULL when its repository is deleted (I10) so
/// the checkout survives while the metric grain is re-resolved. Locks the
/// schema P-A.3's compute + upsert depend on.
#[tokio::test]
async fn repositories_schema_invariants() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let key = format!("host/{tag}/repo");

    // NULL repo_key = local-only, no remote: MANY coexist (nulls distinct).
    let (n1,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES (NULL, 'local-a') RETURNING id",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    let (n2,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES (NULL, 'local-b') RETURNING id",
    )
    .fetch_one(s.pool())
    .await
    .expect("two NULL repo_keys must coexist");

    // A non-null repo_key is UNIQUE: the second insert of the same key fails.
    let (r1,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'canon') RETURNING id",
    )
    .bind(&key)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let dup = sqlx_core::query::query(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'dup')",
    )
    .bind(&key)
    .execute(s.pool())
    .await;
    assert!(dup.is_err(), "duplicate repo_key must violate the unique constraint");

    // folders.repository_id → ON DELETE SET NULL: deleting the repository
    // leaves the checkout folder but clears its (now-stale) repository_id.
    let root_id = s
        .add_watch_root(&format!("/tmp/repo-inv-{tag}"), "inv", &serde_json::json!([]))
        .await
        .unwrap();
    let fid = s
        .upsert_repo_kind(&root_id, "git", "co", &format!("/tmp/repo-inv-{tag}/co"))
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET repository_id = $1 WHERE id = $2")
        .bind(r1)
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(r1)
        .execute(s.pool())
        .await
        .unwrap();
    let (after,): (Option<uuid::Uuid>,) =
        query_as("SELECT repository_id FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(
        after.is_none(),
        "deleting a repository must SET NULL folders.repository_id, not delete the folder"
    );

    // cleanup (shared test DB)
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
        .bind(vec![n1, n2])
        .execute(s.pool())
        .await
        .ok();
}

// ── P-A.3b Stage 0: repository population (folders.repository_id) ─────
/// A git checkout with a remote is linked to a canonical `sensei.repositories`
/// row keyed on the NORMALIZED remote; a remote-less checkout stays NULL
/// (local-only, never federated); re-running is a no-op (idempotent).
#[tokio::test]
async fn assign_repositories_links_folders_to_canonical_repo_key() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let root_id = s
        .add_watch_root(&format!("/tmp/assignrepo-{tag}"), "ar", &serde_json::json!([]))
        .await
        .unwrap();

    // A git checkout WITH a remote (run-unique so it can't collide in the shared DB).
    let with_remote = s
        .upsert_repo_kind(&root_id, "git", "repo", &format!("/tmp/assignrepo-{tag}/repo"))
        .await
        .unwrap();
    let url = format!("git@github.com:Org/Repo-{tag}.git");
    s.update_folder_remotes(&with_remote, &serde_json::json!([{"name": "origin", "url": url}]))
        .await
        .unwrap();
    // A git checkout with NO remote → local-only.
    let no_remote = s
        .upsert_repo_kind(&root_id, "git", "local", &format!("/tmp/assignrepo-{tag}/local"))
        .await
        .unwrap();

    let n = s.assign_repositories(&root_id).await.unwrap();
    assert_eq!(
        n, 1,
        "only the folder with a remote is (re)pointed; the remote-less one stays NULL"
    );

    // The remote folder → a repositories row keyed on the NORMALIZED remote.
    let (rid, key): (Option<uuid::Uuid>, Option<String>) = query_as(
        "SELECT f.repository_id, r.repo_key FROM sensei.folders f \
               LEFT JOIN sensei.repositories r ON r.id = f.repository_id WHERE f.id = $1",
    )
    .bind(with_remote)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(rid.is_some(), "the git checkout with a remote is linked to a repository");
    assert_eq!(
        key.as_deref(),
        Some(format!("github.com/org/repo-{tag}").as_str()),
        "linked via the normalized remote key (scheme/creds/.git stripped, lowercased)"
    );

    // The remote-less folder stays NULL — local-only, never federated.
    let (rid2,): (Option<uuid::Uuid>,) =
        query_as("SELECT repository_id FROM sensei.folders WHERE id = $1")
            .bind(no_remote)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(rid2.is_none(), "a checkout with no remote is left repository_id NULL");

    // Idempotent: a second pass re-points nothing.
    assert_eq!(
        s.assign_repositories(&root_id).await.unwrap(),
        0,
        "re-run is a no-op (no folder changed)"
    );

    // cleanup (delete the watch root → cascades folders; then the repository row)
    let repo_id = rid.unwrap();
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(repo_id)
        .execute(s.pool())
        .await
        .ok();
}

// ── P-A.3b Stage 1: metric_watermarks ────────────────────────────────
/// The watermark is keyed uniquely per (repository, metric_group) and cascades
/// when its repository is deleted (so a repo prune leaves no orphan cursor).
#[tokio::test]
async fn metric_watermarks_pk_and_cascade_on_repository_delete() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let (rid,): (uuid::Uuid,) =
        query_as("INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'wm') RETURNING id")
            .bind(format!("host/wm/{tag}"))
            .fetch_one(s.pool())
            .await
            .unwrap();

    sqlx_core::query::query(
        "INSERT INTO sensei.metric_watermarks (repository_id, metric_group, sealed_through) \
             VALUES ($1, 'session_outcomes', current_date)",
    )
    .bind(rid)
    .execute(s.pool())
    .await
    .unwrap();

    // PK: a duplicate (repository_id, metric_group) is rejected.
    let dup = sqlx_core::query::query(
            "INSERT INTO sensei.metric_watermarks (repository_id, metric_group) VALUES ($1, 'session_outcomes')",
        ).bind(rid).execute(s.pool()).await;
    assert!(dup.is_err(), "duplicate (repository_id, metric_group) violates the PK");

    // FK cascade: deleting the repository drops its watermark (no orphan cursor).
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    let (cnt,): (i64,) =
        query_as("SELECT count(*) FROM sensei.metric_watermarks WHERE repository_id = $1")
            .bind(rid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(cnt, 0, "watermarks cascade on repository delete");
}

// ── P-A.3b Stage 2 foundation: project→repositories resolution ────────
/// A project spans the distinct repositories on its checkout folders, listed
/// shallowest-first (the "primary" = root checkout's repo, where project-level
/// metrics attach); the multi-repo compute iterates this, not one root path.
#[tokio::test]
async fn repositories_for_project_lists_repos_primary_first() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let root_id =
        s.add_watch_root(&format!("/tmp/rfp-{tag}"), "rfp", &serde_json::json!([])).await.unwrap();
    let pid = s.create_project(&format!("_test:rfp:{tag}"), None, None).await.unwrap();

    let (r_root,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'root') RETURNING id",
    )
    .bind(format!("host/{tag}/root"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_project(&s, &r_root, &pid, "root").await;
    let (r_nested,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'nested') RETURNING id",
    )
    .bind(format!("host/{tag}/nested"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_project(&s, &r_nested, &pid, "nested").await;
    // Root checkout (shallow path) + a nested checkout (deeper path), both in the project.
    let f_root =
        s.upsert_repo_kind(&root_id, "git", "root", &format!("/tmp/rfp-{tag}/root")).await.unwrap();
    let f_nested = s
        .upsert_repo_kind(&root_id, "git", "nested", &format!("/tmp/rfp-{tag}/root/vendor/nested"))
        .await
        .unwrap();
    for (f, r) in [(f_root, r_root), (f_nested, r_nested)] {
        sqlx_core::query::query(
            "UPDATE sensei.folders SET repository_id = $2, project_id = $3 WHERE id = $1",
        )
        .bind(f)
        .bind(r)
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
    }

    let repos = s.repositories_for_project(&pid).await.unwrap();
    assert_eq!(repos, vec![r_root, r_nested], "both repos, shallowest (primary) first");
    assert_eq!(
        s.primary_repository_for_project(&pid).await.unwrap(),
        Some(r_root),
        "primary repository = the shallowest checkout's repository"
    );

    // cleanup
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
        .bind(vec![r_root, r_nested])
        .execute(s.pool())
        .await
        .ok();
}

// ── Personas (Phase 3) ──────────────────────────────────────────────────

/// Seed a project + repository + one user-scope metric row for `email`.
async fn seed_identity_row(
    s: &PgStore,
    uniq: &uuid::Uuid,
    email: &str,
    value: f64,
) -> (uuid::Uuid, uuid::Uuid) {
    let pid = s.create_project(&format!("_test:persona:{uniq}"), None, None).await.unwrap();
    let rid = crate::tasks::test_support::seed_bare_repository(s, &pid, uniq).await;
    let mid = seed_metric(s, &format!("_test:persona:{uniq}:ftr"), "ComputeFtr", 0, None).await;
    s.upsert_project_metric_repo(
        &mid,
        &rid,
        "user",
        Some(email),
        None,
        chrono::NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
        "daily",
        value,
        &serde_json::json!({}),
        "measured",
    )
    .await
    .unwrap();
    (pid, rid)
}

#[tokio::test]
async fn resolution_groups_aliases_without_touching_the_raw_email() {
    // The point of the persona layer: two git addresses that belong to one
    // working identity resolve to ONE persona — while `identity` keeps the
    // raw assertion each row was computed from, so the grouping stays a
    // re-derivation rather than a destructive merge.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let a = format!("a-{uniq}@example.com");
    let b = format!("b-{uniq}@example.com");
    let (pid, _rid) = seed_identity_row(&s, &uniq, &a, 0.5).await;
    let uniq2 = uuid::Uuid::new_v4();
    let (pid2, _r2) = seed_identity_row(&s, &uniq2, &b, 0.75).await;

    let persona = s.upsert_persona(&format!("work-{uniq}"), true).await.unwrap();
    s.link_persona_email(&persona, &a, "git").await.unwrap();
    s.link_persona_email(&persona, &b, "git").await.unwrap();
    s.resolve_persona_ids().await.unwrap();

    let (n, distinct_identities): (i64, i64) = query_as(
        "SELECT count(*)::int8, count(DISTINCT identity)::int8 \
               FROM sensei.repository_metrics WHERE persona_id = $1",
    )
    .bind(persona)
    .fetch_one(s.pool())
    .await
    .unwrap();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid2, None, &[]).await;
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(persona)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(n, 2, "both alias rows resolve to the one persona");
    assert_eq!(distinct_identities, 2, "the RAW emails are preserved, not collapsed");
}

#[tokio::test]
async fn an_unrecognised_author_stays_unassigned_rather_than_guessed() {
    // Never-fabricate: an email no persona claims must NOT be folded into the
    // local user's numbers. It stays NULL and is surfaced for assignment.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let stranger = format!("stranger-{uniq}@example.com");
    let (pid, _rid) = seed_identity_row(&s, &uniq, &stranger, 0.9).await;

    s.resolve_persona_ids().await.unwrap();
    let (unresolved,): (i64,) = query_as(
        "SELECT count(*)::int8 FROM sensei.repository_metrics \
              WHERE identity = $1 AND persona_id IS NULL",
    )
    .bind(&stranger)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let pending = s.unassigned_identities().await.unwrap();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;

    assert_eq!(unresolved, 1, "an unclaimed author is left unattributed");
    assert!(
        pending.iter().any(|(e, _)| e == &stranger),
        "and is surfaced for assignment rather than silently ignored"
    );
}

#[tokio::test]
async fn reassigning_an_email_moves_its_rows_on_re_resolution() {
    // The reason `identity` is kept: correcting a persona assignment is a
    // re-run, not a data-loss event. Move the address, re-resolve, and the
    // history follows.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let email = format!("moves-{uniq}@example.com");
    let (pid, _rid) = seed_identity_row(&s, &uniq, &email, 0.5).await;

    let first = s.upsert_persona(&format!("first-{uniq}"), true).await.unwrap();
    let second = s.upsert_persona(&format!("second-{uniq}"), true).await.unwrap();
    s.link_persona_email(&first, &email, "git").await.unwrap();
    s.resolve_persona_ids().await.unwrap();

    // The correction: the address actually belongs to the other identity.
    s.link_persona_email(&second, &email, "git").await.unwrap();
    s.resolve_persona_ids().await.unwrap();

    let (on_second,): (i64,) =
        query_as("SELECT count(*)::int8 FROM sensei.repository_metrics WHERE persona_id = $1")
            .bind(second)
            .fetch_one(s.pool())
            .await
            .unwrap();
    let (on_first,): (i64,) =
        query_as("SELECT count(*)::int8 FROM sensei.repository_metrics WHERE persona_id = $1")
            .bind(first)
            .fetch_one(s.pool())
            .await
            .unwrap();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = ANY($1)")
        .bind(vec![first, second])
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(on_second, 1, "the row followed the reassigned address");
    assert_eq!(on_first, 0, "and no longer counts toward the old persona");
}

#[tokio::test]
async fn two_personas_cannot_share_one_dojo_login() {
    // The privacy boundary made structural. Supabase auto-links identities
    // sharing a verified email and cannot be told not to, so two personas CAN
    // end up pointing at one merged account — this must fail loudly rather
    // than silently file business work under a personal identity.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let a = s.upsert_persona(&format!("pa-{uniq}"), true).await.unwrap();
    let b = s.upsert_persona(&format!("pb-{uniq}"), true).await.unwrap();
    let principal = uuid::Uuid::new_v4();

    sqlx_core::query::query("UPDATE sensei.personas SET principal_id = $2 WHERE id = $1")
        .bind(a)
        .bind(principal)
        .execute(s.pool())
        .await
        .unwrap();
    let clash =
        sqlx_core::query::query("UPDATE sensei.personas SET principal_id = $2 WHERE id = $1")
            .bind(b)
            .bind(principal)
            .execute(s.pool())
            .await;

    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = ANY($1)")
        .bind(vec![a, b])
        .execute(s.pool())
        .await
        .unwrap();

    assert!(clash.is_err(), "a second persona claiming the same login is rejected");
}

#[tokio::test]
async fn all_task_kinds_match_the_database_enum() {
    // Ties the three places a kind must exist into one assertion: the Rust
    // enum, `TaskKind::ALL`, and `sensei.task_execution_kind`.
    //
    // This is the guard the codebase lacked. task_kind was free text until
    // Phase 0, and four kinds had already orphaned their history by being
    // renamed or retired with nothing noticing. Now a half-added kind fails
    // here instead of at an INSERT on a fire-and-forget path that only logs.
    let s = pg_store().await;
    let db: Vec<String> = query_as::<_, (String,)>(
        "SELECT e.enumlabel::text FROM pg_enum e \
               JOIN pg_type t ON t.oid = e.enumtypid \
              WHERE t.typname = 'task_execution_kind'",
    )
    .fetch_all(s.pool())
    .await
    .unwrap()
    .into_iter()
    .map(|(v,)| v)
    .collect();

    // Values kept only to describe historical rows. They have no TaskKind by
    // design — remapping them to a live kind would fabricate history that
    // never happened (compute_metrics SPLIT in two; resolve_edges was retired
    // outright and has no successor at all).
    let retired = [
        "resolve_edges",
        "plan_metric_days",
        "compute_metrics",
        "reconcile_identity",
        "backfill_transcripts",
        "backfill_transcript_file",
    ];

    for k in crate::tasks::TaskKind::ALL {
        let name = k.info().name;
        assert!(
            db.iter().any(|d| d == name),
            "{name} is a live TaskKind but missing from sensei.task_execution_kind — \
                 an execution row for it would fail to INSERT"
        );
    }
    for value in &db {
        if retired.contains(&value.as_str()) {
            continue;
        }
        assert!(
            crate::tasks::TaskKind::ALL.iter().any(|k| k.info().name == value),
            "{value} exists in the DB enum but no TaskKind produces it — \
                 either add the kind or mark the value retired"
        );
    }
}

// ── Sync bookkeeping (Phase 7) ──────────────────────────────────────────

/// A shared repository with one locally-computed metric row.
async fn seed_sync_fixture(s: &PgStore, uniq: &uuid::Uuid) -> (uuid::Uuid, uuid::Uuid) {
    let pid = s.create_project(&format!("_test:sync:{uniq}"), None, None).await.unwrap();
    let rid = crate::tasks::test_support::seed_bare_repository(s, &pid, uniq).await;
    sqlx_core::query::query("UPDATE sensei.repositories SET visibility = 'shared' WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    let mid = seed_metric(s, &format!("_test:sync:{uniq}:ftr"), "ComputeFtr", 0, None).await;
    s.upsert_project_metric_repo(
        &mid,
        &rid,
        "user",
        None,
        None,
        chrono::NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(),
        "daily",
        0.5,
        &serde_json::json!({}),
        "measured",
    )
    .await
    .unwrap();
    (pid, rid)
}

#[tokio::test]
async fn a_pulled_row_is_never_pushed_back() {
    // The loop-breaker. Without `computed_by`, a value dojo handed down is
    // indistinguishable from one this machine produced — so it gets pushed
    // back, pulled again, and the two sides ping-pong forever.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;

    let mine = s.unpushed_metric_rows(100).await.unwrap();
    let before =
        mine.iter().filter(|r| r["repoKey"].as_str() == Some(&format!("test/bare-{uniq}"))).count();

    // Same row, but marked as dojo's.
    sqlx_core::query::query(
        "UPDATE sensei.repository_metrics SET computed_by = 'dojo' WHERE repository_id = $1",
    )
    .bind(rid)
    .execute(s.pool())
    .await
    .unwrap();
    let after = s.unpushed_metric_rows(100).await.unwrap();
    let after_n = after
        .iter()
        .filter(|r| r["repoKey"].as_str() == Some(&format!("test/bare-{uniq}")))
        .count();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;

    assert_eq!(before, 1, "a locally-computed row is queued for push");
    assert_eq!(after_n, 0, "a dojo-computed row is NOT pushed back");
}

#[tokio::test]
async fn a_private_repository_is_skipped_not_queued() {
    // Private is a choice, not a backlog item: its rows must never enter the
    // push queue at all, or every private repo looks like a pending sync.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;
    sqlx_core::query::query("UPDATE sensei.repositories SET visibility = 'private' WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();

    let rows = s.unpushed_metric_rows(100).await.unwrap();
    let n =
        rows.iter().filter(|r| r["repoKey"].as_str() == Some(&format!("test/bare-{uniq}"))).count();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    assert_eq!(n, 0, "a private repository's rows are never queued for push");
}

#[tokio::test]
async fn a_recomputed_row_is_pushed_again() {
    // shared_at alone is not enough: a day that recomputes after being pushed
    // must go again, or dojo keeps a stale value forever behind an
    // already-synced marker.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;
    let key = format!("test/bare-{uniq}");

    let ids: Vec<uuid::Uuid> = s
        .unpushed_metric_rows(100)
        .await
        .unwrap()
        .iter()
        .filter(|r| r["repoKey"].as_str() == Some(&key))
        .filter_map(|r| r["id"].as_str().and_then(|v| uuid::Uuid::parse_str(v).ok()))
        .collect();
    s.mark_metric_rows_shared(&ids).await.unwrap();
    let after_push = s
        .unpushed_metric_rows(100)
        .await
        .unwrap()
        .iter()
        .filter(|r| r["repoKey"].as_str() == Some(&key))
        .count();

    // The day recomputes — modified_at moves past shared_at.
    sqlx_core::query::query(
            "UPDATE sensei.repository_metrics SET modified_at = now() + interval '1 second'               WHERE repository_id = $1")
            .bind(rid).execute(s.pool()).await.unwrap();
    let after_recompute = s
        .unpushed_metric_rows(100)
        .await
        .unwrap()
        .iter()
        .filter(|r| r["repoKey"].as_str() == Some(&key))
        .count();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    assert_eq!(after_push, 0, "a pushed row leaves the queue");
    assert_eq!(after_recompute, 1, "and re-enters it when the value changes");
}

#[tokio::test]
async fn a_sync_error_keeps_when_the_sides_last_agreed() {
    // synced_at must survive a failure. Clearing it would leave no way to tell
    // a never-synced entity from one that has been broken since Tuesday —
    // which is the first thing worth knowing when sync starts failing.
    let s = pg_store().await;
    let key = format!("github.com/test/sync-{}", uuid::Uuid::new_v4());
    let mark =
        crate::db::pg_store::sync::SyncMark { entity: "repository", key: &key, direction: "push" };

    s.mark_synced(&mark, Some(7)).await.unwrap();
    s.mark_sync_error(&mark, "boom").await.unwrap();

    let (state, last_error, synced_at): (String, Option<String>, Option<chrono::DateTime<chrono::Utc>>) =
            query_as("SELECT state, last_error, synced_at FROM sensei.sync_state                        WHERE entity = 'repository' AND entity_key = $1 AND direction = 'push'")
                .bind(&key).fetch_one(s.pool()).await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.sync_state WHERE entity_key = $1")
        .bind(&key)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(state, "error");
    assert_eq!(last_error.as_deref(), Some("boom"));
    assert!(synced_at.is_some(), "the last agreement time survives a later failure");
}

#[tokio::test]
async fn a_verified_login_replaces_a_guessed_label_but_not_a_chosen_one() {
    // The `sensei-hq` vs `sensei-hq-org` case. Before OAuth a label can only
    // be inferred from an email domain or a repo owner, and inference is
    // wrong; after OAuth the login is known. But once a user has verified and
    // then renamed, a later sign-in must not silently overwrite their choice.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let gh_id: i64 = (uniq.as_u128() % 1_000_000) as i64 + 900_000;
    let guessed = format!("guess-{uniq}");

    // Discovered from git: a guessed label, unverified.
    let pid = s.upsert_persona(&guessed, true).await.unwrap();
    let id = s.link_persona_identity(&guessed, "real-login", gh_id, None, &[]).await.unwrap();
    assert_eq!(id, pid, "verification lands on the existing persona");

    let (label, login, verified): (String, Option<String>, Option<chrono::DateTime<chrono::Utc>>) =
        query_as("SELECT label, github_login, verified_at FROM sensei.personas WHERE id = $1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(label, "real-login", "the guess is replaced by the verified login");
    assert_eq!(login.as_deref(), Some("real-login"));
    assert!(verified.is_some());

    // The user renames it, then signs in again.
    sqlx_core::query::query("UPDATE sensei.personas SET label = 'my-name' WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
    s.link_persona_identity("ignored", "real-login", gh_id, None, &[]).await.unwrap();
    let (after,): (String,) = query_as("SELECT label FROM sensei.personas WHERE id = $1")
        .bind(id)
        .fetch_one(s.pool())
        .await
        .unwrap();

    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(after, "my-name", "a chosen display name survives re-verification");
}

#[tokio::test]
async fn a_renamed_github_login_still_matches_the_same_persona() {
    // Matching on the login would fork one human into two personas the day
    // they rename themselves. The numeric id cannot be renamed, so it wins.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let gh_id: i64 = (uniq.as_u128() % 1_000_000) as i64 + 800_000;

    let first =
        s.link_persona_identity(&format!("a-{uniq}"), "old-login", gh_id, None, &[]).await.unwrap();
    let second =
        s.link_persona_identity(&format!("b-{uniq}"), "new-login", gh_id, None, &[]).await.unwrap();

    let (login,): (Option<String>,) =
        query_as("SELECT github_login FROM sensei.personas WHERE id = $1")
            .bind(first)
            .fetch_one(s.pool())
            .await
            .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = ANY($1)")
        .bind(vec![first, second])
        .execute(s.pool())
        .await
        .ok();

    assert_eq!(first, second, "the same GitHub id resolves to one persona");
    assert_eq!(login.as_deref(), Some("new-login"), "and the login is updated in place");
}
