# Checkpoint — indexer v2

**State: DESIGN COMPLETE, IMPLEMENTATION NOT STARTED.**
Read `docs/design/indexer-v2.md` (spec) then `docs/plans/indexer-v2-rust.md`
(build + verification). They are self-contained; this file is orientation only.

## Where the code is

| | |
|---|---|
| last code commit | `bc343622` — v2 steps 1-7 + reconcile |
| gate there | fmt 0, clippy 0, **3,292 tests / 0 failed** |
| safety property | `languages/` UNTOUCHED, v2 has NO CALLER, no DDL |

The shipped indexer still runs and is unaffected. Everything after `bc343622`
is documentation.

## Known-wrong IN THE COMMITTED CODE — superseded by the spec, do not build on

1. **DEMOTION** (`demote_v2_symbol`). Keeps a node and nulls `file_path`, so an
   edge's `target_id` points at a row naming nothing and any consumer reading
   `target_id IS NOT NULL` calls it resolved. 67,839 such edges measured in
   `sensei_test`. Replaced by R10.7 (dirty) + R10.8 (delete-and-unresolve).
2. **`v2_edges_contributed_by`** narrows with
   `AND (s.fqn = ANY($current) OR s.resolved = false)`, so an edge whose source
   this file deleted, and which resolves elsewhere, is never revisited and its
   stale occurrence survives forever. R10.10: the occurrence key is the ONLY
   attribution unit. The narrowing is not needed — 19.7ms scoped by folder
   against the largest folder here (330,437 edges).
3. **11 unverified column mappings**, 6 in `v2_symbol_unchanged`. The round trip
   reads back props the same writer wrote, so column values verify against
   themselves. Fix the class (read the COLUMN), not the fields.

## Start here

**R13 first.** Rename `scan_state` -> `files`, add an `id`, point
`nodes.file_id` at it. It is DDL through dbd, and everything else assumes it —
including the fix for the 8,147 ORPHANED nodes, which the foreign key makes
UNREPRESENTABLE rather than merely detectable.

Then the pipeline top-down: `scan_root` -> `scan_repo` -> `index_file`, per
`docs/plans/indexer-v2-rust.md`.

## Measured facts the design rests on (live graph, 48,654 files)

| | |
|---|---:|
| imports resolved / unresolved | 141,980 / **4** |
| rust calls: real resolved / unresolved / ghost | 46,545 / 29,875 / 4,542 |
| typescript calls resolved / unresolved | 104,634 / 64,050 |
| nodes: COMPLETE / PARTIAL / EXTERNAL / **ORPHANED** | 346,506 / 18,450 / 21,928 / **8,147** |
| `library_packages` (the grouping) | **0 rows** |
| library pages / components / skills / agents | 130 / 128 / 10 / 6 |
| field + enum-variant nodes, any language | **0** |

## The rules that produced this design

- Mint identity from what the CALL SITE can see; if the two sides mint different
  strings they never merge. Broke twice before it was believed.
- A WRONG edge is worse than a MISSING one (R4).
- Absence is not evidence — it is scan-order dependent.
- Defer WORK, never discard EVIDENCE (R11).
- Derived beats stored: derived state cannot drift from what it derives from.
- Structure before work: create the rows, then enqueue the tasks (R14).
- Measure against the live corpus; ASSERT against a fixture.

## Open, not blocking

- `library_packages` is empty, so node -> package -> library -> skills/agents
  stops one link short. Needs `sensei.library.json` ingestion or workspace
  members. Not v2 work.
- 8,147 ORPHANED nodes exist in the SHIPPED graph today and read as complete.
  Partly created by `clear_scan_state_for_root`, which every forced reindex
  calls. Not v2 work; R13 prevents recurrence.
- A consistency review of the spec and plan was run at checkpoint time; fold its
  findings in before implementing.
