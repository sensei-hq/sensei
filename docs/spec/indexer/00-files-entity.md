# Stage 0 — the files entity, and ALL the DDL

Whole-system spec: `docs/design/indexer-v2.md` §7h (the DDL ruling), R13 (the
files entity), R10.7/R10.9 (the detail column), R10.7d (ORPHANED), D10, D11.
Where this document and that one disagree, that one wins and this is a bug.

## 1. Purpose

Open the schema ONCE, before any v2 code exists, so that no later stage has to
work around a column it knows is coming. Seven changes, one `dbd` pass.

Serves both goals indirectly and one directly: after this stage the 8,147
ORPHANED nodes — which today assert "declared in x.rs" about files the scanner
does not track, and read as COMPLETE to every consumer — are not merely swept
but UNREPRESENTABLE. That is a G2 correctness fix that needs no v2 code at all.

This stage writes NO Rust. If you find yourself editing `crates/`, you are in
the wrong stage.

## 2. Inputs and outputs

| | |
|---|---|
| input | `database/ddl/**` as it stands, and the live `sensei` database |
| output | the same tree with seven changes applied through `dbd`, and a live schema matching it |
| purity | **entirely IO.** There is no pure part of this stage. |

The project is PRE-RELEASE for dbd purposes — there is no `database/migrations/`
tree — so the workflow is `dbd reconcile`, NOT hand-written migrations. Verify
that before starting: if a `migrations/` directory has appeared since this was
written, STOP, because the correct workflow has changed and reconcile would be
the wrong tool.

## 3. Requirements

Each cites its whole-system requirement. S1–S3 are one logical change and must
land together; splitting them migrates the same table twice.

- **S1** (R13). Rename `sensei.scan_state` -> `sensei.files`. Preserve
  `(folder_id, file_path)` as a UNIQUE constraint — the walk only ever has a
  path, and dropping it makes the walk's once-per-file lookup a seq scan.
- **S2** (R13). Add `files.id uuid primary key default gen_random_uuid()`.
- **S3** (R13). Replace `nodes.file_path text` with `nodes.file_id uuid`
  references `files(id)`. Nullable — PARTIAL and EXTERNAL nodes carry NULL by
  definition (R10.7d), which is not the same as a missing lookup.
- **S4** (R10.7, R10.9, A9). Add a nullable detail column to `files` for the
  parser's verbatim message with line and column. `skip_reason` already carries
  the code (`parse_error` is an existing enum value, effectively unused); this
  carries the text. It goes on `files`, not on a node's props — splitting the
  code and the detail across two tables recreates the two-copies-of-one-fact
  problem R10.9 exists to avoid.
- **S5** (§7h.4). Widen `sensei.node_kind` for the kinds the walk actually
  produces — at minimum `field` and `variant` — plus one value meaning NOT YET
  KNOWN, for a stub minted before its declaration is seen. This retires the
  `parameter` placeholder, which is WITHDRAWN (§2.1).
- **S6** (§7h.5). Widen `sensei.edge_kind` for the relation kinds R8 needs.
  Enumerate them from R8's seven patterns before writing the ALTER; do not
  guess and do not widen twice.
- **S7** (R10.1). `create index … using gin ((props->'occurrences'))` on
  `sensei.edges`. This is what lets reconcile ask the definitive attribution
  question (`props->'occurrences' ? F`) instead of an fqn approximation.
- **S8** (D11). Regrain the manifest-derived tables to FOLDER, because a
  manifest sits at a folder and every fact read out of it is folder-grained:
  - `project_commands` -> `folder_commands`. Rename only; already keyed
    `folder_id`. 572 rows across 58 folders / 36 repos.
  - `project_dependencies` -> `folder_dependencies`, keyed on the FOLDER pair.
    `from_folder_id` already exists; the two project ids are pre-aggregation
    in the primary key and are derivable. **0 rows, so this is free** — do it
    now, not after something populates it.
  - `project_libraries` -> `library_enablement`. Rename only, **not
    regrained** — it is a user toggle with a global scope (`project_id NULL`),
    not an observation. `project_id` stays as the scope column.
  - `referenced_libraries` — **LEAVE ALONE.** It is already the folder-grained
    library fact (1,016 rows, 49 folders, `version_used` observed from the
    manifest). Do not create a `folder_libraries`; it exists under this name.
- **S10** (D11). Add repository-level and project-level VIEWS over the
  folder-grained bases — dependencies and commands at minimum. Follow
  `sensei.folder_completeness`, which already recurses `parent_id`. Views, not
  tables: an aggregate that is stored can disagree with the manifest it came
  from, and a view cannot.

### EXECUTION ORDER — S1..S10 are NOT a checklist

They are order-dependent, and the table changes name partway through. Run them
in exactly this order and use the name that is live at each step:

