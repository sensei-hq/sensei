use super::*;

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
async fn prune_empty_workspaces_grace_protects_fresh() {
    // A just-created empty `discovery` workspace must survive a grace>0 prune —
    // its folder may still be attaching in a concurrent step. (The grace=0
    // path is deliberate/global and unsafe to exercise in the shared test DB,
    // so it's covered by the exclusion handler in production, not here.)
    let s = pg_store().await;
    let fresh = s
        .create_workspace(&format!("_test:grace-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    s.prune_empty_workspaces(60).await.unwrap();
    let (alive,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.workspaces WHERE id=$1")
            .bind(fresh)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(alive, 1, "grace protects a fresh empty workspace");
    // Direct cleanup (not a grace=0 global prune, which would hit sibling tests).
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id=$1")
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
        .seed_node(&fid, "file", "Widget.svelte", "src/Widget.svelte", None, None, None, None)
        .await
        .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "component",
        "Widget",
        "src/Widget.svelte",
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "hook",
        "effect",
        "src/Widget.svelte",
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    // A plain file with only a function → no framework tag.
    let util =
        s.seed_node(&fid, "file", "util.rs", "src/util.rs", None, None, None, None).await.unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "helper",
        "src/util.rs",
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // File-role by path convention (no symbols needed): SvelteKit routes +
    // middleware, and a Next-style middleware file.
    let page = s
        .seed_node(
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
        .seed_node(&fid, "file", "+server.ts", "src/routes/api/+server.ts", None, None, None, None)
        .await
        .unwrap();
    let hooks = s
        .seed_node(&fid, "file", "hooks.server.ts", "src/hooks.server.ts", None, None, None, None)
        .await
        .unwrap();
    let mw = s
        .seed_node(&fid, "file", "middleware.ts", "middleware.ts", None, None, None, None)
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

// ── Folders tests ────────────────────────────────────────────────

#[tokio::test]
async fn folder_upsert_and_list() {
    let s = pg_store().await;
    let path = format!("/_test/folder_root_{}", uuid::Uuid::new_v4());
    let rid = s.add_watch_root(&path, "test_root", &serde_json::json!([])).await.unwrap();
    let fid = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &rid,
            kind: "git",
            name: "myrepo",
            path: "myrepo",
            abs_path: &format!("{}/myrepo", path),
            parent_id: None,
            workspace_root_id: None,
        })
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
        ("archived", "g"),
    ] {
        let name = format!("repo_{}", suffix);
        let abs_path = format!("{}/{}", root_path, name);
        let fid = s
            .upsert_folder(&crate::db::pg_store::FolderRow {
                root_id: &rid,
                kind: "git",
                name: &name,
                path: &name,
                abs_path: &abs_path,
                parent_id: None,
                workspace_root_id: None,
            })
            .await
            .unwrap();
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
    // start), so resume MUST re-enqueue them. `indexed`/`archived` are
    // terminal and never resumed. (`deferred` was removed: v2 stores no
    // folder it does not index.)
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
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &rid,
            kind: "git",
            name: "r",
            path: "r",
            abs_path: &format!("{root_path}/r"),
            parent_id: None,
            workspace_root_id: None,
        })
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
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &rid,
            kind: "git",
            name: "r",
            path: "r",
            abs_path: &format!("{root_path}/r"),
            parent_id: None,
            workspace_root_id: None,
        })
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

// ── Bug 3: re-absorb a standalone root mis-scoped inside a git repo ────

