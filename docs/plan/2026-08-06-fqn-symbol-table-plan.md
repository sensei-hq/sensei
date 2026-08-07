# FQN symbol-table rebuild — implementation plan (v2)

> **For agentic workers:** every task is TDD (failing test → confirm fail →
> implement → confirm green → `zero-errors-policy` → commit). Forward-only: no
> phase depends on a later one. Never mark done on a masked/piped exit code.
>
> **v2 (2026-08-06):** revised after `sensei-plan-depth-reviewer` returned
> not-ready. All 10 must-fixes folded in; the load-bearing decision (identity
> scope) is now concrete: **folder-scoped `(folder_id, fqn)`**.

**Goal.** Replace bare-name call matching with a **get-or-create-by-FQN symbol
table** (SCIP/LSIF moniker model): every definition AND reference get-or-creates
its FQN node; edges are `source_id → target_id` at emit; a reference makes a stub
that the definition later enriches; external targets become first-class `lib`
nodes. Outcome: **no ambiguity, no "unresolved"** (except true `dyn` dispatch),
correct edges, first-class dependencies, and (via enclosing-type FQNs) the
crate/module/impl structure the Atlas needs.

**Design (source of truth):** [`docs/blueprints/2026-08-06-code-graph-meaningful.md`](../blueprints/2026-08-06-code-graph-meaningful.md).

**Tech stack.** Rust (`crates/senseid`); Postgres DDL via `dbd` (full-file, no
ALTERs); `cargo test -p senseid` + `cargo clippy --all-targets`. Parser work is
in `crates/senseid/src/languages/*` + `tasks/processors/*`.

**Conventions.** Per-task commit to `develop`; explicit `git add <paths>`; leave a
clean tree; every acceptance check asserts a specific graph state (a
count/identity), never a proxy or piped exit code. Never fabricate on failure.

**Layering (decided).** Shared FQN core (format, get-or-create, `lib` namespace,
node→node edges, storage) is language-agnostic; per-language `LanguageAdapter`s
own only the name resolution that produces FQNs. Rust is the reference language.

**Transition (no big-bang).** The new FQN emit path runs per language.
`resolve_edges` (+ the interim guard `2c520f2d`) stays as the fallback ONLY for
not-yet-migrated languages, **filtered to same-language candidates** (0.8) so a
mixed-language folder never cross-contaminates. `resolve_edges` is retired
(Phase 7) once every active adapter emits FQNs.

**Sequencing (forward-only):**
`0 decisions` → `1 FQN core (store+DDL)` → `2 Rust FQN producer (pure)` →
`3 wire Rust emit + node→node edges` → `4 lib nodes` → `5 enclosing-type
structure (D5c) + community-determinism guard` → `6 per-language rollout` →
`7 retire resolve_edges + retrieval/UI + migration`.

---

## Phase 0 — Decisions (no code; record all in the blueprint's decisions block)

- [ ] **0.1 FQN grammar (decided).** Separator is **`·` (U+00B7 middot)** — no
  Rust/TS/Python identifier can contain it. Forms:
  - free fn / const: `<lang>·<package>·<module-path>·<name>`
  - inherent method / assoc fn: `<lang>·<package>·<module-path>·<Type>·<member>`
  - **trait-impl method:** `<lang>·<package>·<module-path>·<Type>·<Trait>·<member>`
    — the **trait qualifier disambiguates** `Display::fmt` vs `Debug::fmt` on the
    same `Foo` (must-fix #6: otherwise the FQN re-collapses the exact bug we fix).
  - lib symbol: `lib·<package>·<path>·<member>`.
  `package` = crate name (Rust), npm/pnpm package (TS), top module (Python).
  `module-path` = the `::`/`.`-joined module chain within the package.
  **Anchoring rule (cross-file consistency — critical):** a **method**'s
  `<module-path>·<Type>` is the **type's canonical definition location**, resolved
  from the `impl`/class's `Self` type — NOT the file the `impl` block lives in. So
  `impl Widget` split across `widget.rs` and `widget_ext.rs` both yield
  `…·widget·Widget·…`, and a reference `Widget::m()` (with `use crate::widget::
  Widget`) computes the same fqn → they merge. A **free fn/const** uses its own
  file's module path (files ARE modules in Rust/TS/Python, so same-named free
  functions in different files get distinct module-paths). Languages with **no
  module system** (C): a file-static symbol includes the FILE segment (two files'
  `static foo` are distinct); an external-linkage global is repo-global.