| # | step | the file table is called |
|---|---|---|
| 1 | S1 rename, S2 add `id` | `scan_state` -> **`files`** |
| 2 | **S9 the ORPHANED sweep** | **`files`** (already renamed) |
| 3 | S3 add `nodes.file_id`, backfill from `file_path`, verify 0 nulls where a path existed, THEN add the FK and drop `file_path` | `files` |
| 4 | S4–S8 detail column, enum widening, gin index, the D11 renames | `files` |
| 5 | S10 the rollup views | `files` |

The sweep sits between the rename and the foreign key because it must run
against the renamed table and the FK cannot be added while an orphan exists.
Backfill-then-verify-then-constrain, never constrain-then-backfill: adding the
FK first fails on the first orphan and tells you nothing about the other 8,146.

### S9 — the ORPHANED sweep, and it is the dangerous one

8,147 nodes name a `file_path` for which there is no row in the file table.
**That table is `files` by the time this runs** — the count was MEASURED
against `scan_state` before the rename, and the population is identical; only
the name changed. S3's foreign key cannot be added while they exist.

**Define the set FIRST, and MATERIALISE it.** All the steps below must operate
on one identical set; re-evaluating the predicate per statement is how they
drift apart.

    CREATE TEMP TABLE orphans ON COMMIT DROP AS
    SELECT n.id, n.name
      FROM sensei.nodes n
      LEFT JOIN sensei.files f
             ON f.folder_id = n.folder_id AND f.file_path = n.file_path
     WHERE n.file_path IS NOT NULL
       AND f.file_path IS NULL;
    -- expect 8,147

`n.name` is selected deliberately — step 1 needs it, and after step 3 it is
gone.

Then, **in ONE transaction, in this order** — R10.8's rule, executed by hand
here for the first time:

1. **Backfill `target_name`, THEN unresolve.**

        UPDATE sensei.edges e SET target_name = o.name
          FROM orphans o WHERE e.target_id = o.id AND e.target_name IS NULL;
        UPDATE sensei.edges SET target_id = NULL
         WHERE target_id IN (SELECT id FROM orphans);

   **MEASURED, and this is why the order matters: all 212 inbound edges have
   `target_name IS NULL`, and all 212 target nodes HAVE a name** (23 distinct:
   `app`, `openapi`, `electron`, `docs`, …). Unresolving without the backfill
   produces an edge pointing at nothing AND naming nothing — not a recorded
   gap with a fill path (R11), just a dead row. The name is recoverable only
   from the node, and only before step 3.

2. **Assert the set is closed over `parent_id`.**

        SELECT count(*) FROM sensei.nodes c
         WHERE c.parent_id IN (SELECT id FROM orphans)
           AND c.id NOT IN (SELECT id FROM orphans);   -- must be 0

   **Measured: 0.** The base predicate is already transitively closed, because
   a member shares its parent's `file_path`. Keep the assertion anyway — it is
   the guard for a child with a NULL or differing `file_path` whose parent is
   an orphan, which the cascade would silently take. If it is ever non-zero,
   the set needs a recursive CTE over `parent_id`, not a manual top-up.

3. `DELETE FROM sensei.nodes WHERE id IN (SELECT id FROM orphans);`

**No source-side concern: 0 orphans are edge SOURCES** (measured), so the
`source_id` cascade has nothing to take. Assert it rather than assume it — the
number is only zero today.

### What the 212 actually are, and one thing they expose

All 212 are `kind = 'references'` from MARKDOWN files (`docs/backlog.md`,
`openspec/specs/openapi/summary.md`) to orphaned `module` nodes named after
directories. Every source node exists and none is itself an orphan.

Worth noting and NOT fixing here: **those markdown source nodes have an EMPTY
fqn.** Under §2 a file's identity is the module it declares, and a markdown
file declares none — so what a `.md` file's node identity should be, or
whether it should have one, is an open question for the walk. Record it; do
not answer it in a DDL stage.

## 4. Failure modes

| input | this stage does |
|---|---|
| a `migrations/` tree exists | STOP. The workflow is wrong, not the DDL. |
| `dbd reconcile` fails partway | do not hand-patch the live DB to match. Fix the DDL and re-run; a hand-patched DB drifts from the tree and the next reconcile fights it. |
| an orphan's inbound edge has NULL `target_name` | **MEASURED: all 212 are.** Backfill from the target node's `name` (all 212 have one) before unresolving — S9 step 1. Do NOT stop, and do NOT unresolve without it: that yields an edge naming nothing. |
| a target node has NO name either | THEN stop and report. Nothing can recover the reference, and an edge with no target and no name is a dead row, not a gap. Measured 0 today. |
| an orphan is an edge SOURCE | measured 0. Assert it; if it is ever non-zero the `source_id` cascade takes those edges and the sweep needs a fourth clause. |
| an orphan has children | fold them into the orphan set and re-run step 2. Never let the cascade take them. |
| `node_kind` widening rejected | an enum value is in use somewhere the ALTER cannot see. Find it; do not work around it with a placeholder. |
| the live DB and the DDL tree already disagree before starting | resolve that FIRST. Reconciling on top of unexplained drift attributes someone else's change to this stage. |