/// #328: a heal that reaches a nested standalone before its workspace is
/// placed (the scanner upserts the folder, then places it) skips the phantom
/// merge. The folder must still end up in the repo's workspace: re-parenting
/// has to move its repository anchor too, because a folder's workspace is read
/// through `folders.repository_id`, and once re-absorbed it is never a
/// candidate again.
#[tokio::test]
async fn a_nested_standalone_healed_before_its_workspace_lands_follows_the_repo() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root_path = format!("/_test/heal_late/{uniq}");
    let root_id =
        s.add_watch_root(&root_path, "heal_late_root", &serde_json::json!([])).await.unwrap();
    let repo_abs = format!("{root_path}/repo");
    let repo_pid =
        s.create_workspace(&format!("_test:heal_late_repo_{uniq}"), None, None).await.unwrap();
    let repo_fid = s.upsert_repo_kind(&root_id, "git", "repo", &repo_abs).await.unwrap();
    crate::tasks::test_support::give_folder_a_repository(
        &s,
        &repo_fid,
        &format!("late-repo-{uniq}"),
    )
    .await
    .unwrap();
    s.set_folder_workspace(&repo_fid, &repo_pid, "root", None).await.unwrap();

    // The standalone is upserted with its own repository, but not placed yet.
    let sub_abs = format!("{repo_abs}/crates/sub");
    let sub_fid = s.upsert_repo_kind(&root_id, "standalone", "sub", &sub_abs).await.unwrap();
    let sub_repo = crate::tasks::test_support::give_folder_a_repository(
        &s,
        &sub_fid,
        &format!("late-sub-{uniq}"),
    )
    .await
    .unwrap();

    s.heal_nested_standalone_roots().await.unwrap();

    // The placement lands after the heal.
    let late_pid =
        s.create_workspace(&format!("_test:heal_late_phantom_{uniq}"), None, None).await.unwrap();
    s.link_workspace_repository(&late_pid, &sub_repo).await.unwrap();

    let (kind, pid): (String, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
        "SELECT kind::text, sensei.sole_workspace_of(id) FROM sensei.folders WHERE id = $1",
    )
    .bind(sub_fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(kind, "folder");
    assert_eq!(
        pid,
        Some(repo_pid),
        "a re-absorbed folder's workspace is the repo's, whatever lands later"
    );
}

