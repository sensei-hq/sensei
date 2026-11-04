---
name: Production readiness
description: Measured gaps between what ships today and what shipping to other people requires
date: 2026-09-25
---

# Production readiness

**Filed as issues 2026-09-25**: #175 (A1) · #176 (A2) · #177 (A3) · #178 (B1) ·
#179 (B3) · #180 (C1) · #181 (C2) · #182 (D1) · #183 (D2) · #184 (E1) · #185 (E2) ·
#186 (E3) · #187 (D4) · #188 (B2) · #189 (G2) · #191 (F). The e2e triage went on
#134; A4 is covered by #141/#146, the a11y contrast by #136, fresh-provision
drift by #142, and dbd version skew by #122. The dbd 0.15.0 switchover is #190.

Every item below is a MEASUREMENT taken on 2026-09-25 against the live daemon,
the live `sensei` database and this working tree — not a checklist copied from
somewhere. Where a number is quoted, the query that produced it is quoted with
it, so a later reader can re-run it rather than trust it.

Ordered by what would hurt most if a second person installed this today.

> **Checked against the code and GitHub on 2026-11-03.** This is a log of measurements taken on
> 2026-09-25, so each section keeps its number, its query and its measurement; what has closed
> since is struck in its heading and named with the commit or `file:line` that closed it. Still
> open: **A4** (#146), **C3** (deliberate), **E1** (#184) and **H**'s last bullet (Swift has no
> adapter). The rest is closed, F and D-12 with it — kavach 1.3.2 landed in `eefc6c119`; where an
> issue is still open, the heading says which half of the item it is (#187's specs passing).

---

## A. Things that are silently wrong

### A1 — ~~`adopt_node_by_identity` fails a whole file when its row moved~~ — closed (#175)

**44 files today, on the CURRENT binary.**

    select substring(error_message from 1 for 70), count(*)
      from activity.task_executions
     where status='failed' and task_kind='process_file'
       and started_at::date = '2026-09-25' group by 1 order by 2 desc;

    adopt node by identity (generatePrescriptionSchedule): no rows returned…   14
    adopt node by identity (uploadPrescriptionImage): no rows returned…         9
    adopt node by identity (findByProviders_id): no rows returned…             7
    …

`upsert_node_ex` inserts `ON CONFLICT (folder_id, fqn)`. A row that exists under
a DIFFERENT fqn is invisible to that clause, so the insert falls through and
hits `nodes_unique_identity`; `adopt_node_by_identity` then re-points it. That
adopt uses **`fetch_one`**, so "no row matched" is `RowNotFound` → the file
fails → `fail_folder` withholds the `files` row → the reconcile re-drives it.

The arm immediately above it already carries the fix for exactly this shape,
and says so: *"Erroring here was a POISON PILL, not a safeguard … the whole repo
never finished indexing."* This path did not get the same treatment.

**Likely cause** (not yet proven): the INSERT and the adopt are two statements
with no transaction between them, and 16 workers run concurrently on one folder.
If a sibling moves the row's `line_start` in the gap, the identity no longer
matches and the UPDATE hits nothing.

**Done looks like:** `fetch_optional`; on `None`, re-drive the upsert once (the
row changed under us) or resolve through `node_id_by_fqn`, as the sibling arm
does. A test that runs two concurrent upserts for one identity with a moving
`line_start` and asserts neither fails.

`crates/senseid/src/db/pg_store/graph.rs:911`

### A2 — ~~cost is never captured, so every cost metric is derived or absent~~ — closed (#176)

    select count(*), count(metered_cost), count(nullif(metered_cost,0)), max(metered_cost)
      from activity.sessions;
    -- 376 | 23 | 0 | 0

`activity.sessions.metered_cost` is **never non-zero**. `Cost per result` is
therefore computed from a configured subscription fee, and exists for 2 of 15
workspaces. There is no measured spend anywhere in the system.

**Done looks like:** either populate `metered_cost` at ingest from the
assistant's own usage report, or retire the column and say plainly that cost is
modelled from a subscription figure — the current state reads like measurement
and is not.

### A3 — ~~`Tokens per day` is ~all cache reads, so it cannot proxy cost~~ — closed (#177)

    select sum(tokens_in), sum(tokens_fresh) from activity.sessions
     where started_at::date = '2026-09-22';
    -- 1,959,227,720 | 7,890      (fresh = 0.0004%)

The metric is defined as *"raw prompt + cache creation + cache reads"*, so it is
not wrong — but a cache read costs an order of magnitude less than a fresh
token, and nothing surfaces `tokens_fresh`, which is the number that tracks
spend.

**Done looks like:** a `Fresh tokens per day` metric beside the existing three,
and `how_to_read` on `Tokens per day` saying explicitly that it is dominated by
cache reads.

### A4 — the graph is 40.3% resolved and nothing states a target

    select count(*) filter (where target_id is not null),
           count(*) filter (where target_id is null) from sensei.edges;
    -- 1,627,350 resolved | 2,415,191 unresolved  (40.3%)

Much of the unresolved side is correct — a call into a library is not a
first-party edge. But no number anywhere says what the ceiling SHOULD be, so
nobody can tell a regression from the normal state.

**Done looks like:** the acceptance run after the wipe publishes a per-language
resolved-rate, and those become ratchets the way A7 and A8 already are.

---

## B. Things that fail loudly but should not fail at all

### B1 — ~~the parse stack is unbounded: one file aborts the daemon~~ — closed (#178)

The bound shipped in 2e5c6c1f and is live on v0.20.2. `MAX_PARSE_BYTES` is 2 MiB and
`MAX_LINE_BYTES` 10,000 (`tasks/handlers/helpers.rs:285,288`); a file past either is refused by
`exceeds_parse_bounds` (`:303`) as `ScanSkipReason::Oversized` (`classifiers.rs:302`), which the
scan and `process_file` both use. Over the 61,349-file corpus the shipped rule refuses exactly the 137
files the measurement found (#178); the live DB shows none today, because its watch roots are narrowed.

A Rust stack overflow is not a panic — no unwind, no backtrace, no file name.
`process_file` runs the parse on `spawn_blocking` so the watchdog can preempt a
parse that HANGS; that does nothing for one that ABORTS, because it is the same
process.

The known trigger is gone (`deba28fb` widened the globs; both watch roots now
parse clean — 9,697 + 17,953 files, 0 offenders). A glob is a heuristic about
NAMES, so an input nobody has seen yet still takes the whole daemon down.

**Done looks like:** refuse to parse above a size or a longest-line bound and
record an explicit skip reason. See the entry in `backlog.md` for why that is
the only option that states a rule.

### B2 — ~~watchdog abandonment is routine, not exceptional~~ — closed (#188)

It is reported as a class now: `GET /api/tasks/abandoned` gives the count per day and kind with the
ten worst offenders (`api/handlers/tasks.rs:54`), and `sensei doctor` prints it as a Watchdog
section (d5bfdc7d). Live on v0.20.2: 204 in 14 days.

    process_git_folder … exceeded 600s and was abandoned   (4 in 7d)
    process_file        … exceeded 180s and was abandoned   (~8 in 7d)

Small numbers, but each one is a folder or file that silently did not finish.
Nothing reported them as a class at the time — the reporting is what closed it.

**Done looks like:** a health surface that counts watchdog abandonment per day
and names the worst offenders, so "slow" is visible before it becomes "missing".

---

## C. Operations — the machine fills up

### C1 — ~~`public.logs` is 25 GB of a 37 GB database~~ — closed (#180)

    select schemaname||'.'||relname, pg_size_pretty(pg_total_relation_size(relid))
      from pg_catalog.pg_statio_user_tables order by 2 desc limit 3;
    -- public.logs 25 GB | sensei.edges 4094 MB | sensei.nodes 4000 MB

**68% of the database is logs.** `log_prune` runs daily and is not keeping up.
This is also why `pg_dump` now takes ~12 minutes and produces a 5.3 GB file.

**Done looks like:** a retention that holds, measured — and the structured log
either partitioned by day (so a prune is a `DROP PARTITION` rather than a
`DELETE`) or moved out of the database that gets dumped before every install.

### C2 — ~~every install writes a 5.3 GB backup, and they accumulate~~ — closed (#181)

    du -sh database/backup   →  24 GB   (6 dumps)

`make install` and `make install-debug` both depend on `db-backup`, with no skip
flag. Today's two builds cost ~24 minutes in `pg_dump` alone. The 7-day prune
keeps six.

**Done looks like:** a `SKIP_DB_BACKUP=1` escape hatch for iteration, and a
backup that excludes `public.logs` (C1 makes it most of the dump).

### C3 — `make install` deletes `target/`, so the next build is always cold

Deliberate and documented (the tree reaches tens of GB). But it means
`install` → `test-app-e2e` pays a full cold rebuild plus a second 5.3 GB backup.
Worth knowing before sequencing a session around it.

---

## D. Testing and CI — the main suite never runs in CI

### D1 — ~~CI runs no Rust tests, no clippy, no fmt~~ — closed (#182)

`.github/workflows/` holds four workflows:

| workflow | trigger | what it runs |
|---|---|---|
| `coverage.yml` | push to main/develop, PR | `bun run test:unit` (app), `bun run test` (dojo) |
| `release.yml` | tag `v*` | release build + assets |
| `cleanup-workflow-runs.yml` | cron | housekeeping |
| `qlty-plugin-bump.yml` | cron | housekeeping |

**At the time, the 3,317-test senseid suite ran in no workflow.** The pre-commit hook ran
`sensei-bootstrap` (174) and the app unit tests (1,698) — not senseid. So the
suite that covers the indexer, the daemon, the task engine and the store is
run only when someone remembers to run it by hand.

**Done looks like:** a `rust.yml` running `cargo test -p senseid --bin senseid`,
`cargo clippy -- -D warnings` and `cargo fmt --check` on push and PR, with a
Postgres service so the DB-coupled half is covered (see D2).

### D2 — ~~DB-coupled tests are not isolated and fail under parallel load~~ — closed (#183, #204)

Closed 2026-10-25 (5656955c, then a database per test by user decision): 198 tests that had passed
on a failed connect now fail, test threads are capped at 16 against the pool, and no test relies on
a sibling's rows. Suite 3,838 green.

Known and recorded: `publish_run::full_bridge_*` assert absolute row counts
polluted by concurrent writers; `metrics::session_outcomes::ftr_parity_*`,
`library_update_scheduler::security_bump_*` and
`version_rescan::rescan_is_crash_safe_*` fail only under parallel load. They
pass in isolation.

This is the blocker for D1 — a CI Postgres service would surface these
immediately.

**Done looks like:** per-test schema or transaction isolation, or a serialized
DB-test lane.

### D3 — ~~the e2e suite is 23/130 red, and its first run could not start at all~~ — closed (#134)

**Measured 2026-09-25: 107 passed, 23 failed, 20 skipped, 19.5 minutes.**

| spec | failures |
|---|---:|
| `configure-assistants` | 4 |
| `boot-flow` | 4 |
| `instruments-t2-slices` | 3 |
| `daemon-verification`, `atlas` | 2 each |
| `zz-a11y`, `multi-window`, `knowledge-sources`, `instruments-observatory`, `dojo-binding`, `db-setup`, `assistants-configure`, `activity-logs` | 1 each |

**The first run produced nothing at all**: `Port 7744 did not open within
240000ms`, zero specs.

**A first reading of this blamed the cold database** — `make test-app-e2e` drops
`sensei_e2e`, so that run had to create it and apply 123 tables. **That reading
is wrong, and the code says so.** `start_server` binds the port BEFORE it
touches the database, deliberately: *"If the DB is down, we want the daemon to
still serve /api/health so the frontend can show the actual cause."* It then
retries the connection and, on persistent failure, serves a degraded router and
self-heals. A slow or absent database therefore cannot stop the port opening —
once `TcpListener::bind` succeeds the OS accepts into the backlog.

**So the daemon never reached `start_server` at all.** Corroborated: on that run
it wrote neither `~/.sensei-e2e/senseid.log` nor `~/.sensei-e2e/serve.pid`, both
of which happen before the server starts. It exited earlier, in `main.rs`.

**The candidate cause** (consistent with the evidence, not proven — the file has
since been overwritten) is the already-running guard:

```rust
if alive {
    eprintln!("senseid: already running (pid {})", pid);
    std::process::exit(1);
}
```

globalSetup `pkill -x senseid`s the real daemon just before; a stale
`~/.sensei-e2e/serve.pid` whose PID had been reused by a live process would trip
this. **And that message goes to the PARENT's stderr** — the app's, which was
`stdio: 'ignore'`. The one line explaining the failure was the exact thing being
discarded, which is why two runs were needed to learn anything.

**And it was undiagnosable**: `globalSetup.ts` spawned the app with
`stdio: 'ignore'`, so the app's stdout and stderr — bootstrap's health
resolution, the daemon spawn, any panic — were discarded. Fixed in this session:
output goes to `/tmp/sensei-e2e-app.log` and the timeout path prints its tail,
because a log nobody prints is a log nobody reads.

**Done looks like:** every pre-bind exit in `main.rs` routes through
`write_startup_error` (see B3), the already-running guard distinguishes a stale
PID file from a live daemon, and the 23 failures are triaged — several look like
one cause (`boot-flow` + `db-setup` + `daemon-verification` are all
bootstrap-gate screens).

### B3 — ~~a pre-bind exit is invisible; a post-bind one is not~~ — closed (#179)

`api/server.rs` has `write_startup_error()`, which drops the reason into
`~/.sensei*/startup-error.log` where the app can read it. **It has three
callers, all inside `start_server`** — the two bind failures and the
degraded-mode branch.

Everything that can kill the daemon BEFORE that — the already-running guard,
and the `.expect()`s on creating the data dir, opening the log and spawning the
child — just `eprintln!`s and exits. In the e2e that goes to a discarded pipe;
under launchd it goes somewhere nobody reads. The mechanism for reporting this
already exists and one class of failure does not use it.

**Done looks like:** those paths call `write_startup_error` too, and
`sensei status` reads it.

### B4 — ~~`Full` and `Degraded` are two states for three situations~~ FIXED

`DaemonDbMode` was binary, so "the database is being built for the first time"
and "the database has gone away" both rendered as degraded — and the first-run
message read *"Database connection failed after startup retries … run
`sensei bootstrap` (or `dbd reset`) to (re)provision"*. `dbd reset` DROPS the
workspace's managed objects: a data-loss command offered to someone whose install
was merely still working.

Two causes under one label, where the label named the rarer and scarier one.

**Fixed.** `DaemonDbMode::Provisioning` is a third state, entered when the
target database does not exist yet. Which failure it is comes from
`database_exists` — the question the workspace already owns — rather than from the
connect error's text, which arrives as a formatted `String` and would make the
state machine depend on a Display format. The classifier is pure and
`None` (the check itself failed) maps to `Degraded`, because not knowing whether
a database exists is not evidence one is being built.

Provisioning writes no `startup-error.log` (it is not a fault), carries no
remedy (there is nothing to fix), and the app exposes `isProvisioning`
separately from `isDegraded` so a surface can say "setting up" instead of
raising a breakage banner. `dbd reset` is gone from the advice entirely,
guarded by a test that fails if any destructive command reappears in it.

### D3b — ~~six e2e failures assert a contract the plugin migration retired~~ — closed (#134)

The six now assert the current contract; no `sensei-hook-dev` reference is left anywhere under
`app/e2e` (checked 2026-11-03).

`configure` no longer writes a hook dispatcher. It registers the marketplace and
runs `claude plugin install sensei`, and the plugin supplies "skills, commands,
agents, marketplace hooks, MCP". `~/.claude/hooks/sensei-hook-dev.ts` is now
**legacy**, and the daemon actively STRIPS it —
`LEGACY_SENSEI_HOOK_BASENAMES` exists for exactly that, with the comment
*"installed by previous versions of the daemon … before the plugin migration"*.

Six tests then still asserted that configure WRITES it, so they could not pass:

| spec | failures |
|---|---:|
| `configure-assistants` | 4 |
| `daemon-verification` (assistants stage) | 1 |
| `assistants-configure` | 1 |

**Done looks like:** they assert the CURRENT contract — after configure, the
marketplace is registered and the `sensei` plugin is installed (observable via
`claude plugin list` / `installed_plugins.json`) — and the removal half asserts
the plugin is uninstalled rather than a file deleted.

### D4 — ~~`app/e2e/**` is outside every static gate~~ — closed in the gate (18eb0cde); #187 waits on the specs

The gate landed: `app`'s `check` runs `tsc --noEmit -p tsconfig.e2e.json` (`app/package.json:11`).
#187 stays open only for its other half — the changed specs passing against the app.

`tsconfig.json` extends `.svelte-kit/tsconfig.json`, whose `include` covered
`src/` only, so `bun run check` and `tsc --noEmit` never looked at the Playwright
suite. A probe config surfaced **9 pre-existing type errors** in it.

**Done looks like:** an `e2e` workspace in the check script, and those 9 fixed.

### D5 — ~~37 commits have never been pushed~~ — resolved

Everything this log dates is on `origin/develop` now — the tip of that branch on 2026-11-03 is
`5f8b3b886`, a merge of this lane's own docs work, and the release train has run since (v0.21.0,
v0.21.1).

    git status -sb  →  ## develop...origin/develop [ahead 37]

Nothing has reached the remote, which is why `gh run list --branch develop`
returns nothing. Every green gate in this session was local.

---

## E. Dōjō — the sync (the repository and metrics path works against the local dōjō since 2026-10-20; membership and the outbox never did — E1 — and production is unverified)

### E1 — enrollment was never completed

    select sync_status, last_heartbeat_at, last_seq from sensei.dojo_memberships;
    -- authenticating | never | 0

    select state, created_at, last_attempt_at from sensei.dojo_outbox;
    -- held | 2026-08-29 | 2026-08-29

    select count(*) from sensei.dojo_outbox where state='sent';   -- 0

**Zero rows have ever been sent.** The one outbox row has sat `held` for a
month. The membership never got past `authenticating`.

### E2 — ~~the daemon dials IPv4, Vite binds IPv6~~ — closed (#185)

`bun run dev` binds `localhost:5173` as `::1` only. `reqwest` resolves
`localhost` to `127.0.0.1`, so `dojo_sync` could never connect:

    could not reach dōjō: error sending request for url (http://127.0.0.1:5173/v1/auth/cli/refresh)

Worked around by starting Vite with `--host 127.0.0.1`; `dojo_sync` went green
immediately. **Nothing in the repo records this** — the next person hits it.

**Done looks like:** the dojo dev script binds IPv4 explicitly, or the daemon
retries the other family. Plus a note in `dojo/CLAUDE.md`.

### E3 — ~~no local-dojo setup path is written down~~ — closed (#186)

`supabase start` + a dev server on :5173 + a membership row is the whole
prerequisite chain for testing sync, and it is folklore. `make supabase-up`
exists; nothing says what to do after it.

---

## F. Dependencies — closed 2026-11-03

Re-measured 2026-11-03 (`npm view` for each, plus the lockfiles):

| package | manifest | resolved | latest | behind |
|---|---|---|---|---|
| app `@rokkit/*` | `^1.14.0` | 1.14.0 | 1.14.0 | — |
| dojo `@rokkit/*` | `^1.14.0` | 1.14.0 | 1.14.0 | — |
| dojo `@kavach/*` | `1.3.2` exact | 1.3.2 | 1.3.2 | — |

The direct dependencies are current — up from the 2026-09-25 measurement (app 1.4.0, dojo 1.3.4,
kavach 1.1.3) — and so is the nested copy that was the rest of F. `@kavach/ui@1.3.0` pinned
`@rokkit/forms` **1.3.7**, which nested `@rokkit/ui`/`states`/`data`/`core` **1.4.0** beside the 1.14.0
direct entries; `eefc6c119` moved every `@kavach/*` to **1.3.2**, and kavach 1.3.2 depends on
`@rokkit/forms` **1.14.0** (`dojo/bun.lock:254`), so `bun.lock` now resolves every `@rokkit/*` package
once, at 1.14.0, with no 1.4.0 entry left (the family is the one block at `:322-342`). F is closed;
#191 is closed and D-12 is resolved.

**28 osv advisories** remain tracked in [#120], all `medium`, all transitive,
none code-fixable — the Tauri Linux GTK stack (not compiled into the shipped
macOS `.app`), `quick-xml`, `unic-*`, `rsa 0.9.10`.

---

## G. Documentation drift

### G1 — ~~the backlog said 12 open issues; there are 67~~ — resolved 2026-11-03

The hand-maintained count is gone: [`backlog.md`](../backlog.md) now indexes open work by area,
names the issues the week's plan depends on, and freezes no total. 90 issues were open on
2026-11-03 (`gh issue list --state open`), which is GitHub's number to keep.

    gh issue list --state open --limit 100  →  67

`backlog.md` was cleaned this session (1,718 → 1,242 lines, nine closed sections
removed) but its "Open GitHub issues (12)" table is still a hand-maintained copy
of a live list. Either generate it or drop it.

### G2 — ~~`sensei.error_signature` is referenced and does not exist~~ (resolved: mistaken premise)

It exists as a **function**, `sensei.error_signature(text)` (since c6c1a206),
backing `activity.task_health` and `activity.task_failures`; it was queried as a
table. Closed as #189 on 2026-10-18.

---

## H. Not blockers, but decide before shipping — one bullet left (Swift)

- ~~**SQL is built and not flipped.** Both readers exist and cover 86% of 7,435 files; flipping takes
  the other ~1,013 (`Unstated` dialect) out of the graph.~~ Closed: SQL is a cutover, in
  `PRODUCTION_LANGUAGES` (`indexer/lang/mod.rs:504`), and its dialect is established from the manifest
  before a file is read (`:499-503`).
- ~~**Kotlin and Swift have no v2 adapter**, so `languages/` cannot be deleted and two indexers' worth
  of code ships.~~ Half closed: Kotlin has an adapter (`indexer/lang/kotlin/mod.rs`, listed at
  `indexer/lang/mod.rs:523`), and the second registry is gone — `languages/` is `fqn.rs`,
  `import_target.rs` and `mod.rs`, whose own header says it parses nothing (`languages/mod.rs:3-9`),
  so there is no second indexer left to delete. **This is H's remaining bullet:** there is no
  `indexer/lang/swift/` and no `Language::Swift` (`indexer/lang/mod.rs`), so Swift is classified by
  path but nothing indexes it.
- ~~**The differential harness was waived**, so acceptance after the wipe is the ONLY empirical check
  that v2 is not a regression against v1.~~ Closed as a decision, not a gap:
  `indexer/quality/acceptance.rs:4-9` records that thresholds replaced the differential harness, and
  why — agreement with the producer being replaced is a question about a transition, not about
  whether the graph is any good.
- ~~**Indexing has never fully completed**: 380 folders `discovered`, 12 `indexing`.~~ Closed: 889
  folders, every one `indexed`, none `discovered` or `indexing` (`select status, count(*) from
  sensei.folders`, live DB, 2026-11-03).

---

## Suggested order

This order was written 2026-09-25 and is kept as it was given. Checked 2026-11-03, its first, second,
third and fifth items are closed and item 4's E2/E3 are too, so what it still points at is **E1**
(#184), **A4** (#146) and **H**'s Swift bullet.

1. **A1** — it is failing files today and the fix shape is already written one
   arm above it.
2. **D1 + D2** — nothing else is trustworthy while the main suite runs only by
   hand. D2 first, or D1 lands red.
3. **C1** — 68% of the database, and it makes every other operation slow.
4. **E1/E2/E3** — dōjō is a headline capability that has never moved a row.
5. **B1** — the cheap half is done; the bound is what makes it a rule.
6. Everything else.
