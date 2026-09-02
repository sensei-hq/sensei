# Checkpoint

**Slice:** stub-GC ordering FIXED and verified in production (`a7a1bee9`), after
the MCP shape fixes (`93cef04a`) and the clean rebuild (`268c242f`). Branch
`develop`. Gates: rust 2,680/0, app 1,698/118, dojo 1,535/138, SQL 8, clippy 0,
fmt clean.

## The GC ran before the thing that creates its work

Per-folder `prune_orphan_stubs` fires at the community terminal barrier; then
`scan_root reconcile` runs `dedup_structural_folder_nodes` and deletes the
structural duplicates — and *that* cascade strips the edges which were the only
reason those stubs were ineligible. Nothing ran the GC again.

`prune_orphan_stubs_scoped` now runs immediately after the dedup, root-scoped in
one statement (8,855 folders under a root, so per-folder round trips would
dominate a single indexed delete), fail-open like the per-folder pass, with
`stubs_collected` in the reconcile summary. The predicate is untouched and
unduplicated — `prune_orphan_stubs` delegates via `std::slice::from_ref`, the
same idiom `get_edges_by_kind` uses, and all three existing stub-GC tests pass
unchanged through the new path.

**Production proof, not just the test:** log `collected 9863 stub(s) the dedup
orphaned` · eligible stubs **9,863 → 0** · `unknown` locality **54,602 → 44,739**
(down exactly 9,863) · the 4 `cluster:*` folders now zero rows.

## Test lesson worth keeping

The test drives `scan_root`, not dedup-then-prune by hand, because the defect IS
the order of two calls inside that reconcile. The first version **passed before
the fix existed**: `reconcile_roots` pruned the whole repo root (the fixture had
no `.git`, so the walk never classified it live) and cascaded the stub away — a
green test proving nothing. Asserting the member *folder* survives exposed it; a
real `.git` made the fixture reach the dedup. Then mutation-probed by removing
only the prune call, refactor left in place.

## Graph state

728,985 nodes / 8,855 folders / 51,971 files. internal 673,870 · unknown 44,739 ·
external 10,376. Growth from the 408,969 at rebuild time is continued indexing,
not duplication (folders 7,642→8,855, files 46,512→51,971 track it). 684
`(project, file_path)` groups still span >1 folder — #150's content-hash
territory.

## Next — pick one

**(a) Resolution, not GC.** The 44,689 remaining `unknown` stubs all have
in-edges, so no GC will touch them. Java-dominated: java 27,967 (51%), ts 10,969,
js 10,409, rust 4,306.

**(b) `imports` 0 of 136,532 resolved**, of which 25,692 are LOCAL and must
resolve (relative 15,924 · alias 8,618 · `crate::` 1,150). This also unblocks
#149: detection reads calls+imports+extends+references (`community.rs:185`) and
three of the four are 0%, which is why 97.8% of communities are single-file.

## Known-broken

`references` 251,185/0 (`doc_indexer.rs:584` and `:587` — two defects).
`extends` 7,901/0 (#147). `calls` 57% for this project. `graphHealth` on
`get_project_summary` now reports these live. `dojo_memberships.sync_status`
dead. `graph-end-state-sketch.md` §1–12 NOT SAFE TO BUILD FROM.

## Traps

`sensei_test` has NO automated schema provisioning — a DDL change must be applied
to BOTH DBs or the DB tests fail on a missing column. Never pipe a gate through
`| head`: SIGPIPE truncated a run and masked a real `fmt --check` failure.
`app/.../wizard-state.spec.svelte.ts` is not hermetic (real fetch to `:7744`).
Wipe needs the daemon STOPPED; `/health` not `/api/health`; a cold graph starves
its own rebuild (`queue.rs:479`).
