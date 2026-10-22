use super::*;

// ── Transitive receiver resolution (hop 2) ──────────────────────────────────

/// Write a real DEFINITION node (has a file_path, so it is not a stub).
async fn rt_def(
    s: &PgStore,
    fid: &uuid::Uuid,
    fqn: &str,
    kind: &str,
    name: &str,
    parent: Option<&uuid::Uuid>,
) -> uuid::Uuid {
    rt_def_at(s, fid, fqn, kind, name, parent, "rust", "src/lib.rs").await
}

/// [`rt_def`] in a named language and a named FILE.
///
/// Both matter. The chain reads a RUST return type, so what a same-named node in
/// another language does is a rule of its own — and `nodes_unique_identity` is
/// `(folder, file_path, kind, name, parent_id, line_start)`, so two same-named
/// types written to the same file are ADOPTED onto one node and a fixture meant
/// to be ambiguous would quietly stop being.
#[allow(clippy::too_many_arguments)]
async fn rt_def_at(
    s: &PgStore,
    fid: &uuid::Uuid,
    fqn: &str,
    kind: &str,
    name: &str,
    parent: Option<&uuid::Uuid>,
    language: &str,
    file_path: &str,
) -> uuid::Uuid {
    s.seed_node_by_fqn(
        fid,
        fqn,
        kind,
        name,
        Some(language),
        Some(FqnDef {
            file_path,
            signature: None,
            line_start: Some(1),
            line_end: Some(2),
            is_exported: true,
            parent_id: parent,
        }),
    )
    .await
    .unwrap()
}

/// `TaskContext` with a `pg` returning a `PgStore`, and a `PgStore` with a
/// `count_edges` — the exact live chain the slice exists to close.
async fn rt_fixture(s: &PgStore, fid: &uuid::Uuid) -> (uuid::Uuid, uuid::Uuid) {
    let store =
        rt_def(s, fid, "rust·recv·db::pg_store·PgStore·item", "struct", "PgStore", None).await;
    let count = rt_def(
        s,
        fid,
        "rust·recv·db::pg_store·PgStore·count_edges·item",
        "method",
        "count_edges",
        Some(&store),
    )
    .await;
    let tc =
        rt_def(s, fid, "rust·recv·executor·TaskContext·item", "struct", "TaskContext", None).await;
    let pg =
        rt_def(s, fid, "rust·recv·executor·TaskContext·pg·item", "method", "pg", Some(&tc)).await;
    (pg, count)
}

async fn resolve_one(
    s: &PgStore,
    fid: &uuid::Uuid,
    hint: ReceiverHint,
    method: &str,
) -> Option<uuid::Uuid> {
    s.resolve_receiver_calls(&[*fid], &[(hint, method.to_string())])
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
}

/// THE PAYOFF HOP. `ctx.pg().count_edges()` records `ReturnOf(TaskContext·pg)`;
/// the resolver reads that node's return type, unwraps it to `PgStore`, and
/// finds `count_edges` under the type of that name.
#[tokio::test]
async fn a_receiver_hint_resolves_through_the_hinted_methods_return_type() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let (pg, count) = rt_fixture(&s, &fid).await;

    assert_eq!(
        resolve_one(
            &s,
            &fid,
            ReceiverHint::ReturnOf("rust·recv·executor·TaskContext·pg·item".into()),
            "count_edges"
        )
        .await,
        None,
        "before the return type is known the hop MISSES — a hint is not a licence to guess"
    );

    s.set_node_return_type(&pg, "&crate::db::pg_store::PgStore").await.unwrap();

    assert_eq!(
        resolve_one(
            &s,
            &fid,
            ReceiverHint::ReturnOf("rust·recv·executor·TaskContext·pg·item".into()),
            "count_edges"
        )
        .await,
        Some(count),
        "the chain closes: pg → &crate::db::pg_store::PgStore → PgStore → count_edges"
    );
}

