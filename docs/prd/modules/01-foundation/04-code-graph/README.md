# Code Graph & Indexing · M4

**Plane:** foundation · **Status:** [roadmap/01-foundation.md](../../../../roadmap/01-foundation.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)), team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

The code graph is sensei's structural memory of every watched repo — `sensei.nodes`
(files, symbols, doc sections), `sensei.edges` (calls / imports / covers / extends /
…), and `inference.communities` (clusters). The Code map (M17), traceability,
semantic search, god-node detection, and impact analysis all read it. This module makes indexing
**convergent**: for a given working tree, N runs — full or incremental, in any order,
across daemon restarts — produce a byte-stable graph. A changed file re-indexes only
its own slice; a removed symbol disappears; nothing duplicates. That convergence — not
any single feature — is the whole requirement, because it is what stopped the
"scattered-circles" regression from silently returning.

> **Context from 2026-08-06 blueprint** — The live sensei graph on 2026-08-06 showed:
> - 1,727 of 5,394 resolved calls (32%) pointed at an *ambiguous* name (bare-name resolution)
> - `new` mega-hub: 1,230 inbound edges collapsed onto one arbitrary node (now distributed across 308 FQN nodes, worst 105)
> - External deps were NOT first-class — now 677 `lib_symbol` nodes (sqlx/axum/serde_json/…)
> - File→type→method structure was missing — now 1,014 methods under structs
> - `is_exported` was false for ALL 6,547 symbols; `call_flow` filtered on it
> - git-subtree dirs (marketplace/homebrew) had no nested `.git` and were missed
>
> **Fix 1 (shipped on `develop`)** — FQN symbol-table rebuild: get-or-create nodes by FQN at emit time; no `resolve_edges` pass; external deps → `lib_symbol` nodes (677); file→type→method nests.

## In scope / out of scope

- In: the scanner → graph-store write path (M3 *queues* folders; this module *parses,
  resolves, and writes*); **FQN symbol-table model** (Fix 1 shipped: get-or-create by FQN, edges node→node at emit, `lib` namespace for externals); idempotent edge identity (D1); derived-edge replacement (D2); node-identity preservation on reindex (D3); community durability/determinism (D4); the grouping + granularity kinds (D5); worker execution guarantees — granular, resumable, resilient, incremental (D6); the retrieval contract (`GET /api/graph/nodes`, `communities/info`, the `/tree` hierarchy endpoint); **`is_exported` per adapter** (`pub`/`export`/`__all__`); **git-subtree detection** via `git log --grep='git-subtree-dir:'`; **DB views** (`graph_edges_resolved`, `community_edges`, `folder_tree`, `node_exports`); **per-language name resolution** (imports/scope/enclosing-type/binding-type); **a role per file** (code · test · docs · config · generated); **Fix 2 (community-edge aggregation)** as a prerequisite of the Code map's Wheel and Communities grouping; **co-change mining** from git history for Hidden coupling; **branch scope** — which branches are synced and which graph each one carries.
