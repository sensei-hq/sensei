# FQN symbol-table rebuild — implementation plan

> **For agentic workers:** every task is TDD (failing test → confirm fail →
> implement → confirm green → `zero-errors-policy` → commit). Forward-only: no
> phase depends on a later one. Never mark done on a masked/piped exit code.

**Goal.** Replace bare-name call matching with a **get-or-create-by-FQN symbol
table** (SCIP/LSIF moniker model): every definition AND reference get-or-creates
its FQN node; edges are `source_id → target_id` at emit; a reference makes a stub
that the definition later enriches; external targets become first-class `lib`
nodes. Outcome: **no ambiguity, no "unresolved"** (except true `dyn` dispatch),
correct edges, first-class dependencies, and (via enclosing-type FQNs) the
crate/module/impl structure the Atlas needs.

**Design (source of truth):** [`docs/blueprints/2026-08-06-code-graph-meaningful.md`](../blueprints/2026-08-06-code-graph-meaningful.md).
Do not re-derive it here; this plan sequences it into TDD tasks.

**Tech stack.** Rust (`crates/senseid`); Postgres DDL via `dbd` (full-file, no
ALTERs); `cargo test -p senseid` + `cargo clippy --all-targets`. Parser work is
in `crates/senseid/src/languages/*` + `tasks/processors/*`.

**Conventions.** Per-task commit to `develop`; explicit `git add <paths>`; leave a
clean tree; every acceptance check asserts the specific graph state (a
count/identity), never a proxy or piped exit code. Never fabricate on a failure
path.

**Layering (decided).** Shared FQN core (format, get-or-create, `lib` namespace,
node→node edges, storage) is language-agnostic; per-language `LanguageAdapter`s
own only the name resolution that produces FQNs. Rust is the reference language.

**Transition strategy (no big-bang).** The new FQN emit path runs per language.
`resolve_edges` (+ the interim ambiguity guard `2c520f2d`) stays as the fallback
ONLY for languages not yet migrated, so the graph is never in a broken dual
state. `resolve_edges` is retired (Phase 7) once every active adapter emits FQNs.

**Sequencing (forward-only, verified):**
`0 decisions` → `1 FQN core (store+DDL)` → `2 Rust FQN producer (pure)` →
`3 wire Rust emit + node→node edges` → `4 lib nodes` → `5 enclosing-type
structure (D5c)` → `6 per-language rollout` → `7 retire resolve_edges +
retrieval/UI + migration`.

---

## Phase 0 — Decisions (no code)

Close every ambiguity before a dependent phase. Output: a short decisions block
appended to the blueprint.

- [ ] **0.1 FQN encoding.** A canonical moniker string, deterministic + stable.
  Proposed: `lang·package·module-path·Type·member` (e.g.
  `rust·senseid·crate::adapters::manifest·ManifestAdapter·parse`), encoded to one
  string with a fixed separator that can't collide with identifier chars. Decide
  the exact grammar + escaping; record it.
- [ ] **0.2 FQN uniqueness scope.** For a reference to merge with its definition,
  `fqn` must be unique across the **project** (so a call in crate A to `B::thing`
  merges). Decide the get-or-create key: `(project_id, fqn)` for a project-scoped
  symbol table (folder rows still carry `folder_id`; the symbol identity is
  project+fqn). Record how this coexists with the current
  `nodes_unique_identity` (line-based) — the fqn index is ADDED; the line key is
  retained for now (a node has both a file location and an fqn).
- [ ] **0.3 Stub vs enriched.** A reference-first node carries `{project_id, fqn,
  kind, name}` and NULL details; a definition fills `signature/line/is_exported/
  file_path/parent_id/…`. Decide the "is this a stub?" signal (e.g.
  `line_start IS NULL` or an explicit `resolved boolean`). Enrichment must be
  idempotent under D3 upsert-then-prune.
- [ ] **0.4 `lib` namespace.** External FQN scheme (`lib·serde·de·from_str`),
  node kind (`lib_symbol`), and how a lib node is grouped for the UI
  (package/crate). Lib nodes have no file location (stub-like, permanent).
- [ ] **0.5 Prune semantics.** With fqn identity, a definition removed from a file
  must un-enrich its node back to a stub (it may still be referenced) rather than
  delete it — extend D3 prune. Decide: a def-gone node with inbound refs → demote
  to stub; with no inbound refs → delete. Record.
- [ ] **0.6 Migration.** The fqn column + index is additive; a full reindex
  repopulates. Confirm the deploy folds into the existing graph-clear gate
  (docs/backlog.md).

