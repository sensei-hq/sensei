//! The Neighbourhood diagram's reads (#220), against a seeded corpus.
//!
//! `analysis::neighbourhood` owns the walk and tests it without a database.
//! What only Postgres can show is that each hop reads the right rows: callers
//! from THIS project, a library callee marked as a leaf, a call's weight as its
//! call sites rather than its edge rows, and the two unplaced counts kept apart.
use super::graph_seed::SeedGraph;
use super::*;
use crate::analysis::neighbourhood::{Call, Side};

struct Corpus {
    project: uuid::Uuid,
    other_project: uuid::Uuid,
    folders: Vec<uuid::Uuid>,
    focus: uuid::Uuid,
    busy_caller: uuid::Uuid,
    quiet_caller: uuid::Uuid,
    callee: uuid::Uuid,
    library: uuid::Uuid,
    foreign_caller: uuid::Uuid,
}

/// `n` call sites in one file, as the indexer records them.
fn sites(n: usize) -> serde_json::Value {
    let uses: Vec<_> = (0..n)
        .map(|i| serde_json::json!({ "at": [i + 1, 0, i + 1, 5], "fact": "use", "kind": "calls" }))
        .collect();
    serde_json::json!({ "occurrences": { "a.rs": uses } })
}

/// One unplaced call site, with the reason the ladder stopped.
fn missed(reason: &str) -> serde_json::Value {
    serde_json::json!({ "occurrences": { "a.rs": [
        { "at": [20, 0, 20, 5], "fact": "use", "kind": "calls", "reason": reason }
    ] } })
}

async fn seed(pg: &PgStore) -> Corpus {
    let tag = uuid::Uuid::new_v4();
    let mut folders = vec![];
    let place = |name: &'static str| {
        let root = format!("/_test/hood/{tag}/{name}");
        async move {
            let root_id = pg.add_watch_root(&root, name, &serde_json::json!([])).await.unwrap();
            let folder = pg.upsert_repo(&root_id, &format!("{name}-{tag}"), &root).await.unwrap();
            let project =
                pg.create_project(&format!("_test:hood:{name}:{tag}"), None, None).await.unwrap();
            crate::tasks::test_support::place_folder_in_project(
                pg,
                &folder,
                &project,
                &format!("{name}-{tag}"),
            )
            .await
            .unwrap();
            (folder, project)
        }
    };
    let (mine, project) = place("mine").await;
    let (theirs, other_project) = place("theirs").await;
    folders.push(mine);
    folders.push(theirs);

    let def = |line| FqnDef {
        file_path: "a.rs",
        signature: None,
        line_start: Some(line),
        line_end: Some(line + 1),
        is_exported: true,
        parent_id: None,
    };
    let node = |folder, fqn: String, name: &'static str, line| async move {
        pg.seed_node_by_fqn(&folder, &fqn, "function", name, Some("rust"), Some(def(line)))
            .await
            .unwrap()
    };
    let focus = node(mine, format!("rust·h{tag}·m·focus"), "focus", 1).await;
    let busy_caller = node(mine, format!("rust·h{tag}·m·busy"), "busy", 3).await;
    let quiet_caller = node(mine, format!("rust·h{tag}·m·quiet"), "quiet", 5).await;
    let callee = node(mine, format!("rust·h{tag}·m·callee"), "callee", 7).await;
    let library = node(mine, format!("lib·serde{tag}·json·to_string"), "to_string", 9).await;
    let foreign_caller = node(theirs, format!("rust·t{tag}·m·foreign"), "foreign", 1).await;

    let call = |folder, source, target: Option<uuid::Uuid>, name: Option<&'static str>, n| async move {
        pg.insert_edge_with_props(
            &folder,
            &source,
            target.as_ref(),
            name,
            None,
            "calls",
            &sites(n),
        )
        .await
        .unwrap();
    };
    call(mine, busy_caller, Some(focus), None, 4).await;
    call(mine, quiet_caller, Some(focus), None, 1).await;
    call(mine, focus, Some(callee), None, 2).await;
    call(mine, focus, Some(library), None, 1).await;
    // Another project's code calling the focus. Real, but not THIS project's
    // neighbourhood — the callers column is scoped like every sibling screen.
    call(theirs, foreign_caller, Some(focus), None, 1).await;
    // Unplaced: one the focus makes, and one that NAMES the focus. Plus a
    // `plumbing` call, which is a verdict rather than a gap and must not count.
    let miss = |source, name: &'static str, reason: &'static str| async move {
        pg.insert_edge_with_props(&mine, &source, None, Some(name), None, "calls", &missed(reason))
            .await
            .unwrap();
    };
    miss(focus, "mystery", "receiver_type_unknown").await;
    miss(focus, "elsewhere", "no_import_in_scope").await;
    miss(focus, "clone", "plumbing").await;
    miss(quiet_caller, "focus", "receiver_type_unknown").await;
    miss(busy_caller, "focus", "plumbing").await;

    Corpus {
        project,
        other_project,
        folders,
        focus,
        busy_caller,
        quiet_caller,
        callee,
        library,
        foreign_caller,
    }
}