/// EVERY hop that cannot be answered yields UNRESOLVED. None of these may
/// return a node: an almost-right target is indistinguishable from a real one
/// to every consumer downstream.
#[tokio::test]
async fn every_failed_hop_in_the_receiver_chain_yields_unresolved() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let (pg, _) = rt_fixture(&s, &fid).await;
    let hint = || ReceiverHint::ReturnOf("rust·recv·executor·TaskContext·pg·item".into());

    // Hop 1: the hinted node is not in the graph at all.
    assert_eq!(
        resolve_one(
            &s,
            &fid,
            ReceiverHint::ReturnOf("rust·recv·nope·Ghost·m·item".into()),
            "count_edges"
        )
        .await,
        None,
        "a hint naming no node is a miss"
    );

    // Hop 2: the return type names no concrete type.
    for opaque in ["impl Store", "Box<dyn Store>", "T"] {
        s.set_node_return_type(&pg, opaque).await.unwrap();
        assert_eq!(
            resolve_one(&s, &fid, hint(), "count_edges").await,
            None,
            "`-> {opaque}` names no receiver type, so the call stays unresolved"
        );
    }

    // Hop 3: the type resolves but carries no such member. This is the guard
    // that keeps a `Result`/`Option` unwrap from wrong-merging — the member has
    // to actually exist on the success type.
    s.set_node_return_type(&pg, "Result<PgStore, Error>").await.unwrap();
    assert_eq!(
        resolve_one(&s, &fid, hint(), "no_such_method").await,
        None,
        "the unwrap only lands if the member really is on the concrete type"
    );
    assert!(
        resolve_one(&s, &fid, hint(), "count_edges").await.is_some(),
        "…and when it is, `Result<PgStore, E>` does reach it"
    );
}

