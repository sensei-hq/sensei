---
name: Implementation Backlog
description: OPEN work — tracked as GitHub issues (sensei-hq/sensei); the issue list is the source of truth
date: 2026-10-13
---

# Implementation Backlog

Work is tracked as **GitHub issues** in
[`sensei-hq/sensei`](https://github.com/sensei-hq/sensei/issues) — the live
issue list is the source of truth. **This file is an index of references, not a
database** (decision 9, docs reorg 2026-10-13): open → referenced here, filed →
an issue, shipped → the issue closes and the row goes away.

The full write-ups of past findings (indexer defects, dbd defects, with their
evidence) are preserved in [`history/backlog/writeups.md`](history/backlog/writeups.md).

## Open issues (as of 2026-10-13)

> Counts and statuses come from the issue list — re-open this table when it
> drifts from GitHub, or drop it and link the filtered query instead.

### Epics
| Issue | Summary |
|-------|---------|
| [#91](https://github.com/sensei-hq/sensei/issues/91) | Dōjō governance track — admin site + policies + preferences + skills/agents |
| [#85](https://github.com/sensei-hq/sensei/issues/85) | Track 3 — Project window (per-screen, separate Tauri window) |

### Observatory · Instruments
| Issue | Summary |
|-------|---------|
| [#96](https://github.com/sensei-hq/sensei/issues/96) | Instruments: background-task visibility — shipped; e2e verify remaining |
| [#90](https://github.com/sensei-hq/sensei/issues/90) | Instruments Replay: usage-verdict classifier — shipped; blocked on Replay screen |
| [#43](https://github.com/sensei-hq/sensei/issues/43) | Observatory Configure section (design + build) |
| [#44](https://github.com/sensei-hq/sensei/issues/44) | J7 Extend & Customize screens (design + build) |
| [#45](https://github.com/sensei-hq/sensei/issues/45) | J5 Pattern knowledge catalog |
| [#46](https://github.com/sensei-hq/sensei/issues/46) | J9 Context-pack tool |

### UI / quality / site
| Issue | Summary |
|-------|---------|
| [#47](https://github.com/sensei-hq/sensei/issues/47) | Mockup component migration to Rokkit (steps 4–14) |
| [#81](https://github.com/sensei-hq/sensei/issues/81) | Update website `/sensei` route to the new mockup |

### Foundation / deferred
| Issue | Summary |
|-------|---------|
| [#39](https://github.com/sensei-hq/sensei/issues/39) | Bootstrap diagnostic logging + debug mode |
| [#50](https://github.com/sensei-hq/sensei/issues/50) | Extract `bootstrap` into a reusable library (deferred) |
| [#258](https://github.com/sensei-hq/sensei/issues/258) | SQL indexer: adopt dbd 0.26's v3 schema model — type uses, identity/generated, partial indexes, checks |

## Open — not yet filed (file as GitHub issues)

Full one-liners and prior context: [`history/backlog/writeups.md`](history/backlog/writeups.md).

- **A branch's DDL change never reaches the dev database on start** — the boot
  deploy (`tasks/schema_deploy.rs`) runs only when the binary's version differs
  from `sensei.config.schema_deployed_version`, so with `SENSEI_DDL_DIR` set (the
  process-compose daemon) a new column on the same version is skipped and every
  query naming it fails (hit 2026-10-22: F11's `sessions.claude_time` → 500 on the
  Tickets screen until a manual `dbd reconcile`). `process-compose.yaml` says the
  opposite. Deploy whenever `SENSEI_DDL_DIR` is set, or key the stamp on the DDL's
  content hash.

- **The daemon's startup schema upgrade fails on any release that adds a
  commented column** — `sensei_bootstrap::database::deploy_to` runs `apply`
  (which re-runs each table's DDL, its `comment on column` included) BEFORE the
  `reconcile` that adds the column, so the comment hits a column that isn't
  there yet and the whole upgrade aborts ("SCHEMA DEPLOY FAILED … retried next
  start" — forever). Hit twice on one install: `sessions.list_cost_usd`
  (0.13 → develop) and `schedules.runs` (0.13 → 0.15.0); both cleared only by a
  manual `dbd reconcile --allow-destructive` and a restart. Every upgraded
  install meets it. Fix in the deploy order (reconcile the tables' columns
  before re-applying their comments), with a `deploy_upgrade.rs` test that adds
  a commented column.
  Until then `sessions.claude_time` and `sessions.user_time` (tracker F11) carry
  inline `--` comments only; give them `comment on column` with the fix.

- **Library capability provisioning + version-aware doc queries** — nothing
  installs a library's declared skills/agents; `get_lib_docs` lacks a `version`
  override.
- **Metrics engine rebuild + Dōjō sync (v1 = local; sync rides Phase 2)** —
  repo-grain watermark engine; current spec draft at `spec/2026-08-18-…`.
- **Operating-model Phase 2 (front-door intake conversation + playbook catalog).**
- **Codebase-wide silent-error audit** (directed after the `node_kind` drop) → roadmap gaps `F-*`.
- **Stale / orphaned project cleanup** — list endpoints + gated purge + housekeeping UI.
- **Activity-data GC** — TTL prune after analysis has derived insights.
- **Credentials — retire the legacy macOS `/usr/bin/security` backend** (decided
  with the user 2026-10-13). `gateway_keys` now uses the `keyring` crate
  (Keychain / Credential Manager / Secret Service); the old shell-out failed on
  Linux with `No such file or directory`. `secret-tool` was rejected (Ubuntu does
  not ship `libsecret-tools`; Windows is planned). **Deliberately retained:**
  `gateway_keys/macos_security.rs`, selectable with
  `SENSEI_CREDENTIAL_BACKEND=security` as a rollback — both address the same
  login-Keychain items. **To delete it:** confirm on a Mac with existing items
  that keyring reads them without a per-item access-prompt storm (or add a
  one-time re-write migration), then remove the module, the `Backend` enum and
  the env var.

- **Jira ticket linkage — local slice (T-1a + T-1d, local-first)** — analysis
  [`analysis/2026-10-15-jira-ticket-linkage.md`](analysis/2026-10-15-jira-ticket-linkage.md),
  spec [`spec/daemon/tracker-jira-linkage.md`](spec/daemon/tracker-jira-linkage.md).
  **Deliberate deviations from T3, decided 2026-10-15:** (1) local daemon first,
  Dōjō rollup behind F-3; (2) link sources `branch` + `prompt` only — `commit`
  deferred (needs new capture, contradicting T3 §2.5's "no new capture"), enum
  value reserved; (3) no assignee synced/stored (no per-person figure);
  (4) T3 §5.1 `sensei.tracker_links` dropped — repo binding is the existing
  `sensei.repositories_in_projects` junction, sync status lives on
  `sensei.trackers`; (5) per-repo key pattern and per-source toggles deferred to
  T-1c — keys match against synced issue keys instead; (6) API-token auth only —
  OAuth (integrations §2.3) deferred; (7) one tracker per project in v1 — T3
  §2.2's secondary issue-only trackers deferred. Blocked on **P0**: session
  synthesis + transcript ingest on Linux (0 sessions today).
- **Tracker metrics write-back** (2026-10-26). Analysis
  [`analysis/2026-10-26-tracker-metrics-writeback.md`](analysis/2026-10-26-tracker-metrics-writeback.md),
  spec [`spec/daemon/tracker-metrics-writeback.md`](spec/daemon/tracker-metrics-writeback.md),
  plan [`plans/2026-10-26-tracker-metrics-writeback.md`](plans/2026-10-26-tracker-metrics-writeback.md).
  **It reverses, with the user's approval (2026-10-26),** the read-only rule (PRD T3,
  `decisions.md` "Read-only means read-only", design §2.6) for usage metrics, and
  deviation (3) above: the assignee is compared, never stored. Write-back is on by default
  for existing boards too, which the user confirmed after being shown the "Read-only" copy
  and that clients may see comments. The docs are narrowed in F5.
- **Daemon routes reachable from any web page** (found by the Jira-spec security
  review 2026-10-15) — `api/server.rs` sets `CorsLayer::allow_origin(Any)` with all
  methods and no auth middleware (loopback bind by default), so any page the user
  visits can call `127.0.0.1:7744` and read most responses. One route is guarded
  (`PATCH /api/repositories/{repo_key}`, `origin_may_mutate`); the Jira slice
  extracts that guard to `api/origin_guard.rs`, adds a loopback-`Host` check (DNS
  rebinding) and applies both to its own routes. Daemon-wide: apply the shared
  guard to every mutating and private-data route.
- **Stale DDL comment** — `database/ddl/table/sensei/repositories.ddl:24-25` still
  says only the repo-root folder carries `repository_id` (I16); `folders.ddl`
  records that D-REPO-ANCHOR (2026-10-01) superseded it — every folder carries it.

- **GitLab tracker adapter** (next after Jira; seam in
  [`spec/daemon/tracker-jira-linkage.md`](spec/daemon/tracker-jira-linkage.md) "Future
  adapters") — decide: iterations (Premium) vs milestones as sprints; `#12` key
  resolution scoped by the session's repo; self-managed host allow-rule (SSRF
  review); PAT vs OAuth.

- **Rust indexer miscounts `impl Trait for &Type`** (found 2026-10-16 by the
  tracker slice) — `indexer::lang::rust::tests::the_symbol_count_equals_an_independent_count_of_declaration_nodes`
  scans this repo as its corpus; a test-module `impl RawGet for &Scripted` made the
  walk produce one more symbol than the independent count (33 vs 32). Confirmed by
  bisection: the same code behind a named wrapper (`impl RawGet for Ref<'_>`)
  agrees. `tracker/transport.rs` tests use that wrapper as a **workaround**;
  decide which side is wrong (walk or counter) and fix it, then the wrapper can go.

- **Indexer corpus tests flake under full-suite load** (found 2026-10-17) —
  `indexer::pipeline::corpus::{a_scanned_repository_records_its_project_in_the_junction,
  every_stage_reaches_the_progress_stream}` pass alone and with every tracker test,
  fail ~1 in 2 full runs (scan finds no repo in a fresh tempdir; or a written repo is
  not linked). Ruled out: env mutation, scan config, fd limit, Postgres connection
  exhaustion. Next: make the assertions print the scan's `unreadable`/`errors`.

- **Incremental tracker relink** (deferred from F3, 2026-10-17) — v1 recomputes
  every candidate session's links on each sync (chunked, prompts capped at 4 KB).
  Next: a per-tracker watermark (sessions with turns newer than the last relink),
  with a full recompute only when the known-key prefix set changes. Measure
  `sum(length(user_text))` per project once transcripts are ingested.

## Where open work is described

| Concern | Source |
|---|---|
| Issue-level work | this file (above) → GitHub |
| Per-module status and the next step | [`roadmap/`](roadmap/README.md) |
| Every known gap, IDed, with evidence | [`roadmap/06-gaps.md`](roadmap/06-gaps.md) |
| Detailed past findings | [`history/backlog/writeups.md`](history/backlog/writeups.md) |