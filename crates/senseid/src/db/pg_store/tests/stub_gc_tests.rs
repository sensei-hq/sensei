use super::*;
use crate::db::pg_store::graph::{EdgeRow, EdgeTarget};

// ── stub GC ──────────────────────────────────────────────────────────────────

/// An unresolved reference stub that nothing points at is garbage, and until now
/// nothing could collect it: `prune_file_nodes` filters `file_path = $2` and
/// every stub has `file_path IS NULL`, so 84,446 accumulated with no GC path.
/// That is also why the invariant `count(*) where locality='unknown'` could not
/// be driven to zero by fixing the parsers alone.
#[tokio::test]
async fn prune_orphan_stubs_collects_unreferenced_stubs_only() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("gc_{}", uuid::Uuid::new_v4())).await;

    // Garbage: a stub nothing references.
    let orphan = s
        .seed_node_by_fqn(&fid, "rust·p·m·Orphan·gone", "function", "gone", Some("rust"), None)
        .await
        .unwrap();
    // Live: a stub that an edge still points at — a caller has not been reindexed
    // yet, and dropping it would lose the fact that the reference exists.
    let referenced = s
        .seed_node_by_fqn(&fid, "rust·p·m·Kept·held", "function", "held", Some("rust"), None)
        .await
        .unwrap();
    let caller = s
        .seed_node(&fid, "function", "caller", "src/a.rs", None, None, Some(1), Some(2))
        .await
        .unwrap();
    s.insert_edge(&fid, &caller, Some(&referenced), None, None, "calls").await.unwrap();
    // Must survive: a real local definition.
    let real = s
        .seed_node(&fid, "function", "real", "src/b.rs", None, None, Some(1), Some(2))
        .await
        .unwrap();
    // Must survive: an external symbol is not a stub.
    let lib = s
        .seed_node_by_fqn(&fid, "lib·axum·axum·Json", "unknown", "Json", Some("rust"), None)
        .await
        .unwrap();

    let removed = s.prune_orphan_stubs(&fid).await.unwrap();
    assert_eq!(removed, 1, "exactly the unreferenced stub");

    let survivors: Vec<(uuid::Uuid,)> =
        sqlx_core::query_as::query_as("SELECT id FROM sensei.nodes WHERE id = ANY($1)")
            .bind(vec![orphan, referenced, real, lib])
            .fetch_all(s.pool())
            .await
            .unwrap();
    let alive: std::collections::HashSet<uuid::Uuid> =
        survivors.into_iter().map(|(id,)| id).collect();
    assert!(!alive.contains(&orphan), "the unreferenced stub is collected");
    assert!(alive.contains(&referenced), "a stub with an in-edge is still evidence of a reference");
    assert!(alive.contains(&real), "a real definition is never a stub");
    assert!(alive.contains(&lib), "a lib_symbol is external, not unresolved");

    // Idempotent: a second pass has nothing left to do.
    assert_eq!(s.prune_orphan_stubs(&fid).await.unwrap(), 0, "second pass is a no-op");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A stub with CHILDREN must not be collected. `nodes.parent_id` cascades on
/// delete, and live there are 42 stub parents carrying 574 real internal method
/// nodes — an unguarded delete would destroy every one of them. The stub is
/// wrong, but it is load-bearing until its children are re-parented.
#[tokio::test]
async fn prune_orphan_stubs_never_cascades_onto_real_children() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("gckids_{}", uuid::Uuid::new_v4())).await;
    let stub_parent = s
        .seed_node_by_fqn(&fid, "java·p·StubClass", "class", "StubClass", Some("java"), None)
        .await
        .unwrap();
    let child = s
        .seed_node(
            &fid,
            "method",
            "setId",
            "src/Model.java",
            Some(&stub_parent),
            None,
            Some(3),
            Some(4),
        )
        .await
        .unwrap();

    assert_eq!(
        s.prune_orphan_stubs(&fid).await.unwrap(),
        0,
        "a stub carrying children is not collected"
    );
    let (kids,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.nodes WHERE id = $1")
            .bind(child)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(kids, 1, "the real method survives — 574 of these exist live");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// Branch belongs in the typed column, and `graph_nodes` must expose it.
///
/// `folders.branch` was declared but had ZERO writers — NULL in all 7,772 rows —
/// while `process_git_folder` wrote `props->>'branch'` instead. Two stores for
/// one fact, and the dead one was the typed, indexable one.
///
/// It is a real partition key: the design is one folder per checkout ("develop
/// vs main = two folders, one repository"), and that is already live — fitness,
/// strategos and website each have two folder rows on two branches. So filtering
/// `graph_nodes` by branch separates the graphs, because folder_id already does.
#[tokio::test]
async fn folder_branch_is_a_typed_column_and_a_graph_nodes_dimension() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("br_{}", uuid::Uuid::new_v4())).await;
    s.set_folder_branch(&fid, "release/v9").await.unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "f",
        "src/a.rs",
        None,
        None,
        Some(1),
        Some(2),
    )
    .await
    .unwrap();

    let (col,): (Option<String>,) =
        sqlx_core::query_as::query_as("SELECT branch FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(col.as_deref(), Some("release/v9"), "written to the typed column");

    let (via_view,): (Option<String>,) = sqlx_core::query_as::query_as(
        "SELECT branch FROM sensei.graph_nodes WHERE folder_id = $1 LIMIT 1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(via_view.as_deref(), Some("release/v9"), "and filterable from the graph view");

    // Idempotent, and a re-index to the same branch changes nothing.
    s.set_folder_branch(&fid, "release/v9").await.unwrap();
    s.set_folder_branch(&fid, "main").await.unwrap();
    let (after,): (Option<String>,) =
        sqlx_core::query_as::query_as("SELECT branch FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(after.as_deref(), Some("main"), "a switch updates it in place");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// "How many nodes belong to which repository" has to be answerable from the
/// view alone, and an unattributed folder has to be DISTINGUISHABLE from an
/// attributed one rather than silently rolled into some default.
///
/// Both clauses matter. Without the first, grouping by repository is a join the
/// caller has to re-derive every time. Without the second, a NULL would be
/// indistinguishable from a fabricated fallback. `folders.repository_id` is no
/// longer sparse — every folder inherits one from its anchor at write time, 0 of
/// 13,722 null on 2026-10-01 — but the column stays NULLABLE and the writer
/// guards for it, so the unattributed bucket remains representable and the view
/// must still answer honestly over it. The fixture uses
/// `create_test_folder_unattributed` precisely because the default can no longer
/// produce that state.
#[tokio::test]
async fn graph_nodes_names_the_repository_and_leaves_an_unattributed_folder_null() {
    let s = pg_store().await;
    let attributed = create_test_folder(&s, &format!("repoA_{}", uuid::Uuid::new_v4())).await;
    let orphan =
        create_test_folder_unattributed(&s, &format!("repoB_{}", uuid::Uuid::new_v4())).await;

    let repo_key = format!("example.test/{}", uuid::Uuid::new_v4());
    let (repo_id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'attributed-repo') \
         RETURNING id",
    )
    .bind(&repo_key)
    .fetch_one(s.pool())
    .await
    .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET repository_id = $1 WHERE id = $2")
        .bind(repo_id)
        .bind(attributed)
        .execute(s.pool())
        .await
        .unwrap();

    for fid in [attributed, orphan] {
        s.seed_node(&fid, "function", "compute", "src/lib.rs", None, None, Some(1), Some(9))
            .await
            .unwrap();
    }

    let (named, named_id): (Option<String>, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
        "SELECT repository, repository_id FROM sensei.graph_nodes WHERE folder_id = $1 LIMIT 1",
    )
    .bind(attributed)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        named.as_deref(),
        Some("attributed-repo"),
        "a node's repository is readable from the view without a second join"
    );
    assert_eq!(named_id, Some(repo_id), "and carries the id, so it groups without a name match");

    let (unattributed, unattributed_id): (Option<String>, Option<uuid::Uuid>) =
        sqlx_core::query_as::query_as(
            "SELECT repository, repository_id FROM sensei.graph_nodes WHERE folder_id = $1 LIMIT 1",
        )
        .bind(orphan)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(
        (unattributed, unattributed_id),
        (None, None),
        "a folder with no repository reads NULL — never a fabricated stand-in"
    );

    s.delete_nodes_by_folder(&attributed).await.unwrap();
    s.delete_nodes_by_folder(&orphan).await.unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET repository_id = NULL WHERE id = $1")
        .bind(attributed)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(repo_id)
        .execute(s.pool())
        .await
        .unwrap();
}

/// Collecting stubs must not leave community rows describing zero nodes.
///
/// `prune_orphan_stubs` deletes nodes outside the detect transaction, so a
/// community whose every member was an orphan stub survived as a row with no
/// members. Measured immediately after the first GC pass: 27,693 such rows, and
/// `list_communities` / the Atlas `communities/info` endpoint read them — a
/// phantom community with a label and a node_count that no longer matches
/// anything. Derived rows with nothing left to describe are garbage by the same
/// rule as the stubs themselves.
#[tokio::test]
async fn prune_orphan_stubs_removes_communities_it_emptied() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("gccom_{}", uuid::Uuid::new_v4())).await;
    let stub = s
        .seed_node_by_fqn(&fid, "rust·p·m·Ghost·gone", "function", "gone", Some("rust"), None)
        .await
        .unwrap();
    let real = s
        .seed_node(&fid, "function", "real", "src/a.rs", None, None, Some(1), Some(2))
        .await
        .unwrap();
    // Community 1 is stub-only (emptied by GC); community 2 keeps a real member.
    for (id, cid) in [(stub, 1i32), (real, 2i32)] {
        sqlx_core::query::query("UPDATE sensei.nodes SET community_id = $2 WHERE id = $1")
            .bind(id)
            .bind(cid)
            .execute(s.pool())
            .await
            .unwrap();
    }
    for cid in [1i32, 2] {
        sqlx_core::query::query(
            "INSERT INTO inference.communities(folder_id, community_id, label, node_count)
             VALUES($1, $2, 'l', 1)",
        )
        .bind(fid)
        .bind(cid)
        .execute(s.pool())
        .await
        .unwrap();
    }

    assert_eq!(s.prune_orphan_stubs(&fid).await.unwrap(), 1, "the orphan stub is collected");

    let rows: Vec<(i32,)> = sqlx_core::query_as::query_as(
        "SELECT community_id FROM inference.communities WHERE folder_id = $1 ORDER BY 1",
    )
    .bind(fid)
    .fetch_all(s.pool())
    .await
    .unwrap();
    let ids: Vec<i32> = rows.into_iter().map(|(c,)| c).collect();
    assert_eq!(ids, vec![2], "the emptied community goes; the one with a real member stays");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// THE REGRESSION: a caller reached through an UNRESOLVED edge must still be