- [ ] **0.2 Identity scope (decided): folder-scoped `(folder_id, fqn)`.** Nodes
  hang off the **repo-root folder** (the git/standalone folder that owns them; it
  carries `project_id`, child folders `parent_id`-ref it). So folder-scoped =
  repo-scoped: intra-repo cross-crate calls merge (all under the root folder),
  cross-*repo* calls → `lib`. **No `nodes.project_id`** (it doesn't exist and
  isn't needed). Everything stays folder-consistent with the existing
  `insert_edge` / prune / edges-unique-index model.
- [ ] **0.3 Stub vs enriched (decided).** Add `nodes.resolved boolean not null
  default false` and make `nodes.file_path` **nullable** (a reference-first stub
  has no known file). A reference get-or-creates `{folder_id, fqn, kind, name,
  resolved=false}`; a definition sets `resolved=true` + fills
  `file_path/signature/line_start/line_end/is_exported/parent_id`. Enrichment is
  idempotent under D3.
- [ ] **0.4 `lib` nodes (decided).** `kind='lib_symbol'`, owned by the **same
  repo-root `folder_id`** (folder-scoped like every node → cascade-consistent,
  no orphan), `fqn='lib·…'`, `resolved=true` (it IS the external symbol; there's
  nothing to enrich), `file_path=NULL`, grouped by package via `props.package` (+
  an optional `lib_package` container node in Phase 4). Deleting the folder
  cascades its lib nodes + their edges — correct, since they're that repo's view
  of its deps.
- [ ] **0.5 Prune / demote-to-stub (decided).** Because identity is
  folder-scoped, `prune_file_nodes`'s folder-scoped inbound-edge check is already
  correct (all edges are in this folder). Rule: a definition removed from a file →
  if the node still has inbound edges, **demote to stub** (`resolved=false`, clear
  `file_path/line_start/signature`, keep `fqn`); if no inbound edges, delete.
  Extend the D3 prune to demote-not-delete a still-referenced def.
- [ ] **0.6 Migration + deploy gate (decided).** `nodes_unique_fqn` is a NEW
  partial unique index → **same live-deploy risk class as the D1 edges indexes**
  (`docs/backlog.md:146`): a live DB must be graph-cleared + reindexed before it
  applies cleanly. Fold into the existing graph-clear gate; note explicitly.
- [ ] **0.7 Binding→type scope (decided — bounds must-fix #5).** The
  per-function binding→type map tracks ONLY these forms (everything else is out
  of scope for v1):
  - `let x = Type::new()` / `Type::assoc()` / `Type { .. }` → `x: Type`
  - `let x: Type = …` (annotation) → `x: Type`
  - typed params `fn f(x: Type)` → `x: Type`
  - `self` → the enclosing `impl` type
  OUT of scope v1 (a call on such a receiver resolves by same-scope unique name
  or else stays a stub keyed on the best-effort trait-method fqn — never a wrong
  merge): chained calls, reassignment, control-flow-dependent bindings,
  inferred-from-return-type. This is a bounded single-pass map, NOT type
  inference. (The blueprint's own lean was "accept unresolved" here; this plan
  narrows that to "resolve the four static forms, stub the rest" — recorded.)
- [ ] **0.8 Mixed-language fallback (decided — must-fix #7).** A folder can hold
  `.rs` + `.ts` + `.py`. The bare-name fallback (`resolve_edges`) must match
  **only within the same language** and only for **not-yet-migrated** languages.
  Add `nodes.language text` (set from the file extension at write time) and filter
  the fallback's candidate pool on it, so an FQN-resolved Rust node is never
  bare-name-matched by a co-resident un-migrated language (and vice versa).

**Acceptance:** decisions block filled — grammar (with trait qualifier +
separator), `(folder_id, fqn)` scope, `resolved`/nullable-`file_path` stub signal,
lib ownership, demote-to-stub prune, deploy-gate note, binding-scope list,
same-language fallback filter — all concrete, zero remaining forks.

---

## Phase 1 — Shared FQN core (DDL + store)

**Precondition (verified):** `nodes` has `folder_id` only (no `project_id`;
`nodes.ddl`), `file_path text NOT NULL`, identity `nodes_unique_identity`
(line-based, `nodes.ddl:32-33`); `insert_edge` folder-scoped (`pg_store.rs`).

- [ ] **1.1** DDL (full-file `nodes.ddl`): add `fqn text`, `resolved boolean not
  null default false`, `language text`; make `file_path` **nullable**; add partial
  unique index `nodes_unique_fqn` on `(folder_id, fqn)` where `fqn IS NOT NULL`.
  It coexists with `nodes_unique_identity` (disjoint row-sets: stubs have NULL
  file/line, defs have both keys). `dbd apply` to the test DB.
- [ ] **1.2 Failing test** `upsert_node_by_fqn_merges_ref_and_def`: a reference
  get-or-creates a stub (`resolved=false`, NULL file); a later definition with the
  same `(folder_id, fqn)` returns the SAME id, sets `resolved=true` + details; two
  refs to one fqn share one node. FAIL → implement `upsert_node_by_fqn(folder_id,
  fqn, kind, name, lang, Option<def-details>)`. PASS.
- [ ] **1.3 Failing test** `lib_node_by_fqn`: an external fqn get-or-creates a
  `lib_symbol` node (`resolved=true`, NULL file, `props.package`) under the repo
  folder; stable id across refs. FAIL → implement. PASS.
- [ ] **1.4** Green. Commit `feat(senseid): FQN node identity (folder-scoped) +
  upsert_node_by_fqn get-or-create + lib nodes`.

**Acceptance:** a ref and a def with the same fqn resolve to one node; stub→
enriched in place; lib fqns get lib nodes; both unique indexes coexist.

---

## Phase 2 — Rust FQN producer (pure name resolution — the engine)

**Precondition (verified):** `rust_lang.rs:429` emits bare `callee_name`; defs
have no qualified path; symbols flat (`process.rs:870-874` parents to file node).

- [ ] **2.1 Failing test** `rust_def_fqn`: `fn`/`impl fn`/`struct` → canonical fqn
  per 0.1 (module path + enclosing type + name); a **trait-impl** method gets the
  trait qualifier (`…·Foo·Display·fmt`). FAIL → implement definition-FQN from AST
  context (module chain + `impl`/trait). PASS.
- [ ] **2.2 Failing test** `rust_ref_fqn_explicit_path`: `use crate::widget::
  Widget; Widget::new()` → `rust·<crate>·widget·Widget·new` (per-file use-map
  expansion). FAIL → implement the use/import map + path canonicalisation. PASS.
- [ ] **2.3 Failing test** `rust_ref_fqn_self_local_bounded`: `self.method()` →
  enclosing `impl` type; a local free `foo()` → module scope; the **four 0.7
  binding forms** (`let x = Foo::new()`, `let x: Foo`, typed param, field) →
  `Foo::method`; an out-of-0.7 receiver → a stub (no wrong merge). FAIL →
  implement the bounded single-pass binding→type map (exactly 0.7's forms). PASS.
- [ ] **2.4 Failing test** `rust_ref_fqn_external_is_lib`: a path resolving to an
  imported external crate → `lib·<crate>·…`. FAIL → classify internal (in-crate
  modules) vs external. PASS.
- [ ] **2.5 Failing test** `rust_adapter_and_trait_methods_do_not_collapse`: two
  structs each with `fn parse` → two distinct fqns; `Display::fmt` + `Debug::fmt`
  on one struct → two distinct fqns (trait qualifier). PASS.
- [ ] **2.6 Failing test** `rust_dyn_receiver_stays_unqualified`: a `dyn Trait`
  receiver method call → the trait-method fqn (a node), never a wrong concrete
  merge. PASS.
- [ ] **2.6b Failing test** `rust_same_name_across_files_disambiguates` (the
  cross-file consistency check): `a.rs::fn parse` and `b.rs::fn parse` → distinct
  fqns (module-path differs); `impl Widget { fn m }` in `widget.rs` AND a second
  `impl Widget { fn n }` in `widget_ext.rs` BOTH anchor on `…·widget·Widget·…`
  (the type's home module, per the 0.1 anchoring rule), and a reference
  `Widget::m()` from a third file resolves to the SAME node (merge, not a stub).
  PASS.
- [ ] **2.7** Green. Commit `feat(senseid): Rust FQN producer — def + ref name
  resolution (bounded binding→type, trait-qualified)`.

**Acceptance:** each def/ref carries a canonical fqn; inherent AND trait methods
disambiguate; the four static binding forms resolve; external → lib; only the
`dyn`/out-of-0.7 tail stays stub (never a wrong merge).

---

## Phase 3 — Wire Rust emit + node→node edges

**Precondition:** Phases 1–2.

- [ ] **3.1 Failing test** `process_file_rust_emits_fqn_nodes_and_resolved_edges`:
  a Rust file (`compute()` calls `helper()` + `Foo::new()`) → nodes carry fqn +
  language='rust'; call edges are `target_id`-resolved AT EMIT (no `resolve_edges`
  run); the `Foo::new` target is a get-or-created node. FAIL → route the Rust path
  through `upsert_node_by_fqn` + emit `source→target` edges directly. PASS.
- [ ] **3.2 Failing test** `rust_call_before_def_creates_stub_then_enriched`:
  process the caller first (target stub), then the callee's file (stub enriched,
  same id, edge still resolved). PASS.
- [ ] **3.3 Failing test** `mixed_language_folder_fallback_is_language_scoped`: a
  folder with a `.rs` (FQN) node named `parse` and a `.py` (fallback) file calling
  `parse` → the Python call does NOT match the Rust node (0.8 same-language
  filter). FAIL → add the `language` filter to `resolve_edges`'s candidate pool.
  PASS.
- [ ] **3.4** Green. Commit `feat(senseid): Rust process_file emits FQN nodes +
  resolved edges; language-scoped bare-name fallback`.

**Acceptance:** Rust calls resolve to the correct FQN target at emit;
order-independent; the fallback can't cross languages; no false hubs.

---

## Phase 4 — `lib` nodes + dependency grouping

**Precondition:** Phase 2 emits external fqns; Phase 3 wires emit.

- [ ] **4.1 Failing test** `external_calls_link_to_lib_nodes`: a Rust file calling
  `serde_json::from_str` → an edge to a `lib_symbol` node grouped under
  `serde_json` (`props.package` + optional `lib_package` container); the dep is
  queryable per repo. FAIL → persist lib nodes + grouping. PASS.
- [ ] **4.2** Green. Commit `feat(senseid): first-class lib symbol nodes for
  external references`.

**Acceptance:** no external call dropped; dependencies are graph-visible per repo.

---

## Phase 5 — Enclosing-type structure (D5c) + community-determinism guard

**Precondition (verified):** `docs/backlog.md:147` requires, "when D5c lands," a
determinism fix. Symbols currently flat (`process.rs:870-874`).

- [ ] **5.1 Failing test** `rust_impl_type_container_nesting`: an `impl Foo`'s
  methods `parent_id`-nest under a `Foo` type/impl node; the module container
  nests under the file. FAIL → emit type/module container nodes + parent_id (a
  reparent updates the fqn-identified row's `parent_id`, not a new insert). PASS.
- [ ] **5.2 Failing test** `community_ids_deterministic_under_nesting`
  (must-fix #8, guards D4a `3cd37bad`): two nodes sharing `file_path/line_start/
  kind/name` under DIFFERENT `parent_id` → community detection stays deterministic
  across re-runs. FAIL → add `parent_id` then `id` (final total-order tiebreak) to
  BOTH `community::natural_key` AND `get_nodes_scoped`'s `ORDER BY`
  (per backlog.md:147). PASS.
- [ ] **5.3** Green. Commit `feat(senseid): D5c enclosing-type/module structure +
  community-determinism tiebreak`.

**Acceptance:** file → type/impl → method + module containers; community_id stays
deterministic with nested same-key siblings (no D4a regression).

---

## Phase 6 — Per-language rollout (one sub-task per adapter)

**Precondition (verified):** 11 adapters under `crates/senseid/src/languages/`:
`rust_lang, typescript, svelte, vue, java, kotlin, swift, python, c_lang, sql,
common`. Each carries an explicit grammar-mapping note + named tests mirroring
Phase 2/3 acceptance. Each commits independently; each un-migrated language keeps
the same-language fallback (0.8) until its task lands.

- [ ] **6.1 TypeScript/JS** — package = npm/pnpm package; module-path = file/module;
  `import`/`export` map; class/`this` scope. Tests `ts_def_fqn`,
  `ts_ref_fqn_import`, `ts_method_scope`, `ts_external_is_lib`.
- [ ] **6.2 Python** — package = top module; module-path = dotted import path;
  `import`/`from` map; class scope; `self` → class. Tests mirror 6.1.
- [ ] **6.3 Java / 6.4 Kotlin / 6.5 Swift** — package = declared package / module;
  fully-qualified class + method; imports map. One sub-task each, tests mirror 6.1.
- [ ] **6.6 C** (`c_lang`) — no modules: package = the repo; scope = file-static
  vs external-linkage globals (a file-static `helper` fqn includes the file; an
  `extern` global is repo-global). Decide the file-static vs extern split in the
  test. Tests `c_def_fqn_static_vs_extern`, `c_ref_fqn`.
- [ ] **6.7 SQL** — package = schema; object = `schema.name`; calls = function/proc
  references + table refs. Map `schema.object` → fqn. Tests `sql_def_fqn`,
  `sql_ref_fqn`.
- [ ] **6.8 Svelte / 6.9 Vue** — SFCs: the `<script>` block maps as TS (reuse 6.1's
  resolver); component name = the file. Tests `svelte_script_fqn`, `vue_script_fqn`.
- [ ] **6.10** Green across all. Commit per language.

**Acceptance:** each migrated language resolves calls at emit with a concrete
grammar mapping; no bare-name fallback remains for it.

---

## Phase 7 — Retire `resolve_edges` + retrieval + migration

**Precondition:** all active adapters emit FQNs (Phase 6).

- [ ] **7.1 Failing test** `pipeline_has_no_resolve_pass`: the scan produces
  correct node→node edges with NO `resolve_edges`/guard in the path;
  `target_name` is vestigial (only the `dyn` residual). FAIL → remove
  `resolve_edges` + the guard `2c520f2d`. PASS.
- [ ] **7.2 Failing test** `graph_nodes_and_tree_expose_fqn_and_containers`:
  `graph/nodes` returns `fqn`; `/tree` renders the type/module containers (Phase
  5). FAIL → widen the projections. PASS. **(The community-edge aggregation view +
  the broader DB-views work are a SEPARATE plan — blueprint Fix 2/3, build-sequence
  items 3–4; not folded here.)**
- [ ] **7.3 Migration + live verify (HALT-ON-FAILURE on the irreversible
  reindex).** Reindex (folds into the graph-clear deploy gate + `nodes_unique_fqn`,
  0.6). Assert on the live sensei graph: the `new`/`parse`/`load`/`GET`/`POST`
  hubs are gone (each fqn has its true inbound set), external deps appear as lib
  nodes, `SELECT count(*) FROM edges WHERE kind='calls' AND target_id IS NULL` is
  tiny (dyn residual only), the Atlas renders nested structure. If a check fails
  after the truncate, **halt and escalate** — do not re-run the truncate.
- [ ] **7.4** Flip the blueprint's resolution section to shipped. Commit.

**Acceptance:** no false mega-hubs; every call resolves to a correct FQN node
(internal or lib); residual unresolved = only genuine dynamic dispatch; the Atlas
shows repo→module→type→member structure.

---

## Final verification (whole plan)
- [ ] `cargo test -p senseid` green (every canonical test above); `cargo clippy
  --all-targets` clean; `make test-fast` green.
- [ ] Live: top `new`/`parse`/`load` nodes each have a plausible small inbound set,
  not one 482-edge hub; unresolved-calls count is the `dyn` residual only.
- [ ] Community_id deterministic under D5c nesting (5.2) — D4a not regressed.
- [ ] Depth ledger: 0 open questions; Phase-0 decisions recorded.

## Self-review (author, v2)
- Phase 0 now decides every fork concretely: grammar+trait qualifier+separator
  (0.1), folder-scoped identity (0.2), `resolved`/nullable-file stub (0.3), lib
  ownership (0.4), demote-to-stub prune (0.5), deploy gate (0.6), bounded
  binding→type (0.7), same-language fallback (0.8) — no TBD crosses a boundary.
- Forward-only preserved; the transition keeps a language-scoped fallback so no
  phase ships a regressed or cross-contaminated graph; the D4a-determinism fix is
  an explicit Phase-5 sub-task; Phase 6 decomposes all 9 remaining languages with
  grammar mappings; the community-edge/DB-views work is explicitly OUT of scope
  (separate plan); the irreversible reindex has HALT-ON-FAILURE.
