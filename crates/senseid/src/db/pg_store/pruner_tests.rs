//! The pruner (#247), against a seeded corpus.
//!
//! Three shapes the pruner can get wrong, each seeded on purpose: a repository
//! with nodes under a nested folder, which exercises `nodes → files ON DELETE
//! RESTRICT` inside a folder cascade; a sibling whose path shares a PREFIX with
//! the pruned one (`a` and `ab`); and a repository that a second checkout still
//! points at, which must survive.
use super::graph_seed::SeedGraph;
use super::*;

struct Corpus {
    root_id: uuid::Uuid,
    root: String,
    a: uuid::Uuid,
    a_sub: uuid::Uuid,
    ab: uuid::Uuid,
    shared_here: uuid::Uuid,
    shared_there: uuid::Uuid,
    a_repo: uuid::Uuid,
    shared_repo: uuid::Uuid,
    a_project: uuid::Uuid,
}

async fn seed(pg: &PgStore) -> Corpus {
    let tag = uuid::Uuid::new_v4();
    let root = format!("/_test/prune/{tag}");
    let root_id = pg.add_watch_root(&root, "prune-wt", &serde_json::json!([])).await.unwrap();
    let repo = |name: &'static str| {
        let abs = format!("{root}/{name}");
        async move {
            crate::tasks::test_support::seed_repo_folder(
                pg,
                &root_id,
                &format!("{name}-{tag}"),
                &abs,
            )
            .await
            .unwrap()
        }
    };
    let a = repo("a").await;
    let ab = repo("ab").await;
    let shared_here = repo("shared").await;
    let a_sub = pg
        .upsert_folder(&root_id, "folder", "sub", "a/sub", &format!("{root}/a/sub"), Some(&a), None)
        .await
        .unwrap();

    // A second checkout of `shared`, outside the pruned path, pointing at the
    // SAME repository row.
    let shared_there = pg
        .upsert_repo(&root_id, &format!("shared-copy-{tag}"), &format!("{root}-elsewhere/shared"))
        .await
        .unwrap();
    let repo_of = |folder: uuid::Uuid| async move {
        let (r,): (Option<uuid::Uuid>,) =
            sqlx_core::query_as::query_as("SELECT repository_id FROM sensei.folders WHERE id = $1")
                .bind(folder)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        r.unwrap()
    };
    let a_repo = repo_of(a).await;
    let shared_repo = repo_of(shared_here).await;
    pg.link_folder_to_repository(&shared_there, &shared_repo).await.unwrap();

    // `a` is a project's only repository, so pruning it empties the project.
    let a_project = pg.create_project(&format!("_test:prune:a:{tag}"), None, None).await.unwrap();
    pg.link_project_repository(&a_project, &a_repo).await.unwrap();

    // Nodes in BOTH `a` and its nested folder, plus an edge between them — the
    // RESTRICT path only fires when a node's file is deleted in the cascade.
    let def = |path: &'static str| FqnDef {
        file_path: path,
        signature: None,
        line_start: Some(1),
        line_end: Some(2),
        is_exported: true,
        parent_id: None,
    };
    let top = pg
        .seed_node_by_fqn(
            &a,
            &format!("rust·p{tag}·m·top·item"),
            "function",
            "top",
            Some("rust"),
            Some(def("lib.rs")),
        )
        .await
        .unwrap();
    let deep = pg
        .seed_node_by_fqn(
            &a_sub,
            &format!("rust·p{tag}·s·deep·item"),
            "function",
            "deep",
            Some("rust"),
            Some(def("x.rs")),
        )
        .await
        .unwrap();
    pg.insert_edge(&a, &top, Some(&deep), None, None, "calls").await.unwrap();
    // ...and one in the prefix-sharing sibling, which must survive.
    pg.seed_node_by_fqn(
        &ab,
        &format!("rust·q{tag}·m·keep·item"),
        "function",
        "keep",
        Some("rust"),
        Some(def("lib.rs")),
    )
    .await
    .unwrap();

    Corpus {
        root_id,
        root,
        a,
        a_sub,
        ab,
        shared_here,
        shared_there,
        a_repo,
        shared_repo,
        a_project,
    }
}

async fn exists(pg: &PgStore, sql: &str, id: uuid::Uuid) -> bool {
    let (n,): (i64,) =
        sqlx_core::query_as::query_as(sql).bind(id).fetch_one(pg.pool()).await.unwrap();
    n > 0
}

async fn folder(pg: &PgStore, id: uuid::Uuid) -> bool {
    exists(pg, "SELECT count(*) FROM sensei.folders WHERE id = $1", id).await
}

async fn repository(pg: &PgStore, id: uuid::Uuid) -> bool {
    exists(pg, "SELECT count(*) FROM sensei.repositories WHERE id = $1", id).await
}

async fn nodes_in(pg: &PgStore, folder: uuid::Uuid) -> bool {
    exists(pg, "SELECT count(*) FROM sensei.nodes WHERE folder_id = $1", folder).await
}

async fn cleanup(pg: &PgStore, c: &Corpus) {
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(c.root_id)
        .execute(pg.pool())
        .await
        .ok();
    for r in [c.a_repo, c.shared_repo] {
        sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
            .bind(r)
            .execute(pg.pool())
            .await
            .ok();
    }
    pg.delete_project(&c.a_project).await.ok();
}