/// A STUB is never the answer. It carries the right name under the right type
/// and no definition behind it, so resolving onto one would launder an
/// unresolved call into a resolved one — the caller could not tell the
/// difference, which is precisely what makes it a fabrication.
///
/// The adversarial row is written with raw SQL because no emit path produces it
/// TODAY: `persist_edge_fact`'s `CreateStub` passes no parent, so the 18,408
/// live fn/method stubs are all parentless and the parent join already skips
/// them (measured: 0 stubs with a parent). That is a property of the current
/// writers, not of this lookup. The `file_id IS NOT NULL` filter is what
/// keeps the property from depending on them — re-parent stubs once (an
/// index-audit repair, a dedup pass) and without it every ghost becomes a
/// resolution target.
#[tokio::test]
async fn a_stub_is_never_the_answer_to_a_receiver_hop() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let ghost = rt_def(&s, &fid, "rust·recv·other·PgStore·item", "struct", "PgStore", None).await;
    let (stub,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.nodes (folder_id, fqn, kind, name, language, parent_id, file_id)
         VALUES ($1, 'rust·recv·other·PgStore·only_stubbed',
                 'function'::sensei.node_kind, 'only_stubbed', 'rust', $2, NULL)
         RETURNING id",
    )
    .bind(fid)
    .bind(ghost)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let _ = stub;

    // A hop that reaches the ghost type by its own fqn — the strongest form of
    // hint there is, so what refuses here is the stub rule and nothing weaker.
    let tc = rt_def(&s, &fid, "rust·recv·other·Ctx·item", "struct", "Ctx", None).await;
    let mk = rt_def(&s, &fid, "rust·recv·other·Ctx·mk·item", "method", "mk", Some(&tc)).await;
    s.set_node_return_type(&mk, "crate::other::PgStore").await.unwrap();

    assert_eq!(
        resolve_one(
            &s,
            &fid,
            ReceiverHint::ReturnOf("rust·recv·other·Ctx·mk·item".into()),
            "only_stubbed"
        )
        .await,
        None,
        "a stub carries the name but no definition — resolving onto it would launder \
         an unresolved call into a resolved one"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// Two candidates is not a coin flip. `sole_definition_id_by_name` bare-name
/// matching is refused everywhere else in this graph for the same reason.
#[tokio::test]
async fn an_ambiguous_member_lookup_refuses_rather_than_picking_one() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let (pg, _) = rt_fixture(&s, &fid).await;
    s.set_node_return_type(&pg, "PgStore").await.unwrap();
    let hint = || ReceiverHint::ReturnOf("rust·recv·executor·TaskContext·pg·item".into());

    assert!(resolve_one(&s, &fid, hint(), "count_edges").await.is_some(), "one candidate resolves");

    // A SECOND type of the same name in the same scope, also carrying the
    // member — the split-impl / trait-qualified shape the live graph is full of.
    let other = rt_def(&s, &fid, "rust·recv·shadow·PgStore·item", "class", "PgStore", None).await;
    rt_def(
        &s,
        &fid,
        "rust·recv·shadow·PgStore·count_edges·item",
        "method",
        "count_edges",
        Some(&other),
    )
    .await;

    assert_eq!(
        resolve_one(&s, &fid, hint(), "count_edges").await,
        None,
        "two definitions of PgStore::count_edges in scope → the graph cannot say which, \
         and saying one is a fabricated link"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// AMBIGUITY IS A PROPERTY OF THE TYPE, NOT OF THE MEMBER. Two same-named types
/// whose member sets are DISJOINT are the normal case for unrelated types that
/// happen to share a name, and gating on "exactly one member matched" waves them
/// through: only one of the two carries the member, so the count is 1 and the
/// call links to whichever type owns it.
///
/// Live shapes this refuses, all real in this repo:
///   - `verdicts.rs` `enum Verdict` (`as_wire`) vs `tasks/verdict_classifier.rs`
///     `enum Verdict` (`as_str`) — one crate, two modules, no member in common.
///   - `languages/import_target.rs` `enum ImportTarget` vs
///     `languages/typescript.rs` `struct ImportTarget`, member `is_external`.
///   - `dojo_client::session·Session` vs `session-report·model·Session`, member
///     `wall_ms` — different CRATES, and `scope_folder_ids` puts every folder of
///     the workspace in one scope, so the join spans them.
#[tokio::test]
async fn two_same_named_types_with_disjoint_members_resolve_to_nothing() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let ctx = rt_def(&s, &fid, "rust·recv·api·Api·item", "struct", "Api", None).await;
    let judge =
        rt_def(&s, &fid, "rust·recv·api·Api·judge·item", "method", "judge", Some(&ctx)).await;
    s.set_node_return_type(&judge, "Verdict").await.unwrap();

    let wire = rt_def_at(
        &s,
        &fid,
        "rust·recv·verdicts·Verdict·item",
        "enum",
        "Verdict",
        None,
        "rust",
        "src/verdicts.rs",
    )
    .await;
    rt_def(&s, &fid, "rust·recv·verdicts·Verdict·as_wire·item", "method", "as_wire", Some(&wire))
        .await;
    let classifier = rt_def_at(
        &s,
        &fid,
        "rust·recv·tasks::verdict_classifier·Verdict·item",
        "enum",
        "Verdict",
        None,
        "rust",
        "src/tasks/verdict_classifier.rs",
    )
    .await;
    rt_def(
        &s,
        &fid,
        "rust·recv·tasks::verdict_classifier·Verdict·as_str·item",
        "method",
        "as_str",
        Some(&classifier),
    )
    .await;
    assert_ne!(wire, classifier, "the fixture must really hold TWO type nodes");

    let hint = || ReceiverHint::ReturnOf("rust·recv·api·Api·judge·item".into());
    assert_eq!(
        resolve_one(&s, &fid, hint(), "as_wire").await,
        None,
        "`-> Verdict` names two different enums in scope; the member sits on one of \
         them, but WHICH Verdict was returned is unknown — linking is a wrong merge"
    );
    assert_eq!(
        resolve_one(&s, &fid, hint(), "as_str").await,
        None,
        "…and the same from the other type's side"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// THE MODULE PATH IS THE ANSWER, NOT A TIE-BREAK. The return type is stored
/// verbatim precisely because the path says WHICH `PgStore` is meant, so a
/// qualified one resolves even where the bare name is ambiguous — refusing here
/// would throw away the very fact the verbatim storage exists to keep.
#[tokio::test]
async fn a_module_qualified_return_type_picks_its_own_type_out_of_two() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let (pg, count) = rt_fixture(&s, &fid).await;
    let shadow = rt_def_at(
        &s,
        &fid,
        "rust·recv·shadow·PgStore·item",
        "struct",
        "PgStore",
        None,
        "rust",
        "src/shadow.rs",
    )
    .await;
    let shadow_count = rt_def_at(
        &s,
        &fid,
        "rust·recv·shadow·PgStore·count_edges·item",
        "method",
        "count_edges",
        Some(&shadow),
        "rust",
        "src/shadow.rs",
    )
    .await;
    assert_ne!(shadow_count, count, "the fixture must really hold TWO count_edges");
    let hint = || ReceiverHint::ReturnOf("rust·recv·executor·TaskContext·pg·item".into());

    s.set_node_return_type(&pg, "&crate::db::pg_store::PgStore").await.unwrap();
    assert_eq!(
        resolve_one(&s, &fid, hint(), "count_edges").await,
        Some(count),
        "`crate::db::pg_store::PgStore` names one of the two exactly"
    );

    s.set_node_return_type(&pg, "crate::shadow::PgStore").await.unwrap();
    assert_eq!(
        resolve_one(&s, &fid, hint(), "count_edges").await,
        Some(shadow_count),
        "…and the other path names the other one — the module segment decides"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// AN EXTERNAL RETURN TYPE RESOLVES TO NOTHING. The producer already refuses to
/// hint an external RECEIVER (a `lib·` node carries no definition), and reducing
/// `reqwest::blocking::Client` to the bare leaf `Client` reopened that hole one
/// level down: the leaf then matches a FIRST-PARTY type of that name and the
/// call is reported `locality: "internal"` with a real file path — a fabricated
/// first-party dependency.
///
/// Live call sites that emit exactly this hint: `cli/src/main.rs` `client()`
/// (`-> reqwest::blocking::Client`), `federation/mod.rs` `http_client()` and
/// `dojo/client.rs` `http()` (`-> &reqwest::Client`).
#[tokio::test]
async fn an_external_return_type_never_lands_on_a_same_named_first_party_type() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let app = rt_def(&s, &fid, "rust·recv·cli·App·item", "struct", "App", None).await;
    let mk =
        rt_def(&s, &fid, "rust·recv·cli·App·client·item", "method", "client", Some(&app)).await;

    // The first-party wrapper that makes the collision bite. It is the ONLY
    // `Client` in scope, so every count gate passes and only the path can refuse.
    let wrapper = rt_def(&s, &fid, "rust·recv·http·Client·item", "struct", "Client", None).await;
    rt_def(&s, &fid, "rust·recv·http·Client·get·item", "method", "get", Some(&wrapper)).await;

    for external in ["reqwest::blocking::Client", "&reqwest::Client", "std::sync::mpsc::Client"] {
        s.set_node_return_type(&mk, external).await.unwrap();
        assert_eq!(
            resolve_one(
                &s,
                &fid,
                ReceiverHint::ReturnOf("rust·recv·cli·App·client·item".into()),
                "get"
            )
            .await,
            None,
            "`-> {external}` names a dependency's type; the first-party `Client` is a \
             different type and linking to it invents a dependency"
        );
    }
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The return-type grammar hop 1 reads is RUST's, so a same-named node in
/// another language cannot be what it named. Hop 1 already gates on
/// `language = 'rust'`; hop 2 did not, so a TypeScript class of the same name
/// answered a question asked in Rust.
#[tokio::test]
async fn a_non_rust_type_never_answers_a_rust_return_type() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let tc = rt_def(&s, &fid, "rust·recv·executor·TaskContext·item", "struct", "TaskContext", None)
        .await;
    let pg =
        rt_def(&s, &fid, "rust·recv·executor·TaskContext·pg·item", "method", "pg", Some(&tc)).await;
    s.set_node_return_type(&pg, "PgStore").await.unwrap();

    let ts = rt_def_at(
        &s,
        &fid,
        "ts·web·store·PgStore",
        "class",
        "PgStore",
        None,
        "typescript",
        "web/store.ts",
    )
    .await;
    rt_def_at(
        &s,
        &fid,
        "ts·web·store·PgStore·count_edges",
        "method",
        "count_edges",
        Some(&ts),
        "typescript",
        "web/store.ts",
    )
    .await;

    assert_eq!(
        resolve_one(
            &s,
            &fid,
            ReceiverHint::ReturnOf("rust·recv·executor·TaskContext·pg·item".into()),
            "count_edges"
        )
        .await,
        None,
        "a rust `-> PgStore` cannot mean a TypeScript class, however alike the names"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// `-> Arc<Self>` is only meaningful against the impl the method sits in, which
/// is the hinted node's PARENT — a fact the graph already holds, so nothing has
/// to be inferred from the caller's file.
#[tokio::test]
async fn self_in_a_return_type_resolves_against_the_hinted_methods_own_type() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("recv_{}", uuid::Uuid::new_v4())).await;
    let store =
        rt_def(&s, &fid, "rust·recv·db::pg_store·PgStore·item", "struct", "PgStore", None).await;
    let clone_ref = rt_def(
        &s,
        &fid,
        "rust·recv·db::pg_store·PgStore·shared·item",
        "method",
        "shared",
        Some(&store),
    )
    .await;
    let count = rt_def(
        &s,
        &fid,
        "rust·recv·db::pg_store·PgStore·count_edges·item",
        "method",
        "count_edges",
        Some(&store),
    )
    .await;
    s.set_node_return_type(&clone_ref, "Arc<Self>").await.unwrap();

    assert_eq!(
        resolve_one(
            &s,
            &fid,
            ReceiverHint::ReturnOf("rust·recv·db::pg_store·PgStore·shared·item".into()),
            "count_edges"
        )
        .await,
        Some(count),
        "Arc<Self> on PgStore::shared is a PgStore receiver"
    );
}

async fn modified_at(s: &PgStore, id: &uuid::Uuid) -> chrono::DateTime<chrono::Utc> {
    let (t,): (chrono::DateTime<chrono::Utc>,) =
        query_as("SELECT modified_at FROM sensei.nodes WHERE id = $1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    t
}

/// The return-type write runs on EVERY function and method of EVERY re-scan —
/// 108,438 of the 136,583 definitions in the live index — so a write that always
/// touches the row makes `modified_at` say "this function changed" on every scan
/// of an unchanged file. Identical value in, no write out.
#[tokio::test]
async fn rewriting_an_identical_return_type_leaves_the_node_untouched() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("rtnoop_{}", uuid::Uuid::new_v4())).await;
    let id = rt_def(&s, &fid, "rust·rtnoop·m·Widget", "struct", "Widget", None).await;

    s.set_node_return_type(&id, "PgStore").await.unwrap();
    let after_first = modified_at(&s, &id).await;

    s.set_node_return_type(&id, "PgStore").await.unwrap();
    assert_eq!(
        modified_at(&s, &id).await,
        after_first,
        "an identical return type is not a change, so the row must not be touched"
    );

    s.set_node_return_type(&id, "OtherStore").await.unwrap();
    let after_change = modified_at(&s, &id).await;
    assert!(after_change > after_first, "a DIFFERENT return type is a change and still writes");

    // Clearing is the same rule from the other side: the first clear removes the
    // key, the second has nothing to remove.
    s.set_node_return_type(&id, "").await.unwrap();
    let after_clear = modified_at(&s, &id).await;
    assert!(after_clear > after_change, "clearing a stored type is a change");
    s.set_node_return_type(&id, "()").await.unwrap();
    assert_eq!(
        modified_at(&s, &id).await,
        after_clear,
        "`()` on a function that already returns nothing writes nothing"
    );

    // The no-op must not swallow the bad-id error: skipping a write and finding
    // no row are different answers and only one of them is success.
    assert!(
        s.set_node_return_type(&uuid::Uuid::new_v4(), "PgStore").await.is_err(),
        "a node that does not exist is still an error, not a silent no-op"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}