/// found. `get_callers_by_name` used to filter `call_graph.target_name`, which
/// is `tgt.name` off the view's LEFT JOIN and therefore NULL for every
/// unresolved edge — the name lives in `e.target_name`. Live consequence:
/// 117,201 of 335,756 `calls` edges (34.9%) were unreachable and the tool
/// returned an empty list for 8,680 symbol names that demonstrably had callers.
///
/// Mutation that must break this test: swap `target_symbol` back to
/// `target_name` in the query.
#[tokio::test]
async fn get_callers_by_name_finds_a_caller_through_an_unresolved_edge() {
    let s = pg_store().await;
    let folder = format!("callers_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    // The target IS defined locally — this is not a phantom symbol.
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "handleAuth",
        "src/auth.rs",
        None,
        None,
        Some(10),
        Some(20),
    )
    .await
    .unwrap();
    // Two callers: one whose edge RESOLVED, one still unresolved (the caller was
    // indexed before the definition, which is the normal steady state for 35% of
    // this graph). Both are real callers and both must be reported.
    let resolved_caller = s
        .seed_node(&fid, "function", "login", "src/login.rs", None, None, Some(1), Some(5))
        .await
        .unwrap();
    let unresolved_caller = s
        .seed_node(&fid, "function", "middleware", "src/mw.rs", None, None, Some(1), Some(5))
        .await
        .unwrap();
    let target_id = s
        .seed_node(&fid, "function", "handleAuth", "src/auth.rs", None, None, Some(10), Some(20))
        .await
        .unwrap();
    s.insert_edge(&fid, &resolved_caller, Some(&target_id), None, None, "calls").await.unwrap();
    s.insert_edge(&fid, &unresolved_caller, None, Some("handleAuth"), None, "calls").await.unwrap();

    let callers = s.get_callers_by_name(&folder, "handleAuth").await.unwrap();
    let names: std::collections::HashSet<String> =
        callers.iter().map(|c| c["name"].as_str().unwrap_or_default().to_string()).collect();
    assert!(names.contains("login"), "the resolved caller was always found");
    assert!(
        names.contains("middleware"),
        "the caller behind an UNRESOLVED edge must be found too — this is the 34.9% \
         that target_name silently dropped"
    );
    // Each row says which it was, so a reader can tell an exact hit from a
    // name-matched one rather than treating the list as uniformly certain.
    let unresolved_row = callers.iter().find(|c| c["name"] == "middleware").unwrap();
    assert_eq!(unresolved_row["resolved"], serde_json::json!(false));

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A caller list that says only `resolved: true|false` is a list of
/// equally-confident facts, and this graph is not equally confident anywhere.
///
/// `declared_here` — the file declares the target itself — and `in_the_prelude`
/// — a name the language puts in scope everywhere, so the "target" is a guess at
/// which of several identically-named things was meant — are both `resolved:
/// true`. They are not the same claim, and on the live graph they are 77,462 and
/// 36,832 edges respectively. The same on the other side: `no_import_in_scope`
/// is a gap someone can close, `external_boundary` is the indexed world simply
/// ending, and a reader triaging one wants nothing to do with the other.
///
/// `sensei.call_graph` has carried both columns since the verdict landed. This
/// query did not select them, so the distinction existed in the database and
/// reached no consumer.
///
/// Seeded through `merge_edge_occurrences`, the path the indexer actually
/// writes — NOT by setting the columns directly. The verdict is a REDUCTION over
/// per-use occurrences, so a test that hand-sets the column proves the select
/// list and nothing about whether the value is real.
///
/// Mutation that must break this test: drop `resolved_via` or
/// `unresolved_reason` from the select list in `get_callers_by_name`.
#[tokio::test]
async fn callers_carry_the_verdict_that_placed_each_edge() {
    let s = pg_store().await;
    let folder = format!("caller_verdict_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    let target = s
        .seed_node(&fid, "function", "handleAuth", "src/auth.rs", None, None, Some(10), Some(20))
        .await
        .unwrap();
    let placed = s
        .seed_node(&fid, "function", "login", "src/login.rs", None, None, Some(1), Some(5))
        .await
        .unwrap();
    let missed = s
        .seed_node(&fid, "function", "middleware", "src/mw.rs", None, None, Some(1), Some(5))
        .await
        .unwrap();

    let placed_edge =
        s.insert_edge(&fid, &placed, Some(&target), None, None, "calls").await.unwrap();
    let missed_edge =
        s.insert_edge(&fid, &missed, None, Some("handleAuth"), None, "calls").await.unwrap();

    s.merge_edge_occurrences(
        &placed_edge,
        "src/login.rs",
        &serde_json::json!([
            { "fact": "use", "kind": "calls", "at": [2, 1, 2, 9], "rung": "declared_here" }
        ]),
    )
    .await
    .unwrap();
    s.merge_edge_occurrences(
        &missed_edge,
        "src/mw.rs",
        &serde_json::json!([
            { "fact": "use", "kind": "calls", "at": [3, 1, 3, 9], "reason": "no_import_in_scope" }
        ]),
    )
    .await
    .unwrap();

    let callers = s.get_callers_by_name(&folder, "handleAuth").await.unwrap();
    let row = |name: &str| callers.iter().find(|c| c["name"] == name).unwrap().clone();

    let login = row("login");
    assert_eq!(
        login["resolved_via"],
        serde_json::json!("declared_here"),
        "a placed edge must say WHICH RUNG placed it, not merely that it landed"
    );
    assert_eq!(
        login["unresolved_reason"],
        serde_json::Value::Null,
        "a placed edge has no reason — carrying both would make the row self-contradictory"
    );

    let middleware = row("middleware");
    assert_eq!(
        middleware["unresolved_reason"],
        serde_json::json!("no_import_in_scope"),
        "a missed edge must say WHY the ladder stopped, so a gap can be told from a limit"
    );
    assert_eq!(middleware["resolved_via"], serde_json::Value::Null, "a missed edge claims no rung");

    // The edge kind travels too: the screen's calls/type-use/imports filter is a
    // property of the edge, and without it every row reads as a call.
    assert_eq!(login["edge_kind"], serde_json::json!("calls"));

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// Both callee paths must emit the SAME KEYS, and the chased one must claim no
/// rung.
///
/// `get_callees_by_name` produces rows two ways: straight off the view, and —
/// for an unresolved member call whose receiver the call site named — from the
/// node the read-path chase found. Those two built separate objects, and the
/// chased one omitted `resolved` entirely. A consumer reading `row["resolved"]`
/// therefore got `null` (falsey) for a call that had just been successfully
/// placed, which is the opposite of what happened.
///
/// The second assertion is the one with a judgement in it. A chased row is
/// `resolved: true`, but the STORED edge is still unresolved and still carries
/// the reason the ladder stopped. Reporting that reason next to `resolved: true`
/// would put one edge in a payload described two contradictory ways — the same
/// disagreement `call_coverage` refuses to create by declining to heal the
/// incoming side. So the row claims no rung and says `placed_by:
/// "receiver_chain"`, which is checkable.
///
/// No database: this is the shape contract, and the whole point is that it holds
/// regardless of which query produced the fields.
///
/// Mutation that must break this test: give `Placement::ReceiverChain` a
/// `resolved_via`, or drop a key from either arm of `callee_row`.
#[test]
fn both_callee_paths_report_the_same_shape() {
    use crate::db::pg_store::graph::{Callee, Placement};

    let indexed = PgStore::callee_row(
        Callee {
            name: "parse",
            kind: Some("function"),
            file: Some("src/parse.rs"),
            line: Some(12),
            locality: "internal",
            edge_kind: "calls",
        },
        &Placement::Indexed {
            resolved_via: Some("through_an_import".to_string()),
            unresolved_reason: None,
        },
    );
    let chased = PgStore::callee_row(
        Callee {
            name: "parse",
            kind: Some("function"),
            file: Some("src/parse.rs"),
            line: Some(12),
            locality: "internal",
            edge_kind: "calls",
        },
        &Placement::ReceiverChain,
    );

    let keys = |v: &serde_json::Value| {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    assert_eq!(
        keys(&indexed),
        keys(&chased),
        "a consumer must not have to know which path produced a row to read it"
    );

    assert_eq!(indexed["resolved"], serde_json::json!(true));
    assert_eq!(indexed["resolved_via"], serde_json::json!("through_an_import"));
    assert_eq!(indexed["placed_by"], serde_json::json!("indexer"));

    assert_eq!(chased["resolved"], serde_json::json!(true), "the chase placed it — say so");
    assert_eq!(
        chased["resolved_via"],
        serde_json::Value::Null,
        "the chase is not a rung on the indexer's ladder and must not borrow one"
    );
    assert_eq!(
        chased["unresolved_reason"],
        serde_json::Value::Null,
        "carrying the stored reason beside resolved:true describes one edge two ways"
    );
    assert_eq!(chased["placed_by"], serde_json::json!("receiver_chain"));

    // A missed edge: no rung, a reason, and `resolved` false.
    let missed = PgStore::callee_row(
        Callee {
            name: "mystery",
            kind: None,
            file: None,
            line: None,
            locality: "unknown",
            edge_kind: "calls",
        },
        &Placement::Indexed {
            resolved_via: None,
            unresolved_reason: Some("dynamic_dispatch".to_string()),
        },
    );
    assert_eq!(missed["resolved"], serde_json::json!(false));
    assert_eq!(missed["unresolved_reason"], serde_json::json!("dynamic_dispatch"));
    assert_eq!(keys(&missed), keys(&indexed));
}

/// The receiver chase must probe an fqn a node can actually have.
///
/// Same defect as the import candidates, different consumer: this built its
/// lookup key with `languages::fqn::item`, which joins FOUR segments, while
/// every node in the graph carries FIVE — the fifth being the reach that keeps
/// a field and a same-named method apart. Measured: 100% of module nodes across
/// all six languages are `…·mod`, and not one is in any other shape. So the
/// chase could not miss sometimes; it had to miss always, which is why
/// `receiver_type_unknown` is a top fault code and accounts for 5 of the 6
/// remaining unplaced edges on `crates/senseid/src/paths.rs`.
///
/// Asserted against `indexer::fqn::refer` rather than a string literal, so the
/// two cannot drift apart again — a literal would pin today's spelling and let
/// the encoder move out from under it, which IS the bug.
///
/// Mutation that must break this test: mint the key with a four-segment join
/// (what the deleted v1 `languages::fqn::item` did).
#[test]
fn the_receiver_chase_probes_the_identity_the_walk_minted() {
    use crate::indexer::facts::Language;
    use crate::indexer::fqn::{Form, Reach, refer};

    let declared = refer(&Form::Item {
        lang: Language::Rust,
        package: "senseid",
        module: "db::pg_store",
        name: "PgStore",
        reach: Reach::Item,
    })
    .expect("the walk mints this for `pub struct PgStore`");

    assert_eq!(
        super::graph::receiver_type_fqn("senseid", "db::pg_store", "PgStore").as_deref(),
        Some(declared.to_string().as_str()),
        "the chase must look up the identity the walk wrote, not a four-segment \
         spelling no node has ever carried"
    );
}

/// **A FORCED RESCAN REOPENS FILES WITHOUT DELETING ANYTHING.**
///
/// The gate is `list_unparsed_files`, which selects on `parsed_at IS NULL`, so
/// a parsed file is never offered again. That is right almost always — it is
/// what makes a re-run cheap — and wrong exactly when the INDEXER changed:
/// rebuilt code produces different facts from bytes that have not moved, and
/// the gate cannot know that.
///
/// The override had to be non-destructive. Deleting `files` rows was the only
/// way before, and it takes the nodes and edges those files own with it — so a
/// scan that then fails leaves the graph emptier than it found it. Clearing
/// `parsed_at` reopens the work and touches nothing else; the writer upserts by
/// path, so the re-parse replaces what it finds.
///
/// A SKIPPED FILE STAYS SKIPPED, and it needs no clause to do so: a skipped file
/// was never parsed, so `parsed_at IS NOT NULL` already excludes it. A
/// `skip_reason IS NULL` guard was written into the UPDATE first and removed
/// when a mutation probe showed it could not fire.
///
/// Mutation that must break this test: have `mark_folder_unparsed` clear
/// nothing, or widen it past `parsed_at`.
#[tokio::test]
async fn a_forced_rescan_reopens_parsed_files_and_leaves_skipped_ones_alone() {
    let s = pg_store().await;
    let folder = format!("force_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    s.upsert_file_row(&fid, "src/parsed.rs", 1, "h1", None).await.unwrap();
    s.upsert_file_row(
        &fid,
        "src/binary.bin",
        1,
        "h2",
        Some(crate::classifiers::ScanSkipReason::BinaryContent),
    )
    .await
    .unwrap();
    s.mark_file_parsed(&fid, "src/parsed.rs").await.unwrap();
    // The skipped file is deliberately NOT marked parsed: `mark_file_parsed`
    // clears `skip_reason`, so "parsed AND skipped" is a state the system
    // cannot be in, and a fixture that built one would be testing a fiction.

    let before = s.list_unparsed_files(&fid).await.unwrap();
    assert!(
        !before.contains(&"src/parsed.rs".to_string()),
        "a parsed file is not offered again — that is the gate working: {before:?}"
    );

    let reopened = s.mark_folder_unparsed(&fid).await.unwrap();
    assert_eq!(reopened, 1, "exactly the one parsed file is reopened, not the skipped one");

    let after = s.list_unparsed_files(&fid).await.unwrap();
    assert!(
        after.contains(&"src/parsed.rs".to_string()),
        "the forced pass must offer the parsed file again: {after:?}"
    );
    assert!(
        !after.contains(&"src/binary.bin".to_string()),
        "a file skipped for a recorded reason stays skipped: {after:?}"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The callee list carries the same verdict the caller list does.
///
/// Sibling of `callers_carry_the_verdict_that_placed_each_edge`. Both sides of a
/// symbol's neighbourhood are read together — what calls this, and what this
/// calls — so a verdict on one side only would let a reader trust the two lists
/// differently for no reason the data supports.
///
/// Mutation that must break this test: drop `cg.resolved_via` or
/// `cg.unresolved_reason` from the select list in `get_callees_by_name`.
#[tokio::test]
async fn callees_carry_the_verdict_that_placed_each_edge() {
    let s = pg_store().await;
    let folder = format!("callee_verdict_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    let source = s
        .seed_node(&fid, "function", "run_pipeline", "src/run.rs", None, None, Some(1), Some(40))
        .await
        .unwrap();
    let placed_target = s
        .seed_node(&fid, "function", "parse_manifest", "src/parse.rs", None, None, Some(5), Some(9))
        .await
        .unwrap();

    let placed_edge =
        s.insert_edge(&fid, &source, Some(&placed_target), None, None, "calls").await.unwrap();
    let missed_edge =
        s.insert_edge(&fid, &source, None, Some("dispatch"), None, "calls").await.unwrap();

    s.merge_edge_occurrences(
        &placed_edge,
        "src/run.rs",
        &serde_json::json!([
            { "fact": "use", "kind": "calls", "at": [7, 1, 7, 9], "rung": "through_an_import" }
        ]),
    )
    .await
    .unwrap();
    s.merge_edge_occurrences(
        &missed_edge,
        "src/run.rs",
        &serde_json::json!([
            { "fact": "use", "kind": "calls", "at": [9, 1, 9, 9], "reason": "dynamic_dispatch" }
        ]),
    )
    .await
    .unwrap();

    let callees = s.get_callees_by_name(&folder, "run_pipeline").await.unwrap();
    let row = |name: &str| callees.iter().find(|c| c["name"] == name).unwrap().clone();

    let parsed = row("parse_manifest");
    assert_eq!(parsed["resolved_via"], serde_json::json!("through_an_import"));
    assert_eq!(parsed["placed_by"], serde_json::json!("indexer"));

    let dispatch = row("dispatch");
    assert_eq!(
        dispatch["unresolved_reason"],
        serde_json::json!("dynamic_dispatch"),
        "a refusal, not a fault — the reader should be able to stop looking at this one"
    );
    assert_eq!(dispatch["resolved"], serde_json::json!(false));

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The verdict COLUMNS are populated by the writer, and re-derived when the
/// occurrence set changes in either direction.
///
/// `the_rung_and_reason_come_from_the_occurrences_the_indexer_writes` proves the
/// view can READ the nested shape. This proves the write path fills the columns,
/// which is what makes the read cheap and the contract explicit in the schema
/// rather than asserted in a comment.
///
/// WHY THE DATABASE DERIVES THEM AND NOT RUST: the reduction is over EVERY file's
/// occurrences, and the indexing process holds one file's. Merging a second file
/// can change the winner, so only the database has the value to reduce over. Both
/// directions are asserted here:
///
///   1. one file, a refusal        -> `plumbing`
///   2. a second file adds a fault -> must become `receiver_type_unknown`
///   3. drop the second file       -> must fall back to `plumbing`
///
/// Step 3 is the one an implementation is most likely to miss: it is easy to
/// derive on merge and forget on drop, which leaves the column claiming a fault
/// that left with the file that held it.
///
/// Mutation that must break this test: remove the `resolved_via`/
/// `unresolved_reason` assignment from `drop_edge_occurrences`, or reverse
/// `edge_verdict`'s `order by rc.precedence`.
#[tokio::test]
async fn the_writer_fills_the_verdict_columns_and_redrives_them_on_change() {
    let s = pg_store().await;
    let folder = format!("verdict_cols_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    let src = s
        .seed_node(&fid, "function", "caller", "src/a.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    let edge = s.insert_edge(&fid, &src, None, Some("nowhere"), None, "calls").await.unwrap();

    let refusal = serde_json::json!([
        { "fact": "use", "kind": "calls", "at": [1, 1, 1, 9], "reason": "plumbing" }
    ]);
    let fault = serde_json::json!([
        { "fact": "use", "kind": "calls", "at": [2, 1, 2, 9], "reason": "receiver_type_unknown" }
    ]);

    async fn stored(s: &PgStore, id: &uuid::Uuid) -> (Option<String>, Option<String>) {
        sqlx_core::query_as::query_as(
            "SELECT resolved_via, unresolved_reason FROM sensei.edges WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&s.pool)
        .await
        .unwrap()
    }

    s.merge_edge_occurrences(&edge, "src/a.rs", &refusal).await.unwrap();
    assert_eq!(
        stored(&s, &edge).await,
        (None, Some("plumbing".to_string())),
        "one file, one refusal: the column carries it and no rung is claimed"
    );

    s.merge_edge_occurrences(&edge, "src/b.rs", &fault).await.unwrap();
    assert_eq!(
        stored(&s, &edge).await,
        (None, Some("receiver_type_unknown".to_string())),
        "a second file adds a FAULT, which outranks the refusal (lower precedence)"
    );

    s.drop_edge_occurrences(&edge, "src/b.rs").await.unwrap();
    assert_eq!(
        stored(&s, &edge).await,
        (None, Some("plumbing".to_string())),
        "dropping the file that held the fault must take the fault with it"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The rung and the reason must be read from the shape THE INDEXER WRITES.
///
/// `graph_resolution_says_which_rung_placed_each_edge` and its boundary sibling
/// both seed props at the TOP level — `{"rung": "declared_here"}` — via
/// `insert_edge_with_props`, which bypasses `persist::write`. The real writer
/// puts the verdict on each OCCURRENCE, nested under the file that made it:
///
///     props.occurrences["src/a.rs"][0].rung
///     props.occurrences["src/a.rs"][0].reason
///
/// `persist::with_outcome` is called from `occurrence_prop`, so there is no path
/// that produces a top-level key. The consequence in production: `resolved_via`
/// was NULL on all 1,640,215 placed edges and `unresolved_reason` NULL on all
/// 2,428,016 missed ones — every match and every miss unclassifiable, in every
/// language — while both existing tests stayed green on a shape nothing emits.
///
/// AN EDGE AGGREGATES MANY OCCURRENCES, so the edge-level label is a REDUCTION,
/// and the rule is the same on both sides: LOWEST PRECEDENCE WINS.
/// `reason_codes.precedence` is climb order for a rung and severity order for a
/// reason, so the strongest proof and the most serious fault both sort first. An
/// edge whose uses are `plumbing` (refusal, 90) and `receiver_type_unknown`
/// (fault, 30) reports the fault — a real gap must not be hidden by a
/// deliberate filter that happens to share the edge.
///
/// Mutation that must break this test: reverse the `order by rc.precedence`, or
/// drop the `jsonb_each` over occurrences and read only the top-level key.
#[tokio::test]
async fn the_rung_and_reason_come_from_the_occurrences_the_indexer_writes() {
    let s = pg_store().await;
    let folder = format!("nested_verdict_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    let caller = s
        .seed_node(&fid, "function", "caller", "src/a.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    let callee = s
        .seed_node(&fid, "function", "callee", "src/b.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();

    // PLACED, with the rung nested exactly as `occurrence_prop` writes it. Two
    // occurrences with different rungs, so the reduction is exercised rather
    // than assumed: `declared_here` (10) must win over `in_the_prelude` (60).
    s.insert_edge_with_props(&EdgeRow {
        folder_id: &fid,
        source_id: &caller,
        target: EdgeTarget::Node(&callee),
        kind: "calls",
        props: &serde_json::json!({ "occurrences": { "src/a.rs": [
            { "fact": "use", "kind": "calls", "at": [1, 1, 1, 9], "rung": "in_the_prelude" },
            { "fact": "use", "kind": "calls", "at": [2, 1, 2, 9], "rung": "declared_here" }
        ]}}),
    })
    .await
    .unwrap();

    // MISSED, likewise nested. A refusal and a fault share the edge; the fault
    // must surface, because an edge reported as `plumbing` is an edge nobody
    // investigates.
    let stray = s
        .seed_node(&fid, "function", "stray", "src/c.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    s.insert_edge_with_props(&EdgeRow { folder_id: &fid, source_id: &stray, target: EdgeTarget::Unresolved { name: Some("nowhere"), file: None }, kind: "calls", props: &serde_json::json!({ "occurrences": { "src/c.rs": [
            { "fact": "use", "kind": "calls", "at": [1, 1, 1, 9], "reason": "plumbing" },
            { "fact": "use", "kind": "calls", "at": [2, 1, 2, 9], "reason": "receiver_type_unknown" }
        ]}}) })
    .await
    .unwrap();

    let via: Option<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT resolved_via FROM sensei.graph_resolution WHERE folder_id = $1",
    )
    .bind(fid)
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert_eq!(
        via.as_deref(),
        Some("declared_here"),
        "the rung is nested under props.occurrences, and the strongest of the two wins"
    );

    let reason: Option<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT reason_code FROM sensei.graph_boundary WHERE folder_id = $1",
    )
    .bind(fid)
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert_eq!(
        reason.as_deref(),
        Some("receiver_type_unknown"),
        "a fault outranks a refusal sharing the same edge"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The verdict must land on the COLUMN at insert, not only in the view.
///
/// The test above reads `sensei.graph_resolution`, which calls `edge_verdict`
/// at QUERY time — so it passes whether or not `edges.resolved_via` was ever
/// written. That is exactly how the column stayed empty without a red test:
/// every consumer that mattered went through a view, and the materialised
/// column was verified by nothing.
///
/// It matters because the column is not decoration. A view recomputing a
/// jsonpath reduction over `props.occurrences` cannot be indexed or grouped at
/// four million rows, which is the whole reason the reduction was promoted to a
/// column in the first place. MEASURED at the time this was written: of 130,614
/// edges the indexer had touched in ten minutes, 14,217 carried a verdict —
/// because only the merge path in `indexer.rs` wrote one, and a first insert
/// never goes through it.
///
/// Mutation that must break this test: drop either `edge_verdict` call from
/// `insert_edge_with_props`.
#[tokio::test]
async fn an_inserted_edge_carries_its_verdict_on_the_column_not_just_the_view() {
    let s = pg_store().await;
    let folder = format!("verdict_col_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    let caller =
        s.seed_node(&fid, "function", "c", "src/a.rs", None, None, Some(1), Some(9)).await.unwrap();
    let callee =
        s.seed_node(&fid, "function", "t", "src/b.rs", None, None, Some(1), Some(9)).await.unwrap();

    let placed = s
        .insert_edge_with_props(&EdgeRow {
            folder_id: &fid,
            source_id: &caller,
            target: EdgeTarget::Node(&callee),
            kind: "calls",
            props: &serde_json::json!({ "occurrences": { "src/a.rs": [
                { "fact": "use", "kind": "calls", "at": [1, 1, 1, 9], "rung": "in_the_prelude" },
                { "fact": "use", "kind": "calls", "at": [2, 1, 2, 9], "rung": "declared_here" }
            ]}}),
        })
        .await
        .unwrap();

    let (via, reason): (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT resolved_via, unresolved_reason FROM sensei.edges WHERE id = $1",
    )
    .bind(placed)
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert_eq!(
        via.as_deref(),
        Some("declared_here"),
        "a placed edge must carry the strongest rung on the column at insert"
    );
    assert_eq!(reason, None, "exactly one of the two is set: a placed edge has no reason");

    let missed = s
        .insert_edge_with_props(&EdgeRow { folder_id: &fid, source_id: &caller, target: EdgeTarget::Unresolved { name: Some("nowhere"), file: None }, kind: "calls", props: &serde_json::json!({ "occurrences": { "src/a.rs": [
                { "fact": "use", "kind": "calls", "at": [3, 1, 3, 9], "reason": "plumbing" },
                { "fact": "use", "kind": "calls", "at": [4, 1, 4, 9], "reason": "receiver_type_unknown" }
            ]}}) })
        .await
        .unwrap();

    let (via2, reason2): (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT resolved_via, unresolved_reason FROM sensei.edges WHERE id = $1",
    )
    .bind(missed)
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert_eq!(
        reason2.as_deref(),
        Some("receiver_type_unknown"),
        "a missed edge must carry the most serious reason on the column at insert"
    );
    assert_eq!(via2, None, "exactly one of the two is set: a missed edge has no rung");

    // AND ON RE-INSERT. The upsert merges props, so the verdict has to be
    // recomputed from the MERGED value — a second occurrence carrying a stronger
    // rung must move the column, or the materialised value drifts from the props
    // it claims to summarise.
    s.insert_edge_with_props(&EdgeRow {
        folder_id: &fid,
        source_id: &caller,
        target: EdgeTarget::Unresolved { name: Some("nowhere"), file: None },
        kind: "calls",
        props: &serde_json::json!({ "occurrences": { "src/a.rs": [
            { "fact": "use", "kind": "calls", "at": [5, 1, 5, 9], "reason": "unhandled_form" }
        ]}}),
    })
    .await
    .unwrap();
    let reason3: Option<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT unresolved_reason FROM sensei.edges WHERE id = $1",
    )
    .bind(missed)
    .fetch_one(&s.pool)
    .await
    .unwrap();
    assert_eq!(
        reason3.as_deref(),
        Some("unhandled_form"),
        "the upsert recomputes from the merged props, so a stronger reason wins"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// `graph_resolution` says how a placed edge was placed.
///
/// The sibling of `graph_boundary`. That one is where the graph stops; this is
/// where it holds and on what evidence. A consumer handed a bare `resolved`
/// treats `declared_here` — a file pointing at its own declaration — exactly
/// like `through_a_glob`, a name the source never wrote down.
///
/// Mutation that must break this test: make the reason_codes join INNER, or
/// drop the `target_id is not null` filter so unplaced edges leak in.
#[tokio::test]
async fn graph_resolution_says_which_rung_placed_each_edge() {
    let s = pg_store().await;
    let folder = format!("rung_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    let target = s
        .seed_node(&fid, "function", "placed", "src/t.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    for (i, rung) in ["declared_here", "through_a_glob", "not_a_seeded_rung"].iter().enumerate() {
        let caller = s
            .seed_node(
                &fid,
                "function",
                &format!("via_{i}"),
                &format!("src/v{i}.rs"),
                None,
                None,
                Some(1),
                Some(9),
            )
            .await
            .unwrap();
        s.insert_edge_with_props(&EdgeRow {
            folder_id: &fid,
            source_id: &caller,
            target: EdgeTarget::Node(&target),
            kind: "calls",
            props: &serde_json::json!({ "rung": rung }),
        })
        .await
        .unwrap();
    }
    // An UNPLACED edge, which this view must not show at all — that one is
    // graph_boundary's, and a row in both would be counted twice by anyone
    // adding the two together.
    let stray = s
        .seed_node(&fid, "function", "stray", "src/s.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    s.insert_edge_with_props(&EdgeRow {
        folder_id: &fid,
        source_id: &stray,
        target: EdgeTarget::Unresolved { name: Some("placed"), file: None },
        kind: "calls",
        props: &serde_json::json!({ "reason": "receiver_type_unknown" }),
    })
    .await
    .unwrap();

    let rows: Vec<(Option<String>, Option<String>)> = sqlx_core::query_as::query_as(
        "SELECT resolved_via, rung_summary FROM sensei.graph_resolution
          WHERE folder_id = $1 ORDER BY source_name",
    )
    .bind(fid)
    .fetch_all(&s.pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 3, "three placed edges, and the unplaced one is not one of them");
    assert_eq!(rows[0].0.as_deref(), Some("declared_here"));
    assert!(
        rows[0].1.as_deref().is_some_and(|s| s.contains("declares the target itself")),
        "the prose comes with it: {:?}",
        rows[0].1
    );
    assert_eq!(rows[1].0.as_deref(), Some("through_a_glob"));
    assert!(
        rows[1].1.as_deref().is_some_and(|s| s.contains("glob")),
        "and it is the rung's own prose, not the first row's"
    );
    let unknown = &rows[2];
    assert_eq!(unknown.0.as_deref(), Some("not_a_seeded_rung"), "the raw rung survives");
    assert_eq!(unknown.1, None, "with no prose, because none is seeded");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// `coverage` says HOW MUCH of a caller list is missing; this says WHY.
///
/// Two integers tell a reader the list is incomplete and nothing else, so the
/// only available next step is "grep everything". A reason narrows it: eleven
/// receivers the walk could not type is a different job from eleven names with
/// no import in scope, and `external_boundary` is not a job at all.
///
/// Mutation that must break this test: drop the `ORDER BY reason_precedence`,
/// or count the whole folder instead of the sites naming this symbol.
#[tokio::test]
async fn call_coverage_reasons_says_why_the_missing_callers_are_missing() {
    let s = pg_store().await;
    let folder = format!("why_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;
    // ONE SOURCE PER SITE. `edges_unique_unresolved` is
    // (folder_id, source_id, target_name, target_file, kind), so several
    // unplaced sites naming one symbol from one body collapse to ONE row with
    // merged props — the schema's edge identity (D1). A fixture that ignored it
    // measured a single surviving reason and read as a query bug.
    for (i, (names, reason)) in [
        ("render", "receiver_type_unknown"),
        ("render", "receiver_type_unknown"),
        ("render", "external_boundary"),
        ("render", "no_import_in_scope"),
        ("unrelated", "receiver_type_unknown"),
    ]
    .into_iter()
    .enumerate()
    {
        let caller = s
            .seed_node(
                &fid,
                "function",
                &format!("caller_{i}"),
                &format!("src/c{i}.rs"),
                None,
                None,
                Some(1),
                Some(9),
            )
            .await
            .unwrap();
        s.insert_edge_with_props(&EdgeRow {
            folder_id: &fid,
            source_id: &caller,
            target: EdgeTarget::Unresolved { name: Some(names), file: None },
            kind: "calls",
            props: &serde_json::json!({ "reason": reason }),
        })
        .await
        .unwrap();
    }

    let why = s
        .call_coverage_reasons(&[fid], "render", crate::db::pg_store::CallDirection::Incoming)
        .await
        .unwrap();

    let codes: Vec<&str> = why.iter().filter_map(|r| r["reason"].as_str()).collect();
    assert_eq!(
        codes,
        ["receiver_type_unknown", "no_import_in_scope", "external_boundary"],
        "ordered by precedence — the most actionable first, the boundary last"
    );
    assert_eq!(why[0]["count"], serde_json::json!(2));
    assert_eq!(why[0]["kind"], serde_json::json!("fault"));
    assert!(
        why[0]["explanation"].as_str().is_some_and(|s| s.contains("called on")),
        "the prose comes with it: {:?}",
        why[0]["explanation"]
    );
    assert_eq!(
        why.iter().map(|r| r["count"].as_i64().unwrap_or(0)).sum::<i64>(),
        4,
        "the site naming `unrelated` is not this symbol's doubt"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The n-depth blast radius, over the stored graph.
///
/// Breadth-first and deduplicated, so a symbol reachable two ways is reported
/// once at its SHORTEST distance and a cycle terminates. `truncated` separates
/// "the graph ended" from "the query did" — a reader told an exact answer might
/// be partial re-runs a query it did not need to.
///
/// Mutation that must break this test: drop the `depth < $3` guard (hangs or
/// over-reports), or report a symbol at its longest distance.
#[tokio::test]
async fn impact_of_symbol_walks_callers_to_depth_and_says_when_it_stopped_early() {
    let s = pg_store().await;
    let folder = format!("impact_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    // bottom <- middle <- upper <- top, plus a cycle top <- bottom.
    let mut id = std::collections::HashMap::new();
    for (n, f) in
        [("bottom", "src/b.rs"), ("middle", "src/m.rs"), ("upper", "src/u.rs"), ("top", "src/t.rs")]
    {
        id.insert(
            n,
            s.seed_node(&fid, "function", n, f, None, None, Some(1), Some(9)).await.unwrap(),
        );
    }
    for (from, to) in
        [("middle", "bottom"), ("upper", "middle"), ("top", "upper"), ("bottom", "top")]
    {
        s.insert_edge(&fid, &id[from], Some(&id[to]), None, None, "calls").await.unwrap();
    }

    let deep = s.impact_of_symbol(&[fid], "bottom", 3).await.unwrap();
    let at = |d: i64| -> Vec<String> {
        let mut v: Vec<String> = deep["reached"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["depth"] == serde_json::json!(d))
            .map(|r| r["name"].as_str().unwrap_or_default().to_string())
            .collect();
        v.sort();
        v
    };
    assert_eq!(at(1), ["middle"]);
    assert_eq!(at(2), ["upper"]);
    assert_eq!(at(3), ["top"]);
    assert_eq!(deep["reached"].as_array().unwrap().len(), 3, "the cycle adds nobody");
    assert_eq!(deep["truncated"], serde_json::json!(false), "the graph ran out first");

    let shallow = s.impact_of_symbol(&[fid], "bottom", 1).await.unwrap();
    assert_eq!(shallow["reached"].as_array().unwrap().len(), 1);
    assert_eq!(
        shallow["truncated"],
        serde_json::json!(true),
        "`middle` has a caller that depth 1 did not reach, and the answer says so"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// `graph_boundary` turns a miss into something a reader can act on.
///
/// The reason has been on every unresolved reference since the ladder was built
/// and written into `edges.props->>'reason'` since persistence landed; nothing
/// ever read it back. This view joins it to the `code_graph` vocabulary in
/// `sensei.reason_codes`, so a consumer gets "the type of the thing being
/// called on is not known here" instead of a bare token — or instead of
/// nothing, which is what "3 callers" over a graph that missed nine amounts to.
///
/// The LEFT join is the load-bearing part. A code with no prose must surface
/// RAW, never drop the row: losing a boundary site from a boundary report is
/// the exact failure the report exists to prevent, and it would make an
/// incomplete answer look complete.
///
/// Mutation that must break this test: make the reason_codes join INNER.
#[tokio::test]
async fn graph_boundary_explains_a_miss_and_keeps_one_it_cannot_explain() {
    let s = pg_store().await;
    let folder = format!("boundary_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    let caller = s
        .seed_node(&fid, "function", "probe_caller", "src/probe.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();

    // Three seeded reasons across both kinds, plus one the vocabulary does not
    // have — a future variant, or a row somebody forgot to seed.
    for (names, reason) in [
        ("start", "receiver_type_unknown"),
        ("pool", "no_import_in_scope"),
        ("trim", "external_boundary"),
        ("mystery", "not_a_seeded_code"),
    ] {
        s.insert_edge_with_props(&EdgeRow {
            folder_id: &fid,
            source_id: &caller,
            target: EdgeTarget::Unresolved { name: Some(names), file: None },
            kind: "calls",
            props: &serde_json::json!({ "reason": reason }),
        })
        .await
        .unwrap();
    }

    /// One boundary row, named so the assertions below read as claims about a
    /// miss rather than as tuple indices.
    struct Site {
        names: String,
        code: Option<String>,
        kind: Option<String>,
        summary: Option<String>,
    }

    let rows: Vec<Site> = sqlx_core::query_as::query_as(
        "SELECT names, reason_code, reason_kind::text, reason_summary
           FROM sensei.graph_boundary
          WHERE folder_id = $1
          ORDER BY names",
    )
    .bind(fid)
    .fetch_all(&s.pool)
    .await
    .unwrap()
    .into_iter()
    .map(|(names, code, kind, summary)| Site { names, code, kind, summary })
    .collect();

    assert_eq!(rows.len(), 4, "every unplaced site is a row, explained or not");

    let by_name: std::collections::HashMap<&str, &Site> =
        rows.iter().map(|r| (r.names.as_str(), r)).collect();

    let typed = by_name["start"];
    assert_eq!(typed.code.as_deref(), Some("receiver_type_unknown"));
    assert_eq!(typed.kind.as_deref(), Some("fault"), "a receiver we failed to type is a fault");
    assert!(
        typed.summary.as_deref().is_some_and(|s| s.contains("type of the thing being called on")),
        "the prose must come through, not just the code: {:?}",
        typed.summary
    );

    assert_eq!(
        by_name["trim"].kind.as_deref(),
        Some("refusal"),
        "the world ending is a decision, not a failure — a reader must be able to \
         exclude it without also excluding real misses"
    );

    let unknown = by_name["mystery"];
    assert_eq!(unknown.code.as_deref(), Some("not_a_seeded_code"), "the raw code survives");
    assert_eq!(unknown.kind, None, "with no prose, because none is seeded");
    assert_eq!(unknown.summary, None);

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// `symbol_definitions` is what separates "no such symbol" from "symbol with no
/// callers" — the two answers a bare `[]` used to conflate. A reference STUB
/// carries the name but is not a definition, so it must NOT count as found;
/// otherwise a symbol that was only ever mentioned would report `found: true`
/// and send a reader looking for a definition that does not exist.
#[tokio::test]
async fn symbol_definitions_does_not_count_a_stub_as_a_definition() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("symdef_{}", uuid::Uuid::new_v4())).await;

    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "realThing",
        "src/real.rs",
        None,
        None,
        Some(7),
        Some(9),
    )
    .await
    .unwrap();
    // A stub: named, but no file_path — an unresolved reference, not a definition.
    s.seed_node_by_fqn(
        &fid,
        "rust·p·m·Stub·ghostThing",
        "function",
        "ghostThing",
        Some("rust"),
        None,
    )
    .await
    .unwrap();

    let defined = s.symbol_definitions(&[fid], "realThing").await.unwrap();
    assert_eq!(defined.len(), 1, "a local definition is found");
    assert_eq!(defined[0]["file_path"], "src/real.rs");
    assert_eq!(defined[0]["line_start"], serde_json::json!(7));

    assert!(
        s.symbol_definitions(&[fid], "ghostThing").await.unwrap().is_empty(),
        "a stub is a mention, not a definition — reporting it as found would promise \
         a definition site that does not exist"
    );
    assert!(
        s.symbol_definitions(&[fid], "neverHeardOfIt").await.unwrap().is_empty(),
        "an absent name is genuinely empty"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// Coverage must count the UNRESOLVED edges too, because that count is the only
/// thing that can say "this list is incomplete". Derived from its own query
/// rather than the returned list, which is `LIMIT 100`.
#[tokio::test]
async fn call_coverage_reports_unresolved_separately_per_direction() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("cov_{}", uuid::Uuid::new_v4())).await;

    let target = s
        .seed_node(&fid, "function", "target", "src/t.rs", None, None, Some(1), Some(2))
        .await
        .unwrap();
    let caller = s
        .seed_node(&fid, "function", "caller", "src/c.rs", None, None, Some(1), Some(2))
        .await
        .unwrap();
    s.insert_edge(&fid, &caller, Some(&target), None, None, "calls").await.unwrap();
    s.insert_edge(&fid, &caller, None, Some("target"), None, "calls").await.unwrap();

    let (resolved, unresolved) =
        s.call_coverage(&[fid], "target", CallDirection::Incoming).await.unwrap();
    assert_eq!(resolved, 1, "one edge landed on the target's id");
    assert_eq!(unresolved, 1, "one names it without landing — the list is incomplete");

    // Outgoing counts the same two edges from the caller's side.
    let (out_resolved, out_unresolved) =
        s.call_coverage(&[fid], "caller", CallDirection::Outgoing).await.unwrap();
    assert_eq!(out_resolved, 1);
    assert_eq!(out_unresolved, 1);

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// `get_callees_by_name` must label each callee's LOCALITY, and must read it
/// from `sensei.graph_nodes` rather than re-deriving it. Before this, every
/// callee came back with `kind`/`file_path`/`line_start` all null and no way to
/// tell a real internal dependency from an external library symbol from a name
/// the indexer never placed — live, `dedup_structural_folder_nodes` returned
/// `map_err`, `bind`, `execute`, `Ok`, `query` as one undifferentiated list.
#[tokio::test]
async fn get_callees_by_name_labels_locality_from_graph_nodes() {
    let s = pg_store().await;
    let folder = format!("callees_{}", uuid::Uuid::new_v4());
    let fid = create_test_folder(&s, &folder).await;

    let caller = s
        .seed_node(&fid, "function", "extract_deps", "src/deps.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    // internal: a local definition.
    let local = s
        .seed_node(&fid, "function", "parse_cargo", "src/cargo.rs", None, None, Some(3), Some(8))
        .await
        .unwrap();
    // external: the writer recorded a dependency's symbol.
    let lib = s
        .seed_node_by_fqn(
            &fid,
            "lib·serde·serde·from_str",
            "unknown",
            "from_str",
            Some("rust"),
            None,
        )
        .await
        .unwrap();

    s.insert_edge(&fid, &caller, Some(&local), None, None, "calls").await.unwrap();
    s.insert_edge(&fid, &caller, Some(&lib), None, None, "calls").await.unwrap();
    // unknown: an unresolved edge has NO target node to classify at all.
    s.insert_edge(&fid, &caller, None, Some("map_err"), None, "calls").await.unwrap();

    let callees = s.get_callees_by_name(&folder, "extract_deps").await.unwrap();
    let by_name: std::collections::HashMap<String, String> = callees
        .iter()
        .map(|c| {
            (
                c["name"].as_str().unwrap_or_default().to_string(),
                c["locality"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    assert_eq!(by_name.get("parse_cargo").map(String::as_str), Some("internal"));
    assert_eq!(by_name.get("from_str").map(String::as_str), Some("external"));
    assert_eq!(
        by_name.get("map_err").map(String::as_str),
        Some("unknown"),
        "an unresolved callee has no target node, so it is unknown — NOT external, \
         which is the misclassification the locality view exists to prevent"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// `node_id_by_fqn` must LOOK UP without creating, and must be folder-scoped.
///
/// The scope is the load-bearing half: fqns are unique per folder
/// (`nodes_unique_fqn` is `(folder_id, fqn)`), and 5 repos on this machine have
/// two checkouts each — an unscoped lookup would resolve an import to the
/// identically-named module in the OTHER checkout.
///
/// Breaking mutation: drop `AND folder_id = $1` from the SELECT — the cross-folder
/// case below starts returning Some.
#[tokio::test]
async fn node_id_by_fqn_looks_up_without_creating_and_is_folder_scoped() {
    let s = pg_store().await;
    let a = create_test_folder(&s, &format!("fqnlook_a_{}", uuid::Uuid::new_v4())).await;
    let b = create_test_folder(&s, &format!("fqnlook_b_{}", uuid::Uuid::new_v4())).await;

    let fqn = "typescript·app··lib/util";
    let id = s.seed_node_by_fqn(&a, fqn, "module", "util", Some("typescript"), None).await.unwrap();

    assert_eq!(s.node_id_by_fqn(&a, fqn).await.unwrap(), Some(id), "finds the node in its folder");
    assert_eq!(
        s.node_id_by_fqn(&b, fqn).await.unwrap(),
        None,
        "the SAME fqn in another folder is a different module — two checkouts of one repo \
         must not resolve into each other"
    );
    assert_eq!(
        s.node_id_by_fqn(&a, "typescript·app·nope/absent").await.unwrap(),
        None,
        "an absent fqn is None"
    );

    // And it must not have CREATED anything while looking.
    let (created,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id = ANY($1) AND fqn LIKE 'typescript·app·nope%'",
    )
    .bind(vec![a, b])
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(created, 0, "a lookup miss creates nothing — that is why it is not the upsert");

    s.delete_nodes_by_folder(&a).await.unwrap();
    s.delete_nodes_by_folder(&b).await.unwrap();
}

/// A doc's symbol mention resolves ONLY when the name is unambiguous.
///
/// A doc writes `` `handleAuth` `` with no signature and no module path. With
/// two same-named definitions there is nothing to choose on, so picking one
/// would publish a guess as a fact — the honest answer is `None`, leaving the
/// edge unresolved with the mention in `target_name`.
///
/// Breaking mutation: change `rows.len() == 1` to `!rows.is_empty()` — the
/// ambiguous case starts returning an arbitrary one of the two.
#[tokio::test]
async fn sole_definition_by_name_returns_none_when_ambiguous() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("soledef_{}", uuid::Uuid::new_v4())).await;

    // Unambiguous: exactly one definition.
    let only = s
        .seed_node(&fid, "function", "uniqueThing", "src/a.rs", None, None, Some(1), Some(2))
        .await
        .unwrap();
    assert_eq!(s.sole_definition_id_by_name(&fid, "uniqueThing").await.unwrap(), Some(only));

    // Ambiguous: two definitions of the same name in different files.
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "dupThing",
        "src/b.rs",
        None,
        None,
        Some(1),
        Some(2),
    )
    .await
    .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "dupThing",
        "src/c.rs",
        None,
        None,
        Some(1),
        Some(2),
    )
    .await
    .unwrap();
    assert_eq!(
        s.sole_definition_id_by_name(&fid, "dupThing").await.unwrap(),
        None,
        "two candidates means 'I don't know which' — never a best guess"
    );

    // Absent.
    assert_eq!(s.sole_definition_id_by_name(&fid, "neverDefined").await.unwrap(), None);

    // A STUB is not a definition: a doc mention must land on real code, not on
    // another unresolved reference to the same name.
    s.seed_node_by_fqn(&fid, "rust·p·m·Ghost·stubbed", "function", "stubbed", Some("rust"), None)
        .await
        .unwrap();
    assert_eq!(
        s.sole_definition_id_by_name(&fid, "stubbed").await.unwrap(),
        None,
        "a stub has no file_path and cannot anchor a doc reference"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A doc's file reference resolves against the repo-relative path, which is the
/// form `nodes.file_path` uses. Before this, refs were stored machine-absolute
/// and could never match.
#[tokio::test]
async fn file_node_lookup_matches_the_repo_relative_path() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("fileref_{}", uuid::Uuid::new_v4())).await;

    let f =
        s.seed_node(&fid, "file", "main.rs", "src/main.rs", None, None, None, None).await.unwrap();
    assert_eq!(s.file_node_id_by_path(&fid, "src/main.rs").await.unwrap(), Some(f));
    assert_eq!(
        s.file_node_id_by_path(&fid, "/abs/root/src/main.rs").await.unwrap(),
        None,
        "an absolute target cannot match a repo-relative file_path — the whole reason \
         doc references resolved at 0%"
    );
    assert_eq!(s.file_node_id_by_path(&fid, "src/absent.rs").await.unwrap(), None);

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// `edges.props` needs a write path, and it must MERGE rather than clobber.
///
/// Measured before this existed: 0 of 724,926 live edge rows carried non-empty
/// props, because neither `insert_edge` branch named the column. Inheritance
/// needs it — `Implements` and `TraitImpl` share the `implements` edge kind and
/// are told apart only by `props.relation`.
///
/// Merge, not overwrite, for the same reason `upsert_node` merges: an edge is
/// re-inserted on every rescan, and a later caller that knows less about the
/// edge must not erase what an earlier one recorded.
///
/// Breaking mutation: change `props = edges.props || EXCLUDED.props` to
/// `props = EXCLUDED.props` — the second-write assertion reads `relation` as
/// NULL and fails.
#[tokio::test]
async fn insert_edge_with_props_stamps_and_merges() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("edgeprops_{}", uuid::Uuid::new_v4())).await;
    let a = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "class",
        "Sub",
        "a.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    let b = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "class",
        "Base",
        "b.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();

    let props = serde_json::json!({ "relation": "trait_impl" });
    let e1 = s
        .insert_edge_with_props(&EdgeRow {
            folder_id: &fid,
            source_id: &a,
            target: EdgeTarget::Node(&b),
            kind: "implements",
            props: &props,
        })
        .await
        .unwrap();

    let stamped: Option<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT props->>'relation' FROM sensei.edges WHERE id = $1",
    )
    .bind(e1)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(stamped.as_deref(), Some("trait_impl"), "the discriminant must persist");

    // A second write with DIFFERENT keys must not erase the first.
    let e2 = s
        .insert_edge_with_props(&EdgeRow {
            folder_id: &fid,
            source_id: &a,
            target: EdgeTarget::Node(&b),
            kind: "implements",
            props: &serde_json::json!({ "other": 1 }),
        })
        .await
        .unwrap();
    assert_eq!(e1, e2, "same edge identity");
    let after: Option<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT props->>'relation' FROM sensei.edges WHERE id = $1",
    )
    .bind(e1)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(after.as_deref(), Some("trait_impl"), "merge must not clobber the discriminant");

    // The UNRESOLVED branch carries props too — an unresolved supertype is the
    // common case on a cold index, and it must still say which relation it is.
    let u = s
        .insert_edge_with_props(&EdgeRow {
            folder_id: &fid,
            source_id: &a,
            target: EdgeTarget::Unresolved { name: Some("Serializable"), file: None },
            kind: "implements",
            props: &props,
        })
        .await
        .unwrap();
    let u_rel: Option<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT props->>'relation' FROM sensei.edges WHERE id = $1",
    )
    .bind(u)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(u_rel.as_deref(), Some("trait_impl"), "unresolved edges need the discriminant too");

    // The plain wrapper must still work and leave props empty.
    let plain = s.insert_edge(&fid, &b, Some(&a), None, None, "calls").await.unwrap();
    let empty: serde_json::Value =
        sqlx_core::query_scalar::query_scalar("SELECT props FROM sensei.edges WHERE id = $1")
            .bind(plain)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(empty, serde_json::json!({}), "insert_edge must not invent props");
}

/// The sweep removes ONLY the mislabelled containment `extends` rows, and
/// cannot touch a real inheritance edge.
///
/// The old emit produced `file -> (unresolved type in that same file)` under
/// the `extends` kind: 7,916 rows in the live graph, all unresolved, all
/// duplicating containment that `nodes.parent_id` already carried. Retiring the
/// emit stops new ones; these are the ones already written.
///
/// The discriminant is the whole predicate. Every edge the inheritance path
/// writes carries `props.relation`; before `insert_edge_with_props` nothing
/// could write props at all — so "unstamped `extends`" names the legacy rows
/// exactly.
///
/// This test earns that narrowing. The first version also asserted a
/// file-source and null-target predicate, and mutation-probing showed the test
/// passed with EITHER clause removed — the fixtures did not isolate them. The
/// third edge below is the isolating case: file-sourced, unresolved, and
/// stamped, which the emit genuinely produces when a child fqn is missing. A
/// predicate keyed on file-source-and-unresolved would delete it.
///
/// Breaking mutation: drop `props->>'relation' IS NULL` from the WHERE — two
/// real inheritance edges are deleted and `removed` is 3.
#[tokio::test]
async fn the_sweep_takes_mislabelled_containment_and_spares_real_inheritance() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("sweep_{}", uuid::Uuid::new_v4())).await;
    let file = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "file",
        "a.rs",
        "a.rs",
        None,
        None,
        Some(1),
        Some(9),
    )
    .await
    .unwrap();
    let sub = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "class",
        "Sub",
        "a.rs",
        None,
        None,
        Some(2),
        Some(3),
    )
    .await
    .unwrap();
    let base = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "class",
        "Base",
        "b.rs",
        None,
        None,
        Some(1),
        Some(3),
    )
    .await
    .unwrap();

    // The mislabelled shape: FILE source, unresolved, no discriminant.
    s.insert_edge(&fid, &file, None, Some("Sub"), None, "extends").await.unwrap();
    // Real inheritance, resolved and stamped.
    s.insert_edge_with_props(&EdgeRow {
        folder_id: &fid,
        source_id: &sub,
        target: EdgeTarget::Node(&base),
        kind: "extends",
        props: &serde_json::json!({ "relation": "extends" }),
    })
    .await
    .unwrap();
    // A real UNRESOLVED inheritance edge — stamped but with no target yet. This
    // is the common cold-index case and must NOT be swept.
    s.insert_edge_with_props(&EdgeRow {
        folder_id: &fid,
        source_id: &sub,
        target: EdgeTarget::Unresolved { name: Some("Missing"), file: None },
        kind: "extends",
        props: &serde_json::json!({ "relation": "extends" }),
    })
    .await
    .unwrap();

    // The case that ISOLATES the discriminant clause: file-sourced AND
    // unresolved AND stamped. The inheritance emit really produces this — it
    // anchors on the file node when a child fqn is missing — so a predicate
    // keyed on "file-sourced and unresolved" would delete a real relation.
    s.insert_edge_with_props(&EdgeRow {
        folder_id: &fid,
        source_id: &file,
        target: EdgeTarget::Unresolved { name: Some("FromFileAnchor"), file: None },
        kind: "extends",
        props: &serde_json::json!({ "relation": "extends" }),
    })
    .await
    .unwrap();

    let removed = s.prune_mislabelled_containment_extends(&[fid]).await.unwrap();
    assert_eq!(removed, 1, "exactly the one mislabelled row");

    let left: Vec<(Option<String>,)> = sqlx_core::query_as::query_as(
        "SELECT props->>'relation' FROM sensei.edges
          WHERE folder_id = $1 AND kind = 'extends'::sensei.edge_kind",
    )
    .bind(fid)
    .fetch_all(s.pool())
    .await
    .unwrap();
    assert_eq!(left.len(), 3, "all three real inheritance edges survive: {left:?}");
    assert!(
        left.iter().all(|(r,)| r.as_deref() == Some("extends")),
        "every survivor is discriminated: {left:?}"
    );
}

/// `persist_edge_fact` must reproduce each inheritance arm's OBSERVED
/// behaviour, not a plausible version of it.
///
/// Written against what the arms at process.rs actually do, because a persister
/// tested against my assumptions would codify the assumptions. Each assertion
/// below mirrors a named existing test: the Lib arm mirrors the external half
/// of `trait_impls_become_relations_and_inherent_impls_do_not`, the stub arm
/// mirrors `a_trait_impl_persists_as_an_implements_edge_before_its_trait_exists`,
/// and the unresolvable arm mirrors the `parent_fqn = None` branch.
///
/// Breaking mutations: (1) consult `known` AFTER the stub instead of before —
/// the in-file assertion gets a different id; (2) drop `props` from the
/// insert — the discriminant assertion fails; (3) make `LeaveUnresolved`
/// create a node — the unresolved assertion finds a target_id.
#[tokio::test]
async fn persist_edge_fact_reproduces_every_inheritance_arm() {
    use crate::graph_facts::{EdgeFact, OnMiss, TargetRef};
    use std::collections::HashMap;

    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("facts_{}", uuid::Uuid::new_v4())).await;
    let sub = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "class",
        "Sub",
        "a.rs",
        None,
        None,
        Some(1),
        Some(3),
    )
    .await
    .unwrap();
    let inflight = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "class",
        "Known",
        "a.rs",
        None,
        None,
        Some(5),
        Some(7),
    )
    .await
    .unwrap();
    let mut known = HashMap::new();
    known.insert("rust·demo·a·Known".to_string(), inflight);

    let props = serde_json::json!({ "relation": "trait_impl" });
    let rel = |id: uuid::Uuid| async move {
        let row: (Option<uuid::Uuid>, Option<String>, Option<String>) =
            sqlx_core::query_as::query_as(
                "SELECT target_id, target_name, props->>'relation' FROM sensei.edges WHERE id = $1",
            )
            .bind(id)
            .fetch_one(pg_store().await.pool())
            .await
            .unwrap();
        row
    };

    // ARM: in-file hit. `known` is consulted FIRST, so no upsert happens and
    // the existing node's id is used — which is what protects it from being
    // relabelled by `language = COALESCE(EXCLUDED.language, nodes.language)`.
    let e = s
        .persist_edge_fact(
            &fid,
            &EdgeFact {
                source_id: sub,
                target: TargetRef::Internal {
                    fqn: "rust·demo·a·Known".into(),
                    name: "Known".into(),
                    on_miss: OnMiss::CreateStub { kind: "class" },
                },
                kind: "implements",
                props: props.clone(),
            },
            &known,
            Some("rust"),
        )
        .await
        .unwrap();
    let (tid, tname, r) = rel(e).await;
    assert_eq!(tid, Some(inflight), "an in-file target must reuse its id, not be re-upserted");
    assert_eq!(tname, None, "resolving erases target_name");
    assert_eq!(r.as_deref(), Some("trait_impl"), "props must reach the row");

    // ARM: stub. The target is unknown, so a placeholder is created with the
    // stated kind and the edge resolves to it — the property that makes
    // resolution order-independent.
    let e = s
        .persist_edge_fact(
            &fid,
            &EdgeFact {
                source_id: sub,
                target: TargetRef::Internal {
                    fqn: "rust·demo·b·Later".into(),
                    name: "Later".into(),
                    on_miss: OnMiss::CreateStub { kind: "class" },
                },
                kind: "implements",
                props: props.clone(),
            },
            &known,
            Some("rust"),
        )
        .await
        .unwrap();
    let (tid, _, _) = rel(e).await;
    assert!(tid.is_some(), "a stub must resolve the edge");
    let stub_kind: Option<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT kind::text FROM sensei.nodes WHERE folder_id = $1 AND fqn = $2",
    )
    .bind(fid)
    .bind("rust·demo·b·Later")
    .fetch_optional(s.pool())
    .await
    .unwrap()
    .flatten();
    assert_eq!(stub_kind.as_deref(), Some("class"), "the stub carries the kind the policy stated");

    // ARM: LeaveUnresolved on a probe miss — no node is invented.
    let e = s
        .persist_edge_fact(
            &fid,
            &EdgeFact {
                source_id: sub,
                target: TargetRef::Internal {
                    fqn: "rust·demo·c·Absent".into(),
                    name: "Absent".into(),
                    on_miss: OnMiss::LeaveUnresolved,
                },
                kind: "implements",
                props: props.clone(),
            },
            &known,
            Some("rust"),
        )
        .await
        .unwrap();
    let (tid, tname, _) = rel(e).await;
    assert_eq!(tid, None, "LeaveUnresolved must NOT create a node");
    assert_eq!(tname.as_deref(), Some("Absent"), "the name is kept so the fact survives");
    let invented: Option<uuid::Uuid> = s.node_id_by_fqn(&fid, "rust·demo·c·Absent").await.unwrap();
    assert!(invented.is_none(), "no row may be minted on a probe miss");

    // ARM: external. Mints a lib_symbol under a lib_package, carrying the
    // EXPLICIT name rather than the fqn's last segment.
    let e = s
        .persist_edge_fact(
            &fid,
            &EdgeFact {
                source_id: sub,
                target: TargetRef::Lib {
                    fqn: "lib·std·std::fmt·Debug".into(),
                    name: "Debug".into(),
                    package: "std".into(),
                },
                kind: "implements",
                props,
            },
            &known,
            Some("rust"),
        )
        .await
        .unwrap();
    let (tid, _, _) = rel(e).await;
    assert!(tid.is_some(), "an external supertype resolves to a lib node");
    let lib_name: Option<String> = sqlx_core::query_scalar::query_scalar(
        "SELECT name FROM sensei.nodes WHERE folder_id = $1 AND fqn = $2",
    )
    .bind(fid)
    .bind("lib·std·std::fmt·Debug")
    .fetch_optional(s.pool())
    .await
    .unwrap()
    .flatten();
    assert_eq!(lib_name.as_deref(), Some("Debug"), "the lib node's name is the one supplied");
}

/// A RESOLVED import must stay distinguishable from an external one.
///
/// `import_target_counts` used `COALESCE(target_name, '')`, justified by a
/// comment reading "MEASURED non-null on all 136,484 import edges". That was
/// true when written — imports were 0% resolved. Resolving ERASES `target_name`
/// (the `target_id` xor `target_name` invariant), so once 25,788 imports
/// resolved, every one of them collapsed into a single `target = ''` group,
/// which `classify_import("")` then reported as ONE external package.
///
/// Live effect: `/api/graph/imports` reported external 136,329 / local 244 when
/// the truth is 110,785 / 25,788. That is the instrument any claim about
/// import resolution would be measured with.
///
/// Breaking mutation: restore `COALESCE(target_name, '')` — the resolved row
/// arrives as `Some("")` instead of `None` and the assertion fails.
#[tokio::test]
async fn a_resolved_import_is_not_reported_as_an_empty_external_target() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("impcount_{}", uuid::Uuid::new_v4())).await;
    let src = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "file",
        "a.ts",
        "a.ts",
        None,
        None,
        Some(1),
        Some(9),
    )
    .await
    .unwrap();
    let tgt = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "module",
        "b",
        "b.ts",
        None,
        None,
        Some(1),
        Some(9),
    )
    .await
    .unwrap();

    // A resolved import: target_id set, target_name NULL — the shape the
    // invariant guarantees.
    s.insert_edge(&fid, &src, Some(&tgt), None, None, "imports").await.unwrap();
    // An external one, unresolved, carrying its specifier.
    s.insert_edge(&fid, &src, None, Some("java.util.List"), None, "imports").await.unwrap();

    let rows = s.import_target_counts().await.unwrap();
    let mine: Vec<_> = rows
        .iter()
        .filter(|(t, _, _)| t.is_none() || t.as_deref() == Some("java.util.List"))
        .collect();

    let resolved = mine
        .iter()
        .find(|(t, _, _)| t.is_none())
        .expect("a resolved import must arrive with target = None, not Some(\"\")");
    assert!(resolved.2 >= 1, "the resolved row must count as resolved: {resolved:?}");

    let external = mine
        .iter()
        .find(|(t, _, _)| t.as_deref() == Some("java.util.List"))
        .expect("the external specifier must survive as itself");
    assert_eq!(external.2, 0, "an unresolved external counts 0 resolved: {external:?}");
}

/// The classes must PARTITION the edges, and each must mean what it is named.
///
/// The view answers exactly one question — does any LOCAL node share the target
/// name — and the classes are named for that, not for a verdict. An earlier
/// version named them `unambiguous-miss`/`ambiguous`/`absent` and this test
/// asserted the first was "the defect class". Adversarial review refuted it:
/// the head of that population is `json` 1,600, `path` 483, `join` 443 —
/// external accessor methods sharing a name with one local symbol — and the
/// only mechanism that drives the number down is `sole_definition_id_by_name`,
/// the bare-name resolver `process.rs` explicitly refuses because "a miss would
/// resolve confidently WRONG".
///
/// So this test now pins the PARTITION and the naming, which are true, and
/// asserts nothing about defects, which was not.
///
/// The solid finding it does protect: `no-local-name` for imports is 109,944 of
/// 110,785 (99.2%), which is why externals need lib nodes rather than better
/// local resolution.
///
/// Breaking mutation: change the view's `= 1` to `>= 1` — degree-one and
/// degree-n collapse and the per-class assertions fail.
#[tokio::test]
async fn the_resolution_classes_are_distinct_and_only_one_is_a_defect() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("rescls_{}", uuid::Uuid::new_v4())).await;
    let src = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "file",
        "a.rs",
        "a.rs",
        None,
        None,
        Some(1),
        Some(9),
    )
    .await
    .unwrap();
    // One local definition named `only` → a miss on it is UNAMBIGUOUS.
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "only",
        "b.rs",
        None,
        None,
        Some(1),
        Some(2),
    )
    .await
    .unwrap();
    // Two named `twin` → a miss on it is AMBIGUOUS, and correctly unresolved.
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "twin",
        "c.rs",
        None,
        None,
        Some(1),
        Some(2),
    )
    .await
    .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "twin",
        "d.rs",
        None,
        None,
        Some(1),
        Some(2),
    )
    .await
    .unwrap();

    for name in ["only", "twin", "nowhere"] {
        s.insert_edge(&fid, &src, None, Some(name), None, "calls").await.unwrap();
    }

    let rows: Vec<(Option<String>, i64)> = sqlx_core::query_as::query_as(
        "SELECT resolution_class, count(*) FROM sensei.edge_resolution_class
          WHERE folder_id = $1 GROUP BY 1",
    )
    .bind(fid)
    .fetch_all(s.pool())
    .await
    .unwrap();
    let get =
        |c: &str| rows.iter().find(|(k, _)| k.as_deref() == Some(c)).map(|(_, n)| *n).unwrap_or(0);

    assert_eq!(get("name-collision-1"), 1, "`only` is borne by exactly one local node: {rows:?}");
    assert_eq!(get("name-collision-n"), 1, "`twin` is borne by two local nodes: {rows:?}");
    assert_eq!(get("no-local-name"), 1, "`nowhere` names nothing local: {rows:?}");

    // The classes must be a PARTITION — every edge in exactly one.
    let total: i64 = rows.iter().map(|(_, n)| *n).sum();
    let edges: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM sensei.edges WHERE folder_id = $1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(total, edges, "every edge must fall in exactly one class");
}

/// A lib node with no edges must be collectable.
///
/// Today it is not: `prune_orphan_stubs_scoped` explicitly EXCLUDES
/// `lib_symbol`/`lib_package`, and no other path deletes them
/// (`delete_nodes_by_file` and `prune_file_nodes` both key on `file_path`,
/// which lib nodes do not have). So a lib node, once minted, is permanent.
///
/// That is why this lands BEFORE any minting. Slice 4 proposes creating
/// ~11,714 lib_symbol rows plus ~3,196 containers from a derivation that has
/// already been wrong once in review; without a collector, a bad mint could
/// only be undone by hand-written SQL against the live graph.
///
/// Containers are collected too, but only when EMPTY — a `lib_package` whose
/// symbols are still referenced must survive, or the next scan re-mints it and
/// the pair churns forever.
///
/// Breaking mutations: (1) drop the lib kinds from the DELETE — the orphan
/// survives; (2) drop the `NOT EXISTS` edge guard — the REFERENCED lib symbol
/// is deleted, which would break live edges.
#[tokio::test]
async fn an_unreferenced_lib_node_is_collected_and_a_referenced_one_survives() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("libgc_{}", uuid::Uuid::new_v4())).await;
    let src = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "file",
        "a.rs",
        "a.rs",
        None,
        None,
        Some(1),
        Some(9),
    )
    .await
    .unwrap();

    // Referenced: a lib symbol with an edge pointing at it.
    let kept = s
        .upsert_lib_node_by_fqn(
            &fid,
            "lib·serde·serde·Serialize",
            "Serialize",
            "serde",
            Some("rust"),
        )
        .await
        .unwrap();
    s.insert_edge(&fid, &src, Some(&kept), None, None, "calls").await.unwrap();

    // Orphan: minted and never pointed at — the shape a wrong derivation leaves.
    let orphan = s
        .upsert_lib_node_by_fqn(&fid, "lib·typo·typo·Nothing", "Nothing", "typo", Some("rust"))
        .await
        .unwrap();

    let removed = s.prune_unreferenced_lib_nodes_scoped(&[fid]).await.unwrap();
    assert!(removed >= 1, "the orphan must be collected, got {removed}");

    assert!(
        s.node_id_by_fqn(&fid, "lib·typo·typo·Nothing").await.unwrap().is_none(),
        "the unreferenced lib symbol is gone"
    );
    assert_eq!(
        s.node_id_by_fqn(&fid, "lib·serde·serde·Serialize").await.unwrap(),
        Some(kept),
        "a REFERENCED lib symbol must survive — deleting it would break a live edge"
    );
    // Its container survives with it.
    assert!(
        s.node_id_by_fqn(&fid, "lib·serde").await.unwrap().is_some(),
        "a container with surviving symbols must not be collected"
    );
    // The orphan's now-empty container goes too.
    assert!(
        s.node_id_by_fqn(&fid, "lib·typo").await.unwrap().is_none(),
        "an empty container is collected with its last symbol"
    );
    let _ = orphan;
}
