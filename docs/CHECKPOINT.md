# Checkpoint — indexer v2

**State: STAGES 0–3 LANDED AND RUN. Folders and files are in the DB and have
been inspected. Lockfile pins and the library SHAPE are done; library
POPULATION (2b S1–S8) is next, then the parse half of v1 retirement.**

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
| 2 S6b/S6c/S6d — lockfile readers | `674af40b` |
| 2b R12/S10 — `library_content` collapse | `52f9f0af` |
| 3 S4b/S4c — derived kind, `deferred` dropped | `4cc905f1` |
| 3 — one `module` kind + `workspace_root_id` | `be3faff0` |
| 3 — `sibling` dropped | `5dd169c7` |
| **2b S1/S2 — libraries POPULATED** | `76e24bab` |
| **2b S7b — local pages + staleness signal** | `48db4934` |
| **2b S8 — registry URLs** | this commit |
| 3 S4b/S4c — derived folder `kind`, `deferred` dropped | this commit |
| 2b — `library_content.package_name` | this commit |

Measured against this repo and written to `sensei`: **1 repository, 18
folders, 2,305 files.** Cold 3.0s, warm 0.44s — the mtime gate skips all 2,305
sha256 reads on a re-run, which changes no row. `folder_completeness` reports
expected=731 / decided=0 / incomplete, because nothing is parsed yet.

Lockfiles: **128 of 219 direct external deps get a corrected version** —
`serde = "1"` cleans to `1` and resolves to `1.0.228`. Readers for
`Cargo.lock`, `bun.lock`, `package-lock.json`; `yarn.lock`/`pnpm-lock.yaml`
deliberately unclaimed (they fall back to the manifest range, a named gap).

Libraries: POPULATED. Scanning `~/Developer` (129 repos, 510 folders, 56,899
files, 467 manifests) finds 3 libraries and writes **30 `library_packages`
rows and 16 `library_content` rows** — rokkit 14 packages / 5 skills / 3
agents, kavach 14 / 4 / 2, dbd 2 / 1 / 1. The G1 chain answers:

    @rokkit/ui -> rokkit -> its 5 skills and 3 agents

which was unanswerable before. `library_packages` had ZERO rows.

Pages are in: **152** across the three (rokkit 94, dbd 43, kavach 15). The
acceptance test in 02b §5 passes —

    @rokkit/ui -> rokkit -> the `list` page, with its content

`library_content.package_name` still has no writer: neither the manifest nor
the local walk states which package a page documents, and inferring it from
the component name would be the R4 guess.

Two schema-tool limitations found and worked around, worth remembering:
`dbd reconcile` does not drop a table whose DDL file was deleted, does not
remove an enum VALUE (it reports the drift in `dbd diff` but will not apply
it — the type must be recreated by hand), and silently reduces an inline
`unique nulls not distinct (...)` to a plain unique.

## Remaining

- **2b S9 — source precedence.** S1/S2/S7b/S8 are done. What remains: pick
  the highest-precedence source THAT CAN SERVE THE PINNED VERSION
  (website > github > local, inverted when the version does not match), and
  LABEL any doc served for a version the project does not pin. An unlabelled
  wrong-version answer is the R4 failure.
- **The github and website routes** are now unblocked (S8 supplies the URL)
  but not built — `index_library` still needs a task carrying one.
- **`referenced_libraries` has no v2 writer.** Dependency detection is not
  wired into the v2 scan, so no folder → library edges exist, and
  `library_update_scheduler` (the only caller of the S8 path) has nothing to
  tick over. The extraction is tested live; it is not yet exercised end to end
  by a scan.
- **4–7** — parse, fqn, persist, reconcile. This IS the rest of v1 retirement.
- **8–10** — commands, incremental, cutover.

## Next command

    cat docs/spec/indexer/02b-library-discovery.md

    # re-run the structure write (defaults to sensei_test):
    TEST_DATABASE_URL="postgresql://localhost:5432/sensei" \
      cargo test -p senseid --bin senseid indexer::pipeline::corpus \
      -- --ignored --nocapture
    # SENSEI_SCAN_DIR=~/Developer scans every repo, not just this one.

## Known-broken

**senseid: 2,821 passing / 167 failing** (was 192). Every remaining failure is
one of stage 0's two drops:
- `nodes.file_path`, replaced by `file_id` (R13) — v1's `graph.rs` still
  references it;
- `node_kind`'s removed `lib_symbol` / `lib_package` values (D12).

Both are stages 4–7 — the retirement itself, not a regression. Stages 1–3 and
the whole library layer are green.

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

- ~~`library_content` shape~~ — DECIDED 2026-09-11: collapse into one table
  with a `kind` discriminator. `library_packages` stays out: it is a grouping,
  not content, and R10.7g resolves it from a version-less fqn.
- ~~The 1,121 package-level `libraries` rows~~ — moot: stage 0's wipe emptied
  the table. `libraries` is now identity-only and rebuilds as such.
- ~~Every non-root folder is written `kind = 'workspace_member'`~~ — RESOLVED
  2026-09-11: added the `package` enum value and DERIVED the distinction from
  declared workspace membership. This repo now reads 8 `workspace_member`
  (the crates the root Cargo.toml declares) and 9 `package`.
- ~~`folder_status.deferred`~~ — REMOVED 2026-09-11. v2 stores no folder it
  does not index, so the value described an unreachable state; it had no
  writer anywhere in the tree.
- ~~`folder_kind.sibling`~~ — REMOVED 2026-09-11, same reasoning: v2's
  `folders` holds repo roots and manifest-bearing modules, so a non-git
  sibling never gets a row. Zero writers; zero rows in any database.
- **`folder_kind.standalone` is next, but NOT yet.** v1 scan paths still write
  it (`scan.rs` x4, `project_detail.rs`). It is dead in v2's model and goes
  when those paths do, in stages 4-7. Dropping it today breaks running code.

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
