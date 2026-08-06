# Code-graph idempotency — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: use `superpowers:subagent-driven-development`
> or `superpowers:executing-plans` to run this task-by-task. Steps use `- [ ]` checkboxes.
> Every task is TDD: write the failing test → confirm it fails → implement → confirm green →
> `zero-errors-policy` → commit. Never mark a task done on a masked/piped exit code.
>
> **Reviewed:** `sensei-plan-depth-reviewer` — READY (all punch-list gaps folded in;
> reality anchors verified against live code, zero wrong claims).

**Goal:** Make the code-graph indexer **convergent** — for a given working tree, N scans (full
or incremental, any order, across daemon restarts) produce a byte-stable graph: no `covers`
duplication, `community_id`/`embedding` survive reindex, stale rows are pruned, the restored
grouping/granularity kinds are emitted, and the retrieval endpoints return the hierarchy + live
membership. Fixes the Atlas regression at its root.

**Design (source of truth):** [`docs/spec/pipeline/code-graph.md`](../spec/pipeline/code-graph.md)
— D1–D6, invariants 1–7, W1–W5, Done/Wrong gates, and the **Depth ledger**. Do not re-derive
the design here; this plan sequences it into TDD tasks with acceptance criteria.

**Evidence:** [`…/deep-dive/10-graph-indexing-regression.md`](../analysis/2026-08-04-deep-dive/10-graph-indexing-regression.md),
[`…/11-regression-churn-lifecycle.md`](../analysis/2026-08-04-deep-dive/11-regression-churn-lifecycle.md).

**Tech stack:** Rust (`crates/senseid`); Postgres DDL via **`dbd`** (full-file DDL, **no
ALTERs**); `cargo test -p senseid` + `cargo clippy`. No app/JS in Phases 0–6; Phase 7 touches
the API handlers (the Svelte Atlas view is a separate follow-on).

**Conventions:**
- **Git hygiene:** the pre-commit hook stages broadly — always `git status` then explicit
  `git add <paths>`; per-task commit to `develop`; leave a clean tree. Pre-commit runs
  `make test-fast` — keep it green.
- **DDL:** rewrite the whole `.ddl` file (no `ALTER`); apply with `dbd`; a test DB is used for
  test runs (`SENSEI_DDL_DIR` local override).
- **Verify the real effect:** every acceptance check asserts the specific graph state (a query
  count/dup-factor), never a proxy or a piped exit code.
- **No fabrication:** on a fatal path, propagate/record the error — never write a placeholder
  node/edge/community/description (honest-empty only when genuinely empty).

**Canonical test names** (identical across spec + this plan — reconciled per the review):
`insert_edge_is_idempotent`, `resolve_edge_merges_on_conflict`, `covers_replaced_not_appended`,
`crash_between_covers_delete_and_insert`, `reindex_preserves_id_community_embedding`,
`vanished_symbol_pruned_inbound_unresolved`, `body_edit_requeues_embedding`,
`crash_mid_node_write_is_atomic`, `concurrent_scan_is_single_writer`,
`folder_status_lifecycle`, `fatal_file_failure_is_recorded_and_retried`,
`restart_mid_scan_converges`, `community_ids_deterministic`, `community_replace_no_stale_rows`,
`community_coverage_full`, `degree_and_god_nodes_populated`, `doc_decomposes_into_sections`,
`monorepo_folders_classified`, `processing_order_invariant`,
`graph_nodes_returns_community_and_structural_edges`, `graph_tree_endpoint_returns_hierarchy`,
`communities_info_uses_live_membership`, `graph_scan_end_to_end`.

**Sequencing (dependency order, verified sound by the review):**
`Phase 0 (decisions)` → `1 edge identity` → `2 derived-set replace` → `3 node identity+embedding`
→ `4 worker robustness+single-writer` → `5 community durability` → `6 grouping/granularity kinds`
→ `7 retrieval + whole-graph integration test` → `8 migration + live verify`.

---

## Phase 0 — Close the open questions + confirm sign-off (no code)

Resolving every ambiguity **before** any dependent phase is the depth guarantee — nothing
enters implementation ambiguous. Output: an updated spec (single source of truth).

