//! DB-touching tests for the node WRITE path (#161).
//!
//! `upsert_node_ex` took nine positional arguments and carried a comment
//! defending them: *"The arguments ARE the columns. A struct here would restate
//! the same names one indirection away without removing a single one."*
//!
//! The verbosity claim is true and beside the point. The value of a row struct
//! is not fewer names — it is that EXHAUSTIVE DESTRUCTURING turns a field added
//! to the producer into a compile error. A positional list cannot do that, which
//! is why `extract_return_type` ran on every rust function for months while no
//! return type reached a column: there was no ninth argument, and nothing
//! anywhere failed.
//!
//! This is the net for converting it. It asserts the round trip of every column
//! the writer touches, each with a value nothing else would produce, so a field
//! that stops being carried — or lands in the wrong column — fails here.
use super::*;

/// The columns read back, in the order the SELECT names them. A type alias
/// because clippy is right that an eight-wide tuple is unreadable — and because
/// the read-back being positional is the hazard the write path just stopped
/// having. Every field is asserted by name below, so a transposition here fails.
type StoredNode =
    (String, String, Option<uuid::Uuid>, Option<String>, Option<i32>, Option<i32>, bool, bool);

/// Every column `upsert_node_ex` writes survives the write, in its own column.
///
/// `parent_id`, `signature`, `line_start`, `line_end` and `is_exported` are the
/// five a conversion can silently drop: the first four are `Option`, and the
/// last is the one `upsert_node` defaults for its callers. All five get a value
/// that is NOT the default.
///
/// Mutation that must break this test: transpose the `.bind(line_start)` and
/// `.bind(line_end)` lines, or bind a literal `false` for `is_exported`.
#[tokio::test]
async fn every_column_a_node_write_touches_survives_the_round_trip() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let uniq = uuid::Uuid::new_v4();
    let root = format!("/_test/nodes/{uniq}");
    let root_id = pg.add_watch_root(&root, "node-wt", &serde_json::json!([])).await.unwrap();
    let folder_id = pg.upsert_repo(&root_id, "nodes", &root).await.unwrap();

    // The file barrier (R13): a node naming an untracked file is refused, so the
    // `files` row comes first — the same order production writes them in.
    let file_path = "deep/a.rs";
    pg.upsert_file_row(&folder_id, file_path, 1, "seed", None).await.unwrap();

    let parent = pg
        .upsert_node(&NodeRow {
            folder_id: &folder_id,
            kind: "module",
            name: "parent",
            file_path,
            parent_id: None,
            signature: None,
            line_start: None,
            line_end: None,
            is_exported: false,
        })
        .await
        .unwrap();

    let id = pg
        .upsert_node(&NodeRow {
            folder_id: &folder_id,
            kind: "function",
            name: "the_name",
            file_path,
            parent_id: Some(&parent),
            signature: Some("fn the_name(x: u8) -> u8"),
            line_start: Some(11),
            line_end: Some(29),
            is_exported: true,
        })
        .await
        .unwrap();

    let (kind, name, parent_id, signature, line_start, line_end, is_exported, file_is_set): StoredNode = sqlx_core::query_as::query_as(
        "SELECT kind::text, name, parent_id, signature, line_start, line_end,
                is_exported, file_id IS NOT NULL
           FROM sensei.nodes WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pg.pool())
    .await
    .unwrap();

    assert_eq!(kind, "function");
    assert_eq!(name, "the_name");
    assert_eq!(parent_id, Some(parent), "parent_id");
    assert_eq!(signature.as_deref(), Some("fn the_name(x: u8) -> u8"), "signature");
    assert_eq!(line_start, Some(11), "line_start — distinct from line_end, so a swap fails");
    assert_eq!(line_end, Some(29), "line_end");
    assert!(is_exported, "is_exported — NOT the default `upsert_node` supplies");
    assert!(file_is_set, "file_id resolved from the path");

    cleanup(&pg, &folder_id, &root_id).await;
}