**Acceptance:** decisions block filled; grammar + scope + stub-signal + lib-scheme
+ prune rule recorded.

---

## Phase 1 — Shared FQN core (DDL + store)

**Precondition:** `nodes` has no `fqn`; identity is line-based
(`nodes_unique_identity`); `insert_edge` takes `target_name`.

- [ ] **1.1** DDL: add `nodes.fqn text` + a partial unique index
  `nodes_unique_fqn` on `(project_id, fqn)` where `fqn IS NOT NULL` (full-file;
  `dbd` apply to test DB). Add `project_id` to `nodes` if not present, or key the
  fqn index on the repo-root folder scope per 0.2.
- [ ] **1.2 Failing test** `upsert_node_by_fqn_merges_ref_and_def`: a reference
  get-or-creates a stub by fqn; a later definition with the same fqn returns the
  SAME id and fills details (stub→enriched). Two refs to the same fqn share one
  node. FAIL → implement `PgStore::upsert_node_by_fqn(scope, fqn, kind, details)`
  get-or-create (0.3 stub signal). PASS.
- [ ] **1.3 Failing test** `lib_node_by_fqn`: an external fqn get-or-creates a
  `lib_symbol` node in the lib namespace, no file location, stable id across
  refs. FAIL → implement. PASS.
- [ ] **1.4** `cargo test -p senseid` + `cargo clippy --all-targets` green.
  Commit `feat(senseid): FQN node identity + upsert_node_by_fqn get-or-create`.

**Acceptance:** a ref and a def with the same fqn resolve to one node; a stub is
enriched in place; lib fqns get lib nodes.

---

## Phase 2 — Rust FQN producer (pure name resolution)

**Precondition:** `rust_lang.rs` emits bare `callee_name` (l.425) + defs with no
qualified path. This is the engine — the largest chunk.

- [ ] **2.1 Failing test** `rust_def_fqn`: a `fn` / `impl fn` / `struct` yields a
  canonical fqn (module path + enclosing `impl` type + name) per 0.1. FAIL →
  implement definition-FQN from AST context. PASS.
- [ ] **2.2 Failing test** `rust_ref_fqn_explicit_path`: `use crate::widget::
  Widget; Widget::new()` → ref fqn `…::widget::Widget::new` (use-map expansion).
  FAIL → implement a per-file use/import map + path canonicalisation. PASS.
- [ ] **2.3 Failing test** `rust_ref_fqn_self_and_local`: `self.method()` → the
  enclosing `impl` type; a local free `foo()` → module scope; `let x = Foo::new();
  x.method()` → `Foo::method` (light per-function binding→type map). FAIL →
  implement enclosing-type + intra-function binding tracking. PASS.
- [ ] **2.4 Failing test** `rust_ref_fqn_external_is_lib`: a path resolving to an
  imported external crate → a `lib·<crate>·…` fqn (marked external). FAIL →
  classify internal (in-project modules) vs external. PASS.
- [ ] **2.5 Failing test** `rust_adapter_methods_do_not_collapse`: two structs
  each with `fn parse` + a call to each → two DISTINCT ref fqns (the adapter case
  that motivated this). PASS.
- [ ] **2.6** Green. Commit `feat(senseid): Rust FQN producer — def + ref name
  resolution`.

**Acceptance:** each def and ref carries a canonical fqn; adapter methods
disambiguate; `x.method()` on a locally-typed binding resolves; external paths →
lib fqns; only `dyn`-typed receivers stay unqualified.

---

## Phase 3 — Wire Rust emit + node→node edges

**Precondition:** Phases 1–2. `process_file` upserts symbols (flat) + `insert_edge`
with `target_name`; `resolve_edges` matches later.

- [ ] **3.1 Failing test** `process_file_rust_emits_fqn_nodes_and_resolved_edges`:
  a Rust file with `compute()` calling `helper()` and `Foo::new()` → nodes carry
  fqn; the call edge is `target_id`-resolved AT EMIT (no `resolve_edges` run);
  the `Foo::new` target is a get-or-created node. FAIL → route the Rust path
  through `upsert_node_by_fqn` for defs + ref targets, emit `source_id→target_id`
  edges directly. PASS.
- [ ] **3.2 Failing test** `rust_call_before_def_creates_stub_then_enriched`:
  process the caller file first (target stub created), then the callee's file
  (stub enriched, same id, edge still resolved). FAIL → confirm order-independence
  via the get-or-create. PASS.
