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

**The workflow is `dbd reconcile`.** This project is pre-release for dbd
purposes and has no `database/migrations/` tree. Edit the DDL under
`database/ddl/**` and reconcile; that is the whole procedure.

**Do NOT create a migrations tree.** Hand-written migrations are the workflow
for a RELEASED dbd project, and introducing one here would switch this project
onto a workflow it is not on — the single worst mistake available in a dbd
tree. If reconcile does not do what you want, the answer is different DDL, not
a different workflow.

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
- **S5** (§7h.4). Widen `sensei.node_kind`. **The current 23 values, listed so
  nobody has to go and look:**

      file | module | package | class | interface | function | method |
      property | field | parameter | type | const | enum | enum_variant |
      section | rationale | struct | component | hook | doc | extension |
      lib_symbol | lib_package

  **`field` and `enum_variant` ARE ALREADY THERE.** An earlier draft of this
  requirement said to add them, and the whole-system spec's "0 field and
  enum-variant nodes in any language" was read as an enum gap. It is not — the
  enum has supported them all along and **the WALK never emitted them**. That
  is a stage 4 problem (04-walk-rust S5) and no DDL fixes it.

  What is genuinely absent, for the Rust walk:

  | add | why | note |
  |---|---|---|
  | `trait` | Rust traits | `interface` exists and holds 7,131 rows from other languages — decide whether Rust maps onto it or gets its own value, and write the decision down |
  | `static` | `static X: T` | distinct from `const` (4,307 rows) |
  | `macro` | `macro_rules!` and proc macros | reach `macro` needs a declaration kind to point at |
  | `unknown` | a stub minted before its declaration is seen | see below |

  `unknown` is a NEW value, not a reused one. The `parameter` placeholder is
  WITHDRAWN (§2.1): `parameter` means something, and a value that means
  something else is not a value that means "not yet known". It is free only
  because D2 makes parameters props — that made it available, never correct.

  **REMOVE `lib_symbol` and `lib_package`** (D12). Kind says WHAT a node is;
  the fqn prefix says WHERE it is from. Measured, the redundancy is total:
  21,928 nodes carry a `lib_` kind, all 21,928 have a `lib·` fqn, and ZERO
  other kinds do. `lib_symbol` also destroys the real kind on 18,240 rows.
  - `lib_package` -> `package`
  - `lib_symbol` -> the real kind where the use site reveals it, else
    `unknown`
  - externality -> `fqn LIKE 'lib·%'`, the one discriminator. **No
    `is_external` column** — a column would store what is derivable.

  **The consumer-facing flag ALREADY EXISTS: `sensei.graph_nodes.locality`**
  (`internal | external | unknown`). Edit its `external` branch from the kind
  test to the fqn test. **KEEP IT THREE-VALUED** — the view's comment
  documents why at length: a boolean bins every unresolved reference stub as
  external, which is the bug it replaced, when the old edge-resolution proxy
  reported 791 of 1,040 of this repo's own modules as dependencies.

  Verified EXACT on the live graph — internal 354,653, external 21,928,
  unknown 18,450 under BOTH rules, with no off-diagonal cell. The swap cannot
  move a node between localities.

  Anything already querying `locality` needs no change; only writers and
  direct `kind IN (...)` readers do.

  **This is the largest single item in stage 0 and it is a CODE migration,
  not DDL: 111 references** across `db/pg_store/{tests,graph}.rs`,
  `tasks/handlers/process.rs`, `graph_facts.rs`, `languages/import_target.rs`,
  `indexer/{community,persist}.rs`, plus `nodes.ddl`, the `graph_nodes` and
  `edge_resolution_class` views, and a 21,928-row backfill. Budget for it
  separately. It is here rather than later only because the alternative is
  letting the v2 walk write `lib_symbol` and migrating a second time.

