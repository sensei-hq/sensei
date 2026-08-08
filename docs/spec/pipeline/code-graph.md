# 網 · Pipeline · Code graph — idempotent indexing & retrieval

**Status:** draft (reviewed — `spec-doc-reviewer` PASS-WITH-CHANGES, revisions folded in) ·
**Segment:** cross-cutting (feeds Atlas + traceability + communities + semantic search)

**Owner files:**
- `crates/senseid/src/tasks/handlers/process.rs` — `ProcessFile` / `ProcessGitFolder`: node + per-file edge writes, the reindex delete path, `unresolve_edges_to_file`
- `crates/senseid/src/tasks/handlers/resolve.rs` — `ResolveEdges` / `BuildConnections`: folder-wide edge reconciliation (the `covers` cartesian product)
- `crates/senseid/src/tasks/handlers/scan_logic.rs` — `plan_reindex` (mtime/hash gate), sub-project + folder classification
- `crates/senseid/src/db/pg_store.rs` — `upsert_node` (~1429), `delete_nodes_by_file` (~1174), `insert_edge` (~1711), `resolve_edge` (~1742), `unresolve_edges_to_file` (~1754), `update_node_community` (~1702), `upsert_community` (~2601), `mark_folder_indexed` (~1377), `get_nodes_scoped` (~9607), `get_edges_scoped` (~9617)
- `crates/senseid/src/indexer/community.rs` — `detect_communities_for_folder`, `build_adjacency` (the `["calls","implements","imports"]` list ~l.85; singleton skip `members.len() < 2` ~l.42)
- `crates/senseid/src/indexer/doc_indexer.rs` — doc parsing; must emit `section`/`rationale` nodes (D5b)
- `crates/senseid/src/tasks/{queue.rs,executor.rs,resume.rs}` — the in-memory task engine (worker invariants, D6)
- `crates/senseid/src/api/handlers/codebase.rs` + `routes.rs` — the graph retrieval endpoints Atlas consumes
- DDL: `database/ddl/table/sensei/{nodes,edges,folders}.ddl`, `database/ddl/table/inference/communities.ddl`, `database/ddl/enum/sensei/{node_kind,edge_kind,folder_kind,folder_status}.ddl`, `database/ddl/table/sensei/scan_state.ddl`

**Task kinds involved:** `ScanRoot`, `ProcessGitFolder`, `ProcessFile`, `ProcessFolder`, `ResolveEdges`, `BuildConnections`, `EmbedNodes`, `DetectCommunities`

**Kanji:** 網 — *net / mesh* — the woven structure the graph is supposed to be.

---

## Purpose

The code graph is sensei's structural memory of every watched repo: `sensei.nodes`
(files, symbols, doc sections), `sensei.edges` (calls/imports/covers/…), and
`inference.communities` (clusters). Atlas, traceability, semantic search, and god-node
detection all read it. It **used to render a legible hierarchy** (docs vs code, packages,
modules, classes, methods, sub-projects) and now renders "scattered circles" — because the
indexing pipeline is **not idempotent under re-runs**. The same scan, run twice, does not
produce the same graph: `covers` edges duplicate ~918× (1.7 M rows for 1,869 real
relations), a changed file resets its nodes' `community_id` to NULL, and stale community
rows accumulate. Root cause:
[`analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md`](../../analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md).

This spec makes the pipeline **convergent**: for a given working tree, N runs — full or
incremental, in any order, across daemon restarts — produce a byte-stable graph (same node
ids, same edge count, same community assignment). A changed file re-indexes only its own
slice; a removed symbol disappears; nothing duplicates. Clearing the graph and re-scanning
must reach the *same* steady state as a long series of incremental scans. That convergence
is the whole requirement — it is what stops the regression from silently returning — and it
is only trustworthy if the **workers** that produce it are granular, resumable, resilient,
and incremental (§Worker execution invariants, D6).

**Scope.** The scanner → graph-store write path, the **worker-execution guarantees** that
make re-runs safe, and the **retrieval contract** the graph API must satisfy. The Atlas
Svelte rendering that consumes that contract is a downstream follow-on, named in *Related*.

---

## Data invariants

### Current (broken) behaviour — what the fix must replace

- **Edges have no identity.** `sensei.edges` has only `id uuid primary key` — **no unique
  constraint**. `insert_edge` (pg_store.rs:1711) is a bare `INSERT … RETURNING id`; every
  call makes a new row. `edges.target_file` is declared but **inert** — no call site ever
  passes it (`process.rs:794,808,816,825,830`, `resolve.rs:130,193` all send `NULL`); D1
  makes it load-bearing.
- **`covers` is re-derived every scan-with-changes and never cleared.** `build_connections`
  (resolve.rs:115) inserts a `covers` edge for every doc-stem == file-stem pair in the
  folder, on the barrier that fires whenever *any* file changed — so an unchanged doc's
  `covers` edges are re-inserted as fresh rows on every neighbouring change. (`reconcile_
  connections` is dead — only a unit test enqueues it.)
