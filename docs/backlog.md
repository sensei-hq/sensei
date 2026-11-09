---
name: Implementation Backlog
description: OPEN work — tracked as GitHub issues (sensei-hq/sensei); the issue list is the source of truth
date: 2026-11-03
---

# Implementation Backlog

Work is tracked as **GitHub issues** in
[`sensei-hq/sensei`](https://github.com/sensei-hq/sensei/issues) — the live
issue list is the source of truth. **This file is an index of references, not a
database** (decision 9, docs reorg 2026-10-13): open → referenced here, filed → an
issue, shipped → the issue closes and the row goes away. It names *what* is open
per area and *which lane* holds it this week; it does not carry a count. The last
count kept here (2026-10-13, "about 14 issues") was 87 issues wrong by 2026-10-27
(`review/2026-10-27-roadmap-vs-state.md` §0.2). Ask GitHub, not this file:
`gh issue list --state open`.

The full write-ups of past findings (indexer defects, dbd defects, with their
evidence) are preserved in [`history/backlog/writeups.md`](history/backlog/writeups.md).

## Open work by area (2026-11-03)

Read from the readiness table in
[`review/2026-11-03-orientation.md`](review/2026-11-03-orientation.md) §2. The last column is the lane
in [`plan/2026-11-03-parallel-lanes.md`](plan/2026-11-03-parallel-lanes.md) §2 that owns the work this
week; `—` means no lane holds it yet. Per-module status and its evidence:
[`roadmap/`](roadmap/README.md).

| Area | Ready | What is built | Biggest gaps | Lane this week |
|---|---:|---|---|---|
| Onboarding / health | ~75% | bootstrap, the 5-step setup, Health + Remedy, Developer ID | first run never mentions the dōjō; #282 Keychain re-asked on rebuilt binaries; #300 / #301 sign-in | L4 (setup step, menus) |
| Diagrams (local) | ~85% | S0–S4, S3 ER, S7 focus (committed) | S4 2c pooling double count; S6 history (proposal); the dōjō side (WP6/7) 0% | L2; dōjō side not this week |
| Metrics engine | ~75% | daily→quarterly views, the perf work, My metrics, team views | ActionItems has no action (0 buttons/links); #156 architecture metrics missing | L3 (ActionItems), L2 (#156) |
| Metrics as **targets** | ~0% | nothing (verified) | gating eligibility (`metric_gating`, TG1), a target store and mode, evaluation, a failure flow (D47/D48; M-7) | L0 (spec), then L3 |
| Sync (WP1–9) | ~40% | WP1–3, WP4 staged | WP4 F4, WP5, WP8 (D46), WP9A set-based registration, #314 production verify; #184 never sent a row | L1 |
| Sync hardening (L3 F8) | — | stored:None complement marks + warns once per cycle | remove the complement when every supported dōjō returns `stored` (then the warn and this row go away) | L3 |
| Governance | ~50% | scope resolution, rule packs + adoption, resolved rules pulled | no violation/override store (G-GOV-AUDIT); no "what adopting changed here"; team scope #318 | — |
| Learnings | ~60% | memories, Learnings screen, share-review, k ≥ 3 promotion, playbook rules | no `report_learning` / `report_memory_use`; no failure → lesson loop; too many competing verbs (D65) | L1 (after #314), L0 (spec) |
| Trackers / sprints | ~40% | Jira, GitLab and GitHub Projects (daemon, D91, D92), boards per level (D40), sprints, Tickets screen | ADO adapter, dōjō-owned connections (D49, behind #310); tracker-ownership conflict; epics unread | L7 |
| Teams / viewers | ~70% | roles, teams, invites, viewers, lead view | no viewer shell (D68); leads have no menu of their own; A4 writable workspaces | L4 |
| Billing / plans | ~15% | `billing_accounts`, `seats`, `ScrBilling` | no provider, no subscription table; D46 entitlement is WP8; price to be announced (D75) | L4 (screen), L1 (WP8) |

## Issues this week's plan depends on

Titles as they read on GitHub on 2026-11-03, all open. Nothing here states a
status — open or closed is GitHub's answer.

| Issue | Title | Why it is in this list |
|---|---|---|
| [#282](https://github.com/sensei-hq/sensei/issues/282) | Releases are ad-hoc signed: keychain and folder permissions are asked again after every upgrade | every upgrade breaks sign-in and sync; the orientation's day-1 item |
| [#301](https://github.com/sensei-hq/sensei/issues/301) | Sign in → sync must just work: auto-elect a connected identity's repositories, and survive upgrades | the sync lane's entry point |
| [#300](https://github.com/sensei-hq/sensei/issues/300) | Sign-in from the app fails for passkey users | the readiness table's onboarding gap |
| [#314](https://github.com/sensei-hq/sensei/issues/314) | Production: nothing in the dōjō plane has been verified in production — sync, Relay, push | the sync lane's last item (WP9A first) |
| [#184](https://github.com/sensei-hq/sensei/issues/184) | Dōjō sync has never sent a row: membership stuck at 'authenticating', outbox held since 2026-08-29 | the upward path; decide write `sync_status` or retire the outbox |
| [#240](https://github.com/sensei-hq/sensei/issues/240) | Dōjō · the 25 user-scoped metrics have no path to the dōjō plane | metrics reach the dōjō only through repositories |
| [#310](https://github.com/sensei-hq/sensei/issues/310) | Security: the dōjō needs a secret store before it holds shared integration credentials | blocks D49's tracker adapters |
| [#318](https://github.com/sensei-hq/sensei/issues/318) | Code lags the decisions: developer role, Create or join, self_hosted, one tracker per project | the governance area's team scope |
| [#156](https://github.com/sensei-hq/sensei/issues/156) | No metric reads the code graph: propose an architecture family | unblocks 4 of T3's 7 target families |
| [#158](https://github.com/sensei-hq/sensei/issues/158) | Per-metric confidence: every reading should carry how much it can be trusted | with #159 and #160, the confidence chain |
| [#159](https://github.com/sensei-hq/sensei/issues/159) | Registry: add a method tier (deterministic \| heuristic \| llm \| derived) | the base #160 builds on |
| [#160](https://github.com/sensei-hq/sensei/issues/160) | Add a `metric_confidence` view and weight composite health by it | closes the coverage inversion |
| [#292](https://github.com/sensei-hq/sensei/issues/292) | Indexer v2 has no markdown adapter, so doc drift never scans a document | M11 is inert without it |
| [#279](https://github.com/sensei-hq/sensei/issues/279) | EPIC (deferred) · Benchmark: re-design proving sensei's value on transcript history | M18's deferred half — deferred, not dropped |
| [#316](https://github.com/sensei-hq/sensei/issues/316) | CI and DORA data need a puller that does not depend on a daemon being online | T5's prerequisite |
| [#234](https://github.com/sensei-hq/sensei/issues/234) | A seeded rule pack cannot be adopted — opt-in means unreachable | still open; check the adoption path before closing |

## Open — not yet filed

- **Known limitation: two windows can race on the single `ui.views` config entry (D87 panel store, 2026-11-04).** The last save
  wins, so a panel toggled in another window at the same moment can be lost. Fix if it's ever seen: per-view keys, or
  compare-and-set on the daemon.

- **Surface `no_package` files as actionable (2026-11-04).** "N files aren't in any package; add a manifest to map them",
  per repository. It follows L1's stuck-index fix.

- **`plan_folders` has no production caller (indexer, 2026-11-04, L1 module-tree analysis).** It says the deepest folder owns
  a file, but the production `repo_scan` puts every file on the root. Either wire it in or delete it. The module tree doesn't
  need it, since it joins on the fqn package.
- **Svelte file nodes are `module`, not `component` (indexer, 2026-11-04).** It's queued after the stuck-index item.

- **Drop `sensei.repositories.visibility` and `repo_visibility` (WP8, 2026-11-04).** WP8 retires the column as a gate; nothing
  reads it after L2's daemon side lands. **Trigger:** the desktop app's sync toggle reads `configurable_by_me` (L2, WP8 daemon
  side). Then drop the column and the enum in one dbd reconcile, deploying twice.

One-liners with prior context: [`history/backlog/writeups.md`](history/backlog/writeups.md). This list is
a **2026-10-13 snapshot and has not been re-checked**; check the code before filing anything from it,
and file what is still true as an issue.

- **Copilot token/cost zeros stored before 07a9def0e may be unknown** (checked 2026-11-04).
  `07a9def0e` stores `None` where the transcript reports no token or cost figure; rows written before it
  hold `Some(0)`, which a reader cannot tell apart from a real zero. The source cannot be re-read to
  repair them.
- **The Rust indexer does not follow `pub use` re-exports.** A use through a re-exported path
  (`pg_store::X` for an item declared in `pg_store::x`) names an identity no declaration mints, so it
  dangles in the code graph (`indexer::resolve` ratchet). It is cross-file resolution, stage-12 work. The
  store's row types were moved instead (decision 2026-10-26, #161); the other re-exports remain.
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
  **In progress:** the sync work packages WP1–9 are this week's L1 lane
  (`plan/2026-11-03-parallel-lanes.md` §L1).
- **Tracker delivery metrics (#351–#356)** — built on `feature/delivery_metrics`: the app's
  Delivery Metrics section, board facts and per-person effort pushed to the Dōjō and merged
  there, per-person visibility, withdrawal (`spec/daemon/tracker-delivery-metrics.md`,
  decisions D87–D90). **In review:** awaiting the PR to develop.
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
  **Built:** the daemon-side Jira adapter is in (PR #274;
  `review/2026-10-27-roadmap-vs-state.md` §1.4 T-1); the deviations below still
  describe it. **Deliberate deviations from T3, decided 2026-10-15:** (1) local daemon first,
  Dōjō rollup behind F-3; (2) link sources `branch` + `prompt` only — `commit`
  deferred (needs new capture, contradicting T3 §2.5's "no new capture"), enum
  value reserved; (3) no assignee synced/stored (no per-person figure);
  (4) T3 §5.1 `sensei.tracker_links` dropped — repo binding is the existing
  `sensei.repositories_in_workspaces` junction, sync status lives on
  `sensei.trackers`; (5) per-repo key pattern and per-source toggles deferred to
  T-1c — keys match against synced issue keys instead; (6) API-token auth only —
  OAuth (integrations §2.3) deferred; (7) one tracker per workspace in v1 — T3
  §2.2's secondary issue-only trackers deferred (#318 tracks the decision drift).
- **Tracker metrics write-back** (2026-10-26). Analysis
  [`analysis/2026-10-26-tracker-metrics-writeback.md`](analysis/2026-10-26-tracker-metrics-writeback.md),
  spec [`spec/daemon/tracker-metrics-writeback.md`](spec/daemon/tracker-metrics-writeback.md),
  plan [`plans/2026-10-26-tracker-metrics-writeback.md`](plans/2026-10-26-tracker-metrics-writeback.md).
  **It reverses, with the user's approval (2026-10-26),** the read-only rule (PRD T3,
  `decisions.md` "Read-only means read-only", design §2.6) for usage metrics (superseded 2026-10-29 (tracker-sdlc-metrics)), and
  deviation (3) above: the assignee is compared, never stored. Write-back was on by default
  for existing boards too, which the user confirmed after being shown the "Read-only" copy
  and that clients may see comments. The docs are narrowed in F5.
  **Since 2026-11-03 (D50) it is off by default:** a workspace opts in; boards that
  already had it keep their setting (the default applies to new rows).
- **Tracker SDLC metrics** (2026-10-29 → 2026-10-31, branch `feature/sdlc_metrics`, F1–F8
  committed, F9 docs in this commit). Analysis
  [`analysis/2026-10-29-sdlc-metrics-extension.md`](analysis/2026-10-29-sdlc-metrics-extension.md),
  spec [`spec/daemon/tracker-sdlc-metrics.md`](spec/daemon/tracker-sdlc-metrics.md) (amendments A1–A10),
  plan [`plans/2026-10-29-tracker-sdlc-metrics.md`](plans/2026-10-29-tracker-sdlc-metrics.md).
  - **What it adds:** ticket, sprint and project SDLC figures, derived from AI sessions'
    hook events plus the tracker's history. They are shown on Tickets and can be mapped into
    write-back fields.
  - **Still open:**
    - the write-back field set to recommend (D9: the user decides);
    - live done gate 9 against a running daemon;
    - the figures stay sparse on Linux until **P0** (session synthesis) lands;
    - the indexer's dangling-reference ratchet: 1,065 of develop's 1,075 ceiling after the 2026-11-03 merge.
- **Dev builds never deploy their branch's schema** (found 2026-11-02, SDLC metrics complete, C40).
  - **What happens:** the boot deploy (`tasks/schema_deploy.rs`, #260) runs only when
    `schema_deployed_version` differs from the binary's `CARGO_PKG_VERSION`. A branch that
    changes DDL without bumping `VERSION` therefore runs its new code on the old schema, even
    with `SENSEI_DDL_DIR` set. Tracker sync failed with `column "estimation_field" does not
    exist`.
  - **Second gap:** `task db:setup` runs plain `dbd deploy`, which never alters an existing
    table, so it fails the same way. The fix that worked was `dbd reconcile --scope default`,
    then `dbd deploy`, the daemon's own order.
  - **Candidates:**
    - key the boot check on a hash of the DDL source when `SENSEI_DDL_DIR` is set;
    - add the reconcile step to `task db:setup`.
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

- **GitLab tracker adapter — closed** (2026-11-07) by
  [`spec/daemon/tracker-gitlab.md`](spec/daemon/tracker-gitlab.md), issues #364–#368:
  iterations where the instance has them, else milestones, as sprints; `#12` resolved by
  the session's repo (`tracker/forge.rs`, shared with GitHub); gitlab.com or a confirmed
  self-managed host, with a public-only resolver and no proxy on every tracker client;
  PAT (`read_api`), no OAuth. Write-back and history are unsupported
  (`unsupported_by_tracker`, 409 `writeback_unsupported`). D49 still puts the new org
  adapters (GitHub Issues, ADO) on the dōjō-owned path first.

- **`reads_history` re-reads the tracker row** (LOW, from the GitLab F1 review) —
  `sdlc::reads_history` (`crates/senseid/src/sdlc/mod.rs`) does one primary-key read of
  `sensei.trackers` per call, so `GET /trackers` pays one per tracker for a kind its caller
  already holds. Pass the row's kind from the callers instead.

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
| What is open this week, by area, and who holds it | [`review/2026-11-03-orientation.md`](review/2026-11-03-orientation.md) §2, [`plan/2026-11-03-parallel-lanes.md`](plan/2026-11-03-parallel-lanes.md) §2 |
| Per-module status and the next step | [`roadmap/`](roadmap/README.md) |
| Every known gap, IDed, with evidence | [`roadmap/06-gaps.md`](roadmap/06-gaps.md) |
| Detailed past findings | [`history/backlog/writeups.md`](history/backlog/writeups.md) |
