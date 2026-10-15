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

### Added

- **Claude Code usage at the source** (#238): the sensei plugin's mods module
  (Claude Code 2.1.300+) reports each session's context fill, rate-limit
  windows and `/cost` total, and each turn's four token counts and model.
  The cost lands in the new `activity.sessions.list_cost_usd`. It is the list
  price, not billed spend: `metered_cost` keeps a source's billed figure.
  The per-assistant contract is in PRD M3 §Usage & cost capture.
- **Subagent model by risk class** (#238): with a `routing.risk_models` map
  configured (e.g. `{"approve":"opus","review":"sonnet","auto":"haiku"}`), a
  spawned agent runs on the model its change's risk class maps to, through
  `POST /api/review/subagent-model`. With no map, nothing is overridden.
- **Opt-in relay tool gate and run drive** (#238): two plugin options, both off
  by default (`pluginConfigs.sensei.options`).
  - `relayGate` puts tool calls to the daemon's gate. It only tightens a
    decision and fails open.
  - `runDrive` submits a run's dōjō nudges as prompts, each once.
  - The never-registered shell `hooks/gate` is removed.
- **Settings · Dōjō says why a checkout was skipped**, in words (#225).
- **A person's own metrics reach their dōjō** (#240): user-scoped rows that are
  provably the signed-in person's — their session metrics on a repository only
  they elected, and commit metrics under an address GitHub verified for them.
  The dōjō files them under the person from the session; their team lead and a
  tenant admin can read them (access matrix). Colleagues' commits and unverified
  addresses stay on the machine, counted in the sync log.
  **Deploy order:** the ingest now calls `dojo.file_metric_rows`; until the
  release's `deploy-dojo` job applies it, metric pushes fail loudly ("function
  does not exist") and retry, rather than the old `upsert_repository_metrics`
  filing a person's row as repository-wide. Drop the old function once deployed.
- **A team sees THAT a run is going on its project** (#252) — status only,
  never content: `GET …/relay/status` over `dojo.peer_run_status`, for the
  run's owner, a member or lead of a team owning the project, and a tenant
  admin. A relay session now carries its owner's project.

- **Instruments lists the discovered MCP servers by assistant, each with an
  on/off toggle** (#245) — restored after the redesign folded them into the
  Playground rail. A failed read says so instead of showing "none", and a
  refused toggle leaves the row as it was.
- **A repository's code graph has a home on the dōjō** (#225, store only): the
  `dojo.graph_*` tables and `dojo.file_graph_slices`, which files a
  repository's graph file by file, one graph per tracked branch
  (`dojo.repository_graph_branches`: the forge default plus any the team adds,
  e.g. production).
  The daemon pushes each tracked branch it has a checkout of after the metrics:
  committed changes only — a file `git status` reports is held back and kept at
  the dōjō's hash — skipping a checkout whose working tree has moved to another
  branch since it was indexed. Each branch's outcome is in `sensei.sync_state`
  (`repository_graph`, `<repo_key>@<branch>`). The dōjō's diagram screens over
  this graph are not built yet.
  **Deploy:** `deploy-dojo` first (tables, functions, `/v1/you/graph`); the daemon
  then needs `sync_entity.repository_graph`, which a released daemon deploys on
  first start — a debug build at the same version needs `dbd reconcile`.

### Fixed

- **A deleted file leaves the index, and so the dōjō** (#225). Two causes:
  - A watcher batch that only deleted reached no repository.
  - A vanished file with no symbols was never pruned. On the development
    machine 363 of 3,078 file rows were stale (185 gone from disk, 178 under a
    since-ignored path), and they rode in the graph manifest.
- **A repository with an unnamed index can push its graph** (#225):
  `dojo.graph_schema_indexes.name` is nullable, as the local column is; the
  whole batch used to be refused.
- **`make test` with Supabase up**: `database/tests/run.sh` runs each directory
  against the database its scope deploys to, so `gateway/` no longer runs
  against the dōjō.
- **The setup/health gate no longer sticks after it recovers** (#245). SvelteKit
  caches a `reroute` result per URL for the whole session, so once `/` resolved
  to the setup screen it stayed there after setup completed — re-running setup
  or a daemon blip was enough. The gate is now a redirect from the root layout
  load, asked on every navigation.
- **A failed config read no longer sends a set-up user back to setup**: the
  wizard read config through a `{}` fallback, which reads as "incomplete".
- **The dōjō org home's "needs a maintainer" band is the open triage queue**
  (#241), not kit fixtures; below maintainer it shows nothing, and a failed read
  is an error. Health's anomaly feed says "not measured" instead of counting 0.

## [0.13.0] — 2026-10-13

**The dōjō knows the daemon only as the signed-in person.** Device tokens are
gone: every tenant call presents the persona's session, signing in joins every
dōjō that person belongs to, and the daemon now deploys its own schema when a
new version starts.

### Added

- **The daemon deploys its schema on boot when its version changed** (#260).
  `brew upgrade` neither deploys nor restarts the service; the first start of a
  new binary now runs the installer's deploy (reconcile, with the backup), and
  records `sensei.config` `schema_deployed_version` only once it succeeded. A
  failure is logged as an error and retried at the next start.

### Changed

- **Signing in is joining a dōjō** (#236 step 4e). The daemon's sync creates a
  local membership for every dōjō the signed-in persona belongs to, bound to
  that persona; the connections screen sends you to sign in instead of
  offering a pairing form.

### Removed

- **BREAKING — the dōjō no longer accepts a device token** (#236 step 4e). The
  daemon's tenant calls (relay, artifacts, rules) present the signed-in person's
  session, so the table policies decide what they read and write; any other
  bearer is a 401, and the service role never stands in for a caller without a
  session. `dojo.memberships.device_token_hash` is dropped. A daemon older than
  0.12.0 — or a membership no signed-in persona has claimed — must sign in to
  reach its dōjō.
- **Device-token pairing in the daemon and the app**: `POST
  /api/dojo/memberships` (now 405), the connect form, and
  `sensei.dojo_memberships.credential_ref`.
- **`SENSEI_DOJO_URL` and the `sensei-config` crate** (#261): one setting says
  where the dōjō is — `DOJO_URL` / the local config, default
  https://dojo.sensei-hq.com.

### Fixed

- **The project About panel shows a failed read as an error** (#262), not as
  "no dōjō binding" or "Repos (0)".
- **A knowledge source needs a name, on both sides** (#263): the form requires
  it and the daemon refuses a blank one; a source no longer takes a key.
- **Waiting on a gate stops at once when the persona must sign in**, instead of
  polling to its deadline and failing open.
- **The connections screen shows a failed read as an error**, not as "no dōjō
  yet, sign in".

### Upgrade notes

- A membership that no signed-in persona has claimed stops reaching its dōjō:
  sign in under Settings → Dōjō and the next sync binds it.
- The dōjō's production schema drops `memberships.device_token_hash`; the
  release's `deploy-dojo` job refuses a DROP unattended, so it is applied once
  by hand (`dbd reconcile --scope dojo --allow-destructive`). The 0.13.0 dōjō
  does not read the column, so the order does not matter.

## [0.12.1] — 2026-10-13

**Upgrading from 0.11 or earlier works again.** An install's schema now
converges with `dbd reconcile` instead of `dbd apply`, which never adds a value
to an existing enum. On an upgraded install, 0.12.0's seed import failed on
`model_capability.decision`, and skips marked `vendored`/`generated` and git
history scans could not be recorded.

### Fixed

- **The daemon's schema deploy reconciles** (install, upgrade and `sensei
  doctor` all use it): new enum values, columns, constraints and indexes reach
  an existing database, and columns the design dropped are removed. A
  destructive plan first copies each table it alters to
  `~/.sensei/backup/pre-reconcile/<UTC time>/` (and does not run if the copy
  fails), then drops the views that read those tables, which the deploy
  recreates from the design. Whole tables are never pruned. Verified on a
  database built from the v0.11.0 schema and seed.
- **`dbd-core` v0.26.0** in the daemon and bootstrap, and the `dbd` CLI that CI
  and the release install. v0.22 reported drift on a database it had just
  created, so every reconcile after the first would have failed. One version
  everywhere is now a test (`crates/bootstrap/tests/dbd_pins.rs`).
- **Svelte script blocks come from Svelte's own parser** in the TypeScript
  driver, not a pattern (CodeQL `js/bad-tag-filter`).

## [0.12.0] — 2026-10-13

**Every call and reference is placed by its language's compiler.** The indexer
now resolves Rust through rust-analyzer and TypeScript, JavaScript and Svelte
through the project's own TypeScript checker, instead of reading names off the
source text. On this repository calls went from 44% placed to 100% (Rust),
99.96% (TS/JS) and 99.94% (Svelte); references to 99.92%, 99.33% and 99.55%.
Every remaining miss carries its root cause. See
`docs/reports/2026-10-13-placement.md`.

### Added

- **Compiler engines in the language adapters** — rust-analyzer (`scip`, all
  features, three target platforms merged) and an embedded TypeScript/Svelte
  driver (`crates/senseid/ts-driver`) that uses the project's own `typescript`,
  runs the project's `svelte-kit sync`, and maps Svelte components through
  svelte2tsx. A repository's projects compile once per state of their sources,
  in a `compile_projects` task the file tasks wait on.
- **New reason codes**, persisted on every edge and visible in
  `sensei.graph_placement`: `resolved_by_compiler` (rung), `engine_not_run`,
  `compiler_no_symbol`, `symbol_unnamed`, `declaration_not_found`,
  `untyped_receiver`.
- **Indexing engines are installed and checked by bootstrap.** `sensei doctor`
  reports them (`--fix` installs what is missing) and the daemon installs
  missing ones at boot: `rust-analyzer` + `rust-src` via rustup, the driver's
  npm packages into `~/.sensei/engines`.
- **Only code the repository owns is indexed** — files `.gitattributes` marks
  `linguist-vendored` / `linguist-generated`, or whose head says `@generated`,
  are recorded with skip reason `vendored` / `generated` and shed their nodes.
- **The placement floor** (`indexer::quality::placement`, nightly workflow):
  no language, package or edge kind may fall below 90%.
- `sensei.resolution_quality` decomposes placement by package (crate or app).
- **Diagrams**: World (#219), Neighbourhood (#220), Layers and Cycles (#222,
  #232), Zones (#223), Schema/ER from the schema store (#221, #251) — with a
  coverage line on each.
- **Schema store** — `schema_entities/_columns/_indexes/_refs`, filled from dbd
  designs and EF Core mappings (#251).
- **dōjō access as the person** (#236): every console read and write and every
  daemon call (runs, projects, artifacts, rules, metrics) runs as the signed-in
  person under row-level security; `GET /v1/you/memberships`; personas claim
  the memberships the dōjō lists under them.
- Reading an assistant's conversation history is its own consent, off by
  default (#218). Removing a scan root asks keep-or-remove (#247).

### Changed

- **R5 is now "externals are named, never walked"**: a compiler may read a
  dependency's declarations to resolve a call; nothing of it becomes a node.
  A use of a local, a parameter, an anonymous function or a value's member is
  no edge at all.
- Twenty-five metric queries and the last positional writers moved into the
  persistence layer as typed rows (#161, #227). Diagram payloads are cached
  against the project's graph version (#233).
- Installs no longer take a full `pg_dump` first; two essential-backup
  snapshots are kept, not five.
- gateway v0.5.1 → v0.7.0; `decision` routes to `Capability::Decision` (#202).

### Fixed

- A file edited during a scan is no longer placed against a stale compile and
  marked parsed; it waits for a fresh compile.
- A call is placed on its own callee, not on a same-named argument
  (`items.filter(filter)` was silently dropped).
- A tool's linked worktree under a repository's dot-directory is no longer
  indexed as a second copy of the repository.
- Capture: a failed tool call (`PostToolUseFailure`) is recorded, and each tool
  call is paired with its own response (#238).
- `git_history` drops stored tips the repository no longer has (#253).
- An isolated daemon instance never writes the user's `~/.claude/CLAUDE.md`
  (#248). Relay data is its owner's alone (#249).


## [0.11.0] — 2026-09-28

**The repository was rebuilt.** `sensei-hq/sensei` is a new repository as of this
release; the pre-scrub original is `sensei-hq/sensei-archive`, private and
archived. Every commit SHA before this release changed. If you have a fork or a
local clone from before 2026-09-28, re-clone rather than pull — the histories
share no commits.

### Security

- **Private client names removed from the whole tree AND from history.** An
  audit found names across tracked files of a PUBLIC repository — in production
  source comments and in test assertions, not only docs — together with metrics
  about those codebases. All of history was rewritten: 4,048 commits, every
  occurrence replaced with an obviously-fictional placeholder, 9 commit messages
  cleaned, and a corporate address removed from commit metadata. Blob counts
  before and after are identical, so nothing was dropped to achieve it.
- **Issues carried names that no history rewrite can reach.** Three private
  names sat in issue bodies, two of which appeared in no git blob at all. All
  200 issues were scrubbed on migration, with their numbers preserved — 173
  distinct `#NNN` references in commit messages would otherwise have broken.
- **The leak guard now catches what it was built to catch.** It only ever
  examined the USERNAME segment of a path, so a client name used as a namespace
  or a package root carried no path and was invisible. There is now a denylist
  rule, sourced from OUTSIDE the repository (`$SENSEI_PRIVATE_NAMES`, else
  `~/.sensei/private-names`) — a committed list of client names would leak
  exactly what it guards.
- **`--audit` asked the wrong question.** Its home-directory rule was two
  whole-file greps: flag a file that contains a home path AND contains no safe
  one *anywhere*. A file holding `/Users/dev/x` on one line and a real home
  directory on the next was reported clean. It is per line now, and names the
  line.
- **A digest is not a name.** A four-letter client name lands inside a checksum
  by chance — 130 times across `Cargo.lock`, `bun.lock` and a tsbuildinfo. Runs
  of 32+ alphanumerics are collapsed before matching, so the rule stops
  accusing hashes. Not an extension exemption: skipping `*.lock` would blind it
  to a real name in a lockfile.

### Added

- **An `architecture` metric family — the first metrics that read the code
  graph.** 27 live metrics across 9 families and not one read `sensei.edges` or
  `sensei.nodes`, while the graph grew to 4,078,536 edges and 1,182,758 nodes.
  Three now do: `graph_confidence` (placed ÷ total edges),
  `public_surface_ratio` (exported ÷ declared symbols) and `symbol_size_p95`
  (p95 callable span, with the over-100 count beside it).
  `graph_confidence` ships first and gates the rest — it is a MEASUREMENT-quality
  signal, and at 31% a structural metric is describing under a third of a
  codebase. Coupling, instability and dependency-cycle metrics are deliberately
  NOT here: the module graph currently collapses to 13 distinct module→module
  pairs, and a confident number over 13 edges is a fabricated one. (#156)
- **`edges.resolved_via` and `edges.unresolved_reason`** — the edge-level
  verdict as columns, reduced from the per-use verdicts in
  `props.occurrences`. Exactly one is set on any edge: a placed edge has a rung,
  a missed one has a reason.
- **An anonymised transcript fixture corpus.** Six adapters were tested from 109
  inline string literals and zero file fixtures, which is a hand-written guess
  at formats that change without notice. `scripts/anonymize-transcript.py` turns
  a real capture into a committable fixture — shape preserved, content replaced —
  verifies its own output and refuses to write when a real home directory,
  mailbox or denylisted name survives. A fixture's `.meta.json` records the tool
  version and a sha256, so a hand-edit fails the build.
- **`copilot` and `vscode` in `sensei.assistant_family`.** Both adapters have
  existed for some time and both returned a family the enum could not store.
- **A gate keeping prose out of column lists** (`make check-ddl-comments`, run
  on every commit), and `docs/database/<schema>.md` as the home for schema design
  history.

### Fixed

- **A first insert left the edge verdict NULL.** Only the merge path derived it,
  and a new edge never goes through that path — of 130,614 edges the indexer
  touched in ten minutes, 14,217 carried a verdict. Both INSERT branches derive
  it now, and the upsert recomputes from the merged props. It went unnoticed
  because every consumer read a VIEW that recomputes the reduction at query
  time, so the materialised column was verified by nothing.
- **Two resolution rungs were never seeded into the live database.**
  `named_by_this_file` and `declared_by_its_type` had been in the seed file since
  2026-09-21 and never imported. The verdict function joins the code table, so an
  occurrence carrying an unseeded rung matched nothing and vanished — 14,486
  placed edges would have been left unclassifiable.
- **Seven DDL files stated no `search_path`**, and four of them were not indexed.
- **The rung and the reason were recorded all along** — the view read one level
  too shallow, at `props->>'rung'` where no writer has ever put them.

### Changed

- **434 lines of prose moved out of 39 column lists.** A `create table` column
  list is a declaration, and paragraphs of rationale between two columns buried
  the shape a reader opened the file for — while being invisible to everything
  except a human reading that one file. What a column MEANS now lives in
  `comment on column`, where the catalog can serve it; why it is that way lives
  in `docs/database/<schema>.md`.
- **dbd 0.19.0 everywhere** — one version, one reference list.

## [0.10.1] — 2026-09-25

**0.10.0 published no artifacts.** Its `build-daemon` aarch64-linux leg failed,
`release` needs every leg, so the GitHub Release, the assets and `update-tap`
were all skipped — while `make bump` had already pushed the tap pointing at
0.10.0 with `REPLACE_WITH_*_SHA256` placeholders. This release is what makes
0.10.0's contents installable; read its notes below for what actually changed.

### Fixed

- **A FRESH POSTGRES COULD NOT PROVISION THE SCHEMA.** Twelve DDL files in the
  `sensei` and `activity` schemas grant to Supabase's `anon`, `authenticated`
  and `service_role`, and nothing in the tree created them. A `GRANT` resolves
  its grantee at apply time, so the first such file aborted the whole deploy
  with `role "authenticated" does not exist`. Those schemas are in the daemon's
  `default` scope, so this was not skippable the way the `policies/dojo/*` files
  are — same end state as 0.10.0's `import_libraries` bug: **a new install came
  up with no daemon.** Invisible on any machine that already had the roles,
  which is every machine that has ever run the dōjō locally.
  `apply/before/roles.sql` creates them NOLOGIN, before the first entity is
  written, and does nothing where they already exist. (#196)
- **The release builds aarch64-linux natively.** It cross-compiled under
  `cross`, where `pg_query`'s bindgen step ran a clang that could not find its
  own `stdbool.h`. `pg_query` arrives through `dbd-core`, so it is not optional.
  The matrix is also no longer fail-fast: one broken leg used to CANCEL the
  other three, so each attempt revealed exactly one problem and each round cost
  a version tag.
- **The Rust CI job can actually provision.** Two errors in the workflow added
  in 0.10.0: the dbd package is `dbd-cli` (the *binary* is `dbd`), and
  `dbd deploy` with no `--scope` applies the full set rather than the `default`
  scope the daemon deploys — which would have given CI a shape production never
  has.
- **Pure-module coverage was running database tests.** A libtest filter is a
  substring match, not a root-module match, so `libraries::` also selected
  `tasks::handlers::libraries::…` — 33 tests pulled into a no-database job, 7 of
  which failed on `pool timed out`. A post-run guard now asserts every test that
  ran has a pure root module.

### Added

- `make check-ddl-grants` — every role the schema grants to (or writes a policy
  for) must be a role the schema creates. Runs in `test-fast`, so on every
  commit, and in CI.

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
- **SQL reading, both dialects.** `lang::sql` detects the dialect and reads
  PostgreSQL and T-SQL through dbd 0.15.0's `parse_sql_as`. A file that states
  no dialect, or states one nothing can read, is REFUSED as
  `GrammarUnavailable` — never read by a lenient parser that would accept it and
  mint nonsense. (A reader for T-SQL was written here first and deleted the same
  release; see Changed.) Built, **not yet flipped** — SQL is not in
  `PRODUCTION_LANGUAGES`, because a reference whose schema is unstated cannot be
  resolved one file at a time.
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
- **Rust has CI.** No workflow ran `cargo test`, `cargo clippy` or `cargo fmt` —
  `coverage.yml` measured Rust coverage, which uploads a number whether the
  tests passed or not. The 3,300-test senseid suite ran only when someone
  remembered to. `rust.yml` adds a fast `lint` job and a `test` job that
  provisions `sensei_test` from the DDL tree (not a released bundle — that
  drift is what hid the install bug below).
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
- **SQL is read through dbd 0.15.0.** This crate's own T-SQL lexer and
  statement-head reader are deleted (1,058 lines) — dbd ships the same reader,
  having reached the same two rules, so a second copy only meant two things to
  re-measure. Identity is preserved exactly (A7 544, 0 intra-file collisions,
  1 miss over 2,419 files); reference extraction is not, at ~52% fewer
  references — raised upstream as sensei-hq/dbd#21.
- **An isolated instance no longer ingests transcripts.** `SENSEI_INSTANCE=e2e`
  gets a throwaway database and data dir, but ingest reached outside both into
  the real `~/.claude` — 590 tasks in the second the port opened.
- The e2e harness captures the app's output instead of discarding it, and waits
  for the daemon to be READY rather than merely bound.
- The indexer spec folder is reorganised around the shipped pipeline, with a
  README index and a vocabulary for the numbers it reports.
- `docs/backlog.md` is an index of OPEN work again (1,718 → 1,242 lines); the
  rules worth keeping moved to `docs/plan/decisions.md`.

### Fixed

- **A FIRST INSTALL COULD NOT PROVISION ITS DATABASE.**
  `staging.import_libraries()` wrote six columns S7b had moved to
  `library_versions` (`version`, `source_type`, `base_url`, `docs_url`,
  `local_path`, `page_count`), so `dbd deploy` failed on any fresh database:

      column "version" of relation "libraries" does not exist

  That took the Database health component to `NeedsHumanAction`, which left
  `daemon_start` skipped on `unmet_deps=[Database]` — **so a new install came up
  with no daemon at all.** Existing installs were provisioned before the drift
  and never re-ran the import, which is why it stayed invisible.
- **An upsert whose identity moved no longer fails the whole file.**
  `ON CONFLICT … DO UPDATE` keeps the OLD `parent_id` when the incoming one is
  NULL, and `parent_id` is part of `nodes_unique_identity` — so the row the
  update collided with and the row `adopt_node_by_identity` searched for were
  different, and `fetch_one` turned that into a failed file. 44 files in one day.
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

- `activity.sessions.metered_cost` is never non-zero, so no cost metric is
  measured rather than modelled (#181).
- Dōjō sync has never sent a row; the membership never left `authenticating`,
  behind an IPv4/IPv6 reachability failure (#184–#186).
- A repository is marked `indexed` when its files are placed, before any
  cross-file resolution runs — there is no repo-scope resolve barrier (#193).
- SQL references lost ~52% in the dbd switchover, and a reference to a table
  whose schema is unstated cannot be resolved until every file is read (#192,
  upstream sensei-hq/dbd#21 and #22).
- SQL, Kotlin and Swift are not flipped, so `crate::languages` still ships.
- The nesting depth a parser accepts is unbounded, so a deeply-nested file
  aborts the process rather than failing the file — the one bundle that
  triggered it is now excluded, but the failure mode is not fixed (#178).
- A retryable failure is logged at `error` on every attempt, so one bad file
  writes a row per retry forever (#194). `public.logs` reached 25 GB of a 37 GB
  database this way; the rows were pruned, the cause was not.
- The e2e suite is outside the static gates and does not compile clean (#187).

[Unreleased]: https://github.com/sensei-hq/sensei/compare/v0.11.0...HEAD
[0.11.0]: https://github.com/sensei-hq/sensei/compare/v0.10.1...v0.11.0
[0.10.1]: https://github.com/sensei-hq/sensei/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/sensei-hq/sensei/compare/v0.9.1...v0.10.0