- [ ] **0.1 Identity-key sign-off (gates Phase 3 scope).** Decide `nodes_unique_identity`: keep
  `line_start`, or change to `(folder_id, file_path, kind, name, parent_id, signature)`.
  **Recommendation:** `signature` — a symbol that only moves keeps its `id` (→ community +
  embedding survive). Add `line_start` back only if a fixture proves a name+parent+signature
  collision. Record in the spec Depth ledger.
- [ ] **0.2 `content_hash` scope (gates Phase 3 re-embed).** Hash **body only** — a signature
  change already re-keys the node; the hash catches a body-only edit under a stable identity.
- [ ] **0.3 Singleton→community rule (gates Phase 5 coverage denominator).** A file belongs to
  exactly one community; a symbol with no adjacency-eligible edge inherits its enclosing
  file/module community. Invariant-5 denominator = **all** nodes (~100%).
- [ ] **0.4 Section identity (gates Phase 6 parser).** `section` keyed on `(folder_id, file_path,
  kind='section', name=heading-path, parent_id)` where `name` is the full heading path
  (`"Design > Auth > Refresh"`), stable across line edits (reuses `nodes_unique_identity`, which
  after 0.1 excludes `line_start`).
- [ ] **0.5 Backoff primitive (gates Phase 4 retry).** No delayed-task primitive exists; backoff
  is a spawned `tokio::spawn(async { sleep(backoff).await; queue.enqueue(retry) })`, not a
  blocking wait. No `run_after` column.
- [ ] **0.6 Fault-injection + concurrency-race harness (gates Phase 3/4 tests).** No
  fault-injection harness exists in `crates/senseid` today (verified). Decide the mechanism:
  (a) a test-only failure seam on `PgStore` (e.g. a `#[cfg(test)]` injectable error hook on the
  write methods) to drive `fatal_file_failure_is_recorded_and_retried`,
  `crash_between_covers_delete_and_insert`, `crash_mid_node_write_is_atomic`; (b) a real
  two-`tokio::task` race against the shared `TaskQueue` (no mock) to drive
  `concurrent_scan_is_single_writer`. Record the seam's shape so implementers don't each invent one.
- [ ] **0.7 `DetectCommunities` chaining (gates Phase 5).** Confirm: no timing-debounce beyond
  the existing idempotent-replace no-op (an unchanged folder's re-detect is a no-op by
  construction, D4.2); the folder reaches `indexed` only at the **terminal community barrier**
  (see 5.2), so `indexed` implies communities computed. Record in the spec (adjust D6a wording).
- [ ] **0.8** Update `docs/spec/pipeline/code-graph.md` Depth ledger with 0.1–0.7 outcomes
  (Open-questions block → empty); commit `docs: resolve code-graph plan open questions (Phase 0)`.

**Acceptance:** the spec's Open-questions block is empty; the identity-key + fault-seam +
chaining decisions are recorded as resolved.

---

## Phase 1 — Edge identity & idempotent `insert_edge`/`resolve_edge` (D1)

**Precondition (verified):** `edges` has only a PK; `insert_edge` bare INSERT (pg_store.rs:1711);
`resolve_edge` bare UPDATE (1742); 5 `insert_edge` call sites at process.rs:794/808/816/825/830.

- [ ] **1.1** DDL: add `edges_unique_resolved` + `edges_unique_unresolved` partial indexes
  (spec D1) to `database/ddl/table/sensei/edges.ddl` (full-file); `dbd apply` to the test DB.
- [ ] **1.2 Failing test** `insert_edge_is_idempotent`: same resolved edge twice → `count(*)=1`,
  equal returned ids; same for an unresolved edge by `(target_name,target_file)`. Run → FAIL.
- [ ] **1.3** Implement branched `insert_edge(folder,source,target_id,target_name,target_file,
  kind,confidence)` (spec D1 SQL); update all 5 call sites + resolve.rs to pass `target_file` for
  unresolved targets. Run 1.2 → PASS.
- [ ] **1.4 Failing test** `resolve_edge_merges_on_conflict`: resolving an unresolved edge onto an
  existing resolved target → one row remains, no unique violation. Run → FAIL.
- [ ] **1.5** Implement conflict-safe `resolve_edge` (spec D1 merge-then-delete-loser) in one
  `pool.begin()` tx. Run 1.4 → PASS.
- [ ] **1.6** `cargo test -p senseid` + `cargo clippy --all-targets` green. Commit
  `feat(senseid): edge identity — unique indexes + idempotent insert_edge/resolve_edge`.