## 5. Checkpoint output

One JSON line appended to `~/.sensei/scan-progress.jsonl`:

    {"stage":"00-files-entity","at":"<iso8601>",
     "files_rows":48665,"files_indexed":48646,"files_skipped":19,
     "nodes_before":395031,"orphans_swept":8147,"nodes_after":386884,
     "edges_before":82913,"edges_after":82913,
     "target_names_backfilled":212,"edges_unresolved_by_sweep":212,
     "orphan_children_outside_set":0,"orphans_as_edge_source":0,
     "nodes_with_file_id":346506,"nodes_file_id_null":40378,
     "ddl":{"s1":true,"s2":true,"s3":true,"s4":true,
            "s5":true,"s6":true,"s7":true,"s8":true},
     "sample_orphan":{"fqn":"…","name":"…","file_path":"…",
                      "inbound_edges":3,"target_names_present":3}}

The sample is not decoration. A count of 8,147 swept rows tells you the DELETE
ran; one rendered orphan with its inbound-edge names tells you it swept the
right population. `nodes_after` must equal `nodes_before - orphans_swept`
exactly — any other number means something cascaded.

## 6. Verification

Every check names the one-line mutation that must break it.

| check | mutation that must break it |
|---|---|
| `files` exists, `scan_state` does not | revert S1 |
| a node's file is reachable by FK join, and `nodes.file_path` no longer exists | revert S3 |
| inserting a node with a `file_id` naming no `files` row is REJECTED | drop the FK |
| after the sweep, `count(nodes) = before - 8147` **and `count(edges)` is UNCHANGED** | remove S9 step 1's unresolve — the cascade takes the 212 inbound edges, and the NODE count still matches while the edge count silently drops by 212. The node count alone cannot tell "the sweep worked" from "the sweep worked and took 212 edges with it". |
| every edge whose target was swept has `target_id IS NULL` **and a non-null `target_name`** | remove the backfill from S9 step 1 — 212 edges then survive as `target_id NULL, target_name NULL`, which passes a naive "edges still exist" check while carrying no information |
| the materialised `orphans` set is used by all three steps, not re-derived per statement | inline the predicate into each statement |
| the parse-detail column accepts and returns a multi-line parser message verbatim | truncate it to `varchar(80)` |
| `node_kind` accepts `field` and `variant` | revert S5 |
| `EXPLAIN` on `props->'occurrences' ? $1` uses the gin index | drop it (S7) |
| `folder_commands` has 572 rows across 58 folders | revert S8 |
| `dbd doctor` is clean | any of the above |

The edge-count assertion is the one that matters most and is the easiest to
omit: node counts alone cannot distinguish "the sweep worked" from "the sweep
worked and took 3,070 edges with it".

## 7. Watch out

**The sweep is irreversible and runs against the live daemon DB.** Take a
backup first (`database/backup/` is the existing home) and record the row counts
in the checkpoint BEFORE the transaction, not after. This is the only stage in
the whole plan that destroys production rows.

**`dbd reconcile`, not migrations** — verified pre-release at the time of
writing (no `database/migrations/`). Running reconcile against a released
project is the single worst mistake available in a dbd tree.

**134 references across 10 Rust files** name `scan_state` or `nodes.file_path`,
plus `design.dbml` and the `folder_completeness` view. They are mechanical but
they are not optional — the tree must compile at the end of this stage even
though no v2 code has been written. Measured, bounded, and the reason S1–S3
land together.

**Do not add `ON DELETE CASCADE` to `nodes.file_id`.** It is tempting and it is
wrong: deleting a file row would then silently delete every declaration in it,
which is exactly the uncounted mass deletion R10.2's measurement warns about.
A file's removal goes through reconcile (R10.8), one declaration at a time,
with inbound edges unresolved. `ON DELETE RESTRICT` is the honest choice.

## 8. Definition of done

- All eight DDL changes applied through `dbd`, tree and live DB in agreement,
  `dbd doctor` clean.
- The ORPHANED sweep ran in one transaction; `nodes_after` and the edge count
  both reconcile exactly; the checkpoint line carries a rendered sample.
- ORPHANED is now unrepresentable — the FK rejects it — so it is dropped from
  the completeness enum, leaving FIVE values (R10.7d), and from the R11.3 gap
  queue.
- `cargo build --workspace` passes with `scan_state` and `nodes.file_path` gone.
- No Rust file under `crates/senseid/src/indexer/` was modified.