- **S6** (§7h.5). Check `sensei.edge_kind` before widening it. **Current 11
  values:**

      calls | implements | extends | imports | depends_on | traces_to |
      references | covers | rationale_for | duplicates | similar_to

  **MEASURED AND SETTLED: THE DIFF IS EMPTY. S6 IS A NO-OP.** Recorded here
  rather than merely skipped, because otherwise the next reader re-opens it.

  Derived from CODE, not from reading this spec:
  `indexer/persist.rs:575` (`reference_edge_kind`) and `:589`
  (`relation_edge_kind`) are TOTAL functions from the 6 `RefKind`s and 6
  `RelationKind`s onto `{calls, references, extends, implements}` — all four
  already exist. Every finer distinction survives per-occurrence in `props`:

  | R8 fact | representation | new kind? |
  |---|---|---|
  | field / param / return types | node props (D2) + `references`, `ref_kind = type_use` | no |
  | extends / implements | the existing values | no |
  | trait impl, mixin | `implements` + `relation_kind` in props | no |
  | construction | `references`, `ref_kind = constructs` | no |
  | member ownership | `references`, `relation_kind = owns` | no |
  | macro invocation | `calls`, `ref_kind = macro_invokes` | no |

  `calls` stays reserved for calls alone so `get_callers` keeps its meaning
  and a construction is not counted as a call.

  Live usage confirms nothing is starved: `calls` 410,678 · `references`
  253,305 · `imports` 141,984 · `implements` 2,317 · `extends` 1,932; the
  other six values carry 0 rows. **Do not add a value nothing writes.**

  Follow-up that is NOT DDL: those two `persist.rs` functions carry comments
  justifying the coarse mapping as deferred pending "step 9's decision". Step
  9 no longer exists and the mapping is now the correct design, so those
  comments read as a permanent open TODO. Re-word them when stage 6 is
  touched.
- **S7** (R10.1). `create index … using gin ((props->'occurrences'))` on
  `sensei.edges`. This is what lets reconcile ask the definitive attribution
  question (`props->'occurrences' ? F`) instead of an fqn approximation.
- **S7b** (02b S10, decided 2026-09-10). **Add `sensei.library_versions` and
  repoint library CONTENT at it**, so docs, skills and agents can be
  version-specific:

      library_versions (id, library_id -> libraries(id), version,
                        resolved_version, source_type, base_url, docs_url,
                        fetched_at, props)
        unique (library_id, version)

      library_pages.library_id  -> library_version_id
      library_skills.library_id -> library_version_id
      library_agents.library_id -> library_version_id

  **THE BACKFILL KEY IS `coalesce(nullif(version,''), 'latest')`, NOT
  `version`.** An earlier draft of this requirement cited "`version` is
  populated on 1,083 of 1,121 rows" as though that meant the migration was
  mostly covered. The statistic is TRUE and it points exactly BACKWARDS —
  measured, the CONTENT lives on the 38 unpopulated rows:

  | library | version | pages | skills | agents |
  |---|---|---:|---:|---:|
  | rokkit | **NULL** | 94 | 5 | 3 |
  | dbd | **NULL** | 36 | 1 | 1 |
  | kavach | 1.1.3 | 0 | 4 | 2 |

  Those are the ONLY three libraries with any content. Keying the backfill on
  the raw column orphans **140 of 146 content rows — including every single
  page.** `coalesce(...,'latest')` is required, and it is not a workaround:
  `latest` is what this spec already says the key may be, and a library whose
  version was never observed is honestly "latest" with `resolved_version`
  NULL. It cannot collide, because `libraries` is one row per library.

  Migration order: create `library_versions`, insert one row per library on
  the coalesced key, backfill the three content tables' FK, **assert zero
  orphans with a `RAISE EXCEPTION` gate**, THEN drop `libraries.version` and
  the old FKs. Same backfill-verify-constrain discipline as S3.

  Two blockers found by probing rather than reading, both in rolled-back
  transactions: `libraries.version` cannot be dropped while
  `project_libraries_resolved` selects it, so that view must be dropped and
  recreated explicitly — **never `DROP COLUMN ... CASCADE`**, which takes the
  view silently. And `library.rs:708` recomputes `page_count` via
  `library_pages.library_id`, which stops existing; whether `page_count` then
  means all versions or the current one is an open question to answer before
  editing it.

  `version` is the KEY and may be the literal `latest`; `resolved_version`
  records what `latest` actually meant at `fetched_at`.

  **`library_packages` is NOT repointed** — it stays keyed on `library_id`.
  The grouping is identity, and R10.7g's `node -> package -> library` chain
  starts from an fqn with no version in it, so a version-keyed grouping could
  not be walked from a node.

  **`referenced_libraries.version_used` stays TEXT.** A folder can pin a
  version whose docs were never fetched; an FK would block that or force a
  phantom row — the get-or-create failure R13 forbids one table over.

  Content is small (130 pages, 10 skills, 6 agents), so this migration is
  cheap. Do it now rather than after stage 2b starts writing.
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

**Define the set FIRST, and MATERIALISE it into a TEMP TABLE.**

    CREATE TEMP TABLE orphans ON COMMIT DROP AS
    SELECT n.id, n.name
      FROM sensei.nodes n
      LEFT JOIN sensei.files f
             ON f.folder_id = n.folder_id AND f.file_path = n.file_path
     WHERE n.file_path IS NOT NULL
       AND f.file_path IS NULL;
    -- expect 8,147