**Acceptance (spec Done-gate row 1):** re-inserting any edge set is a no-op; dup-factor 1.0.

---

## Phase 2 — Derived-set replace, in one transaction (D2)

**Precondition (verified):** `build_connections` (resolve.rs:81) inserts covers in a
loop-of-autocommits; `reconcile_connections` dead; `pool.begin()` pattern at pg_store.rs:3811.

- [ ] **2.1 Failing test** `covers_replaced_not_appended`: build covers; delete a covered stem
  match; re-run → stale covers row **gone**, no dupes. Run → FAIL.
- [ ] **2.2** Add `PgStore::replace_edges_of_kind(tx, folder_id, kind, edges)` (DELETE-by-
  folder+kind + batch insert, one tx). Rewrite `build_connections` to compute the full covers
  set then call it; add per-file out-edge reconcile (delete-by-source-then-insert in the file's
  tx). Retire `reconcile_connections`. Run 2.1 → PASS.
- [ ] **2.3 Failing test** `crash_between_covers_delete_and_insert`: abort the replace tx mid-way
  (via the Phase-0.6 failure seam) → folder **not** `indexed`, re-run restores the full set;
  never a partial/zero-covers `indexed` folder. Run → FAIL → implement tx boundary + the D6d
  hook. PASS.
- [ ] **2.4** Green. Commit `feat(senseid): replace_edges_of_kind — covers/out-edges reconciled
  transactionally (D2)`.

**Acceptance (spec Done-gate row 1 + D2 shrink test):** stale derived edges vanish; a crash
mid-replace never yields a partial-covers `indexed` folder.

---

## Phase 3 — Node identity, prune, and embedding survival (D3)

**Precondition (verified):** `process_file` runs `unresolve_edges_to_file` +
`delete_nodes_by_file` then `upsert_node` (process.rs:754); on-conflict updates only
`signature`/`line_end`, never `embedding`; no `content_hash` column. Applies Phase-0.1/0.2.

- [ ] **3.1** DDL: add `content_hash text` to `nodes.ddl`; if 0.1 chose the signature key,
  rewrite `nodes_unique_identity` (full-file, `dbd apply`).
- [ ] **3.2 Failing test** `reindex_preserves_id_community_embedding`: assign community+embedding,
  edit an unrelated line (signature unchanged), reindex → symbol keeps id, community_id,
  embedding. Run → FAIL.
- [ ] **3.3** Implement **upsert-then-prune** (spec D3 steps 1–4), the **entire file write inside
  one `pool.begin()` tx** (upsert current → unresolve-scoped-to-vanished → delete vanished →
  reconcile out-edges): so a crash mid-write is atomic. `upsert_node` on-conflict: compare
  `content_hash`, `SET embedding=NULL` (re-queue) only on change, else preserve. Run 3.2 → PASS.
- [ ] **3.4 Failing test** `vanished_symbol_pruned_inbound_unresolved`: delete a referenced symbol
  → gone AND inbound edge unresolved (`target_id NULL`, `target_name` kept), not cascade-deleted.
  Run → FAIL → implement → PASS.
- [ ] **3.5 Failing test** `body_edit_requeues_embedding`: body edit (hash change) → embedding
  re-queued; comment-only edit → embedding preserved. PASS.
- [ ] **3.6 Failing test** `crash_mid_node_write_is_atomic`: abort mid node-write (0.6 seam) →
  the file's prior nodes/edges intact, folder does not advance past its current status (mirrors
  2.3 for the 4-step multi-table write). Run → FAIL → confirm the single-tx boundary from 3.3
  covers it. PASS.
- [ ] **3.7** Green. Commit `feat(senseid): upsert-then-prune node identity + embedding survival,
  atomic per-file write (D3)`.

**Acceptance (spec Done-gate: scoped-incremental + embedding-survival):** a surviving symbol
keeps id/community/embedding; a vanished one is pruned with inbound edges unresolved; a crash
mid-write leaves a consistent file slice; coverage does not regress.

---

## Phase 4 — Worker robustness: single-writer, folder-status, retry, fail-closed (D6 + W1–W5)