- Out: rendering the graph (the Code map, Delivery M17 — Fix 4's structural view lives there); capture / watchers / branch flips (M3 — they enqueue the work); semantic search UX; the dōjō side of graph sync (`spec/dojo/daemon-sync.md`). **Fix 2** and **Fix 3 (DB views)** are spec'd in blueprint but not yet built.

## Surfaces

- **API** — `GET /api/graph/nodes?repoId=`, `GET /api/graph/communities/info`,
  `GET /api/graph/:repoId/tree`, the graph/imports breakdown endpoint.
- **Daemon** — `process_file` / `resolve_edge` / `build_connections` workers; task queue
  (`crates/senseid/src/tasks/…`); folder-status machine (`indexed`, `archived`, …); **per-language resolvers** plug into shared FQN core (`LanguageAdapter::resolve_fqn`); **SQL adapter consults dbd `SchemaModel`** for bare-name resolution (since dbd v0.22).
- **App** — code-graph + communities views in the Workspace window (layout edges
  `calls,imports,extends,implements`; containment `covers,references,rationale_for` overlay-only).

## The FQN Model (Fix 1 — shipped)

| Aspect | Design Decision |
|--------|-----------------|
| **Identity** | SCIP/LSIF moniker: every symbol has a stable FQN; `upsert_node_by_fqn(fqn) → node_id` get-or-create |
| **Definition** | crate/module path + enclosing `impl`/type + name → `crate::widget::Widget::new` |
| **Reference** | Resolve call path against file's scope: explicit `Foo::new()`, `self.method()`, `x.method()`, external `lib` FQN |
| **External** | `lib_symbol` nodes grouped by crate/pkg (`serde::de::from_str` etc.) |
| **Layering** | Shared FQN core + per-language resolvers (Rust first); cross-language refs merge on same FQN |
| **No `resolve_edges`** | Edges are `source_id → target_id` from emit; no limbo, no ambiguity guard |

**Name-resolution rules (Rust):** explicit path via `use` imports; `self`/`super`/same-`impl` → enclosing type; local `foo()` → module scope; `x.method()` on typed local → binding→type map; dynamic `dyn Trait` → trait method FQN (not nothing).

## File roles

Every file node carries one **role**: **code · test · docs · config · generated**. The
role comes from path conventions and the indexer's file kind, decided once at index
time and stored on the node. The Code map's L0 role bar, its L1 columns and its Show
filter read it; nothing downstream re-derives a role from a path.

| Role | Source |
|---|---|
| test | path convention — `languages::is_test_path`, stored as `nodes.is_test` (**built**) |
| docs | the indexer's file kind — design docs emit `doc` / `section` nodes (**built** as kinds, not as a role) |
| config | path convention (manifests, tool configs, env schemas) — **not built** |
| generated | path convention (`__generated__/`, generated clients, protobuf output) — **not built** |
| code | every other source file |

## Fix 2 · community-edge aggregation

Edges between two communities aggregate into one weighted relation
(`community_edges`, exposed on `communities/info`). It is a **prerequisite** for the
Code map's **Wheel** lens and its **Communities** grouping, both of which band
communities, not symbols. Not built.

## Co-change mining

**Hidden coupling** needs pairs of files that change in the same commits. The facts are
built: `ScanGitHistory` writes `sensei.commits` and `sensei.commit_files`
(repository-grain, insert-only). The mining is not: group `commit_files` by commit,
take the paths pairwise, drop any commit touching more than 50 files (a sweep, not
coupling), keep only paths that exist in `sensei.files`, and roll the pairs up to
modules. A pair with a static edge between the two modules is not hidden coupling.

## Branch scope

- Code graphs are **tagged by branch**. For each branch, the latest push wins.
- sensei syncs only the branches mapped to its three **environments** — **production ·
  testing · development** (D18). Each repository maps them to whatever branch it uses
  (`main`/`master`, `test`/`qa`, `develop`/`dev`). Production defaults to the forge's
  **default branch**; an unmapped environment **falls back to production**.
- Sprint targets read **development**; release gates run in order **development → testing
  → production** — a break in testing predicts one in production. The Code map shows
  development by default and production for a release.
- **Today:** `folders.branch` records the checked-out branch per checkout folder, as a
  metrics-only seam. The code graph is **not** branch-aware yet.

## Data invariants

1. **Edge uniqueness.** No two `edges` rows share `(folder_id, source_id, target_id, kind)`
   where `target_id IS NOT NULL`; nor `(folder_id, source_id, target_name, target_file, kind)`
   where `target_id IS NULL`. `count(*) / count(DISTINCT …) = 1.0` for every kind,
   `covers` included.
2. **Idempotent re-run.** Scanning an unchanged tree twice changes zero rows: identical
   `nodes.id` set, identical edge counts per kind, identical `community_id` per node.
3. **Scoped incremental reindex.** A change to one file mutates only that file's nodes and
   its outgoing edges; unchanged files keep `id`, `community_id`, `embedding`. A deleted
   symbol is pruned, and any *other* file's edge pointing at it is **unresolved**
   (`target_id→NULL, target_name kept`), not cascade-deleted.
4. **Derived edges are replaced, not appended.** `covers` (and any fully-derived kind) for a
   folder equals exactly what the current tree implies.
5. **Community durability + integrity.** Every `nodes.community_id` resolves to a live
   `inference.communities` row in the same folder; per folder, `sum(node_count)` equals the
   count of nodes carrying a `community_id` (±0); coverage ≈100% (singletons inherit their
   file/module community); values deterministic for an unchanged tree.
6. **Hierarchy + granularity present.** Monorepo members carry `folders.kind ∈
   {subtree, workspace_member}`; design docs decompose into `section` nodes; code carries
   `rationale` nodes; `parent_id` chains render docs-vs-code / module / class / method.
7. **Convergence.** `clear-graph + full-scan` reaches the identical steady state as an
   arbitrary sequence of incremental scans — including across a daemon restart mid-scan.
8. **One role per file.** Every file node has exactly one role; the role is stored,
   never re-derived by a reader.
9. **One graph per core branch.** A synced graph names its branch; a newer push for a
   branch replaces the older one; a branch outside the configured core set is never
   synced.

## Signals shown

| Element | Value | Meaning |
|---|---|---|
| Graph layout | nodes + edges (calls/imports/extends/implements) | structural map; community colour shows clusters |
| `community_id` per node | returned in the node projection | membership drives the cluster view |
| `/tree` hierarchy | folder → file → class → method → doc section | docs-vs-code and module nesting |
| Communities panel | live membership counts, per-community description/god-nodes | no more stale `node_count` sizing |
| `is_exported` | `pub`/`export`/`__all__` per adapter | exports populate everywhere |
| `git-subtree-dir` | `git log --grep='git-subtree-dir:'` prefixes | marketplace/homebrew scope dropdown shows them |

## Done gate

- **Idempotent re-run** — two scans of a fixture root: zero net rows, `dup = 1.00` for every
  `edges.kind`:
  ```sql
  SELECT kind, count(*), count(DISTINCT (source_id,target_id,target_name,target_file)) d,
         round(count(*)::numeric/greatest(count(DISTINCT (source_id,target_id,target_name,target_file)),1),2) dup
  FROM sensei.edges GROUP BY kind ORDER BY 1;            -- dup = 1.00 every row
  ```
- **Community coverage + per-folder integrity**:
  ```sql
  SELECT round(100.0*count(community_id)/count(*),1) coverage FROM sensei.nodes;   -- ~100
  SELECT count(*) FILTER (WHERE claimed <> real) mismatched_folders FROM ( … );     -- 0
  ```
- **No templated descriptions** — `props->>'source'` is `'narration-cache'` or honest-
  `'null'`, never a static-fallback marker; `description` never templated.
- **Scoped incremental** — touch one file: only that file's nodes change `modified_at`;
  others keep `id` + `community_id`; a deleted symbol disappears; no edge dupes.
- **Determinism** — two `clear + full-scan` runs produce identical `community_id` assignment.
- **Resilience** — 🟡 spec'd, **not yet built** ([D6](#open-gaps-from-blueprint--indexer-capability-coverage)
  gap): `fatal_file_failure_is_recorded_and_retried` and `restart_mid_scan_converges` are
  described in [`spec/pipeline/code-graph.md`](../../../../spec/pipeline/code-graph.md) §Resilience
  but exist in no crate today. Done means: after a simulated restart mid-scan,
  `activity.task_executions` where `status='running'` reconciles to 0 and the folder reaches
  `indexed`.