**A TEMP TABLE, not a view.** Two reasons, and neither is drift — a view would
return the same rows on each reference, since nothing between steps 1 and 2
modifies `nodes` or `files`:

1. **A view is EMPTY the moment step 3 runs**, because it selects from
   `nodes`. It cannot carry `name` past the delete, and `name` is exactly what
   step 1 backfills from and step 3 destroys. A materialised set holds both
   sides of that ordering.
2. **A view is a schema object dbd would then own** — a file under `ddl/view/`
   and a permanent artifact for a one-time sweep.

`ON COMMIT DROP` is correct here: the table's whole job is to hold one
identical set across the three statements inside the transaction.

**Verification does NOT depend on it, and must not.** A temp table is invisible
from another session, so checking the sweep afterwards happens against the
DURABLE tables — `nodes` down by exactly 8,147, and 212 `edges` rows now
`target_id IS NULL` with a non-null `target_name`. Both queryable from any
psql session, which is the point. The irreversible-change record is the backup
taken beforehand, not a leftover temp table.

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
| `dbd reconcile` will not express the change | change the DDL. **Do not reach for a migrations tree** — that is the released-project workflow and this project is not on it. |
| `dbd reconcile` fails partway | do not hand-patch the live DB to match. Fix the DDL and re-run; a hand-patched DB drifts from the tree and the next reconcile fights it. |
| an orphan's inbound edge has NULL `target_name` | **MEASURED: all 212 are.** Backfill from the target node's `name` (all 212 have one) before unresolving — S9 step 1. Do NOT stop, and do NOT unresolve without it: that yields an edge naming nothing. |
| a target node has NO name either | THEN stop and report. Nothing can recover the reference, and an edge with no target and no name is a dead row, not a gap. Measured 0 today. |
| an orphan is an edge SOURCE | measured 0. Assert it; if it is ever non-zero the `source_id` cascade takes those edges and the sweep needs a fourth clause. |
| an orphan has children | fold them into the orphan set and re-run step 2. Never let the cascade take them. |
| `node_kind` widening rejected | an enum value is in use somewhere the ALTER cannot see. Find it; do not work around it with a placeholder. |
| the live DB and the DDL tree already disagree before starting | resolve that FIRST. Reconciling on top of unexplained drift attributes someone else's change to this stage. |

## 5. Verification

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
| `node_kind` accepts `trait`, `static`, `macro`, `unknown` | revert S5's additions |
| `node_kind` REJECTS `lib_symbol` and `lib_package` | leave them in the enum |
| every former `lib_package` row is `package`; every former `lib_symbol` row has a real kind or `unknown`; all 21,928 still have a `lib·` fqn | backfill the kind without checking the fqn survived |
| "which symbols are external" returns 21,928 via `fqn LIKE 'lib·%'`, matching the pre-migration `kind IN (...)` count exactly | drop a row in the backfill — the counts diverge and nothing else notices |
| `graph_nodes.locality` still returns THREE values, and the per-value counts are unchanged: internal 354,653 / external 21,928 / unknown 18,450 | collapse `locality` to a boolean — every stub becomes `external` and the 791-of-1,040 false-positive bug returns, while every "is it external" query still answers |
| no `is_external` column was added to `nodes` | add one |
| `EXPLAIN` on `props->'occurrences' ? $1` uses the gin index | drop it (S7) |
| `folder_commands` has 572 rows across 58 folders | revert S8 |
| `dbd doctor` is clean | any of the above |

The edge-count assertion is the one that matters most and is the easiest to
omit: node counts alone cannot distinguish "the sweep worked" from "the sweep
worked and took 3,070 edges with it".

## 6. Watch out

**The sweep is irreversible and runs against the live daemon DB.** Take a
backup first (`database/backup/` is the existing home) and record the row counts
before the transaction, not after. This is the only stage in
the whole plan that destroys production rows.

**`dbd reconcile`, and do not invent a migrations tree to get around a
stubborn change.** The reconcile-vs-migrations choice is driven by the
project's release state, not by convenience, and this project is pre-release.
Creating `database/migrations/` would silently move it onto the other
workflow.

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

## 7. Definition of done

- All eight DDL changes applied through `dbd`, tree and live DB in agreement,
  `dbd doctor` clean.
- The ORPHANED sweep ran in one transaction; `nodes_after` and the edge count
  both reconcile exactly, verified by querying `nodes` and `edges` after.
- ORPHANED is now unrepresentable — the FK rejects it — so it is dropped from
  the completeness enum, leaving FIVE values (R10.7d), and from the R11.3 gap
  queue.
- `cargo build --workspace` passes with `scan_state` and `nodes.file_path` gone.
- No Rust file under `crates/senseid/src/indexer/` was modified.
