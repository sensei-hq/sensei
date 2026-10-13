# Code Graph & Indexing · M4

**Plane:** foundation · **Status:** [roadmap/01-foundation.md](../../../../roadmap/01-foundation.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)), team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

The code graph is sensei's structural memory of every watched repo — `sensei.nodes`
(files, symbols, doc sections), `sensei.edges` (calls / imports / covers / extends /
…), and `inference.communities` (clusters). Atlas traceability, semantic search,
god-node detection, and impact analysis all read it. This module makes indexing
**convergent**: for a given working tree, N runs — full or incremental, in any order,
across daemon restarts — produce a byte-stable graph. A changed file re-indexes only
its own slice; a removed symbol disappears; nothing duplicates. That convergence — not
any single feature — is the whole requirement, because it is what stopped the
"scattered-circles" regression from silently returning.

## In scope / out of scope

- In: the scanner → graph-store write path (M3 *queues* folders; this module *parses,
  resolves, and writes*); idempotent edge identity (D1); derived-edge replacement (D2);
  node-identity preservation on reindex (D3); community durability/determinism (D4);
  the grouping + granularity kinds (D5); worker execution guarantees — granular, resumable,
  resilient, incremental (D6); the retrieval contract (`GET /api/graph/nodes`,
  `communities/info`, the new `/tree` hierarchy endpoint).
- Out: the Atlas Svelte rendering for the retrieval contract (downstream follow-on);
  capture / watchers / branch flips (M3 — they enqueue the work); semantic search UX.

## Surfaces

- **API** — `GET /api/graph/nodes?repoId=`, `GET /api/graph/communities/info`,
  `GET /api/graph/:repoId/tree` (new), the graph/imports breakdown endpoint.
- **Daemon** — `process_file` / `resolve_edge` / `build_connections` workers; task queue
  (`crates/senseid/src/tasks/…`); folder-status machine (`indexed`, `archived`, …).
- **App** — code-graph + communities views in the Project window (layout edges
  `calls,imports,extends,implements`; containment `covers,references,rationale_for` overlay-only).

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

## Signals shown

| Element | Value | Meaning |
|---|---|---|
| Graph layout | nodes + edges (calls/imports/extends/implements) | structural map; community colour shows clusters |
| `community_id` per node | returned in the node projection | membership drives the cluster view |
| `/tree` hierarchy | folder → file → class → method → doc section | docs-vs-code and module nesting |
| Communities panel | live membership counts, per-community description/god-nodes | no more stale `node_count` sizing |

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
- **Resilience** — `fatal_file_failure_is_recorded_and_retried` and
  `restart_mid_scan_converges` pass; after a simulated restart mid-scan,
  `activity.task_executions` where `status='running'` reconciles to 0 and the folder reaches
  `indexed`.
- **Retrieval** — `GET /api/graph/nodes?repoId=<fixture>` returns per-node `community_id` and
  `calls+imports+extends` edges; `/tree` returns a nested structure.

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

## Scenarios

- [`scenarios/code-graph-convergence.feature`](scenarios/code-graph-convergence.feature) —
  idempotency, scoped reindex, communities, worker guarantees.
- [`scenarios/code-graph-retrieval.feature`](scenarios/code-graph-retrieval.feature) — the
  graph API contract the UI consumes.

## Open gaps

- [G-1](../../../../roadmap/06-gaps.md) — graph/layered view unreadable at granular scale, no
  L0/L1/L2 level of detail (listed under M17; retrieval feeds it).
- Watch: worker state is in-memory today (100% queue.rs) and folder-status states collapse
  to `indexed`/`archived` — D6 makes these durable and complete; until landing, restart
  resume coverage is partial.

## Related

- [spec/pipeline/code-graph](../../../../spec/pipeline/code-graph.md) — the six fixes D1–D6,
  target + worker invariants, depth ledger (legacy depth, folds here).
- [analysis/2026-08-04-deep-dive/10-graph-indexing-regression](../../../../history/analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md) — the measured regression.
- [spec/indexer/20-schema-entities](../../../../spec/indexer/20-schema-entities.md) —
  node/edge/kinds detail.
- [prd/02-objectives.md](../../../02-objectives.md) §04 The project window ·
  [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/{indexer,graph}`; fixtures: `crates/senseid/tests/fixtures/graph-scan/`.