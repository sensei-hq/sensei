use super::*;

// ── resolve_repo_anchor / sensei.repo_anchor_for (spec 2026-08-18) ────────
/// Create an anchor-test folder at `abs_path` with an explicit kind + optional workspace.
async fn mk_anchor_folder(
    s: &PgStore,
    abs_path: &str,
    kind: &str,
    workspace_id: Option<uuid::Uuid>,
) -> uuid::Uuid {
    use sqlx_core::query_as::query_as;
    s.execute_raw("INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING").await.unwrap();
    let name = abs_path.rsplit('/').next().unwrap_or(abs_path);
    let row: (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) \
             VALUES('00000000-0000-0000-0000-000000000001', $1::sensei.folder_kind, $2, $2, $3) \
             ON CONFLICT(abs_path) DO UPDATE SET kind = EXCLUDED.kind RETURNING id",
    )
    .bind(kind)
    .bind(name)
    .bind(abs_path)
    .fetch_one(s.pool())
    .await
    .unwrap();
    // Membership is the REPOSITORY's (#211), so a fixture that wants this folder
    // in a workspace has to give it one and link that — the folder has no column
    // of its own to stamp.
    if let Some(pid) = workspace_id {
        let rid = s.upsert_repository(&format!("test/{}", row.0), None).await.unwrap();
        s.link_folder_to_repository(&row.0, &rid).await.unwrap();
        s.link_workspace_repository(&pid, &rid).await.unwrap();
    }
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
    mk_anchor_folder(&s, &format!("{b}/mono/packages/pkg"), "module", None).await;
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
async fn repo_anchor_standalone_workspace_root_anchors_but_bare_does_not() {
    // D1/E8: a standalone dir anchors only when it is a tracked workspace root (workspace_id set).
    let s = pg_store().await;
    let b = "/_test/anchor_std";
    let pid = s.create_workspace("_test:anchor_std", None, None).await.unwrap();
    let proj = mk_anchor_folder(&s, &format!("{b}/proj"), "standalone", Some(pid)).await;
    mk_anchor_folder(&s, &format!("{b}/bare"), "standalone", None).await; // not a workspace root
    assert_eq!(
        s.resolve_repo_anchor(&format!("{b}/proj/file")).await.unwrap().unwrap().repo_folder_id,
        proj,
        "a standalone workspace root anchors"
    );
    assert!(
        s.resolve_repo_anchor(&format!("{b}/bare/file")).await.unwrap().is_none(),
        "a workspace-less standalone dir is NOT an anchor"
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
async fn a_declared_package_resolves_to_its_parent_library() {
    // Two identities for one library never met. Detection reads package.json and
    // makes a row per PACKAGE (measured: 11 `@rokkit/*`, 7 `@devuser/dbd-*`),
    // while the manifest calls itself `rokkit` and the skills hang off THAT row.
    // So a workspace depending on `@rokkit/ui` could not reach rokkit's four curated
    // skills — nothing connected the name it knows to the name they live under.
    use crate::libraries::manifest::ProvidedSkill;
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let parent = format!("_testlib_parent_{uniq}");
    let pkg = format!("@{parent}/ui");
    let lid = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: &parent,
            ecosystem: "npm",
            version: Some(">=1.0"),
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
    s.replace_library_capabilities(
        &lid,
        "manifest",
        Some(">=1.0"),
        &[ProvidedSkill {
            name: "styling".into(),
            focus: "styling".into(),
            path: Some("p/s.md".into()),
            body: Some("# styling".into()),
        }],
        &[],
    )
    .await
    .unwrap();

    // Before the link, the package name resolves to nothing — which is the bug.
    assert_eq!(
        s.library_id_for(&pkg).await.unwrap(),
        None,
        "an unlinked package name is not a library",
    );

    let n = s.replace_library_packages(&lid, "manifest", std::slice::from_ref(&pkg)).await.unwrap();
    assert_eq!(n, 1);

    // The parent's own name still resolves — the link is an addition, not a
    // replacement of how libraries are addressed.
    assert_eq!(s.library_id_for(&parent).await.unwrap(), Some(lid));
    assert_eq!(s.library_id_for(&pkg).await.unwrap(), Some(lid), "the package resolves to it");

    // And the payoff: the package name reaches the parent's skills.
    let skills = s.list_library_skills(&pkg).await.unwrap();
    assert_eq!(skills.len(), 1, "asking by package name returns the library's skills");
    assert_eq!(skills[0]["focus"], "styling");

    // Manifest-authoritative, like the capabilities: a package dropped from the
    // manifest stops resolving instead of lingering.
    s.replace_library_packages(&lid, "manifest", &[]).await.unwrap();
    assert_eq!(
        s.library_id_for(&pkg).await.unwrap(),
        None,
        "a package removed from the manifest stops resolving",
    );

    sqlx_core::query::query("DELETE FROM sensei.libraries WHERE id = $1")
        .bind(lid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn replace_library_capabilities_is_manifest_authoritative() {
    use crate::libraries::manifest::{ProvidedAgent, ProvidedSkill};
    let s = pg_store().await;
    let lib = format!("_testlib_{}", uuid::Uuid::new_v4());
    let lid = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: &lib,
            ecosystem: "npm",
            version: Some(">=1.0"),
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
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
async fn list_workspace_library_capabilities_suggests_from_a_workspaces_deps() {
    use crate::libraries::manifest::ProvidedSkill;
    let s = pg_store().await;
    let pid =
        s.create_workspace(&format!("_libcap_{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
    let lib = format!("_libcapdep_{}", uuid::Uuid::new_v4());
    let lid = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: &lib,
            ecosystem: "npm",
            version: Some("1"),
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
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
    // The workspace depends on the library.
    s.execute_raw(&format!(
            "INSERT INTO sensei.library_enablement(library_id, workspace_id, enabled) VALUES('{lid}','{pid}',true) ON CONFLICT DO NOTHING"
        )).await.unwrap();

    let caps = s.list_workspace_library_capabilities(&pid).await.unwrap();
    let skills = caps["suggested_skills"].as_array().unwrap();
    assert!(
        skills
            .iter()
            .any(|x| x["name"] == "semantic-styles-rokkit" && x["library"] == lib.as_str()),
        "the workspace's dependency contributes its skill: {caps:?}"
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
    // path (and its subdirs) must still resolve to the folder + workspace.
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

/// EVERY package the repo owns is first-party — not just the file's own.
///
/// `World::first_party` is documented as "Every package this scan owns the source
/// of, from the manifests". The construction at `pipeline.rs:895` seeded it with
/// ONE package, so on sensei's own 16-package workspace **zero** edges ever
/// resolved across a package boundary: `sensei-cli` calls `senseid` and none of
/// it placed.
///
/// `ProcessManifest` already writes one `folders` row per manifest with
/// `kind='module'`, and `folders.name` IS the package name — so the set is one
/// indexed query over manifest-derived rows, which is exactly what the contract
/// asks for.
///
/// Breaking mutation: drop the `kind = 'module'` predicate — ordinary `folder`
/// rows flood in and every directory becomes a first-party "package", so an
/// import naming any directory resolves local.
/// A subfolder upsert CANNOT plant a workspace that contradicts its repository —
/// and now there is no parameter with which to try.
///
/// The upserts used to take `workspace_id` from the caller, which is how the
/// intent (`pipeline.rs`: "a module belongs to its repo's workspace") came to be
/// violated: the client-q `documentation` checkout ended with its root in
/// workspace `client-q` and its 362 subfolders in `documentation`. The first fix
/// DERIVED the stored column from the repo anchor and ignored the caller.
///
/// #211 removed the column and the parameter outright, so the guarantee is now
/// structural rather than enforced: a subfolder inherits its REPOSITORY, and
/// its workspace is whatever that repository's `repositories_in_workspaces` row says.
/// There is nothing left to contradict.
///
/// Breaking mutation: drop `repository_id` from the inherited CTE — the
/// subfolder inherits nothing, resolves to no workspace, and the assertion on
/// `sole_workspace_of` goes NULL.
#[tokio::test]
async fn a_subfolder_cannot_be_written_into_a_different_workspace_than_its_repo() {
    let s = pg_store().await;
    let root_id =
        s.add_watch_root(&tkey("inh", "root"), "inh-root", &serde_json::json!([])).await.unwrap();
    let owner = s.create_workspace(&tkey("inh", "owner"), None, None).await.unwrap();

    let repo_path = tkey("inh", "/repo");
    let repo = s.upsert_repo_kind(&root_id, "git", "inh-repo", &repo_path).await.unwrap();
    let repository = s.upsert_repository("inh-repo", None).await.unwrap();
    s.link_folder_to_repository(&repo, &repository).await.unwrap();
    s.set_folder_workspace(&repo, &owner, "root", None).await.unwrap();

    // The caller cannot name a workspace at all. The repository says `owner`.
    let sub_path = format!("{repo_path}/src");
    let sub = s
        .upsert_subfolder(&crate::db::pg_store::SubfolderRow {
            root_id: &root_id,
            kind: "folder",
            name: "src",
            path: &sub_path,
            abs_path: &sub_path,
            parent_id: Some(&repo),
        })
        .await
        .unwrap();

    let stored: (Option<uuid::Uuid>, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
        "SELECT sensei.sole_workspace_of(id), repository_id FROM sensei.folders WHERE id = $1",
    )
    .bind(sub)
    .fetch_one(s.pool())
    .await
    .unwrap();

    assert_eq!(
        stored.0,
        Some(owner),
        "a subfolder did not resolve to its repository's workspace — this is how \
         client-q put 362 folders in the wrong workspace"
    );

    // AND the repository itself, which is what removes `repo_anchor_for` from
    // every read. `repository_id` used to be set ONLY on the anchor, so a
    // folder's repository could be found only by walking ancestors through a
    // set-returning function — and a predicate cannot be pushed into a function
    // scan, which made `WHERE workspace = $1` a seq scan of all 13,715 folders.
    // Resolving the anchor ONCE at write time makes every read an index lookup.
    //
    // Breaking mutation: drop `repository_id` from the inherited CTE — the
    // subfolder keeps NULL and the function scan comes back.
    assert_eq!(
        stored.1,
        Some(repository),
        "a subfolder did not inherit its anchor's repository, so its workspace can \
         only be found by walking ancestors at read time"
    );

    s.remove_watch_root(&root_id).await.ok();
}

/// A folder NEVER carries its own workspace — it resolves through its repository.
///
/// `folders.workspace_id` was settable independently on all 13,697 rows, so a
/// repository's folders could disagree about their workspace, and one did: the
/// client-q `documentation` checkout's root sat in workspace `client-q` while its 362
/// subfolders sat in `documentation`. Membership now lives once in
/// `repositories_in_workspaces`, keyed on the repository, and `folder_workspaces`
/// resolves it.
///
/// SEEDS THE DRIFT RATHER THAN LOOKING FOR IT. The first version of this test
/// asserted over the live shape and was VACUOUS: the test database is clean, so
/// there was no drift to find and pointing the view back at `folders.workspace_id`
/// did not fail it. Reproducing the client-q shape is what makes the assertion
/// able to fail.
///
/// Breaking mutation: drop `repository_id` from the inherited CTE in
/// `upsert_subfolder` — the subfolder inherits no repository, resolves
/// through nothing, and reports no workspace at all.
///
/// (The ORIGINAL mutation — point `folder_workspaces` at `f.workspace_id` — is no
/// longer expressible: #211 removed the column.)
#[tokio::test]
async fn a_folder_resolves_the_workspace_of_its_repository_not_its_own_column() {
    let s = pg_store().await;
    let root_id = s
        .add_watch_root(&tkey("fproj", "root"), "fproj-root", &serde_json::json!([]))
        .await
        .unwrap();
    let owner = s.create_workspace(&tkey("fproj", "owner"), None, None).await.unwrap();

    let repo_path = tkey("fproj", "/repo");
    let repo = s.upsert_repo_kind(&root_id, "git", "fproj-repo", &repo_path).await.unwrap();
    // The anchor needs a `repositories` row: membership is keyed on the
    // repository, so a checkout with none has nowhere to record it. (Ten real
    // git folders holding 1,704 nodes are in exactly that state — see #210.)
    let repository = s.upsert_repository("fproj-repo", None).await.unwrap();
    s.link_folder_to_repository(&repo, &repository).await.unwrap();
    // The anchor belongs to `owner`, and this is what also records the junction row.
    s.set_folder_workspace(&repo, &owner, "root", None).await.unwrap();

    // A subfolder. It cannot be given a workspace of its own — there is no
    // column and no parameter — so the only answer available is its
    // repository's, which is the whole point.
    let sub_path = format!("{repo_path}/docs");
    let sub = s
        .upsert_subfolder(&crate::db::pg_store::SubfolderRow {
            root_id: &root_id,
            kind: "folder",
            name: "docs",
            path: &sub_path,
            abs_path: &sub_path,
            parent_id: Some(&repo),
        })
        .await
        .unwrap();

    let resolved: Vec<(Option<String>,)> = sqlx_core::query_as::query_as(
        "SELECT workspace FROM sensei.folder_workspaces WHERE folder_id = $1",
    )
    .bind(sub)
    .fetch_all(s.pool())
    .await
    .unwrap();

    let names: Vec<String> = resolved.into_iter().filter_map(|(n,)| n).collect();
    assert_eq!(
        names,
        vec![tkey("fproj", "owner")],
        "the subfolder did not resolve to its repository's workspace — this is the \
         client-q drift, 362 folders in the wrong workspace"
    );

    s.remove_watch_root(&root_id).await.ok();
}

#[tokio::test]
async fn first_party_packages_are_every_module_the_workspace_declares() {
    let s = pg_store().await;
    let root_id =
        s.add_watch_root(&tkey("fp", "root"), "fp-root", &serde_json::json!([])).await.unwrap();
    let repo = s.upsert_repo_kind(&root_id, "git", "fp-repo", &tkey("fp", "/repo")).await.unwrap();
    crate::tasks::test_support::give_folder_a_repository(&s, &repo, &tkey("fp", "repo"))
        .await
        .unwrap();
    let workspace = s.create_workspace(&tkey("fp", "proj"), None, None).await.unwrap();
    s.set_folder_workspace(&repo, &workspace, "root", None).await.unwrap();

    // Two manifest-declared packages, and one ordinary folder that is NOT one.
    for (name, path) in [("alpha", "/repo/crates/alpha"), ("beta", "/repo/crates/beta")] {
        let abs = tkey("fp", path);
        s.upsert_subfolder(&crate::db::pg_store::SubfolderRow {
            root_id: &root_id,
            kind: "module",
            name,
            path: &abs,
            abs_path: &abs,
            parent_id: Some(&repo),
        })
        .await
        .unwrap();
    }
    let src = tkey("fp", "/repo/src");
    s.upsert_subfolder(&crate::db::pg_store::SubfolderRow {
        root_id: &root_id,
        kind: "folder",
        name: "src",
        path: &src,
        abs_path: &src,
        parent_id: Some(&repo),
    })
    .await
    .unwrap();

    let pkgs = s.first_party_packages(&repo).await.unwrap();

    assert!(pkgs.contains("alpha"), "a manifest-declared package is first-party: {pkgs:?}");
    assert!(pkgs.contains("beta"), "AND its sibling — the whole point: {pkgs:?}");
    assert!(
        !pkgs.contains("src"),
        "an ordinary folder is not a package; treating it as one makes every \
         directory resolve local: {pkgs:?}"
    );

    s.remove_watch_root(&root_id).await.ok();
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
    s.upsert_subfolder(&crate::db::pg_store::SubfolderRow {
        root_id: &root_id,
        kind: "folder",
        name: "chart",
        path: "mono/packages/chart",
        abs_path: &member,
        parent_id: Some(&repo_fid),
    })
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
        .upsert_subfolder(&crate::db::pg_store::SubfolderRow {
            root_id: &root_id,
            kind: "folder",
            name: "lib",
            path: "app/src/lib",
            abs_path: &comp,
            parent_id: Some(&repo_fid),
        })
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

    let nid = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        &uniq,
        "x.rs",
        None,
        None,
        Some(1),
        Some(2),
    )
    .await
    .unwrap();
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
async fn get_folder_commands_marks_and_ranks_the_preferred_tool() {
    // G10: when several commands share a category, the user's dojo_preference
    // marks one preferred and ranks it first.
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:g10-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let fid = create_test_folder(&s, &format!("g10_{}", uuid::Uuid::new_v4())).await;
    s.set_folder_workspace(&fid, &pid, "primary", None).await.unwrap();
    // Two `test` commands; alphabetical order is jest, then vitest.
    sqlx_core::query::query(
            "INSERT INTO sensei.folder_commands (folder_id, raw_name, command_line, category, ecosystem)
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
    let before = s.get_folder_commands(&pid, Some("test")).await.unwrap();
    assert_eq!(before.len(), 2);
    assert_eq!(before[0]["raw_name"], "jest");
    assert!(
        before.iter().all(|c| c["preferred"] == serde_json::json!(false)),
        "no preference → none preferred"
    );

    // Prefer vitest → it is marked and ranked first (ahead of the alphabetically-first jest).
    s.upsert_command_preference("user", "test", "vitest", None).await.unwrap();
    let after = s.get_folder_commands(&pid, Some("test")).await.unwrap();
    assert_eq!(after[0]["raw_name"], "vitest", "preferred ranked first");
    assert_eq!(after[0]["preferred"], serde_json::json!(true));
    assert_eq!(after[1]["raw_name"], "jest");
    assert_eq!(
        after[1]["preferred"],
        serde_json::json!(false),
        "the non-preferred sibling isn't marked"
    );

    let pool = s.pool();
    sqlx_core::query::query("DELETE FROM sensei.folder_commands WHERE folder_id=$1")
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
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id=$1")
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

/// #238 step 2: a session's list cost is the largest /cost total any
/// SessionMeasure reported. The total only grows within a session, and a
/// dead-lettered measure can be drained after a later live one — so a smaller
/// figure arriving late must not regress the column.
#[tokio::test]
async fn raise_session_list_cost_keeps_the_largest_total() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, "sess-list-cost").await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    let id = s.record_session_event(&sid, &fid, None, "claude", false).await.unwrap();
    let read = |s: &PgStore| {
        let s = s.clone();
        async move {
            let row: (Option<f64>,) = sqlx_core::query_as::query_as(
                "SELECT list_cost_usd::float8 FROM activity.sessions WHERE id = $1",
            )
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
            row.0
        }
    };

    assert_eq!(read(&s).await, None, "no measure yet: NULL, not 0");
    assert!(s.raise_session_list_cost(&sid, 1.25).await.unwrap());
    assert_eq!(read(&s).await, Some(1.25));
    assert!(s.raise_session_list_cost(&sid, 4.5).await.unwrap());
    assert_eq!(read(&s).await, Some(4.5));
    assert!(s.raise_session_list_cost(&sid, 2.0).await.unwrap());
    assert_eq!(read(&s).await, Some(4.5), "a late, smaller total must not regress it");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

/// A measure for a session with no row is reported as a miss, never ok.
#[tokio::test]
async fn raise_session_list_cost_reports_a_missing_session() {
    let s = pg_store().await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    assert!(!s.raise_session_list_cost(&sid, 1.0).await.unwrap());
}

#[tokio::test]
async fn list_all_sessions_joins_workspace_and_uses_camelcase_times() {
    // #61: the observatory reads workspace name + startedAt/completedAt. The
    // returned row must carry the joined workspace NAME (not a bare folder
    // uuid) under camelCase timestamp keys, with completedAt set once the
    // session ends — otherwise every displayed column renders blank.
    let s = pg_store().await;
    let proj_name = format!("_test:obs-{}", uuid::Uuid::new_v4());
    let pid = s.create_workspace(&proj_name, None, None).await.unwrap();
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
        row["workspace"],
        serde_json::json!(proj_name),
        "workspace name is joined, not a folder uuid"
    );
    assert!(row["startedAt"].as_str().is_some(), "startedAt present (camelCase)");
    assert!(row.get("started_at").is_none(), "no stale snake_case started_at key");
    assert!(row["completedAt"].as_str().is_some(), "completedAt set after the end event");
    assert!(row.get("folder_id").is_none(), "folder_id no longer leaks in place of the workspace");

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
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
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
    let (pid, fid) = create_test_workspace_and_folder(&s, &suffix).await;
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
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id=$1")
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
    let pid = s
        .create_workspace(&format!("_test:sum-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
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
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn get_workspace_repos_excludes_subfolder_tree() {
    // #62: a single-repo workspace with subfolders must list only its repo
    // root(s), never the kind='folder' subfolder tree — else the UI shows it
    // as a multi-repo workspace with every folder as a repo.
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:repos-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
    let git_abs = format!("/_test/repos-git-{}", uuid::Uuid::new_v4());
    let sub_abs = format!("/_test/repos-sub-{}", uuid::Uuid::new_v4());
    let mem_abs = format!("/_test/repos-mem-{}", uuid::Uuid::new_v4());
    // ONE repository for all three, which is the shape under test: the repo root
    // and the folders beneath it belong to the same repository, and membership
    // comes from that repository rather than from each folder.
    let rid = s.upsert_repository(&format!("test/repos-{pid}"), None).await.unwrap();
    sqlx_core::query::query(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, repository_id) VALUES
               ('00000000-0000-0000-0000-000000000001','git'::sensei.folder_kind,'the-repo','the-repo',$1,$3),
               ('00000000-0000-0000-0000-000000000001','folder'::sensei.folder_kind,'subdir','subdir',$2,$3),
               ('00000000-0000-0000-0000-000000000001','module'::sensei.folder_kind,'member','member',$4,$3)"
        ).bind(&git_abs).bind(&sub_abs).bind(rid).bind(&mem_abs).execute(s.pool()).await.unwrap();
    s.link_workspace_repository(&pid, &rid).await.unwrap();

    let repos = s.get_workspace_repos(&pid).await.unwrap();
    let kinds: Vec<String> =
        repos.iter().filter_map(|r| r["kind"].as_str().map(str::to_string)).collect();
    assert!(kinds.iter().any(|k| k == "git"), "the repo root is listed: {kinds:?}");
    assert!(!kinds.iter().any(|k| k == "folder"), "kind=folder subfolders excluded: {kinds:?}");
    // D5a: monorepo members are the structural tree, NOT separate repos — else
    // a monorepo with N members regresses to an N+1-repo workspace (#62).
    assert!(!kinds.iter().any(|k| k == "module"), "kind=module excluded from repos: {kinds:?}");

    sqlx_core::query::query(
        "DELETE FROM sensei.folders f USING sensei.folder_workspaces fp \
          WHERE fp.folder_id = f.id AND fp.workspace_id = $1",
    )
    .bind(pid)
    .execute(s.pool())
    .await
    .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn upsert_subfolder_relabels_structural_but_preserves_root() {
    // D5a: upsert_subfolder relabels between the two STRUCTURAL kinds
    // (folder ↔ workspace_member) on conflict, but NEVER reclassifies a path
    // that is actually a nested workspace ROOT (git/standalone/subtree).
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
    s.upsert_subfolder(&crate::db::pg_store::SubfolderRow {
        root_id: &rid,
        kind: "folder",
        name: "a",
        path: "a",
        abs_path: &a,
        parent_id: None,
    })
    .await
    .unwrap();
    assert_eq!(kind_at(&s, a.clone()).await, "folder", "first upsert is a plain folder");
    s.upsert_subfolder(&crate::db::pg_store::SubfolderRow {
        root_id: &rid,
        kind: "module",
        name: "a",
        path: "a",
        abs_path: &a,
        parent_id: None,
    })
    .await
    .unwrap();
    assert_eq!(kind_at(&s, a.clone()).await, "module", "relabelled folder → workspace_member");

    // A nested workspace root (subtree) must NOT be reclassified by a member upsert.
    let b = format!("/_test/sfk-b-{}", uuid::Uuid::new_v4());
    s.upsert_repo_kind(&rid, "subtree", "b", &b).await.unwrap();
    s.upsert_subfolder(&crate::db::pg_store::SubfolderRow {
        root_id: &rid,
        kind: "module",
        name: "b",
        path: "b",
        abs_path: &b,
        parent_id: None,
    })
    .await
    .unwrap();
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
async fn workspaces_with_session_activity_reports_the_workspace() {
    // #67: the scheduler reads (workspace_id, latest activity) to decide what
    // to re-analyze. A workspace with attributed sessions must appear.
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:act-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let fid = create_test_folder(&s, &format!("act-{}", uuid::Uuid::new_v4())).await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();

    let activity = s.get_workspaces_with_session_activity().await.unwrap();
    let row =
        activity.iter().find(|(p, _)| *p == pid).expect("workspace appears in session-activity");
    assert!(row.1.timestamp() > 0, "carries a real latest-activity timestamp");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE workspace_id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn workspace_ftr_and_quality_decode_numeric_metrics() {
    // Regression: the headline (Σ props / value) and the daily AVG(...) trend /
    // ftr_7d / avg_duration_ms are all NUMERIC; without ::float8 casts sqlx
    // fails to decode into f64 and the endpoint 500s (masked by the client's
    // default-on-error). The workspace must have BOTH a stored `ftr` row (so the
    // headline decodes a real number, not a short-circuiting NULL) AND an
    // analyzed session in the window (so the inline trend decodes a numeric row).
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:ftr-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let fid = create_test_folder(&s, &format!("ftr-{}", uuid::Uuid::new_v4())).await;
    let sid = format!("_test-sid-{}", uuid::Uuid::new_v4());
    let session_id = s.record_session_event(&sid, &fid, Some(&pid), "claude", true).await.unwrap();
    s.update_session_metrics(&SessionMetricsRow {
        session_id: &session_id,
        turns: 3,
        corrections: 0,
        outcome: "completed",
        ftr: true,
        duration_ms: Some(1000),
        claude_ms: Some(1000),
        user_ms: Some(0),
        module: None,
        tool_usage: &serde_json::json!({}),
    })
    .await
    .unwrap();
    // Stored daily ftr row in the 14d window → the headline decodes a real value.
    let (ftr_mid,): (uuid::Uuid,) = query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
        .fetch_one(s.pool())
        .await
        .unwrap();
    // Repo-grain (_v2): the shared `ftr` metric_id is unique per (metric, repo,
    // user, day), so seed a per-test repository — a NULL-repository row would
    // collide with a sibling workspace's `ftr` row on the same day.
    let (rid,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftr-decode') RETURNING id",
    )
    .bind(format!("test/ftr-decode-{}", uuid::Uuid::new_v4()))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_workspace(&s, &rid, &pid, "ftr-decode").await;
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: chrono::Utc::now().date_naive(),
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({"numerator": 1, "denominator": 1}),
        source: "measured",
    })
    .await
    .unwrap();

    let ftr = s.get_workspace_ftr(&pid).await.expect("get_workspace_ftr decodes numeric metrics");
    assert!(ftr["ftr14d"].as_f64().is_some(), "ftr14d decodes a real number from the stored row");
    assert!(
        ftr["ftrTrend"].as_array().is_some_and(|a| !a.is_empty()),
        "daily trend decodes a numeric row"
    );
    s.get_quality_signals(&pid).await.expect("get_quality_signals decodes numeric metrics");
    s.get_tool_usage_stats().await.expect("get_tool_usage_stats decodes numeric avg_duration_ms");

    sqlx_core::query::query("DELETE FROM activity.sessions WHERE workspace_id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn library_upsert_updates() {
    let s = pg_store().await;
    let id1 = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: "_test:react",
            ecosystem: "npm",
            version: Some("18"),
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
    let id2 = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: "_test:react",
            ecosystem: "npm",
            version: Some("19"),
            description: Some("UI library"),
            source_type: None,
            base_url: None,
        })
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
    let id1 = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: "_test:lib_a",
            ecosystem: "npm",
            version: None,
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
    let id2 = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: "_test:lib_b",
            ecosystem: "cargo",
            version: None,
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
    let all = s.list_libraries().await.unwrap();
    assert!(all.iter().any(|l| l["name"] == "_test:lib_a"));
    assert!(all.iter().any(|l| l["name"] == "_test:lib_b"));
    s.delete_library(&id1).await.unwrap();
    s.delete_library(&id2).await.unwrap();
}

#[tokio::test]
async fn library_delete() {
    let s = pg_store().await;
    let id = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: "_test:deleteme",
            ecosystem: "npm",
            version: None,
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
    s.delete_library(&id).await.unwrap();
    assert!(s.get_library(&id).await.unwrap().is_none());
}