- **Retrieval** — `GET /api/graph/nodes?repoId=<fixture>` returns per-node `community_id` and
  `calls+imports+extends` edges; `/tree` returns a nested structure.
- **`is_exported` populated** — per-adapter visibility (`pub`/`export`/`__all__`); `call_flow` reads it.
- **Subtree folders registered** — `folders.kind='subtree'` for marketplace/homebrew.

## Wrong gate

- **Edge count climbs on a no-op re-scan** — the exact regression returning.
- **`resolve_edges` starts erroring** — `resolve_edge` not conflict-safe against
  `edges_unique_resolved`.
- **`community_id` NULL after any scan** — community detection not chained, or reindex still
  deletes+reinserts nodes.
- **`communities` claims more members than nodes carry** — integrity check run globally
  instead of per-folder (over- and under-claiming folders cancel out).
- **`community_id`s reshuffle between two identical scans** — non-deterministic labels.
- **Every repo >500 nodes renders as a flat same-colour bubble cloud** — retrieval still
  reads stale `node_count`/KIND instead of live membership.
- **`is_exported` all false** — `call_flow` filters on it and finds nothing.
- **Subtree folders missing from scope dropdown** — marketplace/homebrew not registered.

## Scenarios

- [`scenarios/code-graph-convergence.feature`](scenarios/code-graph-convergence.feature) —
  idempotency, scoped reindex, communities, worker guarantees.
- [`scenarios/code-graph-retrieval.feature`](scenarios/code-graph-retrieval.feature) — the
  graph API contract the UI consumes.

## Open gaps (from blueprint + indexer capability coverage)