/// Mutations that must break this: drop the repository delete, drop the
/// project prune, or match `starts_with(abs_path, $1)` without the slash.
#[tokio::test]
async fn pruning_a_repository_root_removes_everything_it_owned_and_nothing_beside_it() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let c = seed(&pg).await;

    let report = pg.prune_repository_root(&format!("{}/a/", c.root)).await.unwrap();

    assert_eq!(report.folders, 2, "`a` and `a/sub` — the trailing slash is not a different path");
    assert!(!folder(&pg, c.a).await && !folder(&pg, c.a_sub).await);
    assert!(
        !nodes_in(&pg, c.a).await && !nodes_in(&pg, c.a_sub).await,
        "the graph goes with the folders"
    );
    assert_eq!(report.repositories, 1);
    assert!(!repository(&pg, c.a_repo).await, "nothing else points at `a`'s repository");
    assert!(report.projects >= 1, "{report:?}");
    assert!(
        pg.get_project(&c.a_project).await.unwrap().is_none(),
        "`a` was the project's only repository, so the project is empty"
    );

    // `ab` shares a PREFIX with `a`, not a path.
    assert!(folder(&pg, c.ab).await, "a sibling sharing a prefix survives");
    assert!(nodes_in(&pg, c.ab).await);

    cleanup(&pg, &c).await;
}

/// Mutation that must break this: delete every repository the pruned folders
/// pointed at, without the `NOT EXISTS` guard.
#[tokio::test]
async fn a_repository_another_checkout_still_uses_survives() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let c = seed(&pg).await;

    let report = pg.prune_repository_root(&format!("{}/shared", c.root)).await.unwrap();

    assert_eq!(report.folders, 1);
    assert!(!folder(&pg, c.shared_here).await);
    assert_eq!(report.repositories, 0, "{report:?}");
    assert!(repository(&pg, c.shared_repo).await, "the other checkout still points at it");
    assert!(folder(&pg, c.shared_there).await);

    cleanup(&pg, &c).await;
}

/// Pruning a path nothing lives under is a report of zeros, not an error — the
/// UI may offer to remove something a scan already pruned.
#[tokio::test]
async fn pruning_nothing_reports_nothing() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let report = pg
        .prune_repository_root(&format!("/_test/prune/{}/none", uuid::Uuid::new_v4()))
        .await
        .unwrap();
    assert_eq!((report.folders, report.repositories), (0, 0));
}

/// The root that owns a path is found on a SEGMENT boundary, and an underscore
/// in a root's path is a character, not a wildcard. `LIKE path || '/%'` read
/// `my_dir` as "my, any one character, dir", so the root at `/…/my_dir` claimed
/// `/…/myXdir/…` — and the pruner would add an exclusion to the wrong root.
///
/// Mutation that must break this: restore the `LIKE` predicate.
#[tokio::test]
async fn the_enclosing_root_is_matched_literally_not_as_a_like_pattern() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    // NOT under `/_test`: the shared fixture registers a root there, which
    // legitimately encloses anything beneath it.
    let base = format!("/_encl_test/{}", uuid::Uuid::new_v4());
    let root = format!("{base}/my_dir");
    let id = pg.add_watch_root(&root, "encl", &serde_json::json!([])).await.unwrap();

    let inside = pg.enclosing_watch_root(&format!("{root}/repo")).await.unwrap();
    assert_eq!(inside.map(|(i, _)| i), Some(id));
    assert!(
        pg.enclosing_watch_root(&format!("{base}/myXdir/repo")).await.unwrap().is_none(),
        "`_` is not a wildcard"
    );
    assert!(pg.enclosing_watch_root(&format!("{root}-old/repo")).await.unwrap().is_none());

    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(id)
        .execute(pg.pool())
        .await
        .ok();
}

/// A paused root keeps its data and is not synced: boot, the reconcile tick and
/// the version rescan all enumerate roots through `list_watch_roots_to_sync`.
///
/// Mutation that must break this: drop the status filter.
#[tokio::test]
async fn a_paused_root_is_not_a_root_to_sync() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let root = format!("/_test/paused/{}", uuid::Uuid::new_v4());
    let id = pg.add_watch_root(&root, "paused", &serde_json::json!([])).await.unwrap();
    let ids = |rows: Vec<serde_json::Value>| -> Vec<String> {
        rows.iter().filter_map(|r| r["id"].as_str().map(str::to_string)).collect()
    };

    assert!(ids(pg.list_watch_roots_to_sync().await.unwrap()).contains(&id.to_string()));
    pg.update_watch_status(&id, "paused").await.unwrap();
    assert!(!ids(pg.list_watch_roots_to_sync().await.unwrap()).contains(&id.to_string()));
    assert!(
        ids(pg.list_watch_roots().await.unwrap()).contains(&id.to_string()),
        "the settings screen still lists it, so it can be resumed or removed"
    );

    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(id)
        .execute(pg.pool())
        .await
        .ok();
}

/// A prune removes the projects IT emptied, and no other. Calling the global
/// `prune_empty_projects(0)` deleted every empty discovery project in the
/// database — a project the user created a moment ago and has not yet given a
/// repository included.
///
/// Mutation that must break this: prune projects without the id scope.
#[tokio::test]
async fn a_prune_leaves_an_unrelated_empty_project_alone() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let c = seed(&pg).await;
    let bystander = pg
        .create_project(&format!("_test:prune:bystander:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    // Old enough that no grace window protects it — only the scope can.
    sqlx_core::query::query(
        "UPDATE sensei.projects SET modified_at = now() - interval '1 day' WHERE id = $1",
    )
    .bind(bystander)
    .execute(pg.pool())
    .await
    .unwrap();

    pg.prune_repository_root(&format!("{}/a", c.root)).await.unwrap();

    assert!(pg.get_project(&bystander).await.unwrap().is_some(), "not this prune's project");
    assert!(pg.get_project(&c.a_project).await.unwrap().is_none(), "but `a`'s project is");

    pg.delete_project(&bystander).await.ok();
    cleanup(&pg, &c).await;
}
