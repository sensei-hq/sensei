# Checkpoint — indexer v2

**State: DESIGN + SPECS COMPLETE, IMPLEMENTATION NOT STARTED.**

Read in this order:
1. `docs/plans/indexer-v2-sequence.md` — the master plan, 12 stages
2. `docs/spec/indexer/00-files-entity.md` — the next thing to build
3. `docs/design/indexer-v2.md` — the whole-system spec, when a stage cites it

`docs/plans/indexer-v2-rust.md` is SUPERSEDED — it inverts the DDL ordering
and builds demotion. Marked at the top of the file.

## Next command

    # stage 0 — ALL the DDL, one dbd pass. Pre-release project: reconcile,
    # NOT hand-written migrations. Verify no database/migrations/ first.
    cat docs/spec/indexer/00-files-entity.md

## Where the code is

| | |
|---|---|
| last CODE commit | `bc343622` — v2 steps 1-7 + reconcile |
| gate there | fmt 0, clippy 0, **3,292 tests / 0 failed** |
| safety property | `languages/` UNTOUCHED, v2 has NO CALLER, no DDL |

Everything since is documentation. The shipped indexer runs, unaffected.

## Known-wrong IN THE COMMITTED CODE — do not build on

1. **`demote_v2_symbol`** — keeps a node, nulls `file_path`, so `target_id`
   stays non-null and every consumer reads it as resolved. 67,839 such edges
   measured. Fixed by `07-reconcile.md` S3-S6 (delete + unresolve, ordered).
2. **`v2_edges_contributed_by`** narrows with
   `AND (s.fqn = ANY($3) OR s.resolved = false)` — an edge whose source this
   file deleted, resolving elsewhere, is never revisited. Fixed by S7.
3. **11 self-verifying column mappings**, 6 in `v2_symbol_unchanged`. Fixed as
   a class in `06-persist.md`.

## Open questions

- `library_content` shape — should `skill|agent|page|package` collapse into one
  table with a discriminator? **Decide BEFORE stage 2b populates
  `library_packages`**, or it becomes a migration of 146+ live rows.
- `sensei.libraries` currently holds PACKAGES (1,121 rows, all `detected`).
  What happens to those rows when the library level above them appears?

## Known-broken

- 8,147 ORPHANED nodes in the SHIPPED graph read as COMPLETE. Stage 0 sweeps
  them and the FK makes the state unrepresentable.
- `library_packages` has 0 rows, so 130 pages / 10 skills / 6 agents are
  unreachable from any node. Stage 2b.

## Measured facts the design rests on (live DB, verified this session)

| | |
|---|---:|
| `files` rows / indexed / skipped | 48,665 / 48,646 / 19 |
| nodes COMPLETE / PARTIAL / EXTERNAL / **ORPHANED** | 346,506 / 18,450 / 21,928 / **8,147** |
| repositories / folders with `repository_id` / projects | 68 / 69 / 147 |
| commands / folders / repositories | 572 / 58 / 36 |
| `library_packages` | **0 rows** |
| libraries with any URL | **2 of 1,121** |
| field + enum-variant nodes, any language | **0** (the enum HAS both values — the walk never emitted them) |

## The rules that produced this design

- Mint identity from what the CALL SITE can see. Broke twice before believed.
- A WRONG edge is worse than a MISSING one (R4).
- Absence is not evidence — it is scan-order dependent (R6).
- Defer WORK, never discard EVIDENCE (R11).
- Derived beats stored: derived state cannot drift from what it derives from.
- Structure before work: create the rows, then enqueue the tasks (R14).
- Measure against the live corpus; ASSERT against a fixture.
- **Inventory what exists before specifying it as new work** — the file entity,
  the library level, manifest extraction and gitignore handling were each
  specified as new and each already existed, wired.