- **Changed files lose node identity.** `process_file` calls `unresolve_edges_to_file`
  (pg_store.rs:1754 — clears inbound edges' `target_id`, keeps `target_name`) **then**
  `delete_nodes_by_file` (pg_store.rs:1174) **before** `upsert_node` (pg_store.rs:1429), so
  the `ON CONFLICT ON CONSTRAINT nodes_unique_identity` path never fires for a reindex. New
  UUIDs ⇒ `community_id` reset to NULL, outgoing edges cascade-dropped, embeddings recomputed.
- **`resolve_edge` (pg_store.rs:1742) is a bare `UPDATE … SET target_id=…`** with no conflict
  handling, and `resolve_edges` can resolve two different unresolved edges from the same
  `(source_id, kind)` to the *same* `target_id` (the `imports` branch substring-matches a
  file, resolve.rs:51-54). Harmless today; **after D1 it will throw a unique violation** — D1
  must make it conflict-safe.
- **Communities are never restored by a scan and never pruned.** `DetectCommunities` is
  enqueued only by the analyzer's daily full-refresh (analyzer_scheduler.rs:233), not chained
  off a scan. `detect_communities_for_folder` upserts on `(folder_id, community_id)` with no
  delete-first, and label ids drift run-to-run ⇒ orphaned `inference.communities` rows.
  Result: `community_id` on **1.1 %** of nodes; the table claims 43,290 members, 5,443 real.
- **Adjacency depends on a dead edge kind.** `build_adjacency` reads
  `["calls","implements","imports"]`; `implements` is never emitted, and `const` (63 % of
  nodes) has none of these, so most nodes are singletons skipped at `members.len() < 2`.
- **Grouping + granularity kinds are empty.** 14 of 21 `node_kind` values are emitted; **7
  produce 0 rows** — exactly the levels that made the old graph legible:

  | Not-emitted kind | Represents | Consequence |
  |---|---|---|
  | `package` | container above `module` | no package grouping (user-reported) |
  | `section` | doc heading H1→H2→H3 (a **design feature / requirement**) | design docs stay whole-file `doc` nodes |
  | `rationale` | NOTE/WHY/HACK/TODO comment | design rationale absent from the graph |
  | `property`,`field`,`parameter`,`enum_variant` | sub-symbol members | finest code granularity absent |

  The `nodes.ddl` comment documents the intended hierarchy `file → section (H1→H2→H3)` and
  `function → rationale`. `folder_kind` defines `subtree`/`workspace_member` but scanning never
  writes either (verified 2026-08-06): `scan_root` classifies project roots `git`/`standalone`
  (scan.rs:99, authoritative each scan); nested **git subtrees** register as `git` via
  `upsert_repo` (process.rs:462-467); **monorepo members** register as `folder` + a partial
  `folders.role` via `upsert_subfolder` (process.rs:653-672). So the two granularity kinds exist
  in the enum but nothing emits them.

### Target invariants — what must be true after every run

1. **Edge uniqueness.** No two `edges` rows share `(folder_id, source_id, target_id, kind)`
   where `target_id IS NOT NULL`, nor `(folder_id, source_id, target_name, target_file, kind)`
   where `target_id IS NULL` (which requires call sites to populate `target_file`, D1).
   `count(*) / count(DISTINCT …) = 1.0` for every kind, `covers` included.
2. **Idempotent re-run.** Scanning an unchanged tree twice changes **zero** rows: identical
   `nodes.id` set, identical edge counts per kind, identical `nodes.community_id` per node,
   identical `communities` rows.
3. **Scoped incremental reindex.** A change to one file mutates only that file's nodes and
   its outgoing edges; unchanged files keep their `id`, `community_id`, `embedding`. A symbol
   deleted from the file is pruned, and any *other* file's edge that pointed at it is
   **unresolved (target_id→NULL, target_name kept), not cascade-deleted** (D3.3).
4. **Derived-set edges are replaced, not appended.** `covers` (and any fully-derived kind)
   for a folder equals exactly the set the current tree implies — stale relations removed.
5. **Community durability + integrity.** Every `nodes.community_id` resolves to a live
   `inference.communities` row for the same folder; **per folder**, `sum(node_count)` equals
   the count of nodes actually carrying a `community_id` (±0). Every non-isolated node is
   assigned; singletons inherit their file/module community (D4.4), so coverage is ~100 % of
   all nodes. `community_id` values are deterministic for an unchanged tree.
6. **Hierarchy + granularity present.** Sub-projects/monorepo members carry
   `folders.kind ∈ {subtree, workspace_member}`; design docs decompose into `section` nodes
   (`file → H1 → H2 → H3`, level in `props`); code carries `rationale` nodes; `package`
   containers sit above `module`/`file` (D5c). The folder tree + `nodes.parent_id`
   (91.5 % populated) render docs-vs-code / module / class / method nesting.
7. **Convergence.** `clear-graph + full-scan` reaches the identical steady state
   (invariants 1–6) as an arbitrary sequence of incremental scans, **including across a
   daemon restart mid-scan** (W2).

### Worker execution invariants — granular · resumable · resilient · incremental

The graph is only as trustworthy as the workers that build it. **Verified current reality**
(re-review, see Depth ledger): the task queue is **100 % in-memory** (queue.rs), lost on
restart; only `discovered`/`queued` git/subtree folders are re-enqueued on boot (resume.rs);
there is **no retry** (`retry_number` always 0); barriers **fail-open** (a failed dependency
releases dependents on partial data, queue.rs:262); the `folder_status` enum's 7 states are
represented by only `indexed`/`archived` in code; **no enqueue site guards against the same
folder being scanned twice concurrently** (the `has_pending_kind_path` primitive exists,
queue.rs:339, but scan.rs:128 / resume.rs:50 / version_rescan.rs:92 / workspace.rs don't use
it); and — critically — **`process_file`/`resolve_edges`/`build_connections` structurally
never return `Err`** (process.rs:680 documents it: "Returning Ok is critical: a failed
ProcessFile would block its folder's barrier"). That last fact reshapes D6 (below): the
resilience model is **failure-recording**, not `Err`-propagation. The graph pipeline must
satisfy:

- **W1 — Granular & bounded.** No task performs unbounded whole-folder work that a crash
  forces to redo from scratch. Per-file work is already a `ProcessFile` task; the coarse
  barriers (`build_connections`, `detect_communities`, `embed_nodes`) must be **either
  chunked with a resumable cursor or idempotent-replace**, so re-running one is cheap and
  correct (guaranteed by D1–D4).
- **W2 — Resumable.** The `folder_status` lifecycle is honoured
  (`discovered → queued → indexing → indexed | failed`); on boot, non-terminal folders are
  re-enqueued and orphaned `task_executions` rows (`status='running'` from a dead session)
  are reconciled to a terminal state. A crash mid-scan re-derives outstanding work from
  `scan_state` diff + folder status: **no completed file is redone, no in-flight file is
  lost** (invariant 7).
- **W3 — Resilient (fatal errors propagate and are reported; parse errors are tolerated).**
  A *fatal* step (a DB write / transaction, not a tolerated parse/lex error) **propagates as
  `Err` and is reported** — recorded to `task_executions.status='failed'`+`error_message` and
  surfaced on the logs/health screen — and it does **not** advance the file's `scan_state`, so
  the next scan retries it (bounded, D6c). A *tolerated* parse error keeps `Ok` and advances
  `scan_state`, so one malformed file never fails its whole folder. Correctness-critical
  barriers (`build_connections`, `detect_communities`) **fail-closed** — folder left `failed`,
  not `indexed`, on any recorded file failure or upstream-chain failure. Fail-open partial data
  is allowed only for non-authoritative enrichment (`communities.description`, D4.5). Errors are
  never silently swallowed (reverses today's `warn!`-and-continue on fatal paths).
- **W4 — Incremental.** `scan_state` per-file hash is the authority; unchanged files are
  skipped without a read; changed files reconcile scope-locally (D3); derived artifacts
  (edges, communities) reconcile incrementally (D1/D2/D4), never only-on-full-rebuild.
- **W5 — Single-writer per scope.** At most one in-flight task mutates a given folder's graph
  at a time, and at most one mutates a given file. Concurrent `ProcessGitFolder`/`ProcessFile`
  on the same path is prevented at enqueue (a `has_pending_kind_path` guard), because two
  concurrent `ProcessFile`s interleave `unresolve → delete → upsert` with no shared
  transaction, and two concurrent `BuildConnections` both DELETE-then-INSERT the same `covers`
  set. Without W5, D1–D4 idempotency is defeated by races even though each is individually
  correct.

---

## Design — the six fixes

Priorities: **P0** unblocks Atlas + stops duplication; **P1** restores communities +
hierarchy + worker robustness; **P2** is cleanup/backfill.

### D1 (P0) — Give edges an identity; make `insert_edge` and `resolve_edge` idempotent

**DDL** (`edges.ddl`, full-file rewrite per repo convention) — two partial unique indexes
(nullable `target_id` forces the split):

```sql
create unique index if not exists edges_unique_resolved
    on sensei.edges (folder_id, source_id, target_id, kind)
 where target_id is not null;

create unique index if not exists edges_unique_unresolved
    on sensei.edges (folder_id, source_id, target_name, target_file, kind) nulls not distinct
 where target_id is null;
```

**`insert_edge`** (pg_store.rs:1711) — branch on `target_id`, add a `target_file` param, let
the matching partial index absorb the conflict, always return the surviving id:

```sql
-- resolved
INSERT INTO sensei.edges(folder_id, source_id, target_id, kind, confidence)
VALUES($1,$2,$3,$4::sensei.edge_kind,$5)
ON CONFLICT (folder_id, source_id, target_id, kind) WHERE target_id IS NOT NULL
  DO UPDATE SET modified_at = now()
RETURNING id;
-- unresolved
INSERT INTO sensei.edges(folder_id, source_id, target_name, target_file, kind, confidence)
VALUES($1,$2,$3,$4,$5::sensei.edge_kind,$6)
ON CONFLICT (folder_id, source_id, target_name, target_file, kind) WHERE target_id IS NULL
  DO UPDATE SET modified_at = now()
RETURNING id;
```

Every unresolved call site (`process.rs` covers/imports/references, `resolve.rs`) must now
pass `target_file` (the folder-relative path of the intended target) so same-named symbols in
different files don't collide under the unresolved index. `DO UPDATE SET modified_at=now()`
(not `DO NOTHING`) so `RETURNING id` is always non-empty.

**`resolve_edge`** (pg_store.rs:1742) must become conflict-safe: promoting an unresolved edge
to `target_id=X` can collide with an existing resolved `(folder_id, source_id, X, kind)`.
Merge-then-delete-loser — `UPDATE … SET target_id=$2 WHERE id=$1 AND NOT EXISTS (SELECT 1
FROM sensei.edges d WHERE d.folder_id=… AND d.source_id=… AND d.target_id=$2 AND d.kind=…)`;
if `0` rows updated, `DELETE` the now-redundant unresolved row. Add `resolve_edge` tests.

### D2 (P0) — Own and *replace* derived edge sets instead of appending

Uniqueness stops duplicates but not **stale** edges. Assign each kind an owner that
reconciles its full set transactionally:

| Edge kind(s) | Owner | Reconciliation |
|---|---|---|
| `calls`,`imports`,`extends`,`references`,`rationale_for` (parsed from a source file) | `ProcessFile` for that file | after upserting the file's nodes, `DELETE FROM edges WHERE folder_id=$1 AND source_id = ANY($file_node_ids)` then re-insert current — scoped to that file's out-edges |
| `covers` (doc-stem × file-stem, folder-derived) | `BuildConnections` | `DELETE FROM edges WHERE folder_id=$1 AND kind='covers'` then insert the freshly computed set, one transaction |
| `traces_to`,`duplicates`,`similar_to` (inference-derived) | their inference task | same replace-per-scope pattern |

`covers` becomes a pure function of the current `(docs, files)` set — idempotent, and stale
coverage vanishes. Retire dead `reconcile_connections`. **Ordering:** `rationale_for` edges
depend on `rationale` nodes (0 rows until D5b) — its reconciliation is a no-op until D5b lands.

**Implementation surface (not optional):** "one transaction" has no path today —
`build_connections` (resolve.rs:81) calls autocommitting `insert_edge`/delete in a loop. Add a
single `replace_edges_of_kind(tx, folder_id, kind, edges)` on `PgStore` built on the existing
`pool.begin()` pattern (pg_store.rs:3811), doing `DELETE … WHERE folder_id AND kind` + batch
insert inside one tx, and call it from `build_connections` and each per-file reconcile. Without
this named method an implementer keeps the loop-of-autocommits and a crash between DELETE and
re-INSERT leaves the folder with **no** `covers` — so D6d must also treat "folder has 0 covers
but ≥1 doc+code stem match" as a failed, not indexed, state.

> **Semantic caveat (roadmap, not this spec).** `covers` is *file-stem proximity*
> (`docs/api/auth.md` ↔ `src/api/auth.ts`), a doc→file N×M match — not identifier-level
> traceability. D1/D2 fix its 918× *duplication*; they do **not** make it precise. Real
> requirement→symbol tracing needs `section` nodes (D5b) + a `traces_to` producer — see the
> [indexer capability roadmap](../../analysis/2026-08-05-indexer-capability-coverage.md).

### D3 (P0) — Stop destroying node identity on reindex

Replace `process_file`'s delete-then-insert (process.rs:754-759) with **upsert-then-prune**
keyed on node identity, so surviving symbols keep their `id` (and thus `community_id`,
`embedding`, inbound edges):

1. Parse → current node set for the file.
2. `upsert_node` each (`ON CONFLICT … DO UPDATE`) — must **not** clobber `community_id`,
   `embedding`, `degree` (today it updates only `signature`,`line_end`; keep it so).
3. For the symbols that **vanished** (present in DB for this file, absent from the parse):
   first run the `unresolve_edges_to_file`-equivalent scoped to *those node ids* (clear inbound
   edges' `target_id`, keep `target_name` — invariant 3), then
   `DELETE FROM sensei.nodes WHERE folder_id=$1 AND file_path=$2 AND id <> ALL($kept_ids)`.
4. Reconcile out-edges per D2.

**Embedding survival (concrete, not implied).** `nodes.ddl` has **no `content_hash` column**,
and `upsert_node`'s `ON CONFLICT DO UPDATE` (pg_store.rs:1441) refreshes only
`signature`/`line_end` — it never touches `embedding`. So a symbol whose *body* changed but
whose signature/line didn't would keep a **stale embedding forever**, and a symbol that moved
lines mints a new row with a NULL embedding (the 26.4 %-coverage root cause). Fix: add a
`content_hash text` column to `nodes.ddl`; in `upsert_node`, on conflict compare the incoming
hash and `SET embedding = NULL` (re-queue for embedding) only when it changed, otherwise
**preserve** the existing embedding. This is what makes "surviving symbol keeps its embedding"
(invariant 3) true rather than aspirational, and is the prerequisite the search/traceability
audit named for semantic recall.

**Identity evolution (P1, recommended, needs sign-off).** `nodes_unique_identity` includes
`line_start`, so a symbol that merely *moves* gets a new id and loses its `community_id`.
Change the key to `(folder_id, file_path, kind, name, parent_id, signature)` — `signature`
disambiguates overloads (why `line_start` was there) while staying stable across line moves.
DDL + `upsert_node`/prune change + a one-time reindex; **raise with the user before landing**
(changes node-identity semantics).

### D4 (P1) — Community durability, coverage, determinism, enrichment

1. **Chain `DetectCommunities` into the scan** — enqueue on the `ProcessGitFolder` barrier
   after `BuildConnections` for any folder whose node/edge set changed (an unchanged folder's
   re-detect is a no-op by construction — no timing-debounce needed). This is the **terminal
   barrier** that flips `folder_status` to `indexed` (D6a).
2. **Transactional replace per folder** — in one tx: `DELETE FROM inference.communities
   WHERE folder_id=$1`, clear `nodes.community_id` for the folder, recompute, upsert
   communities + `update_node_community`. Kills stale rows and orphans (invariant 5).
3. **Deterministic ids** — order nodes by natural key before propagation; remap raw labels to
   `community_id` by a stable rule (rank communities by the min natural-key of their members →
   `1..k`). Identical tree ⇒ identical ids (invariant 2).
4. **Broaden adjacency** (`build_adjacency`) — use `calls,imports,extends,references` **plus**
   `parent_id` containment; drop the dead `implements`. Assign singletons to their
   file/module community instead of skipping, so coverage → ~100 %.
5. **Enrich** — populate `communities.god_node_ids` (top-5 by `degree`) and `description` via
   [[pipeline/insight-copy]]; populate `nodes.degree` during `ResolveEdges`. On insight-copy
   failure, leave `description` **NULL (honest-empty), never a templated placeholder** — per
   the never-fabricate rule.

### D5 — Restore the grouping + granularity kinds (the 7 not-emitted kinds)

- **D5a (P1) — Sub-project boundary.** Detection already exists; the classification WRITE is
  wrong at **two** sites (verified 2026-08-06):
  - **Monorepo members** — `is_monorepo` → `find_subprojects` → `upsert_subfolder` (hardcoded
    `kind='folder'`) + `update_folder_role` (process.rs:653-672). Write `kind='workspace_member'`
    (a kind-aware subfolder upsert), keeping the role. `detect_workspace_members` is already
    computed at process.rs:210 (for the `pkg:` module id) — reuse it to mark the member dirs.
  - **Nested git subtrees** — `detect_git_subtrees_pub` → `upsert_repo` → `upsert_folder(git)`
    (process.rs:462-467), so a subtree gets `kind='git'`, not `subtree`. Write `kind='subtree'`
    (`upsert_repo_kind`'s `ON CONFLICT` already preserves a non-git/standalone kind, so a later
    `scan_root` pass that rediscovers the nested repo won't clobber it back to `git`).
  - **Reconcile-path caveat (must preserve):** `index_audit` ghost-prune (index_audit.rs:110/167),
    `dedup_structural_folder_nodes`, and `prune_vanished_folders` key on `kind='folder'`. Their
    kind allow/deny lists must be extended so a reclassified `workspace_member`/`subtree` is
    neither treated as a false ghost nor allowed to escape a genuine vanished-dir prune.
- **D5b (P1) — Doc decomposition (`section` + `rationale`).** The "granular design feature /
  requirement" level. `doc_indexer.rs` emits one `section` node per markdown heading, nested
  `file → H1 → H2 → H3` via `parent_id` with `props.level`, plus `rationale` nodes from
  NOTE/WHY/HACK/TODO/IMPORTANT comments — so a design/spec doc becomes a tree of
  features/requirements and `covers`/`traces_to` can point at a *requirement*, not a whole
  file. Routed through the D3 upsert/prune path (re-scanning a doc reconciles its section set,
  no duplicate headings).
- **D5c (P2) — Package + sub-symbol containers.** Emit `package`/`module` containers above
  `file` (Cargo/npm/Python packages) and `property`/`field`/`parameter`/`enum_variant`. Until
  D5c lands, Atlas nests on the folder tree + `nodes.parent_id`.

### D6 (P1) — Worker robustness (granular · resumable · resilient · incremental)

Satisfy W1–W5 reusing existing mechanisms — **not** a durable-queue rewrite (that is a larger
cross-cutting change tracked separately; the graph pipeline reaches resumability via
folder-status + `scan_state` convergence + the idempotency of D1–D4). Each item names the new
DB write or method so an implementer has a concrete surface, not a wish.

- **D6a — Folder-status lifecycle (new writers).** The enum already defines the states but
  code writes only `indexed` (pg_store.rs:1380) and `archived` (2136). Add writers:
  `queued` at enqueue of `ProcessGitFolder`; `indexing` at its start; `indexed` at the
  **terminal community barrier** (`DetectCommunities`, D4.1) success — so `indexed` implies
  communities are computed, and a mid-detect crash (atomic replace, D4.2) can't leave a
  half-indexed folder; `failed` when D6d trips. Add `update_folder_status(folder_id, status)` to
  `PgStore`. Enables precise resume (W2) and fail-closed barriers (W3).
- **D6b — Boot reconcile.** On startup, before workers spawn: (1) reconcile orphaned
  `task_executions` rows still `running` from a prior session to a terminal state — they can be
  detected by a per-session id (the daemon start time), since `task_id` resets per session
  (task_executions.ddl:56); (2) extend `resume_pending_scans` (resume.rs:18) to re-enqueue
  folders in `queued`/`indexing`/`failed`, not just `discovered`. Completed files skip via
  `scan_state` (W4), so resume is cheap.
- **D6c — Bounded retry with a retry identity.** *Reality:* `Task` (mod.rs:253) has no attempt
  field and `enqueue` mints a fresh `task_id` (queue.rs:71), so there is nothing to carry an
  attempt count. Add a `retry_number: u32` to `Task`;
  on a **recorded fatal failure** (see D6c-trigger) re-enqueue the same `(kind, path)` with
  `retry_number+1` up to N (default 3), writing that number to
  `task_executions.retry_number` (currently always 0). Backoff: the queue has **no
  delayed-task primitive**, so backoff is a spawned `sleep(backoff)`-then-`enqueue`, never a
  blocking wait that would pin a worker. Terminal `failed` after N.
- **D6c-trigger — Fatal errors propagate and are reported (approved 2026-08-05).** Introduce a
  typed distinction inside `process_file`/`resolve_edges`/`build_connections` between a
  *tolerated* failure (a parse/lex error → keep `Ok`, still advance `scan_state`) and a *fatal*
  failure (a DB-write/transaction error). A fatal failure now **propagates as `Err`** — reversing
  process.rs:680's old "always return `Ok`" rule — **and is reported**, never swallowed: written
  to `task_executions.status='failed'` + `error_message` keyed by `(folder_id, path)`, surfaced
  on the logs/health screen, and it does **not** advance the file's `scan_state`. The old
  barrier-blocking concern is handled by D6d (fail-closed folder status) + D6c (retry), not by
  hiding the error. This reverses documented behavior deliberately and with sign-off; a tolerated
  parse error must still be distinguished so a malformed file doesn't fail its whole folder.
- **D6d — Fail-closed barriers (whole upstream chain).** At `build_connections` (and the
  chained `detect_communities`), before `mark_folder_indexed`, check the folder has **no**
  recorded fatal file failure (D6c-trigger) **and** no upstream-chain task failed — not just
  "all `ProcessFile` completed", because `resolve_edges`/`build_connections` can themselves
  fail (a covers-replace tx abort) while `queue.rs:262` fail-open still releases the dependent.
  The concrete predicate is `folder_status != 'failed'` after D6a marks it. If failed, leave
  `failed` for D6b/D6c to re-drive; do not mark `indexed`.
- **D6e — Single-writer guard (implements W5).** Add a `has_pending_kind_path(ProcessGitFolder,
  folder_path)` / `has_pending_kind_path(ProcessFile, file_path)` guard at **every** enqueue
  site that lacks one: scan.rs:128 & 474, resume.rs:50, process.rs:427 (subtree spawn),
  workspace.rs:437/494/594 (user rescan), and **version_rescan.rs:92** (which, unlike
  reconcile_scheduler.rs:126, does not guard — so a version-bump boot races the reconcile
  tick). Idempotency (D1–D4) is the correctness backstop; the guard prevents the race that
  defeats it.
- **D6f — Task idempotency is the contract.** Duplicate `(kind, path)` tasks must be **safe**
  (guaranteed by D1–D4 + W5); the `has_pending_kind*` guards are an optimization on top, not
  the correctness mechanism.
- **D6g — Chunk the coarse tasks (P2).** `build_connections` per-doc chunk; `embed_nodes`
  batched with a cursor. `detect_communities` stays whole-folder but its idempotent replace
  (D4.2) makes a restart cheap.

---

## Signals produced & retrieval contract

**Tables written:** `sensei.nodes`, `sensei.edges`, `inference.communities`,
`sensei.folders.{kind,role,status}`, `nodes.{community_id,degree,embedding}`.

**Graph API — corrected against the real endpoints** (`routes.rs`, `codebase.rs`):

| Endpoint | Today | Required change |
|---|---|---|
| `GET /api/graph/nodes?repoId=` (codebase.rs:33) | returns nodes + edges but `get_edges_scoped(&ids,"calls")` fetches **only `calls`** (too sparse — misses imports/extends), and the node projection omits `community_id` | fetch `calls,imports,extends` (+`implements` when emitted) for layout; add `community_id` to the `get_nodes_scoped` projection (pg_store.rs:9607/9617 signature changes) |
| `GET /api/graph/communities/info` | flat, monochrome overview sized by the **stale/inflated** `communities.node_count`, coloured by KIND (no per-node membership) — the actual "scattered circles" source | drive from real per-node membership (`nodes.community_id`, now durable via D4); colour by community; size by live counts |
| `GET /api/graph/:repoId/tree` (**new**) | — | the hierarchy: folder tree (`folders.parent_id`, split by `kind`/`role` → docs vs code vs sub-project) → file → `nodes.parent_id` chain (class → method) → doc `section` tree |

Layout edges = `calls,imports,extends,implements`. Containment (`covers`,`references`,
`rationale_for`) is overlay-only. This is what turns "scattered circles" back into a nested map.

---

## Test plan

Every invariant has a test that fails if its fix regresses (TDD, per repo rules). Fixtures
live in `crates/senseid/tests/fixtures/graph-scan/` — **not** the live sensei repo — so tests
are hermetic and deterministic.

**Unit (per fix).**
- D1: `insert_edge` upserts (second identical call returns same id, no new row); resolved &
  unresolved indexes both enforced; `resolve_edge` merges into an existing resolved edge
  instead of throwing.
- D2: renaming/deleting a covered file removes the **stale** `covers` row (shrink case — the
  "replaced, not appended" guarantee, which nothing exercises today).
- D3: upsert-then-prune keeps a surviving symbol's `id` + `community_id`; a vanished symbol is
  pruned and an inbound cross-file edge to it is **unresolved, not deleted**.
- D4: two clean detections over the same graph yield identical `community_id`s (determinism);
  the transactional replace leaves `sum(node_count)=real` per folder.
- D5a: a monorepo fixture yields `folders.kind ∈ {subtree,workspace_member}`.
- D5b: a design-doc fixture (H1/H2/H3 + a TODO comment) yields nested `section` nodes and a
  `rationale` node; re-scanning it does **not** duplicate sections.
- D6a/c/d: folder-status transitions; a transient failure increments `retry_number` and
  retries; a child failure leaves the folder `failed` (not `indexed`).

**Property / idempotency.**
- `scan_twice_zero_delta`: scan an unchanged fixture twice → identical node-id set, per-kind
  edge counts, `community_id` per node, `communities` rows.
- `edge_dup_factor_is_one`: `count(*)/count(DISTINCT …)=1.0` for every kind.
- `processing_order_invariant`: shuffling `ProcessFile` order yields the same graph.

**Incremental.**
- change one file → only its slice mutates; unchanged nodes keep `id`+`community_id`; a
  deleted symbol is pruned; an added symbol appears.

**Resilience / resume.**
- `fatal_file_failure_is_recorded_and_retried`: inject a **fatal (DB-write) failure** in one
  `ProcessFile` → the failure is recorded (`task_executions.status='failed'`, `scan_state` **not**
  advanced), siblings complete, the barrier is fail-closed (`folder_status='failed'`, not
  `indexed`), and the next scan retries the file with `retry_number` incremented up to the cap
  (D6c). A *tolerated* parse error, by contrast, keeps `Ok` and advances `scan_state`.
- `restart_mid_scan_converges`: drive a scan, drop the in-memory queue mid-flight (simulated
  restart), run boot-reconcile + resume → orphaned `running` rows are terminal, the graph
  converges to the same steady state, no duplication, and already-fingerprinted files are
  **not** re-indexed.
- `concurrent_scan_is_single_writer` (W5/D6e): enqueue two scans of the same folder → the guard
  admits one; the graph is identical to a single scan (no duplication, no interleaved
  delete/upsert). Also assert `version_rescan` + manual rescan of one root don't both run.
- `embedding_survives_signature_stable_reindex` (D3): edit a comment (signature unchanged) →
  node keeps `id` **and** `embedding`; edit a body (hash changed) → embedding re-queued.
- `crash_between_covers_delete_and_insert` (D2): abort the covers-replace tx mid-way → folder
  is left `failed` (not `indexed`) and a re-run restores the full covers set — never a
  partial/zero-covers `indexed` folder.

**Integration — the whole-graph test (the one that verifies the graph, not the pieces).**
`graph_scan_end_to_end`: scan a committed fixture repo — multi-language code with a
class→method nesting, a design doc with H1/H2/H3 sections + NOTE/TODO comments, and a
monorepo sub-project — **end-to-end through the real task engine** (`spawn_workers`, real
barriers), then assert the entire graph at once:
1. **Kinds & hierarchy** — `section`/`rationale` present (+`package` when D5c); the doc
   decomposes `file → H1 → H2 → H3`; the sub-project folder is `subtree`/`workspace_member`;
   class→method nesting via `parent_id`.
2. **Edges** — dup-factor `1.0` every kind; structural:`covers` ratio sane; the
   `/api/graph/nodes` layout fetch returns `calls+imports+extends` and per-node `community_id`.
3. **Communities** — deterministic ids; coverage per invariant 5; **per-folder** integrity
   (`claimed == real`); `god_node_ids`/`description` populated (or honest-NULL, never templated).
4. **Retrieval** — `/api/graph/:repoId/tree` returns the nested docs/code/sub-project
   structure; `communities/info` uses live membership.
5. **Idempotency at integration level** — run the whole scan **again** → byte-identical graph.
6. **Incremental at integration level** — mutate one code file + one doc heading + delete one
   symbol → assert scoped update, pruned symbol gone, unchanged `community_id` preserved,
   still dup-factor `1.0`.
7. **Restart at integration level** — restart the engine between two runs → convergence (7).

This single test is the gate for "the graph is correct as a whole," and it is the one the
Done gate points at.

---

## Done gate

- **Idempotent re-run** — after two scans of a fixture root, zero net rows and
  `dup = 1.00` for every kind:
  ```sql
  SELECT kind, count(*), count(DISTINCT (source_id,target_id,target_name,target_file)) d,
         round(count(*)::numeric/greatest(count(DISTINCT (source_id,target_id,target_name,target_file)),1),2) dup
  FROM sensei.edges GROUP BY kind ORDER BY 1;            -- dup = 1.00 every row
  ```
- **Community coverage + per-folder integrity** (per-folder, not global):
  ```sql
  SELECT round(100.0*count(community_id)/count(*),1) coverage FROM sensei.nodes;   -- ~100
  SELECT count(*) FILTER (WHERE claimed <> real) mismatched_folders FROM (
    SELECT c.folder_id,
           sum(c.node_count) claimed,
           (SELECT count(*) FROM sensei.nodes n WHERE n.folder_id=c.folder_id AND n.community_id IS NOT NULL) real
    FROM inference.communities c GROUP BY c.folder_id) t;                          -- 0
  ```
- **Community enrichment** — a scanned fixture has `god_node_ids`; `description` is either
  model-authored or honest-NULL, **never templated**. Since SQL can't tell prose from a
  template, the writer must stamp provenance (`props.source ∈ {'insight-copy','null'}`, never a
  static-fallback marker) and the gate checks that, not just non-NULL:
  ```sql
  SELECT count(*) FILTER (WHERE array_length(god_node_ids,1) IS NULL) no_god,
         count(*) FILTER (WHERE description IS NOT NULL) with_desc,
         count(*) FILTER (WHERE props->>'source' = 'template') templated  -- must be 0
  FROM inference.communities;
  ```
- **Embedding survival across reindex** — re-scan a file whose *signature is unchanged* (edit a
  comment): the node keeps its `id` **and** its `embedding` (not re-nulled); edit a *body* so
  the content hash changes: the embedding is re-queued. Coverage does not regress:
  ```sql
  SELECT round(100.0*count(embedding)/count(*),1) emb_pct FROM sensei.nodes;  -- ≥ prior run
  ```
- **Doc/design granularity restored (P1 gate — `package` is P2/D5c, not gated here):**
  ```sql
  SELECT kind, count(*) FROM sensei.nodes WHERE kind IN ('section','rationale') GROUP BY kind; -- all > 0
  SELECT count(*) FROM sensei.nodes s JOIN sensei.nodes f ON s.parent_id=f.id
   WHERE s.kind='section' AND f.kind IN ('doc','file','section');                  -- > 0, nested
  SELECT count(*) FROM sensei.folders WHERE kind IN ('subtree','workspace_member'); -- > 0
  ```
- **Scoped incremental** — touch one file, re-scan: only that file's nodes change
  `modified_at`; others keep `id`+`community_id`; a deleted symbol disappears; no edge dupes.
- **Determinism** — two `clear + full-scan` runs produce identical `community_id` assignment.
- **Resilience/resume** — the `fatal_file_failure_is_recorded_and_retried` and
  `restart_mid_scan_converges` tests pass; after a simulated restart mid-scan,
  `SELECT count(*) FROM activity.task_executions WHERE status='running'` reconciles to 0 and the
  folder reaches `indexed`.
- **Convergence** — steady-state counts from `clear + full-scan` equal those from replaying
  the same edits incrementally (`graph_scan_end_to_end` step 5/6).
- **Retrieval** — `GET /api/graph/nodes?repoId=<fixture>` returns per-node `community_id` and
  `calls+imports+extends` edges; `GET /api/graph/<fixture>/tree` returns a nested structure.

## Wrong gate

- **Edge count climbs on a no-op re-scan** — `ON CONFLICT`/replace not wired; `covers` grows
  again (the exact regression).
- **`resolve_edges` starts erroring** — `resolve_edge` not made conflict-safe against
  `edges_unique_resolved` (D1).
- **`community_id` NULL after a scan** — `DetectCommunities` not chained, or reindex still
  deletes+reinserts nodes.
- **`communities` claims more members than nodes carry** — stale rows not pruned; or the
  Done-gate integrity check run globally instead of per-folder (a folder that over- and one
  that under-claims cancel out).
- **`community_id`s reshuffle between two identical scans** — non-deterministic labels.
- **Every repo > 500 nodes still renders as a flat, same-colour bubble cloud** — the
  `communities/info` overview still reads stale `node_count` / KIND instead of live membership.
- **A deleted symbol lingers, or an inbound edge to it vanishes** — prune-not-in missing, or
  the unresolve-before-prune step (D3.3) dropped, cascade-deleting cross-file edges.
- **Fabricated grouping / description** — empty `package` nodes, defaulted `subtree` kinds, or
  a templated community `description` on insight-copy failure (must be honest-empty).
- **Re-scan is a full rebuild** — the `scan_state` gate bypassed, "idempotent" only by doing
  all the work every time (ties to the churn regression — see *Related*).
- **A daemon restart mid-scan loses or duplicates work** — boot-reconcile/resume (D6b) not
  wired; folders stuck at `discovered`/`indexing`, or `task_executions` stuck at `running`.
- **A barrier marks a folder `indexed` after a child failed** — fail-closed (D6d) not enforced.
- **Two scans of the same folder race** — no `has_pending_kind_path` guard (D6e/W5), so two
  `ProcessFile`s interleave `unresolve→delete→upsert` or two `BuildConnections` both
  replace `covers` — reintroducing duplication/loss even with D1–D4 correct. (Reproduce:
  enqueue a `version_rescan` and a manual rescan of the same root simultaneously.)
- **A crash between `DELETE covers` and re-INSERT leaves a folder with zero covers marked
  `indexed`** — D2's replace not one transaction, or D6d doesn't treat "0 covers but stem
  matches exist" as failed.
- **Retry silently doesn't happen** — `Task` still has no `retry_number` field (D6c), so a
  fatal `ProcessFile` failure is recorded once and never re-driven; `task_executions.retry_number`
  stays 0 forever.
- **A stale embedding survives a body edit** — no `content_hash` compare in `upsert_node`
  (D3), so a symbol whose body changed keeps its old vector and semantic search returns the
  pre-edit meaning.

## Depth ledger — assumptions & reality anchors

The purpose of this ledger is that **no one reaches phase 3 and says "reality is different from
the spec."** Every design decision below is anchored to verified current code/DDL/DB
(`spec-doc-reviewer` + two capability audits confirmed each `file:line`/query on 2026-08-04/05).
Anything not verifiable is listed as an open question to resolve *before* coding, not during.

**Reality anchors (verified — the design rests on these).**

| The design assumes… | Status | Anchor |
|---|---|---|
| `edges` has no unique constraint; `insert_edge` is a bare INSERT | ✅ verified | `edges.ddl` (PK only); pg_store.rs:1711 |
| `nodes_unique_identity` exists and includes `line_start` | ✅ verified | nodes.ddl:26 |
| `process_file` deletes then re-inserts a changed file's nodes (new UUIDs) | ✅ verified | process.rs:754-758; pg_store.rs:1174 |
| handlers `process_file`/`resolve_edges`/`build_connections` **never return `Err`** (deliberate) | ✅ verified | process.rs:680-684 |
| task queue is in-memory; `Task` has no `retry_number`; fresh `task_id` per enqueue | ✅ verified | queue.rs:15-32, 71; mod.rs:253 |
| `folder_status` writes only `indexed`/`archived` (5 states dead) | ✅ verified | pg_store.rs:1380, 2136 |
| no enqueue site guards concurrent same-folder scans; `has_pending_kind_path` exists unused on the scan path | ✅ verified | scan.rs:128; version_rescan.rs:92; queue.rs:339 |
| `DetectCommunities` chained only from the daily analyzer refresh, not a scan | ✅ verified | analyzer_scheduler.rs:233 |
| `build_adjacency` reads `["calls","implements","imports"]`; `implements` = 0 rows | ✅ verified | community.rs:85; DB query |
| `/api/graph/nodes` fetches only `"calls"`; `communities/info` is node_count-flat | ✅ verified | codebase.rs:48, 194 |
| `TRUNCATE nodes CASCADE` also truncates `inference.drift_items` (only other referrer) | ✅ verified | drift_items.ddl:5; grep |

**Effort honesty — "rewire existing" vs "build new" (so nobody mis-estimates a phase).**

| Item | Nature | Note |
|---|---|---|
| D1 edge uniqueness + branched `insert_edge` | rewire | constraint + `ON CONFLICT` on an existing insert |
| D1 `resolve_edge` conflict-safety | rewire | small |
| D2 `replace_edges_of_kind(tx,…)` | **build new** method | today it's a loop-of-autocommits (resolve.rs:81) |
| D3 upsert-then-prune | rewire | invert existing delete-then-insert |
| D3 `content_hash` column + embedding-null-on-change | **build new** (DDL + write) | column does not exist |
| D4 community replace + determinism + adjacency | rewire (community.rs) | logic change, no new table |
| D5a folder kind=subtree/workspace_member | rewire (2 sites) | detection exists; member writes `folder` (process.rs:662), subtree writes `git` (process.rs:462); must extend the kind lists in index_audit/dedup/prune |
| **D5b `section`/`rationale` emission** | **wire existing + new** | the heading parser already exists but is test-gated (`doc_indexer.rs:157` `extract_sections`/`process_ir`, "test-only; will be wired"); D5b = un-gate+wire it + write section nodes via D3 + **NEW rationale-comment extraction** (absent today) |
| D5c `package`/sub-symbol nodes | **build new** | P2 |
| D6a `update_folder_status` + lifecycle writers | **build new** | 5 states have no writer |
| D6b boot reconcile of orphaned `running` rows | **build new** | none today |
| D6c `retry_number` on `Task` + backoff (spawned sleep→enqueue) | **build new** | no attempt field, no delayed-task primitive |
| D6e single-writer guards | rewire | apply the existing `has_pending_kind_path` at ~7 sites |

**Decisions log (reversals of documented/approved behavior).**
1. **D6c-trigger — fatal errors propagate as `Err` and are reported** — **APPROVED 2026-08-05**
   ("errors should be reported"). Reverses process.rs:680's "always return Ok"; tolerated parse
   errors still stay `Ok`.
2. **Migration may clear `inference.drift_items`** — **APPROVED 2026-08-05**. Still re-derive
   traceability after the rebuild so history is restored, not permanently lost (mitigation, not a
   blocker).
3. **D3 identity-key change** `line_start → signature` — **still open**; confirm at build-plan.
   Deferrable within D3 and **not needed for D1/D2**, so it does not gate the first phase.

**Open questions to close before implementation (do not discover these at phase 3).**
- Backoff scheduling: the queue has no delayed-task primitive — confirm the spawned-`sleep`→`enqueue` approach vs adding a `run_after` to the queue.
- `content_hash` scope: hash the symbol body only, or signature+body? (affects when embeddings re-queue).
- Singleton→file-community assignment (D4.4): is a file always in exactly one community, or can a file span communities? (affects the "~100 % coverage" invariant's denominator).
- ~~Section identity: `section` nodes keyed on `(file, heading-path)` or `(file, line)`?~~ **RESOLVED (D5b):** keyed on `(folder, file, kind='section', name=heading-path, parent_id, line_start=NULL)` — heading-path in `name` + a NULL `line_start` makes identity line-INDEPENDENT (stable across edits) without the reverted 0.1 key change. Identical-text SIBLINGS under the same parent are disambiguated with a deterministic ` #N` suffix on the 2nd+ occurrence (which flows into the child path too), so two `## Setup` under one H1 are distinct nodes rather than the second silently clobbering the first (correctness-review catch).

> **Out of scope, explicitly (so the spec isn't read as covering them).** Architectural
> pattern detection (adapter/strategy/…), duplication *persistence* (`duplicates`/`similar_to`
> edges), structural anti-patterns/god-nodes, FTS, `traces_to`, and section-level drift are
> **not** in this spec. They are audited and planned in the
> [indexer capability roadmap](../../analysis/2026-08-05-indexer-capability-coverage.md); this
> spec only makes the graph they depend on *correct, idempotent, and durable*.

## Migration (one-time)

Clearing **graph** data to reindex is approved, and clearing `inference.drift_items` with it is
**approved (2026-08-05)**. **Disclosure (still true):** `TRUNCATE sensei.nodes CASCADE` also
truncates `inference.drift_items` (`drift_items.doc_node_id references sensei.nodes(id) on delete
cascade`; Postgres `TRUNCATE … CASCADE` truncates all referencing tables regardless of the FK's
own action) — so doc-drift/traceability history is wiped too. **Mitigation (required):** re-run
the traceability scanner after the rebuild so drift is re-derived from the fresh graph — history
is restored, not permanently lost.

Preferred path:
1. Land D1 (edge indexes + branched `insert_edge` + conflict-safe `resolve_edge`) and
   D2/D3/D4/D6 behind it.
2. Clear the graph: `TRUNCATE sensei.edges, sensei.nodes CASCADE; TRUNCATE inference.communities;`
   (DDL is daemon-owned runtime state; apply via `dbd` per the repo's DDL-sync rules — full-file
   DDL, no ALTERs). **Then re-derive traceability** (re-run the drift scanner, [[pipeline/traceability]])
   so drift history is rebuilt from the fresh graph.
3. Re-enqueue `ScanRoot` for every `folders_to_watch` root; the now-idempotent pipeline (with
   chained `DetectCommunities` and the folder-status lifecycle) rebuilds.
4. Verify the Done-gate SQL against the live DB.

Drift-preserving fallback (no `drift_items` loss): dedup edges in place instead of truncating —
create a temp index on `(folder_id, source_id, kind, target_id, target_name)` first, then
`DELETE FROM sensei.edges a USING sensei.edges b WHERE a.id > b.id AND a.folder_id=b.folder_id
AND a.source_id=b.source_id AND a.kind=b.kind AND a.target_id IS NOT DISTINCT FROM b.target_id
AND a.target_name IS NOT DISTINCT FROM b.target_name` — then create the unique indexes and run
`DetectCommunities` folder-by-folder.

> **Related churn note.** `version_rescan` re-scans every root on each daemon version bump (an
> 8.6× indexing amplifier — see
> [`analysis/2026-08-04-deep-dive/11-regression-churn-lifecycle.md`](../../analysis/2026-08-04-deep-dive/11-regression-churn-lifecycle.md)).
> Idempotency (this spec) makes those re-scans *safe*; debouncing the trigger so they are also
> *cheap* is tracked separately.

## Related

- [[pipeline/capture]] — owns hook events + repo/sub-project detection that feeds `folders`
- [[pipeline/traceability]] — consumes `covers`/`traces_to`; benefits from D2 + D5b; **its
  `drift_items` are re-derived after the migration**
- [[pipeline/analyzer]] — enqueues `DetectCommunities` (daily today; D4 also chains it to scans)
- [[pipeline/semantic-search]] — consumes `nodes.embedding` (must survive reindex — D3)
- Atlas rendering (screen) — downstream consumer of the retrieval contract; **no spec yet**,
  should be written against the `tree` + per-node `community_id` contract above
- Evidence: [`analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md`](../../analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md)
