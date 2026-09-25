# Changelog

All notable changes to this project are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While the major version is `0`, a MINOR bump may carry a breaking change; those
are marked **BREAKING**.

> **Earlier releases.** This file starts at `0.10.0`. The 67 tags before it
> (`v0.1.0` … `v0.9.1`, from 2026-05-03) are not reconstructed here — writing
> entries for releases nobody summarised at the time would mean inventing them.
> Use `git log v0.9.0..v0.9.1` for any of them; `make bump` has tagged every
> release since the beginning, so the history is complete in git.

## [Unreleased]

## [0.10.0] — 2026-09-25

The release that makes indexer v2 the production path for seven languages and
retires roughly 5,600 lines of v1.

### Added

- **Seven languages cut over to indexer v2** — Rust, TypeScript (with
  JavaScript, Svelte and Vue), Java, Python, C#, PHP and C. `PRODUCTION_LANGUAGES`
  in `indexer/lang/mod.rs` is the single frontier: adding a language there IS its
  cutover, paired with deleting its v1 parser.
  - C# and PHP were **additive** — 19,404 and 2,251 files that had no graph at
    all now have one.
  - C narrowed on the way: v1 claimed `.cpp`/`.hpp`/`.cc` and read them
    line-by-line; v2 claims `.c`/`.h` only, and `is_cpp` refuses a `.h` holding
    C++ (1,817 of 3,776 vendored files, 0 false positives on real C).
- **SQL reading, both dialects.** `lang::sql` detects the dialect and dispatches.
  `lang::sql::tsql` is this crate's own lexer and statement-head reader, built
  because nothing off the shelf declares a stored procedure — `tree-sitter-sequel`'s
  grammar says `// TODO: procedure` and `sqlparser`'s `MsSqlDialect` fails on the
  parenless `CREATE PROCEDURE @p int AS` form. The PostgreSQL half rides dbd
  0.14.0's `parse_sql`. Built, **not yet flipped**.
- **A Kotlin adapter** — 245 files, 0 unreadable, 3,038 declarations. Built, not
  flipped: two shapes remain (anonymous object expressions, Android product
  flavours).
- **`DbdManifestAdapter`** reads `design.yaml`: `project.name` names the package,
  extensions are dependencies, and `source.dialect` STATES the SQL dialect.
  Before it, no manifest above a `.ddl` file named a package.
- **`CMakeManifestAdapter`** — `project(name)` is the one place a C project
  states its name.
- **Observability views** — one view per question: `graph_nodes` and
  `graph_resolution` carry repository; `activity.task_health` and
  `task_failures` are the restart list.
- **Transcript ingestion has a schedule.** `ingest_captures` joins
  `schedule::SCHEDULABLE` with a 300s cadence.
- **A `provisioning` daemon DB state.** `DaemonDbMode` was `full | degraded` —
  two states for three situations. A first install has no database until
  bootstrap creates it, and the daemon stays up through that window on purpose
  (it binds its port before it touches Postgres), so that window could only be
  reported as `degraded`, which means "it was working and stopped". A normal
  first run announced itself as a fault. Which failure it is now comes from
  `database_exists` rather than the connect error's text; provisioning writes no
  `startup-error.log`, carries no remedy and logs INFO, and the app exposes
  `isProvisioning` separately from `isDegraded`.

### Changed

- **BREAKING — v2 is the only scan and process path.** There is no v1 fallback
  anywhere, deliberately: the two mint different identities
  (`languages/fqn.rs` against `indexer/fqn.rs`), so a fallback would put two
  identity schemes in one graph where an edge minted by one could never meet a
  node minted by the other. For a language v2 does not claim, the file is **not
  indexed** — a visible gap, never a silent hand-off.
- The indexer spec folder is reorganised around the shipped pipeline, with a
  README index and a vocabulary for the numbers it reports.
- `docs/backlog.md` is an index of OPEN work again (1,718 → 1,242 lines); the
  rules worth keeping moved to `docs/plan/decisions.md`.

### Fixed

- **A name is a name, never a program.** 357 edges carried a `target_name` that
  was source text rather than an identifier — an IIFE's whole body via
  `name_callee`'s catch-all, and `declare global { … }` via a type reduction that
  validated only its first character. Ten broke a btree or GIN index, and because
  the failing INSERT aborts the task, **nine files lost every symbol and edge they
  had** and retried forever. The guard now lives in `lang::common::readable`,
  which every language already goes through.
- **UTF-16 source is read.** `classifiers::decode_source` owns the decode and
  both the scan gate and the parse path call it; the BOM is checked BEFORE the
  null-byte test, because UTF-16LE ASCII is `X 00 X 00` and the null test was
  calling all **754** such files `binary_content`. Worth 377 more files, +329
  stored procedures, +7,560 references.
- **One bad byte no longer strands the capture spool.** A single invalid UTF-8
  byte held **176 MB of hook events for 8 days**; `parse_spool_lines` now decodes
  per line and skips only the bad one.
- **Transcript ingestion ran once per daemon start and never again** — it was
  enqueued only from the boot block and named in no schedule. Every watermark and
  turn stopped thirteen seconds after boot.
- **Minified bundles with a second suffix are excluded.** `**/*.min.js` requires
  the name to END there, so a vendored bundle saved as `*.min.new.js` was indexed
  — and it is the file whose nesting overflows the parse stack, which aborts the
  process rather than failing the file.
- **A C# conditional-compilation set is one callable and an arm per condition**
  (414 → 392 collisions); the same shape fixed Java overload sets (409 → 14) and
  Rust `cfg`-gated members.
- **A TypeScript function body names what it declares** — A7 collisions 511 → 10,
  then to **0** once a local value stopped minting a node.
- **Python's A7 gate went 72 → 0** while cutting over.
- **`reconcile` is wired** — an earlier note claiming no caller was stale.
- A derived member is a declaration (Rust).

### Known issues

See [`docs/production-readiness.md`](docs/production-readiness.md), which
measures each of these against the live system:

- `adopt_node_by_identity` fails a whole file when its row moved underneath the
  upsert — 44 files on the day of release.
- `activity.sessions.metered_cost` is never non-zero, so no cost metric is
  measured rather than modelled.
- `public.logs` is 25 GB of a 37 GB database.
- CI runs no Rust tests, no clippy and no fmt.
- Dōjō sync has never sent a row; the membership never left `authenticating`.
- SQL, Kotlin and Swift are not flipped, so `crate::languages` still ships.

[Unreleased]: https://github.com/sensei-hq/sensei/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/sensei-hq/sensei/compare/v0.9.1...v0.10.0
