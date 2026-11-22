//! Consolidation: adding a parent root absorbs the roots it encloses.
//!
//! A parent's scan covers every inner root's tree, so keeping the inner rows
//! nests roots — one tree's folders split across root_ids, and paused rows the
//! user thought they removed lingering under a watching parent. The inner ROWS
//! go; their DATA stays: folders are re-pointed at the parent (never pruned)
//! and their exclusions are rebased onto it, so the same absolute subtrees stay
//! skipped.
use super::*;

async fn root_row(pg: &PgStore, id: uuid::Uuid) -> bool {
    let (n,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.folders_to_watch WHERE id = $1")
            .bind(id)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    n > 0
}

async fn folder_root(pg: &PgStore, folder: uuid::Uuid) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) =
        sqlx_core::query_as::query_as("SELECT root_id FROM sensei.folders WHERE id = $1")
            .bind(folder)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    id
}

async fn seed_inner(
    pg: &PgStore,
    abs: &str,
    excluded: serde_json::Value,
) -> (uuid::Uuid, uuid::Uuid) {
    let root_id = pg.add_watch_root(abs, "inner", &excluded).await.unwrap();
    let folder =
        crate::tasks::test_support::seed_repo_folder(pg, &root_id, "repo", abs).await.unwrap();
    (root_id, folder)
}

/// The core move: the parent add absorbs enclosed rows, keeps every folder,
/// and says which rows went.
#[tokio::test]
async fn adding_a_parent_absorbs_the_roots_it_encloses() {
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let parent = format!("/_test/consolidate/{tag}");
    let (inner_a, folder_a) = seed_inner(&pg, &format!("{parent}/a"), serde_json::json!([])).await;
    let (inner_b, folder_b) = seed_inner(&pg, &format!("{parent}/b"), serde_json::json!([])).await;
    // Shares a PREFIX with the parent, not a path — never absorbed.
    let (sibling, folder_sibling) =
        seed_inner(&pg, &format!("{parent}-elsewhere/other"), serde_json::json!([])).await;

    let (parent_id, absorbed) =
        pg.add_watch_root_consolidated(&parent, "parent", &serde_json::json!([])).await.unwrap();

    let paths: Vec<String> = absorbed.iter().map(|r| r.path.clone()).collect();
    assert_eq!(paths, vec![format!("{parent}/a"), format!("{parent}/b")]);
    assert!(!root_row(&pg, inner_a).await && !root_row(&pg, inner_b).await, "inner rows are gone");
    assert!(root_row(&pg, parent_id).await);
    assert_eq!(
        folder_root(&pg, folder_a).await,
        parent_id,
        "folders move to the parent, not the bin"
    );
    assert_eq!(folder_root(&pg, folder_b).await, parent_id);
    let under: Vec<String> = pg
        .repositories_under_root(&parent_id)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, p, _)| p)
        .collect();
    assert!(under.contains(&format!("{parent}/a")) && under.contains(&format!("{parent}/b")));

    // The prefix-sharing sibling is untouched — row, folder, and root_id.
    assert!(root_row(&pg, sibling).await);
    assert_eq!(folder_root(&pg, folder_sibling).await, sibling);
}

/// An exclusion is a privacy boundary: the rebased entries must cover the SAME
/// absolute subtrees, and the parent's own entries must survive the merge.
#[tokio::test]
async fn consolidation_rebases_exclusions_with_identical_coverage() {
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let parent = format!("/_test/consolidate-excl/{tag}");
    let inner = format!("{parent}/inner");
    seed_inner(&pg, &inner, serde_json::json!(["skipme", "sub/deep"])).await;
    let before: Vec<String> = ["skipme", "sub/deep"]
        .iter()
        .map(|e| super::folders::resolve_exclusion(&inner, e))
        .collect();

    let (parent_id, _) = pg
        .add_watch_root_consolidated(&parent, "parent", &serde_json::json!(["mine"]))
        .await
        .unwrap();

    let (_, excluded) = pg.get_watch_root(&parent_id).await.unwrap().unwrap();
    let after: Vec<String> =
        excluded.iter().map(|e| super::folders::resolve_exclusion(&parent, e)).collect();
    for abs in &before {
        assert!(after.contains(abs), "still skipped after the move: {abs}");
    }
    assert!(
        after.contains(&super::folders::resolve_exclusion(&parent, "mine")),
        "the parent's own exclusion survives the merge"
    );
}

/// The live shape: the parent already exists and the children nested under it
/// afterwards. Re-adding the parent (an upsert) consolidates all the same.
#[tokio::test]
async fn re_adding_a_parent_absorbs_nested_children() {
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let parent = format!("/_test/consolidate-readd/{tag}");
    let parent_id = pg.add_watch_root(&parent, "parent", &serde_json::json!([])).await.unwrap();
    let (child, folder) = seed_inner(&pg, &format!("{parent}/child"), serde_json::json!([])).await;
    assert!(root_row(&pg, child).await, "the child nested under the parent");

    let (same_id, absorbed) =
        pg.add_watch_root_consolidated(&parent, "parent", &serde_json::json!([])).await.unwrap();

    assert_eq!(same_id, parent_id, "re-adding is an upsert, not a second row");
    assert_eq!(
        absorbed.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
        vec![format!("{parent}/child")]
    );
    assert!(!root_row(&pg, child).await);
    assert_eq!(folder_root(&pg, folder).await, parent_id);
}

/// One direction only: a child added under a parent nests (unchanged
/// behaviour) — the parent is never absorbed upward.
#[tokio::test]
async fn adding_a_child_leaves_its_parent_alone() {
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let parent = format!("/_test/consolidate-child/{tag}");
    let parent_id = pg.add_watch_root(&parent, "parent", &serde_json::json!([])).await.unwrap();

    let (child_id, absorbed) = pg
        .add_watch_root_consolidated(&format!("{parent}/child"), "child", &serde_json::json!([]))
        .await
        .unwrap();

    assert!(absorbed.is_empty());
    assert!(root_row(&pg, parent_id).await && root_row(&pg, child_id).await);
}

/// The rebase only changes SPELLING (inner-relative → parent-relative): the
/// absolute subtree covered is identical, so scanner and watcher — which both
/// resolve through `resolve_exclusion` — cannot tell the move happened.
#[test]
fn rebase_exclusion_spells_the_same_absolute_from_the_parent() {
    let parent = "/Users/test/Developer";
    let inner = "/Users/test/Developer/dbd";
    for entry in ["skipme", "sub/deep", "/leading/slash", "trailing/"] {
        let rebased = super::folders::rebase_exclusion(parent, inner, entry).unwrap();
        assert_eq!(
            super::folders::resolve_exclusion(parent, &rebased),
            super::folders::resolve_exclusion(inner, entry),
            "coverage must not move for {entry:?}"
        );
    }
    assert!(
        super::folders::rebase_exclusion(parent, inner, "  ").is_none(),
        "blank entries do not survive"
    );
}