#[tokio::test]
async fn heal_nested_standalone_roots_reabsorbs_and_removes_phantom() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let root_path = format!("/_test/heal_nested/{uniq}");
    let root_id =
        s.add_watch_root(&root_path, "heal_nested_root", &serde_json::json!([])).await.unwrap();

    // A git repo (like the sensei monorepo) with its own workspace.
    let repo_abs = format!("{root_path}/repo");
    let repo_pid =
        s.create_workspace(&format!("_test:heal_repo_{uniq}"), None, None).await.unwrap();
    let repo_fid = s.upsert_repo_kind(&root_id, "git", "repo", &repo_abs).await.unwrap();
    crate::tasks::test_support::give_folder_a_repository(
        &s,
        &repo_fid,
        &format!("heal-repo-{uniq}"),
    )
    .await
    .unwrap();
    s.set_folder_workspace(&repo_fid, &repo_pid, "root", None).await.unwrap();

    // A sub-crate INSIDE the repo, mis-scoped as its own standalone workspace
    // (the Bug 3 phantom). Give it a node so we can prove its nodes are dropped.
    let crate_abs = format!("{repo_abs}/crates/dojo-mind");
    let phantom_pid =
        s.create_workspace(&format!("_test:heal_phantom_{uniq}"), None, None).await.unwrap();
    // Built as `git` and turned `standalone` only once its node and `files` rows
    // exist: the heal is database-wide and other tests run it, and a phantom
    // that is heal-eligible mid-setup had its rows dropped under `seed_node`
    // (nodes_file_id_fkey).
    let crate_fid = s.upsert_repo_kind(&root_id, "git", "dojo-mind", &crate_abs).await.unwrap();
    crate::tasks::test_support::give_folder_a_repository(
        &s,
        &crate_fid,
        &format!("heal-phantom-{uniq}"),
    )
    .await
    .unwrap();
    s.set_folder_workspace(&crate_fid, &phantom_pid, "root", None).await.unwrap();
    let node_id = s
        .seed_node(&crate_fid, "struct", "DojoStore", "src/store.rs", None, None, None, None)
        .await
        .unwrap();
    // Its `files` rows too. The heal dropped nodes and re-classified the folder but
    // left these rows behind, so a folder healed long ago still looked fully
    // indexed by content while holding no content nodes. Measured live:
    // `cluster/server` 1 node against 1,970 stale rows, `cluster/scheduler`
    // 1/1,816, `sensei/marketplace` 1/77 — enough to make a content-based
    // duplicate check report six false positives out of seven.
    s.upsert_scan_state(&crate_fid, "src/store.rs", 1, &format!("hash-{uniq}")).await.unwrap();
    let phantom =
        s.upsert_repo_kind(&root_id, "standalone", "dojo-mind", &crate_abs).await.unwrap();
    assert_eq!(phantom, crate_fid, "the same folder, now mis-scoped as standalone");

    // Heal. The heal is database-wide and two other tests here run it too, so
    // its count is not this test's to assert: a concurrent heal can re-absorb
    // this phantom first and leave this call 0. What this test owns is its own
    // folder's end state, asserted below — and that fails if the heal does
    // nothing.
    s.heal_nested_standalone_roots().await.unwrap();

    // The nested root is now a folder of the repo's workspace, parented to the repo.
    let (kind, pid, parent): (String, Option<uuid::Uuid>, Option<uuid::Uuid>) =
        sqlx_core::query_as::query_as(
            "SELECT kind::text, sensei.sole_workspace_of(id), parent_id FROM sensei.folders WHERE id = $1",
        )
        .bind(crate_fid)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(kind, "folder", "mis-scoped standalone should be re-classified as a folder");
    assert_eq!(pid, Some(repo_pid), "should now belong to the enclosing repo's workspace");
    assert_eq!(parent, Some(repo_fid), "should be parented under the enclosing repo");

    // Its own nodes were dropped (the repo re-indexes the subtree).
    let (node_exists,): (bool,) =
        sqlx_core::query_as::query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id = $1)")
            .bind(node_id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(!node_exists, "the mis-scoped root's own nodes should be pruned");

    // AND its `files` rows, which describe an indexing unit that no longer
    // exists. Leaving it made the folder look fully indexed by content while
    // holding no content nodes — the residue that produced six false positives
    // in the contained-duplicate check.
    let (stale_rows,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.files WHERE folder_id = $1")
            .bind(crate_fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(stale_rows, 0, "the mis-scoped root's `files` rows must go with its nodes");

    // The phantom workspace (lived entirely inside the repo) is gone.
    let (phantom_exists,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT EXISTS(SELECT 1 FROM sensei.workspaces WHERE id = $1)",
    )
    .bind(phantom_pid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(!phantom_exists, "the phantom workspace should be merged away");

    // Idempotent for THIS test's rows: a second run leaves the phantom merged
    // away. The returned count is GLOBAL — db-gated tests share `sensei_test`
    // and other tests (e.g. the index-audit suite) may seed nested-standalone
    // rows concurrently — so assert on our own row, not the global count.
    s.heal_nested_standalone_roots().await.unwrap();
    let (phantom_gone_after_rerun,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT NOT EXISTS(SELECT 1 FROM sensei.workspaces WHERE id = $1)",
    )
    .bind(phantom_pid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(phantom_gone_after_rerun, "re-run leaves the phantom merged away (idempotent)");

    // cleanup
    s.delete_folder_tree(&repo_fid).await.ok();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
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

    crate::tasks::test_support::seed_node(&s, &fid, "file", "a.rs", "a.rs", None, None, None, None)
        .await
        .unwrap();
    crate::tasks::test_support::seed_node(&s, &fid, "struct", "B", "b.rs", None, None, None, None)
        .await
        .unwrap();
    // A module node records an ABSOLUTE dir path — must be excluded so it never
    // pollutes the rel-path comparison in prune_vanished. It names a DIRECTORY,
    // which has no `files` row and never will (R13), so it takes the directory
    // writer rather than the file one.
    s.upsert_dir_node(&fid, "module", "src", &format!("{repo_abs}/src")).await.unwrap();

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
