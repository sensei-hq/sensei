# Checkpoint

**Slice** — indexer-v2. Review round: the receiver-return rung's type-home
derivation, the Rust binding rung's missing clear, and three untested clauses.

**Done**

- `Ladder::types_home_of` was a third, unguarded copy of a derivation
  `SuppliedMembers::of` does carefully. Both now go through one `hung_on`, and
  the rung narrows by PACKAGE, by LANGUAGE and by AMBIGUITY. Four tests, four
  mutations, each clause individually load-bearing.
- The Rust walk forgets what a name held when a `let` or a `for` rebinds it —
  the guard `Flow::bind` has had in TypeScript since 04b S1. Both call sites
  mutation-checked on their own.
- Three array-element clauses that killed no test now do: the clear in
  `Flow::bind`, the intersection in `Flow::join`, the carry in `Walk::captured`.
- The by-kind table grew a `narrowed` column. `lost` is defined by whichever
  narrowings `NamedBy::verdict` applies; three were added mid-sweep and the
  drops read as resolver progress. The column separates the two: ts 1778
  nodes are out of `lost` because of a narrowing, rust 181.

**The table moved, and all 19 newly-lost nodes are accounted for by name**

rust Field 1433→1443, rust Method 593→602, ts Field 2121→2118. Listed, not
inferred: 2 are declarations this change itself added to the corpus; 5 and 7
are `FileFacts` (`indexer/structure.rs` and `indexer/facts.rs`) and
`LanguageAdapter` (`languages/mod.rs` and `indexer/lang/mod.rs`) — each declared
twice in package `senseid`, so each of those edges was right only because
`indexer::facts` sorts before `indexer::structure`; 3 are the same class in
`playbook.rs`; 2 are `PgStore::prune_logs` and `enrich_assistant_events`, true
edges that survived only on a stale binding — `build_full_app(pg: PgStore)`
shadowed by `let pg = state.pg.clone()`, which `simple_type_name` refuses
because an `Arc<T>` may own the member. 3 ts field edges were GAINED.

**Remaining** — ts Field 2118, rust Field 1443, rust Method 602, ts Method 59,
ts Property 3. The two `PgStore` methods come back with an `Arc<T>` deref rule,
which `lang/rust/types.rs` defers deliberately.

**Next command**

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Next cause, already sized** — a svelte `{#each xs as x}` binding is untyped
exactly as a callback parameter was, and names 1,491 unresolved field reads
(the largest remaining bucket). It lives in `lang/svelte.rs`, not the JS walk.
After that: a chained `xs.filter(..).map(..)` keeping its element type, and
2,798 lambda-parameter receivers.

**Open questions** — (1) `type_segment` reduces `Promise<Project>` to
`Promise`. (2) should a bare identifier READ be a reference? Decides ts Const
2708 / rust Const 472. (3) `TypeHomes` is keyed by (package, name) with no
LANGUAGE, so one package holding two languages marks a shared type name
ambiguous; no effect on this corpus, where package and language are 1:1.

**Known broken** — none here. `indexer::lib_indexer::tests::
real_dbd_single_file_has_deploy_component` (`--ignored`) reads the hardcoded
path `/Users/Jerry/Developer/dbd-rs`, which is not checked out on this machine;
verified failing identically on the pre-change tree.