/// The upsert is STABLE: the same identity returns the same row, and the
/// mutable columns move while the id does not.
///
/// `nodes_unique_identity` is `(folder_id, file_id, kind, name, parent_id,
/// line_start)` with NULLS NOT DISTINCT. A conversion that dropped a key column
/// would mint a second node on re-scan instead of updating — and a second node
/// is not a failure anywhere, it is just a graph with two of something.
///
/// Mutation that must break this test: change `DO UPDATE` to `DO NOTHING`, or
/// drop `line_start` from the conflict target.
#[tokio::test]
async fn re_writing_one_identity_keeps_the_same_node_and_moves_the_rest() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let uniq = uuid::Uuid::new_v4();
    let root = format!("/_test/nodes-stable/{uniq}");
    let root_id = pg.add_watch_root(&root, "node-wt2", &serde_json::json!([])).await.unwrap();
    let folder_id = pg.upsert_repo(&root_id, "nodes2", &root).await.unwrap();
    let file_path = "b.rs";
    pg.upsert_file_row(&folder_id, file_path, 1, "seed", None).await.unwrap();

    let first = pg
        .upsert_node(&NodeRow {
            folder_id: &folder_id,
            kind: "function",
            name: "same",
            file_path,
            parent_id: None,
            signature: Some("v1"),
            line_start: Some(3),
            line_end: Some(4),
            is_exported: false,
        })
        .await
        .unwrap();
    let again = pg
        .upsert_node(&NodeRow {
            folder_id: &folder_id,
            kind: "function",
            name: "same",
            file_path,
            parent_id: None,
            signature: Some("v2"),
            line_start: Some(3),
            line_end: Some(9),
            is_exported: true,
        })
        .await
        .unwrap();
    assert_eq!(first, again, "the same identity is the same row");

    let (signature, line_end, is_exported): (Option<String>, Option<i32>, bool) =
        sqlx_core::query_as::query_as(
            "SELECT signature, line_end, is_exported FROM sensei.nodes WHERE id = $1",
        )
        .bind(first)
        .fetch_one(pg.pool())
        .await
        .unwrap();
    assert_eq!(signature.as_deref(), Some("v2"), "the second write's signature won");
    assert_eq!(line_end, Some(9), "and its line_end");
    assert!(is_exported, "and its visibility");

    // A different `line_start` is a different identity — that column is in the key.
    let moved = pg
        .upsert_node(&NodeRow {
            folder_id: &folder_id,
            kind: "function",
            name: "same",
            file_path,
            parent_id: None,
            signature: Some("v3"),
            line_start: Some(77),
            line_end: Some(80),
            is_exported: false,
        })
        .await
        .unwrap();
    assert_ne!(first, moved, "line_start is part of the identity");

    cleanup(&pg, &folder_id, &root_id).await;
}

async fn cleanup(pg: &PgStore, folder_id: &uuid::Uuid, root_id: &uuid::Uuid) {
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(folder_id)
        .execute(pg.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root_id)
        .execute(pg.pool())
        .await
        .ok();
}

/// RE-MINTING AN UNCHANGED LIBRARY NODE DOES NOT MOVE IT.
///
/// `upsert_lib_node_by_fqn` stamped `modified_at = now()` on every conflicting
/// insert, so a node nothing had changed looked freshly written on every scan.
/// Nothing noticed while only call and reference targets reached library
/// surface, because `reconcile`'s idempotence test imported a name its fixture
/// never used. Once imports started placing there (#242) a re-index of an
/// untouched file churned two rows per external dependency.
///
/// Both rows are asserted: the symbol AND the `package` container it hangs off,
/// which is a second statement with the same defect.
///
/// Mutation that must break this test: replace either `modified_at` CASE with a
/// bare `now()`.
#[tokio::test]
async fn re_minting_an_unchanged_library_node_does_not_move_it() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let uniq = uuid::Uuid::new_v4();
    let root = format!("/_test/libnodes/{uniq}");
    let root_id = pg.add_watch_root(&root, "lib-wt", &serde_json::json!([])).await.unwrap();
    let folder_id = pg.upsert_repo(&root_id, "libnodes", &root).await.unwrap();

    let fqn = format!("lib·probe-{uniq}·collections::BTreeMap");
    let package = format!("probe-{uniq}");
    let mint = || pg.upsert_lib_node_by_fqn(&folder_id, &fqn, "BTreeMap", &package, Some("rust"));
    mint().await.unwrap();

    async fn stamp(pg: &PgStore, fqn: &str) -> chrono::DateTime<chrono::Utc> {
        let (at,): (chrono::DateTime<chrono::Utc>,) =
            sqlx_core::query_as::query_as("SELECT modified_at FROM sensei.nodes WHERE fqn = $1")
                .bind(fqn)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        at
    }
    let container = format!("lib·{package}");
    let before = (stamp(&pg, &fqn).await, stamp(&pg, &container).await);

    // The SAME write again, exactly as a re-scan of an unchanged file makes it.
    mint().await.unwrap();
    let after = (stamp(&pg, &fqn).await, stamp(&pg, &container).await);

    assert_eq!(after.0, before.0, "the symbol row did not change, so it must not look written");
    assert_eq!(after.1, before.1, "nor the package container it hangs off");

    cleanup(&pg, &folder_id, &root_id).await;
}
