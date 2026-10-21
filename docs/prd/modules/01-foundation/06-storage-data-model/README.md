# Storage & Data Model · M6

**Plane:** foundation · **Status:** [roadmap/01-foundation.md](../../../../roadmap/01-foundation.md) ·
**Serves:** every module — the schema is the substrate the whole core loop turns on

## Purpose

One PostgreSQL database, `sensei`, on the standard PostgreSQL port (5432 — the *daemon*
listens on 7744; `crates/bootstrap/src/config.rs`: `POSTGRES_PORT = 5432`,
`DAEMON_PORT = 7744`). The DDL under `database/ddl/` — organised
by object type (`enum/`, `table/`, `function/`, `procedure/`, `view/`) then by schema — is
the declarative source of truth, applied with **dbd**. The daemon owns runtime migrations.
Five local schemas (plus the cloud `dojo` schema on Supabase) separate the knowledge core
(`sensei`), captured behaviour (`activity`), learned output (`inference`), LLM-routing
config (`gateway`), and the structured-log sink (`public`). The user never sees this module
directly — they feel it as *"settings survive wipes, scans don't double, and nothing I
learned ever vanishes"*.

## In scope / out of scope

- In: the DDL source (full-definition per object, no ALTERs); dbd apply/deploy/reconcile
  flow; staging tables + timestamp-guarded seed imports; the one-owner invariant (one file
  = one folder = one owner); bridge-table partial-unique-index scoping pattern; enum
  alphabetical-deploy rule; pgvector (HNSW) semantic-search shape; retention.
- Out: scan/indexing/capture semantics (M3/M4 — they *write* to these tables); dojo cloud
  schema authoring (T1, separate service); the gateway's chain semantics (M5 — `gateway.*`
  tables store its config).

## Surfaces

- **Daemon** — boot auto-apply reads the *released* DDL bundle (`database@vVERSION`);
  `SENSEI_DDL_DIR` for local DDL; `make bump` to promote.
- **CLI** — `dbd deploy` / `apply` / `graph` / `reconcile`; seeding via import procedures.
- **Tests/CI** — `check-sql-in-layer`, `check-grant-targets`, `check-ddl-comments`,
  `check-brew-tokens` run in the repo gate; schema round-trip tests in `crates/senseid`.

## Data invariants

- **DDL-source-first**: edit the `.ddl` first, then apply — otherwise boot auto-apply
  recreates the old object from the bundle.
- Full-DDL, no ALTERs in source; migrations are dbd's job. Pre-release additive columns go
  through `dbd reconcile` (incremental); `deploy` is a declarative snapshot and won't
  ALTER-add.
- Enum variants deploy **alphabetically** — never rely on declaration order; rank with a CASE.
- **One-owner invariant**: every file is owned by exactly one folder (repo/git-root);
  structural subfolders (`kind='folder'`) are members and own no code nodes — enforced at
  scan-classification + `dedup_structural_folder_nodes` reconcile.
- **Config vs content separated**: user intent (`folders_to_watch` etc.) survives a content
  wipe; discovered FS state (`folders`) is fully re-derivable by a re-scan.
- `hook_events` PK is `bigserial`, not uuid — append-only high-write stream, sequential
  B-tree friendly, 8 vs 16 bytes.
- **Bridge-table scoping** uses partial unique indexes, not composite PKs: `project_id IS
  NULL` = global, `= X` = scoped, no row = inactive; `*_resolved` views tag `scope` and sort
  workspace rows above global (reused across libraries/extensions/instruments).
- Semantic search is a single SQL query: pgvector cosine (`nodes.embedding vector(384)`,
  `library_pages.embedding vector(768)`, HNSW indexes) + relational filters.
- Seeding is idempotent and guarded: staging tables + import procedures with a timestamp
  guard so a reseed never overwrites production data.

## Signals shown

| Element | Value | Meaning |
|---|---|---|
| Schema drift | dbd reconcile/deploy output | the schema is the source of truth; drift fails loudly |
| `data.md` "Where the gaps are" | orphaned/empty tables | honesty surface (e.g. no writer on `inference.*`) |
| Gate checks | check-* CI lines | layer rules: no SQL outside `db/pg_store/`, grants owned by the schema, comments well-formed |

## Done gate

- Every schema fact survives a round-trip (`every_schema_fact_survives_the_round_trip`);
  the dbd design lands detail on the walked nodes (`a_dbd_design_lands_its_detail_on_the_walked_nodes`).
- A schema-scoped write cannot read or leak another workspace's schema (`a_project_reads_only_its_own_schema`).
- The four CI layer checks pass: no production SQL outside `db/pg_store/`, every grantee is
  created by the schema or built in, no prose between columns in DDL, every brew tap is real.
- Retention prunes analyzed sessions past the cutoff and **keeps** unanalyzed ones
  (`prune_activity_deletes_analyzed_sessions_past_cutoff_and_children`,
  `prune_activity_keeps_unanalyzed_sessions_even_when_old`).

## Wrong gate

- **Editing the DDL without applying** — boot re-creates the old object and the new shape
  silently disappears.
- **An enum's declared order is assumed** — deploy alphabetises; anything but a CASE rank is broke.
- **A bridge table gains a composite PK** — "one global row" can't be enforced (NULLs
  distinct); the partial-unique-index pattern is the rule.
- **A content wipe destroys user preferences** — config/content separation regressed.
- **A seeding procedure overwrites production data** — the timestamp guard exists exactly to
  prevent this.
- **The graph stays non-deterministic across restarts despite DDL looking right** — schema
  is fine but M4 worker guarantees aren't; this module's Done gate is about schema truth,
  not graph convergence (see M4).

## Scenarios

- [`scenarios/data-model.feature`](scenarios/data-model.feature) — round-trip truth,
  scoping, one-owner, retention.

## Open gaps

- Orphaned tables in `inference` (no writer, superseded by `narration_cache`) and
  empty-by-design dojo tables — see [06-gaps.md](../../../../roadmap/06-gaps.md).
- [D-12/D-13](../../../../roadmap/06-gaps.md) — dependency drift (P0(deps)) live in the
  register; not schema, but they press on `database@vVERSION` bump discipline.

## Related

- [architecture/data.md](../../../../architecture/data.md) — schemas, conventions,
  one-owner, metadata model (legacy depth, folds here).
- [database/ddl/](../../../../../database/ddl/) — the declarative source of truth.
- [dbd](https://github.com/sensei-hq/dbd) — design.yaml + apply/deploy/reconcile/import.
- [prd/02-objectives.md](../../../02-objectives.md) §Cross-cutting ·
  [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/db/`; gate: the four check-* runner scripts.