| Gap | Description | Priority | Blocks |
|-----|-------------|----------|--------|
| [G-1](../../../../roadmap/06-gaps.md) | Graph/layered view unreadable at granular scale, no L0/L1/L2 level of detail | P1 (M17) | Code map levels |
| [G-2](../../../../roadmap/06-gaps.md) | No ignore mechanism (seeded from `.gitignore` + qlty `exclude_patterns`) | P2 | Graph/metric pollution |
| **D1–D6** | Code-graph pipeline fixes (idempotent edge, derived-edge replace, node identity, community durability, grouping kinds, worker guarantees) | P1 | All downstream |
| **D3** | Embedding survival across reindex (26% coverage; process_file deletes+reinserts with NULL embedding) | P1 | Semantic search, duplication persistence |
| **D3** | Deterministic, line-independent node IDs (stable across reindex) | P1 | Stable search, drift/pattern FKs |
| **D5b** | `section` node emission (0 rows today; blocks traceability) | P1 | Requirement-level traceability, `traces_to` |
| **Fix 2** | Community-edge aggregation (`community_edges` view, `communities/info` relations) | P2 | Code map Wheel + Communities grouping |
| **File roles** | config and generated roles are not tagged (test is `nodes.is_test`) | P2 | Code map role bar + L1 columns |
| **Co-change mining** | pairwise co-change from `commit_files` (≤ 50 files a commit), rolled up to modules | P2 | Code map Hidden coupling |
| **Branch scope** | graph not tagged by branch; no core-branch configuration or environment mapping | P1 | Graph sync, sprint vs release reads |
| **Fix 3** | DB views (`graph_edges_resolved`, `folder_tree`, `node_exports`) | P2 | API simplification |
| **Git-subtree detection** | `git log --grep='git-subtree-dir:'` prefixes (marketplace/homebrew) | P2 | Scope dropdown |
| **`is_exported` per adapter** | `pub`/`export`/`__all__`/`__tablename__`; TDD per adapter | P1 | Exports everywhere |
| **SQL bare-name resolution** | Consult dbd `SchemaModel` (v0.22+) for in-pass resolution | P1 | Cross-schema imports |
| **Verdict columns** | `resolved_via` (8 rungs), `unresolved_reason` (10 codes: fault/refusal) | P1 | Edge confidence for metrics |
| **Verdict columns** | 240,439 of 244,219 `imports` misses carry no reason | P1 | Diagnosability |
| **Embedding coverage** | 26.4% (skewed: `module` 0%, `const` 3.5%, `type` 3.6% vs `method` 95%) | P1 | Semantic search recall |
| **FTS** | No tsvector; lexical arm is `ILIKE '%q%'` over `name`+`signature` only | P2 | Search > grep |

## Related

- [spec/pipeline/code-graph](../../../../spec/pipeline/code-graph.md) — the six fixes D1–D6,
  target + worker invariants, depth ledger (legacy depth, folds here).
- [analysis/2026-08-04-deep-dive/10-graph-indexing-regression](../../../../history/analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md) — the measured regression.
- [history/blueprints/2026-08-06-code-graph-meaningful.md](../../../../history/blueprints/2026-08-06-code-graph-meaningful.md) — Fix 1/2/3/4 blueprint.
- [history/analysis/2026-08-05-indexer-capability-coverage.md](../../../../history/analysis/2026-08-05-indexer-capability-coverage.md) — capability audit (patterns/search/traceability).
- [history/analysis/2026-09-29-architecture-metrics-and-visuals.md](../../../../history/analysis/2026-09-29-architecture-metrics-and-visuals.md) — architecture metrics unlocked by graph fixes.
- [history/analysis/2026-09-27-entity-graph-feasibility.md](../../../../history/analysis/2026-09-27-entity-graph-feasibility.md) — schema entities in the graph.
- [spec/indexer/20-schema-entities](../../../../spec/indexer/20-schema-entities.md) — node/edge/kinds detail.
- [17-code-map](../../03-delivery/17-code-map/README.md) — the view over this graph;
  consumes file roles, Fix 2 and co-change.
- [spec/dojo/daemon-sync](../../../../spec/dojo/daemon-sync.md) — the sync direction
  table, including branch scope.
- [prd/02-objectives.md](../../../02-objectives.md) §04 The workspace window ·
  [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/{indexer,graph}`; fixtures: `crates/senseid/tests/fixtures/graph-scan/`.