**Precondition (verified):** queue in-memory; no `has_pending_kind_path` on the scan path
(scan.rs:128, version_rescan.rs:92…); `Task` no retry field (mod.rs:253); only
`indexed`/`archived` folder-status writers; handlers never `Err` (process.rs:680). Uses the
Phase-0.6 fault seam + race harness. **Fatal-error propagation APPROVED (2026-08-05).**

- [ ] **4.1 Failing test** `concurrent_scan_is_single_writer` (W5/D6e): two real `tokio::task`s
  race a scan of one folder against the shared `TaskQueue` (0.6b) → only one runs; graph equals a
  single scan (no dup/interleave). FAIL.
- [ ] **4.2** Add `has_pending_kind_path(ProcessGitFolder/ProcessFile, path)` guards at the ~7
  enqueue sites (scan.rs:128/474, resume.rs:50, process.rs:427, workspace.rs:437/494/594,
  version_rescan.rs:92). Run 4.1 → PASS.
- [ ] **4.3 Failing test** `folder_status_lifecycle`: assert `queued`/`indexing`/`indexed`/`failed`
  transitions. FAIL → add `PgStore::update_folder_status` + lifecycle writers (D6a; `indexed` at
  the terminal barrier per 0.7). PASS.
- [ ] **4.4 Failing test** `fatal_file_failure_is_recorded_and_retried` (D6c/W3): inject a fatal
  DB-write error (0.6a) → propagates as `Err`, recorded (`task_executions.status='failed'`+
  `error_message`), `scan_state` **not** advanced, barrier fail-closed (`folder_status='failed'`),
  retries with `retry_number+1` up to N; a *tolerated* parse error stays `Ok` and advances
  `scan_state`. FAIL → implement: `retry_number` on `Task`, typed fatal-vs-tolerated split,
  `Err` propagation + reporting, backoff via spawned `sleep`→`enqueue` (0.5), D6d whole-chain
  `folder_status != 'failed'` predicate before `mark_folder_indexed`. PASS.
- [ ] **4.5 Failing test** `restart_mid_scan_converges` (W2): drive a scan, drop the queue
  mid-flight, run boot-reconcile (orphaned `running`→terminal) + extended `resume_pending_scans`
  (`queued`/`indexing`/`failed`) → convergence, no dup, no redo of fingerprinted files. FAIL →
  implement D6b → PASS.
- [ ] **4.6** Green. Commit `feat(senseid): worker robustness — single-writer, folder-status,
  bounded retry, fail-closed barriers (D6)`.

**Acceptance (spec Done-gate: resilience/resume; Wrong-gate: concurrent-scan, retry, restart).**

---

## Phase 5 — Community durability, determinism, coverage, enrichment (D4)

