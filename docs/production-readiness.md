---
name: Production readiness
description: Measured gaps between what ships today and what shipping to other people requires
date: 2026-09-25
---

# Production readiness

Every item below is a MEASUREMENT taken on 2026-09-25 against the live daemon,
the live `sensei` database and this working tree — not a checklist copied from
somewhere. Where a number is quoted, the query that produced it is quoted with
it, so a later reader can re-run it rather than trust it.

Ordered by what would hurt most if a second person installed this today.

---

## A. Things that are silently wrong

### A1 — `adopt_node_by_identity` fails a whole file when its row moved

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

### A2 — cost is never captured, so every cost metric is derived or absent

    select count(*), count(metered_cost), count(nullif(metered_cost,0)), max(metered_cost)
      from activity.sessions;
    -- 376 | 23 | 0 | 0

`activity.sessions.metered_cost` is **never non-zero**. `Cost per result` is
therefore computed from a configured subscription fee, and exists for 2 of 15
projects. There is no measured spend anywhere in the system.

**Done looks like:** either populate `metered_cost` at ingest from the
assistant's own usage report, or retire the column and say plainly that cost is
modelled from a subscription figure — the current state reads like measurement
and is not.

### A3 — `Tokens per day` is ~all cache reads, so it cannot proxy cost

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

### B1 — the parse stack is unbounded: one file aborts the daemon

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

### B2 — watchdog abandonment is routine, not exceptional

    process_git_folder … exceeded 600s and was abandoned   (4 in 7d)
    process_file        … exceeded 180s and was abandoned   (~8 in 7d)

Small numbers, but each one is a folder or file that silently did not finish.
Nothing reports them as a class.

**Done looks like:** a health surface that counts watchdog abandonment per day
and names the worst offenders, so "slow" is visible before it becomes "missing".

---

## C. Operations — the machine fills up

### C1 — `public.logs` is 25 GB of a 37 GB database

    select schemaname||'.'||relname, pg_size_pretty(pg_total_relation_size(relid))
      from pg_catalog.pg_statio_user_tables order by 2 desc limit 3;
    -- public.logs 25 GB | sensei.edges 4094 MB | sensei.nodes 4000 MB

**68% of the database is logs.** `log_prune` runs daily and is not keeping up.
This is also why `pg_dump` now takes ~12 minutes and produces a 5.3 GB file.

**Done looks like:** a retention that holds, measured — and the structured log
either partitioned by day (so a prune is a `DROP PARTITION` rather than a
`DELETE`) or moved out of the database that gets dumped before every install.

### C2 — every install writes a 5.3 GB backup, and they accumulate

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

### D1 — CI runs no Rust tests, no clippy, no fmt

`.github/workflows/` holds four workflows:

| workflow | trigger | what it runs |
|---|---|---|
| `coverage.yml` | push to main/develop, PR | `bun run test:unit` (app), `bun run test` (dojo) |
| `release.yml` | tag `v*` | release build + assets |
| `cleanup-workflow-runs.yml` | cron | housekeeping |
| `qlty-plugin-bump.yml` | cron | housekeeping |

**The 3,317-test senseid suite runs in no workflow.** The pre-commit hook runs
`sensei-bootstrap` (174) and the app unit tests (1,698) — not senseid. So the
suite that covers the indexer, the daemon, the task engine and the store is
run only when someone remembers to run it by hand.

**Done looks like:** a `rust.yml` running `cargo test -p senseid --bin senseid`,
`cargo clippy -- -D warnings` and `cargo fmt --check` on push and PR, with a
Postgres service so the DB-coupled half is covered (see D2).

### D2 — DB-coupled tests are not isolated and fail under parallel load

Known and recorded: `publish_run::full_bridge_*` assert absolute row counts
polluted by concurrent writers; `metrics::session_outcomes::ftr_parity_*`,
`library_update_scheduler::security_bump_*` and
`version_rescan::rescan_is_crash_safe_*` fail only under parallel load. They
pass in isolation.

This is the blocker for D1 — a CI Postgres service would surface these
immediately.

**Done looks like:** per-test schema or transaction isolation, or a serialized
DB-test lane.

### D3 — the e2e suite is 23/130 red, and its first run could not start at all

**Measured 2026-09-25: 107 passed, 23 failed, 20 skipped, 19.5 minutes.**

| spec | failures |
|---|---:|
| `configure-assistants` | 4 |
| `boot-flow` | 4 |
| `instruments-t2-slices` | 3 |
| `daemon-verification`, `atlas` | 2 each |
| `zz-a11y`, `multi-window`, `knowledge-sources`, `instruments-observatory`, `dojo-binding`, `db-setup`, `assistants-configure`, `activity-logs` | 1 each |

**The first run produced nothing at all**: `Port 7744 did not open within
240000ms`, zero specs. `make test-app-e2e` drops `sensei_e2e` for a clean slate,
so that run had to CREATE the database and apply 123 tables before the daemon
could bind. It did — `sensei_e2e` exists, created 08:39:15 with 123 tables — and
then ran out of the 240s budget. The second run found the database already
provisioned and booted immediately.

