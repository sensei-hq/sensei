use super::*;

// ── Workspace merge tests (#41) ──────────────────────────────────────

#[tokio::test]
async fn merge_workspaces_moves_folders_sessions_memories_and_deletes_source() {
    let s = pg_store().await;
    let suffix = format!("merge_{}", uuid::Uuid::new_v4());
    let (src, src_folder) = create_test_workspace_and_folder(&s, &format!("{}_src", suffix)).await;
    let (tgt, _tgt_folder) = create_test_workspace_and_folder(&s, &format!("{}_tgt", suffix)).await;

    // Seed a memory attributed to the source workspace so we can prove it
    // survives the merge (only its workspace_id shifts).
    let mem_id: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.memories(workspace_id, scope, type, title, content, origin)
             VALUES($1, 'workspace'::sensei.memory_scope, 'convention'::sensei.memory_type, $2, 'body', 'user')
             RETURNING id"
        ).bind(src).bind(format!("_test:merge_memory_{}", uuid::Uuid::new_v4()))
            .fetch_one(s.pool()).await.unwrap();

    s.merge_workspaces(&src, &tgt).await.unwrap();

    // Source workspace row is gone.
    let src_exists: (bool,) = sqlx_core::query_as::query_as(
        "SELECT EXISTS(SELECT 1 FROM sensei.workspaces WHERE id = $1)",
    )
    .bind(src)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(!src_exists.0, "source workspace should be deleted after merge");

    // The source's folder now lives under the target workspace.
    let (folder_workspace,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
        "SELECT sensei.sole_workspace_of(id) FROM sensei.folders WHERE id = $1",
    )
    .bind(src_folder)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(folder_workspace, Some(tgt), "folder should be reassigned to target");

    // The memory survived and points at the target.
    let (mem_workspace,): (Option<uuid::Uuid>,) =
        sqlx_core::query_as::query_as("SELECT workspace_id FROM sensei.memories WHERE id = $1")
            .bind(mem_id.0)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(mem_workspace, Some(tgt), "user-authored memory should follow to target");

    // cleanup
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(mem_id.0)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(tgt)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn merge_workspaces_rejects_self_merge() {
    let s = pg_store().await;
    let (pid, _fid) =
        create_test_workspace_and_folder(&s, &format!("selfmerge_{}", uuid::Uuid::new_v4())).await;
    let err = s.merge_workspaces(&pid, &pid).await.unwrap_err();
    assert!(err.contains("must differ"), "got: {err}");
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn merge_workspaces_errors_on_missing_ids() {
    let s = pg_store().await;
    let ghost = uuid::Uuid::new_v4();
    let (real, _fid) =
        create_test_workspace_and_folder(&s, &format!("mergemiss_{}", uuid::Uuid::new_v4())).await;
    let err = s.merge_workspaces(&ghost, &real).await.unwrap_err();
    assert!(err.contains("expected source + target to exist"), "got: {err}");
    // The real workspace is untouched.
    let exists: (bool,) = sqlx_core::query_as::query_as(
        "SELECT EXISTS(SELECT 1 FROM sensei.workspaces WHERE id = $1)",
    )
    .bind(real)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(exists.0);
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(real)
        .execute(s.pool())
        .await
        .ok();
}

// ── Workspaces tests ────────────────────────────────────────────────

#[tokio::test]
async fn workspace_create_and_get() {
    let s = pg_store().await;
    let id = s.create_workspace("_test:proj:create", Some("desc"), Some("client")).await.unwrap();
    let p = s.get_workspace(&id).await.unwrap().unwrap();
    assert_eq!(p["name"], "_test:proj:create");
    assert_eq!(p["description"], "desc");
    assert_eq!(p["client"], "client");
    assert_eq!(p["maturity"], "discovery"); // default
    s.delete_workspace(&id).await.unwrap();
}

#[tokio::test]
async fn workspace_list() {
    let s = pg_store().await;
    let id1 = s.create_workspace("_test:proj:list_a", None, None).await.unwrap();
    let id2 = s.create_workspace("_test:proj:list_b", None, None).await.unwrap();
    let all = s.list_workspaces().await.unwrap();
    let names: Vec<&str> = all.iter().filter_map(|p| p["name"].as_str()).collect();
    assert!(names.contains(&"_test:proj:list_a"));
    assert!(names.contains(&"_test:proj:list_b"));
    s.delete_workspace(&id1).await.unwrap();
    s.delete_workspace(&id2).await.unwrap();
}

#[tokio::test]
async fn list_workspaces_under_filters_by_folder_path_boundary() {
    // find_workspaces (MCP) needs a folder-scoped view: only workspaces whose
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
    let a = s.ensure_test_workspace(&format!("fpu-a-{short}")).await.unwrap();
    let fa = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &root,
            kind: "git",
            name: "a",
            path: "x/a",
            abs_path: &format!("{under}/a"),
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    crate::tasks::test_support::place_folder_in_workspace(&s, &fa, &a, "fpu-a").await.unwrap();
    // B: folder exactly equal to `under` (boundary: abs_path == under).
    let b = s.ensure_test_workspace(&format!("fpu-b-{short}")).await.unwrap();
    let fb = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &root,
            kind: "git",
            name: "b",
            path: "x",
            abs_path: &under,
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    crate::tasks::test_support::place_folder_in_workspace(&s, &fb, &b, "fpu-b").await.unwrap();
    // C: folder elsewhere under base but outside `under`.
    let c = s.ensure_test_workspace(&format!("fpu-c-{short}")).await.unwrap();
    let fc = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &root,
            kind: "git",
            name: "c",
            path: "elsewhere",
            abs_path: &format!("{base}/elsewhere"),
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    crate::tasks::test_support::place_folder_in_workspace(&s, &fc, &c, "fpu-c").await.unwrap();
    // D: sibling sharing the `under` prefix textually but across a path boundary.
    let d = s.ensure_test_workspace(&format!("fpu-d-{short}")).await.unwrap();
    let fd = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &root,
            kind: "git",
            name: "d",
            path: "x-other",
            abs_path: &format!("{under}-other/z"),
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    crate::tasks::test_support::place_folder_in_workspace(&s, &fd, &d, "fpu-d").await.unwrap();

    let scoped: Vec<String> = s
        .list_workspaces_under(Some(&under))
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
        .list_workspaces_under(None)
        .await
        .unwrap()
        .iter()
        .filter_map(|p| p["id"].as_str().map(str::to_string))
        .collect();
    for id in [&a, &b, &c, &d] {
        assert!(all.contains(&id.to_string()), "no-filter list must include every workspace");
    }
    // list_workspaces() (public no-arg) is equivalent to the None filter.
    let plain: Vec<String> = s
        .list_workspaces()
        .await
        .unwrap()
        .iter()
        .filter_map(|p| p["id"].as_str().map(str::to_string))
        .collect();
    assert!(
        plain.contains(&a.to_string()) && plain.contains(&c.to_string()),
        "list_workspaces() must stay unfiltered"
    );

    for id in [a, b, c, d] {
        s.delete_workspace(&id).await.unwrap();
    }
}

#[tokio::test]
async fn list_root_folders_excludes_nested_folder_descendants() {
    // find_workspaces (`?under=`) must return the COMPACT folder set — repo
    // roots only. The hundreds of nested `kind:'folder'` descendants are the
    // MCP token-cap bloat; list_root_folders_by_workspace drops them while
    // list_folders_by_workspace (app path) keeps the whole tree.
    let s = pg_store().await;
    let short = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
    let base = format!("/tmp/_test-rootf-{short}");
    let root =
        s.add_watch_root(&base, &format!("rootf-{short}"), &serde_json::json!([])).await.unwrap();
    let p = s.ensure_test_workspace(&format!("rootf-{short}")).await.unwrap();

    // One git repo root …
    // Each ROOT gets its own repository, and both are put in `p`. The 30
    // descendants below need none: they inherit the repo anchor's repository,
    // which is where the workspace lives.
    let f_repo = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &root,
            kind: "git",
            name: "repo",
            path: "repo",
            abs_path: &format!("{base}/repo"),
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    crate::tasks::test_support::place_folder_in_workspace(&s, &f_repo, &p, "rootf-repo")
        .await
        .unwrap();
    // … plus one standalone root …
    let f_lib = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &root,
            kind: "standalone",
            name: "lib",
            path: "lib",
            abs_path: &format!("{base}/lib"),
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    crate::tasks::test_support::place_folder_in_workspace(&s, &f_lib, &p, "rootf-lib")
        .await
        .unwrap();
    // … plus many nested `kind:'folder'` descendants (the bloat).
    for i in 0..30 {
        s.upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &root,
            kind: "folder",
            name: &format!("d{i}"),
            path: &format!("repo/src/d{i}"),
            abs_path: &format!("{base}/repo/src/d{i}"),
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    }

    let all = s.list_folders_by_workspace(&p).await.unwrap();
    assert_eq!(all.len(), 32, "full list keeps roots + all descendants");

    let roots = s.list_root_folders_by_workspace(&p).await.unwrap();
    assert_eq!(roots.len(), 2, "root list is repo roots only");
    assert!(
        roots.iter().all(|f| matches!(f["kind"].as_str(), Some("git") | Some("standalone"))),
        "root list must contain no `kind:'folder'` descendants",
    );
    // The repo root's abs_path (what cwd→workspace resolution needs) survives.
    assert!(roots.iter().any(|f| f["abs_path"] == format!("{base}/repo")));

    s.delete_workspace(&p).await.unwrap();
}

#[tokio::test]
async fn workspace_update() {
    let s = pg_store().await;
    let id = s.create_workspace("_test:proj:update", None, None).await.unwrap();
    s.update_workspace(
        &id,
        &WorkspacePatch { name: Some("renamed"), maturity: Some("active"), ..Default::default() },
    )
    .await
    .unwrap();
    let p = s.get_workspace(&id).await.unwrap().unwrap();
    assert_eq!(p["name"], "renamed");
    assert_eq!(p["maturity"], "active");
    s.delete_workspace(&id).await.unwrap();
}

#[tokio::test]
async fn workspace_update_persists_widened_identity_fields() {
    // The About-edit form PUTs goal/icon/stack/links/client/preferred_acp
    // alongside name/maturity; all must round-trip through update_workspace.
    let s = pg_store().await;
    let id = s.create_workspace("_test:proj:widen", None, None).await.unwrap();
    let icon =
        serde_json::json!({"kind":"kanji","value":"識","bg":"var(--shu-soft)","fg":"var(--shu)"});
    let stack = serde_json::json!({"languages":["rust"],"frameworks":["axum"]});
    let links =
        serde_json::json!([{"id":"1","kind":"docs","label":"Docs","url":"https://example.com"}]);
    s.update_workspace(
        &id,
        &WorkspacePatch {
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

    let p = s.get_workspace(&id).await.unwrap().unwrap();
    assert_eq!(p["goal"], "teach sensei");
    assert_eq!(p["client"], "acme");
    assert_eq!(p["maturity"], "active");
    assert_eq!(p["icon"], icon, "icon jsonb must persist verbatim");
    assert_eq!(p["stack"], stack, "stack jsonb must persist verbatim");
    assert_eq!(p["links"], links, "links jsonb must persist verbatim");

    // preferred_acp isn't in get_workspace's projection — read it directly.
    let (acp,): (Option<String>,) =
        query_as("SELECT preferred_acp FROM sensei.workspaces WHERE id = $1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(acp.as_deref(), Some("zed"), "preferred_acp text must persist");

    s.delete_workspace(&id).await.unwrap();
}

#[tokio::test]
async fn set_workspace_icon_round_trips() {
    // The inferred-icon write path persists the jsonb verbatim, overwriting
    // the '{}' default the row was created with.
    let s = pg_store().await;
    let id = s.create_workspace("_test:proj:icon", None, None).await.unwrap();
    let icon = serde_json::json!({"kind":"kanji","value":"鉄","source":"kanji_map"});
    s.set_workspace_icon(&id, &icon).await.unwrap();
    let p = s.get_workspace(&id).await.unwrap().unwrap();
    assert_eq!(p["icon"], icon, "inferred icon jsonb must persist verbatim");
    s.delete_workspace(&id).await.unwrap();
}

#[tokio::test]
async fn workspace_update_rejects_unknown_maturity() {
    // maturity is the sensei.workspace_maturity enum — an unknown value must
    // be rejected (Err → 400 at the HTTP layer), never a raw cast 500, and
    // the row must be left untouched.
    let s = pg_store().await;
    let id = s.create_workspace("_test:proj:badmaturity", None, None).await.unwrap();
    let res = s
        .update_workspace(&id, &WorkspacePatch { maturity: Some("spike"), ..Default::default() })
        .await;
    assert!(res.is_err(), "unknown maturity must be rejected");
    let p = s.get_workspace(&id).await.unwrap().unwrap();
    assert_eq!(p["maturity"], "discovery", "rejected update must not mutate the row");
    s.delete_workspace(&id).await.unwrap();
}

#[tokio::test]
async fn workspace_update_omitted_fields_unchanged() {
    // Partial-update (COALESCE) semantics: a patch that only sets `name`
    // must leave goal/client/description/maturity exactly as they were.
    let s = pg_store().await;
    let id = s
        .create_workspace("_test:proj:partial", Some("orig desc"), Some("orig client"))
        .await
        .unwrap();
    s.update_workspace(
        &id,
        &WorkspacePatch { goal: Some("g1"), maturity: Some("active"), ..Default::default() },
    )
    .await
    .unwrap();
    s.update_workspace(&id, &WorkspacePatch { name: Some("renamed2"), ..Default::default() })
        .await
        .unwrap();

    let p = s.get_workspace(&id).await.unwrap().unwrap();
    assert_eq!(p["name"], "renamed2");
    assert_eq!(p["goal"], "g1", "omitted goal must be unchanged");
    assert_eq!(p["client"], "orig client", "omitted client must be unchanged");
    assert_eq!(p["description"], "orig desc", "omitted description must be unchanged");
    assert_eq!(p["maturity"], "active", "omitted maturity must be unchanged");
    s.delete_workspace(&id).await.unwrap();
}

#[tokio::test]
async fn workspace_delete() {
    let s = pg_store().await;
    let id = s.create_workspace("_test:proj:delete", None, None).await.unwrap();
    s.delete_workspace(&id).await.unwrap();
    assert!(s.get_workspace(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn workspace_get_nonexistent() {
    let s = pg_store().await;
    let fake = uuid::Uuid::new_v4();
    assert!(s.get_workspace(&fake).await.unwrap().is_none());
}

// ── Name-duplicate phantom-workspace guard (creation + heal) ─────────

/// Guard A: the scan-time creation path is get-or-adopt, not
/// select-then-insert — a repeat call for the same name ADOPTS the existing
/// row instead of minting a second (the mechanism that produced the 0-folder
/// phantom).
#[tokio::test]
async fn get_or_create_workspace_by_name_is_idempotent() {
    let s = PgStore::test_store().await;
    let name = format!("_test:dupname:{}", uuid::Uuid::new_v4());

    let (id1, created1) = s.get_or_create_workspace_by_name(&name).await.unwrap();
    assert!(created1, "first call should mint the workspace");
    let (id2, created2) = s.get_or_create_workspace_by_name(&name).await.unwrap();
    assert!(!created2, "second call should adopt, not create");
    assert_eq!(id1, id2, "same name must resolve to the same workspace id");

    let (count,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.workspaces WHERE name = $1")
            .bind(&name)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(count, 1, "exactly one workspace row for the name");

    s.delete_workspace(&id1).await.ok();
}

/// Guard A: when a folder-bearing workspace of the name already exists, the
/// creation path adopts THAT one (not a fresh row) — no second "sensei".
#[tokio::test]
async fn get_or_create_adopts_folder_bearing_workspace_no_duplicate() {
    let s = PgStore::test_store().await;
    let name = format!("_test:dupname:{}", uuid::Uuid::new_v4());

    let keep = s.create_workspace(&name, None, None).await.unwrap();
    let fid = create_test_folder(&s, &format!("dupname-{}", uuid::Uuid::new_v4())).await;
    s.set_folder_workspace(&fid, &keep, "root", None).await.unwrap();

    let (resolved, created) = s.get_or_create_workspace_by_name(&name).await.unwrap();
    assert!(!created, "should adopt the existing folder-bearing workspace");
    assert_eq!(resolved, keep, "must resolve to the folder-bearing workspace");

    let (count,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.workspaces WHERE name = $1")
            .bind(&name)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(count, 1, "exactly one workspace row for the name");

    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(fid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_workspace(&keep).await.ok();
}

/// Guard B: a 0-folder discovery phantom sharing its name with a
/// folder-bearing workspace is pruned (merged into the survivor); its FK rows
/// (here a session) are reassigned, never orphaned; a re-run is a no-op.
#[tokio::test]
async fn heal_duplicate_name_workspaces_prunes_empty_dupe_idempotently() {
    let s = PgStore::test_store().await;
    let name = format!("_test:dupheal:{}", uuid::Uuid::new_v4());

    // Survivor: folder-bearing workspace.
    let keep = s.create_workspace(&name, None, None).await.unwrap();
    let fid = create_test_folder(&s, &format!("dupheal-{}", uuid::Uuid::new_v4())).await;
    s.set_folder_workspace(&fid, &keep, "root", None).await.unwrap();

    // Phantom: second same-name workspace, 0 folders, maturity=discovery (default),
    // carrying a session so we can prove the heal reassigns FK rows.
    let phantom = s.create_workspace(&name, None, None).await.unwrap();
    let (sess,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.sessions(folder_id, workspace_id, task) VALUES($1, $2, $3) RETURNING id"
        ).bind(fid).bind(phantom).bind("_test:dupheal-session").fetch_one(s.pool()).await.unwrap();

    s.heal_duplicate_name_workspaces().await.unwrap();

    // Phantom gone, survivor stays.
    let (phantom_exists,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT EXISTS(SELECT 1 FROM sensei.workspaces WHERE id = $1)",
    )
    .bind(phantom)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(!phantom_exists, "empty phantom should be pruned");
    assert!(s.get_workspace(&keep).await.unwrap().is_some(), "folder-bearing survivor must remain");

    // Survivor still owns its folder; the phantom's session followed it.
    let (folder_workspace,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
        "SELECT sensei.sole_workspace_of(id) FROM sensei.folders WHERE id = $1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(folder_workspace, Some(keep), "survivor keeps its folder");
    let (sess_workspace,): (Option<uuid::Uuid>,) =
        sqlx_core::query_as::query_as("SELECT workspace_id FROM activity.sessions WHERE id = $1")
            .bind(sess)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(
        sess_workspace,
        Some(keep),
        "phantom's session must be reassigned to the survivor, not orphaned"
    );

    // Idempotent: after the heal exactly one workspace remains for the name and
    // a re-run leaves it untouched.
    s.heal_duplicate_name_workspaces().await.unwrap();
    let (count,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.workspaces WHERE name = $1")
            .bind(&name)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(count, 1, "exactly one workspace remains for the name; re-run is a no-op");

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
    s.delete_workspace(&keep).await.ok();
}

/// Guard B negative: two DIFFERENT repos (different paths) that share a name
/// are BOTH folder-bearing — legitimately distinct workspaces (identity =
/// path, not name) — and must NOT be merged.
#[tokio::test]
async fn heal_leaves_two_folder_bearing_same_name_workspaces() {
    let s = PgStore::test_store().await;
    let name = format!("_test:dupneg:{}", uuid::Uuid::new_v4());

    let a = s.create_workspace(&name, None, None).await.unwrap();
    let fa = create_test_folder(&s, &format!("dupneg-a-{}", uuid::Uuid::new_v4())).await;
    s.set_folder_workspace(&fa, &a, "root", None).await.unwrap();

    let b = s.create_workspace(&name, None, None).await.unwrap();
    let fb = create_test_folder(&s, &format!("dupneg-b-{}", uuid::Uuid::new_v4())).await;
    s.set_folder_workspace(&fb, &b, "root", None).await.unwrap();

    s.heal_duplicate_name_workspaces().await.unwrap();

    assert!(
        s.get_workspace(&a).await.unwrap().is_some(),
        "first folder-bearing workspace must survive"
    );
    assert!(
        s.get_workspace(&b).await.unwrap().is_some(),
        "second folder-bearing workspace must survive"
    );
    let (count,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.workspaces WHERE name = $1")
            .bind(&name)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(count, 2, "two folder-bearing same-name workspaces must both survive");

    for (p, f) in [(a, fa), (b, fb)] {
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(f)
            .execute(s.pool())
            .await
            .ok();
        s.delete_workspace(&p).await.ok();
    }
}

// ── scope_folder_ids tests (#60) ─────────────────────────────────

/// Build an isolated workspace + root folder + child subfolder for scope tests.
async fn setup_scope_test(s: &PgStore, suffix: &str) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let proj_name = format!("_test:scope:{}", suffix);
    let proj_id = s.create_workspace(&proj_name, None, None).await.unwrap();

    // Root folder: upsert into folders_to_watch first (foreign-key for root_id).
    let watch_path = format!("/_test/scope_{}", suffix);
    let watch_id = s
        .add_watch_root(&watch_path, &format!("scope_root_{}", suffix), &serde_json::json!([]))
        .await
        .unwrap();

    // Root repo folder (kind='git', owns root_id = watch_id).
    let root_abs = format!("/_test/scope_{}/root", suffix);
    let root_name = format!("scope_root_{}", suffix);
    // Through the scan-shaped helper: the workspace is the REPOSITORY's since
    // #211, so a folder with no `repositories` row can hold no workspace and
    // every scoping read below would come back empty.
    let root_id = crate::tasks::test_support::seed_repo_folder(s, &watch_id, &root_name, &root_abs)
        .await
        .unwrap();
    s.set_folder_workspace(&root_id, &proj_id, "main", None).await.unwrap();

    // Child subfolder (kind='folder', parent = root, workspace = proj_id).
    let child_abs = format!("/_test/scope_{}/root/child", suffix);
    let child_name = format!("scope_child_{}", suffix);
    let child_id = s
        .upsert_subfolder(&watch_id, &child_name, &child_name, &child_abs, Some(&root_id))
        .await
        .unwrap();

    (proj_id, root_id, child_id)
}

#[tokio::test]
async fn scope_folder_ids_by_workspace_name_returns_all_folders() {
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
    s.delete_workspace(&proj_id).await.unwrap();
}

// ── workspace-scoped query variants tests (#60) ─────────────────────

#[tokio::test]
async fn scoped_search_and_count_across_child_folder() {
    let s = pg_store().await;
    let uid = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let (proj_id, root_id, child_id) = setup_scope_test(&s, &uid).await;
    let proj_name = format!("_test:scope:{}", uid);

    // Insert a function node in the CHILD folder.
    let fn_id = s
        .seed_node(
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
        .seed_node(
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

    // get_callers_by_name with workspace name: render_widget is called by widget_builder.
    let callers = s.get_callers_by_name(&proj_name, "render_widget").await.unwrap();
    assert!(
        callers.iter().any(|c| c["name"] == "widget_builder"),
        "expected widget_builder as caller of render_widget; got {:?}",
        callers
    );

    // get_callees_by_name with workspace name: widget_builder calls render_widget.
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
    s.delete_workspace(&proj_id).await.unwrap();
    let _ = (fn_id, tgt_id);
}

// ── One repository per checkout (#271) ─────────────────────────────────────
//
// A checkout with no remote used to mint a NEW keyless repository on every
// rescan (nulls never collide), and the reconcile rescans every ~300s: 13,346
// keyless rows for 2 indexed checkouts, 5,618 of them still linked into workspaces,
// which Settings → Workspaces listed as that workspace's repositories.

async fn drop_folder(s: &PgStore, id: &uuid::Uuid) {
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

async fn drop_repository(s: &PgStore, id: &uuid::Uuid) {
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_remote_less_checkout_keeps_one_repository_across_rescans() {
    let s = pg_store().await;
    let fid =
        create_test_folder_unattributed(&s, &format!("keyless-{}", uuid::Uuid::new_v4())).await;

    let first = s.repository_for_checkout(&fid, "keyless", None).await.unwrap();
    s.link_folder_to_repository(&fid, &first).await.unwrap();
    let second = s.repository_for_checkout(&fid, "keyless", None).await.unwrap();

    drop_folder(&s, &fid).await;
    drop_repository(&s, &first).await;
    if second != first {
        drop_repository(&s, &second).await;
    }
    assert_eq!(second, first, "a rescan reuses the checkout's own keyless repository");
}

#[tokio::test]
async fn a_keyed_checkout_resolves_by_its_remote_from_any_folder() {
    let s = pg_store().await;
    let u = uuid::Uuid::new_v4();
    let remote = format!("git@github.com:test/keyed-{u}.git");
    let a = create_test_folder_unattributed(&s, &format!("keyed-a-{u}")).await;
    let b = create_test_folder_unattributed(&s, &format!("keyed-b-{u}")).await;

    let ra = s.repository_for_checkout(&a, "keyed", Some(&remote)).await.unwrap();
    let rb = s.repository_for_checkout(&b, "keyed", Some(&remote)).await.unwrap();

    drop_folder(&s, &a).await;
    drop_folder(&s, &b).await;
    drop_repository(&s, &ra).await;
    assert_eq!(ra, rb, "two clones of one remote are one repository");
}

#[tokio::test]
async fn prune_orphan_keyless_repositories_removes_only_old_unreferenced_keyless_rows() {
    let s = pg_store().await;
    let u = uuid::Uuid::new_v4();
    let mk = |key: Option<String>, name: String| {
        let s = s.clone();
        async move {
            let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
                "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, $2) RETURNING id",
            )
            .bind(key)
            .bind(name)
            .fetch_one(s.pool())
            .await
            .unwrap();
            id
        }
    };
    let orphan = mk(None, format!("orphan-{u}")).await;
    let referenced = mk(None, format!("referenced-{u}")).await;
    let keyed = mk(Some(format!("github.com/test/keyed-orphan-{u}")), format!("keyed-{u}")).await;
    let fresh = mk(None, format!("fresh-{u}")).await;
    let fid = create_test_folder_unattributed(&s, &format!("prune-ref-{u}")).await;
    s.link_folder_to_repository(&fid, &referenced).await.unwrap();
    sqlx_core::query::query(
        "UPDATE sensei.repositories SET created_at = now() - interval '10 minutes' WHERE id = ANY($1)",
    )
    .bind(vec![orphan, referenced, keyed])
    .execute(s.pool())
    .await
    .unwrap();

    s.prune_orphan_keyless_repositories(60).await.unwrap();
    let left: Vec<uuid::Uuid> = sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
        "SELECT id FROM sensei.repositories WHERE id = ANY($1)",
    )
    .bind(vec![orphan, referenced, keyed, fresh])
    .fetch_all(s.pool())
    .await
    .unwrap()
    .into_iter()
    .map(|(id,)| id)
    .collect();

    drop_folder(&s, &fid).await;
    for id in [orphan, referenced, keyed, fresh] {
        drop_repository(&s, &id).await;
    }
    assert!(!left.contains(&orphan), "an old keyless repository no folder references is pruned");
    assert!(left.contains(&referenced), "a keyless repository a folder references is kept");
    assert!(left.contains(&keyed), "a keyed repository is kept: it carries identity and history");
    assert!(left.contains(&fresh), "a keyless repository inside the grace may be mid-link");
}

/// A repository whose parsed code is entirely tests is a Quality repository
/// (#269, #271 item 3). Only when no role is set: a chosen role is never
/// overridden, and a repository with no code is not "all tests".
#[tokio::test]
async fn a_repository_whose_code_is_all_tests_is_tagged_quality() {
    let s = pg_store().await;
    let u = uuid::Uuid::new_v4();
    let only_tests = create_test_folder_unattributed(&s, &format!("q-tests-{u}")).await;
    let mixed = create_test_folder_unattributed(&s, &format!("q-mixed-{u}")).await;
    let chosen = create_test_folder_unattributed(&s, &format!("q-chosen-{u}")).await;
    let empty = create_test_folder_unattributed(&s, &format!("q-empty-{u}")).await;
    for (fid, test_only) in [(only_tests, true), (mixed, false), (chosen, true)] {
        let t = s
            .seed_node(&fid, "function", "it_works", "tests/a.spec.ts", None, None, None, None)
            .await
            .unwrap();
        sqlx_core::query::query("UPDATE sensei.nodes SET is_test = true WHERE id = $1")
            .bind(t)
            .execute(s.pool())
            .await
            .unwrap();
        if !test_only {
            s.seed_node(&fid, "function", "real", "src/a.ts", None, None, None, None)
                .await
                .unwrap();
        }
    }
    s.update_folder_role(&chosen, Some("library")).await.unwrap();

    let assigned = s.assign_quality_roles(&[only_tests, mixed, chosen, empty]).await.unwrap();
    let roles: Vec<(uuid::Uuid, Option<String>)> = sqlx_core::query_as::query_as(
        "SELECT id, role::text FROM sensei.folders WHERE id = ANY($1)",
    )
    .bind(vec![only_tests, mixed, chosen, empty])
    .fetch_all(s.pool())
    .await
    .unwrap();
    let role = |id: uuid::Uuid| roles.iter().find(|(f, _)| *f == id).and_then(|(_, r)| r.clone());

    for fid in [only_tests, mixed, chosen, empty] {
        drop_folder(&s, &fid).await;
    }
    assert_eq!(assigned, 1);
    assert_eq!(role(only_tests).as_deref(), Some("quality"), "all tests, no role → quality");
    assert_eq!(role(mixed), None, "real code beside tests is not a quality repository");
    assert_eq!(role(chosen).as_deref(), Some("library"), "a chosen role is never overridden");
    assert_eq!(role(empty), None, "no code is not 'all tests'");
}
