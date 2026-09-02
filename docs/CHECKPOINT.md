# Checkpoint

**Slice:** MCP shape + `call_graph` column fixes DONE (`93cef04a`, pushed), after
the clean rebuild (`268c242f`: 7,642 folders / 408,969 nodes / 731,370 edges).
Gates: rust 2,679/0, app 1,698/118, dojo 1,535/138, SQL 8, clippy 0, fmt clean.

## What was wrong

`get_callers("prune_orphan_stubs")` returned `[]` for a symbol with two real
callers — and the SAME bytes for one that does not exist. `call_graph` exposes
`tgt.name AS target_name` off a LEFT JOIN, NULL for every unresolved edge while
the name sits in `unresolved_target`, so filtering the resolved-only column made
**117,201 of 335,756 `calls` edges (34.9%) unreachable**. The view's own "Common
queries" comment recommended that exact filter. Six fixes, each TDD +
mutation-probed:

| # | fix | live proof |
|---|---|---|
| 1 | `target_symbol` coalesce on `call_graph` | 117,201 edges recovered |
| 2 | `found` + `coverage` envelope (#148) | not-found now `found:false` |
| 3 | `is_test_path` bare `tests.rs` | 373 test fns were `is_test=false` |
| 4 | exact-name-first search | definition #2 → **#1** |
| 5 | `locality` on callees (read from `graph_nodes`) | `query`/`Ok` external |
| 6 | `graphHealth` on summary | calls 57%, refs/imports/extends 0% |

Left undone on purpose: search relevance score + `exclude_tests` (needs SQL
plumbing and a manifest input — own slice); no combinator denylist for callees,
since `locality:"unknown"` already filters them.

## Next: stub-GC ordering bug (open from `268c242f`)

9,863 stubs match the GC predicate yet survive, 100% attributed: per-folder GC
fires at the community terminal barrier, then `scan_root reconcile` runs
`dedup_structural_folder_nodes` (`graph.rs:1830`), deleting 36,032 duplicates
whose cascade strips the in-edges that made those stubs ineligible — and nothing
GCs again. All 9,863 sit in `kind='folder'` folders across 4. **Red-first:** a
test that dedups a structural folder and asserts no orphan stub survives the
reconcile, then call `prune_orphan_stubs` after the dedup. Fix the ordering.

## Known-broken

`imports` 0 of 136,532 resolved (25,692 are LOCAL and must). Detection reads
calls+imports+extends+references (`community.rs:185`) and three are 0% — hence
97.8% single-file communities (#149: cross-file stub share 20.3%). `references`
251,185/0 (`doc_indexer.rs:584` and `:587` — two defects). `extends` 7,901/0
(#147). `dojo_memberships.sync_status` dead. `graph-end-state-sketch.md` §1–12
NOT SAFE TO BUILD FROM.

## Traps

`sensei_test` has NO automated schema provisioning (manual prerequisite), so a
DDL change must be applied to BOTH DBs or the DB tests fail on a missing column.
`app/.../wizard-state.spec.svelte.ts` is not hermetic — real fetch to `:7744`.
Never pipe a gate through `| head`: SIGPIPE truncated a run and masked a real
`fmt --check` failure. Wipe needs the daemon STOPPED; `/health` not
`/api/health`; a cold graph starves its own rebuild (`queue.rs:479`).