So the timeout is sized for a warm boot and the harness guarantees a cold one.

**And it was undiagnosable**: `globalSetup.ts` spawned the app with
`stdio: 'ignore'`, so the app's stdout and stderr — bootstrap's health
resolution, the daemon spawn, any panic — were discarded. Fixed in this session:
output goes to `/tmp/sensei-e2e-app.log` and the timeout path prints its tail,
because a log nobody prints is a log nobody reads.

**Done looks like:** the cold-provision path gets its own budget (or globalSetup
waits for the DB to exist before starting the port clock), and the 23 failures
are triaged — several look like one cause (`boot-flow` + `db-setup` +
`daemon-verification` are all bootstrap-gate screens).

### D4 — `app/e2e/**` is outside every static gate

`tsconfig.json` extends `.svelte-kit/tsconfig.json`, whose `include` covers
`src/` only, so `bun run check` and `tsc --noEmit` never look at the Playwright
suite. A probe config surfaces **9 pre-existing type errors** in it.

**Done looks like:** an `e2e` project in the check script, and those 9 fixed.

### D5 — 37 commits have never been pushed

    git status -sb  →  ## develop...origin/develop [ahead 37]

Nothing has reached the remote, which is why `gh run list --branch develop`
returns nothing. Every green gate in this session was local.

---

## E. Dōjō — the sync has never worked

### E1 — enrollment was never completed

    select sync_status, last_heartbeat_at, last_seq from sensei.dojo_memberships;
    -- authenticating | never | 0

    select state, created_at, last_attempt_at from sensei.dojo_outbox;
    -- held | 2026-08-29 | 2026-08-29

    select count(*) from sensei.dojo_outbox where state='sent';   -- 0

**Zero rows have ever been sent.** The one outbox row has sat `held` for a
month. The membership never got past `authenticating`.

### E2 — the daemon dials IPv4, Vite binds IPv6 (FIXED in-session, not in code)

`bun run dev` binds `localhost:5173` as `::1` only. `reqwest` resolves
`localhost` to `127.0.0.1`, so `dojo_sync` could never connect:

    could not reach dōjō: error sending request for url (http://127.0.0.1:5173/v1/auth/cli/refresh)

Worked around by starting Vite with `--host 127.0.0.1`; `dojo_sync` went green
immediately. **Nothing in the repo records this** — the next person hits it.

**Done looks like:** the dojo dev script binds IPv4 explicitly, or the daemon
retries the other family. Plus a note in `dojo/CLAUDE.md`.

### E3 — no local-dojo setup path is written down

`supabase start` + a dev server on :5173 + a membership row is the whole
prerequisite chain for testing sync, and it is folklore. `make supabase-up`
exists; nothing says what to do after it.

---

## F. Dependencies

| package | manifest | installed | latest | behind |
|---|---|---|---|---|
| app `@rokkit/*` | `^1.4.0` | 1.4.0 | 1.6.0 | 2 minors |
| dojo `@rokkit/*` | `^1.2.0` | **1.3.4** | 1.6.0 | 3 minors |
| dojo `@kavach/*` | `1.1.3` exact | 1.1.3 | 1.2.0 | 1 minor |

The carets already permit the newer rokkit; the lockfiles are holding it back.
Kavach is pinned exact and needs a manifest edit.

**28 osv advisories** remain tracked in [#120], all `medium`, all transitive,
none code-fixable — the Tauri Linux GTK stack (not compiled into the shipped
macOS `.app`), `quick-xml`, `unic-*`, `rsa 0.9.10`.

---

## G. Documentation drift

### G1 — the backlog said 12 open issues; there are 67

    gh issue list --state open --limit 100  →  67

`backlog.md` was cleaned this session (1,718 → 1,242 lines, nine closed sections
removed) but its "Open GitHub issues (12)" table is still a hand-maintained copy
of a live list. Either generate it or drop it.

### G2 — `sensei.error_signature` is referenced and does not exist

The checkpoint skill names it as the way failures are grouped by cause. It is
not in the database.

---

## H. Not blockers, but decide before shipping

- **SQL is built and not flipped.** Both readers exist and cover 86% of 7,435
  files; flipping takes the other ~1,013 (`Unstated` dialect) out of the graph.
- **Kotlin and Swift have no v2 adapter**, so `languages/` cannot be deleted and
  two indexers' worth of code ships.
- **The differential harness was waived**, so acceptance after the wipe is the
  ONLY empirical check that v2 is not a regression against v1.
- **Indexing has never fully completed**: 380 folders `discovered`, 12
  `indexing` as of this writing.

---

## Suggested order

1. **A1** — it is failing files today and the fix shape is already written one
   arm above it.
2. **D1 + D2** — nothing else is trustworthy while the main suite runs only by
   hand. D2 first, or D1 lands red.
3. **C1** — 68% of the database, and it makes every other operation slow.
4. **E1/E2/E3** — dōjō is a headline capability that has never moved a row.
5. **B1** — the cheap half is done; the bound is what makes it a rule.
6. Everything else.