- [ ] **3.3** Keep `resolve_edges` as the FALLBACK for non-Rust (bare-name +
  guard); Rust edges skip it (already resolved). Test both coexist.
- [ ] **3.4** Green. Commit `feat(senseid): Rust process_file emits FQN nodes +
  resolved edges (no post-hoc resolution)`.

**Acceptance:** on a Rust file, calls resolve to the correct FQN target at emit;
order-independent; non-Rust still uses the fallback; no false hubs.

---

## Phase 4 — `lib` nodes + dependency grouping

**Precondition:** Phase 2 emits external fqns.

- [ ] **4.1 Failing test** `external_calls_link_to_lib_nodes`: a Rust file calling
  `serde_json::from_str` → an edge to a `lib_symbol` node grouped under `serde_json`;
  the dependency is queryable. FAIL → persist lib nodes + a library grouping.
  PASS.
- [ ] **4.2** Green. Commit `feat(senseid): first-class lib symbol nodes for
  external references`.

**Acceptance:** no external call is dropped; dependencies are graph-visible.

---

## Phase 5 — Enclosing-type structure nodes (D5c)

**Precondition:** fqn encodes the enclosing `impl`/type/module (Phase 2).

- [ ] **5.1 Failing test** `rust_impl_type_container_nesting`: an `impl Foo`'s
  methods are `parent_id`-nested under a `Foo` type/impl node (not flat under the
  file); the module container nests under the file. FAIL → emit type/module
  container nodes + parent_id. PASS.
- [ ] **5.2** Green. Commit `feat(senseid): D5c enclosing-type/module structure
  nodes`.

**Acceptance:** the graph tree has file → type/impl → method + module containers
(the structural clustering the UI needs).

---

## Phase 6 — Per-language rollout

**Precondition:** the Rust reference (Phases 2–5) is the template.

- [ ] **6.1** TypeScript/JS FQN producer (imports/exports/class scope) → FQN emit
  + node→node edges + lib nodes. TDD mirroring Phase 2/3 acceptance.
- [ ] **6.2** Python FQN producer (imports/class scope). TDD.
- [ ] **6.3** (Other adapters as present.) Each commits independently.

**Acceptance:** each migrated language resolves calls at emit, no bare-name
fallback for it.

---

## Phase 7 — Retire `resolve_edges` + retrieval/UI + migration

**Precondition:** all active adapters emit FQNs (Phase 6).

- [ ] **7.1** Remove `resolve_edges` + the interim guard `2c520f2d`; `target_name`
  becomes vestigial (kept only for truly-`dyn` residuals). Test the pipeline with
  no resolution pass.
- [ ] **7.2** Retrieval: `graph/nodes` + `/tree` expose fqn + the type/module
  containers; add the community-edge aggregation view (blueprint Fix 2/3).
- [ ] **7.3** Migration + live verify: reindex (folds into the graph-clear deploy
  gate); assert on the live sensei graph — the `new`/`parse`/`load` hubs are gone
  (each fqn has its true inbound set), external deps appear as lib nodes, coverage
  of resolved calls ≈ 100% minus true `dyn`, and the Atlas renders nested
  structure. **HALT-ON-FAILURE** on the irreversible reindex.
- [ ] **7.4** Flip the blueprint's resolution section to shipped. Commit.

**Acceptance (the whole point):** the live graph has no false mega-hubs; every
call resolves to a correct FQN node (internal or lib); the residual unresolved
set is only genuine dynamic dispatch; the Atlas shows repo→module→type→member
structure.

---

## Final verification (whole plan)
- [ ] `cargo test -p senseid` green (every canonical test above); `cargo clippy
  --all-targets` clean; `make test-fast` green.
- [ ] Live: the top `new`/`parse`/`load`/`GET`/`POST` nodes each have a plausible
  (small) inbound set, not one 482-edge hub; `SELECT count(*) FROM edges WHERE
  kind='calls' AND target_id IS NULL` is tiny (dynamic-dispatch residual only).
- [ ] Depth ledger: 0 open questions; Phase-0 decisions recorded.

## Self-review (author)
- Phase 0 closes every ambiguity (fqn grammar, scope, stub signal, lib scheme,
  prune) before dependent phases.
- Forward-only: the store core (1) precedes the producer (2), which precedes the
  wiring (3); lib (4) and structure (5) build on the fqn (2); per-language (6)
  follows the Rust reference; `resolve_edges` retires last (7) so the graph is
  never in a broken dual state.
- The transition keeps the old fallback per-language so no phase ships a
  regressed graph; the irreversible reindex has an explicit halt-on-failure.