**Precondition (verified):** `DetectCommunities` chained only from the daily analyzer
(analyzer_scheduler.rs:233); no delete-before-upsert; label ids drift; `build_adjacency` uses a
dead `implements`; singletons skipped. Applies Phase-0.3/0.7. Depends on Ph3 (stable ids) + Ph4
(single-writer, so the scan-chained detect doesn't race).

- [ ] **5.1 Failing test** `community_ids_deterministic`: two clean scans → identical
  `community_id` per node. FAIL → deterministic label remap (rank by min natural-key → `1..k`). PASS.
- [ ] **5.2 Failing test** `community_replace_no_stale_rows`: re-detect a shrunk folder →
  `sum(node_count)=count(nodes with id)` per folder, 0 orphans. FAIL → transactional
  replace-per-folder + **chain `DetectCommunities` as the terminal scan barrier that sets
  `folder_status='indexed'`** (0.7 — so `indexed` implies communities; a mid-detect crash is an
  atomic no-op and cannot leave a half-indexed folder). PASS.
- [ ] **5.3 Failing test** `community_coverage_full`: every node has a `community_id` (singletons
  inherit file/module community, 0.3) → ~100%. FAIL → broaden `build_adjacency`
  (`calls,imports,extends,references`+`parent_id`; drop `implements`) + singleton assignment. PASS.
- [ ] **5.4 Failing test** `degree_and_god_nodes_populated`: `nodes.degree` set during
  `ResolveEdges`; `god_node_ids` = top-5 by degree; `description` provenance stamped —
  **assert `props.source ∈ {'insight-copy','null'}`** (positive membership, not just `!=
  'template'`), honest-NULL on insight-copy failure. FAIL → implement → PASS.
- [ ] **5.5** Green. Commit `feat(senseid): community durability, determinism, coverage,
  enrichment (D4)`.

**Acceptance (spec Done-gate: coverage + per-folder integrity + enrichment; Wrong-gate:
reshuffle, over-claim).**

---

## Phase 6 — Restore grouping/granularity kinds: section/rationale, subtree (D5a/b)

**Precondition (verified 2026-08-06):** `section`/`rationale`/`package`/… = 0 rows.
`folders.kind` is NOT hardcoded `'folder'` (spec D5a corrected): `scan_root` writes
`git`/`standalone`; nested git subtrees write `git` (process.rs:462, `upsert_repo`); monorepo
members write `folder`+`role` (process.rs:662, `upsert_subfolder`). D5a must change BOTH write
sites (member→`workspace_member`, subtree→`subtree`) and extend the `kind='folder'` allow/deny
lists in `index_audit`/`dedup_structural_folder_nodes`/`prune_vanished_folders` — see spec D5a.
Section identity (Phase-0.4) keys on heading-path with `line_start=NULL` (D3 kept `line_start`
in `nodes_unique_identity`, so a NULL line makes section identity line-independent WITHOUT the
0.1 key change). **`doc_indexer.rs:157-267` already has a tested heading
parser** (`extract_sections`/`parse_heading`/`parse_to_ir` → `IRSection` with level+nesting;
tests l.580-589) and `processors/doc.rs:38-51` has `process_ir` commented *"test-only; will be
wired into the processing pipeline."* — all `#[cfg(test)]`-gated and disconnected from the
production doc path (`doc.rs::process()` never calls it). Applies Phase-0.4 section identity.

- [ ] **6.1 Failing test** `doc_decomposes_into_sections`: index a fixture design doc (H1/H2/H3 +
  a TODO comment) → nested `section` nodes (`file→H1→H2→H3` via `parent_id`, `props.level`) + a
  `rationale` node; re-index → no duplicate sections. FAIL → implement: **(a) un-gate + wire the
  existing `process_ir`/`extract_sections` into the production doc path; (b) write `section` nodes
  through the D3 upsert/prune path; (c) NEW: rationale-comment (NOTE/WHY/HACK/TODO/IMPORTANT)
  extraction (absent today — this is the genuinely new parser piece).** PASS.
- [ ] **6.2 Failing test** `monorepo_folders_classified`: a monorepo fixture → members
  `folders.kind='workspace_member'` (process.rs:662 site, kind-aware upsert), nested repos
  `'subtree'` (process.rs:462 site). FAIL → change both write sites + extend the kind lists in
  `index_audit`/`dedup_structural_folder_nodes`/`prune_vanished_folders` so a reclassified
  member/subtree is neither a false ghost nor escapes a genuine prune. PASS.
- [ ] **6.3** Green. Commit `feat(senseid): wire section/rationale emission + subtree folder kinds
  (D5a/b)`.

> **Cut from this plan (logged in `docs/backlog.md`):** D5c `package`/sub-symbol
> (`property`/`field`/`parameter`/`enum_variant`) node emission — P2, not required by any P1
> Done-gate. Final Verification below **excludes** a `package` test.

**Acceptance (spec Done-gate: section/rationale > 0 and nested; subtree/workspace_member > 0).**

---

## Phase 7 — Retrieval contract + the whole-graph integration test

**Precondition (verified):** `/api/graph/nodes` fetches only `"calls"` (codebase.rs:48), omits
`community_id`; `communities/info` node_count-flat (194); no `/tree`; `get_nodes_scoped`/
`get_edges_scoped` at 9607/9617.

- [ ] **7.1 Failing test** `graph_nodes_returns_community_and_structural_edges`: handler returns
  per-node `community_id` + `calls+imports+extends` edges. FAIL → widen `get_edges_scoped` to
  multi-kind; add `community_id` to `get_nodes_scoped`; update the handler. PASS.
- [ ] **7.2 Failing test** `graph_tree_endpoint_returns_hierarchy`: new `GET /api/graph/:repoId/
  tree` → folder-tree (by `kind`/`role`) → file → `parent_id` chain → doc `section` tree. FAIL →
  implement. PASS.
- [ ] **7.3 Failing test** `communities_info_uses_live_membership`: overview driven by
  `nodes.community_id`, not stale `node_count`. FAIL → implement. PASS.
- [ ] **7.4 Failing test** `processing_order_invariant` (spec Test plan): process the fixture
  repo's files in ≥2 different orders → identical resulting graph (node-id set, per-kind edge
  counts, community_id per node). FAIL → confirm order-independence (any residual order-dependence
  is a bug to fix). PASS.
