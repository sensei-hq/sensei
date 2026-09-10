# Checkpoint — indexer v2

**State: STAGES 0–3 LANDED AND RUN. Folders and files are in the DB and have
been inspected. Stage 2b (libraries) is next, then v1 retirement.**

Read in this order:
1. `docs/plans/indexer-v2-sequence.md` — the master plan, 12 stages
2. `docs/spec/indexer/02b-library-discovery.md` — the next thing to build
3. `docs/design/indexer-v2.md` — the whole-system spec, when a stage cites it

`docs/plans/indexer-v2-rust.md` is SUPERSEDED — it inverts the DDL ordering
and builds demotion. Marked at the top of the file.

## Slice

Retire v1, implement v2. Stage 0 (all DDL) is applied to `sensei`,
`sensei_test` and `sensei_e2e` — the three are in sync. Pre-release project:
`dbd reconcile`, never hand-written migrations.

## Done

| stage | commit |
|---|---|
| 0 — all DDL, code graph wiped | `9e6be200` |
| 1 — scan root | `2a2b15c2` |
| 2 — scan repo (pure, then walk) | `45ffd35f`, `1991d258` |
| 3 — structure planner | `ffa27e06` |
| 1–3 IO half, run against this repo | `a969758f` |

Measured against this repo and written to `sensei`: **1 repository, 18
folders, 2,305 files.** Cold 3.0s, warm 0.44s — the mtime gate skips all 2,305
sha256 reads on a re-run, which changes no row. `folder_completeness` reports
expected=731 / decided=0 / incomplete, because nothing is parsed yet.

## Remaining

- **2b — libraries.** The inspection checkpoint is folders + files +
  libraries; the first two are done.
- **4–7** — parse, fqn, persist, reconcile. This IS the v1 retirement.
- **8–10** — commands, incremental, cutover.

## Next command

    cat docs/spec/indexer/02b-library-discovery.md

    # re-run the structure write (defaults to sensei_test):
    TEST_DATABASE_URL="postgresql://localhost:5432/sensei" \
      cargo test -p senseid --bin senseid indexer::pipeline::corpus \
      -- --ignored --nocapture
    # SENSEI_SCAN_DIR=~/Developer scans every repo, not just this one.

## Known-broken

**33 `indexer::reconcile` tests, plus the wider v1 suite, fail on
`column "file_path" does not exist`.** Expected: stage 0 dropped
`nodes.file_path` for `file_id`, and v1's `graph.rs` store functions still
reference it. Stages 4–7 resolve this — it is the retirement itself, not a
regression. Stages 1–3 are green (37 tests).

## Known-wrong IN THE COMMITTED v1 CODE — do not build on

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
- `sensei.libraries` holds 1,121 PACKAGE rows, all `detected`. What happens to
  them when the library level above them appears.
- Every non-root folder is written `kind = 'workspace_member'`. The enum has no
  value for "holds a manifest but is not a declared member" — `marketplace/`
  and two fixtures under `crates/senseid/tests/fixtures/` are in that class,
  and `standalone` means non-git, so it does not fit. Asserted, not derived.

## Measured facts the design rests on

| | |
|---|---:|
| this repo: repositories / folders / files | 1 / 18 / 2,305 |
| pre-wipe `files` rows / indexed / skipped | 48,665 / 48,646 / 19 |
| pre-wipe nodes COMPLETE / PARTIAL / EXTERNAL / **ORPHANED** | 346,506 / 18,450 / 21,928 / **8,147** |
| repositories kept through the wipe | 68 |
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
- **A green test may be green for the wrong reason.** `folder_completeness`
  had a passing test while its `decided` predicate was a tautology; only
  looking at real data exposed it.