async fn cleanup(pg: &PgStore, c: &Corpus) {
    for f in &c.folders {
        sqlx_core::query::query(
            "DELETE FROM sensei.folders_to_watch w USING sensei.folders f
              WHERE f.root_id = w.id AND f.id = $1",
        )
        .bind(f)
        .execute(pg.pool())
        .await
        .ok();
    }
    pg.delete_project(&c.project).await.ok();
    pg.delete_project(&c.other_project).await.ok();
}

fn by_far(calls: &[Call], side: Side) -> Vec<(uuid::Uuid, i64, bool)> {
    let mut v: Vec<_> = calls
        .iter()
        .map(|c| {
            let far = if side == Side::In { c.source } else { c.target };
            (far, c.occurrences, c.far_walkable)
        })
        .collect();
    v.sort();
    v
}

/// Mutations that must break this: drop the project scope on the `In` hop, or
/// weight by `count(*)` of edge rows instead of call sites.
#[tokio::test]
async fn the_callers_hop_stays_in_the_project_and_weighs_call_sites() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let c = seed(&pg).await;

    let calls = pg.neighbourhood_hop(&c.project, &[c.focus], Side::In).await.unwrap();
    let mut want = vec![(c.busy_caller, 4, true), (c.quiet_caller, 1, true)];
    want.sort();
    assert_eq!(by_far(&calls, Side::In), want, "foreign_caller is another project's");
    assert!(calls.iter().all(|k| k.source != c.foreign_caller));

    cleanup(&pg, &c).await;
}

/// Mutation that must break this: compute `far_walkable` without the `lib·`
/// test, or include unplaced calls (target NULL) in the hop.
#[tokio::test]
async fn the_callees_hop_reaches_a_library_but_marks_it_a_leaf() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let c = seed(&pg).await;

    let calls = pg.neighbourhood_hop(&c.project, &[c.focus], Side::Out).await.unwrap();
    let mut want = vec![(c.callee, 2, true), (c.library, 1, false)];
    want.sort();
    assert_eq!(by_far(&calls, Side::Out), want);

    cleanup(&pg, &c).await;
}

/// The focus is looked up IN the project: a node of another project is a miss,
/// not a neighbourhood drawn somewhere the reader is not standing.
///
/// Mutation that must break this: drop the `project_ids` test from the focus read.
#[tokio::test]
async fn a_focus_from_another_project_is_a_miss() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let c = seed(&pg).await;

    let found = pg.neighbourhood_focus(&c.project, &c.focus).await.unwrap();
    let found = found.expect("the focus is this project's");
    assert_eq!(found.name, "focus");
    assert_eq!(found.file_path.as_deref(), Some("a.rs"));
    assert!(pg.neighbourhood_focus(&c.project, &c.foreign_caller).await.unwrap().is_none());

    let nodes = pg.neighbourhood_nodes(&[c.focus, c.library]).await.unwrap();
    let lib = nodes.iter().find(|n| n.id == c.library).expect("library node");
    assert_eq!(lib.locality.as_deref(), Some("external"));

    cleanup(&pg, &c).await;
}

/// Two different facts, never summed: calls the focus makes that the graph could
/// not place, and unplaced calls elsewhere in the project that NAME it.
///
/// `plumbing` and `external_boundary` are excluded through
/// `Reason::casts_doubt`, the one owner of that partition: they are the ladder
/// placing a site OUTSIDE, not losing it.
///
/// Mutations that must break this: swap the two counts, or count every reason.
#[tokio::test]
async fn the_unplaced_counts_are_kept_apart() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let c = seed(&pg).await;

    let u = pg.neighbourhood_unplaced(&c.project, &c.focus, "focus").await.unwrap();
    assert_eq!(u.callees, 2, "`mystery` and `elsewhere` are gaps; `clone` is plumbing");
    assert_eq!(u.named, 1, "one doubtful unplaced call names `focus`; the plumbing one is not");

    cleanup(&pg, &c).await;
}