- [ ] **7.5 THE whole-graph integration test** `graph_scan_end_to_end` (spec Test plan): scan the
  committed fixture repo (`crates/senseid/tests/fixtures/graph-scan/` — multi-lang code with
  class→method, a design doc with H1/H2/H3 + NOTE/TODO, a monorepo sub-project) through the real
  task engine (`spawn_workers`), assert the entire graph at once: kinds+hierarchy; dup-factor 1.0
  + structural:covers ratio; deterministic communities + per-folder integrity; retrieval `tree` +
  per-node `community_id` + live overview. Then **run again → byte-identical** (idempotency). Then
  mutate one file + one heading + delete one symbol → scoped incremental, pruned symbol gone,
  unchanged `community_id` preserved, dup-factor still 1.0. Then simulated restart between runs →
  convergence.
- [ ] **7.6** Build the fixture repo under `tests/fixtures/graph-scan/` (committed). Green.
  Commit `feat(senseid): graph retrieval contract + whole-graph integration test`.

**Acceptance (spec Done-gate: retrieval + convergence; the integration test is the gate the Done
gate points at).**

---

## Phase 8 — Migration + live verification (one-time)

**Precondition:** Phases 1–7 green. Migration + clearing `inference.drift_items` **APPROVED
(2026-08-05)**; re-derive traceability after (required mitigation).

- [ ] **8.1** On a **snapshot/dev DB first**: `TRUNCATE sensei.edges, sensei.nodes CASCADE;
  TRUNCATE inference.communities;` (via `dbd`), re-enqueue `ScanRoot` for every
  `folders_to_watch` root, let the idempotent pipeline rebuild, then re-run the traceability
  scanner to re-derive `drift_items`.
- [ ] **8.2 Live Done-gate verification** — run the spec's Done-gate SQL against the rebuilt DB:
  dup-factor 1.0 every kind; coverage ~100% + 0 per-folder mismatches; section/rationale + subtree
  rows > 0; embedding coverage ≥ prior; `/api/graph/<root>/tree` nested; `communities/info`
  non-flat. Record the outputs.
  **HALT-ON-FAILURE (irreversible step):** `8.1`'s TRUNCATE is not reversible for the graph. If
  `8.2` fails after `8.1` ran, **halt and escalate to a human** — do NOT re-run `8.1` or proceed
  to `8.3`/`8.4` automatically. (Depth-bar terminal behavior for an irreversible step.)
- [ ] **8.3** Scan a second time → assert **zero net rows** (convergence proof on real data).
- [ ] **8.4** Once gates pass on live data, flip the spec status to `draft` (shipped). Commit
  `chore(senseid): reindex to idempotent graph + verify Done-gate on live data`.

**Acceptance:** the live DB satisfies every spec invariant; a repeat scan is a no-op; a second
scan that grew any edge count = FAIL.

---

## Final verification (whole plan)
- [ ] `cargo test -p senseid` green (**every canonical test above**, incl. `graph_scan_end_to_end`;
  **excludes** any `package` test — D5c is cut to backlog); `cargo clippy --all-targets` clean;
  `make test-fast` green.
- [ ] Live Done-gate SQL (8.2) recorded + passing; second-scan-zero-delta (8.3) proven.
- [ ] Spec Wrong-gate walked (`wrong-gate-hunter`-style): edge growth on no-op, concurrent-scan
  race, restart loss, stale embedding, templated description — all absent.
- [ ] Depth ledger: 0 open questions; sign-off items recorded resolved.

## Self-review (author)
- Phase 0 closes every ambiguity (incl. the fault-injection seam and the `indexed`-at-terminal-
  barrier decision) before any dependent phase — no phase begins with a TBD.
- Every phase has an observable acceptance criterion tied to a spec Done-gate SQL/test; test
  names are reconciled to one canonical set across spec + plan; the irreversible Phase-8 step has
  an explicit halt-on-failure clause.
