# Issue burndown: what to do first to close the most issues soonest

Written 2026-10-18 from the audit of all open issues. Each kept issue carries a status comment
listing what is done and what remains. 109 issues were open; 20 were closed as fixed, superseded or
duplicate, leaving 89. The production-readiness items
([`07-quality.md`](07-quality.md) A–H) map onto these issues and are folded in below.

## How it is ranked

1. **Ship what is already done.** A merge and a release close issues with no new code.
2. **Answer the decision-only issues.** Some issues need a decision, not code; recording it closes them.
3. **The user-visible break first:** Settings → Workspaces.
4. **Small remainders.** A partial with under about half a day left closes for very little effort.
5. **Clusters:** one change that closes several issues.
6. **Make the test suite and CI trustworthy** before the larger indexer work.
7. **Epics and feature work last.** They close one issue for a lot of work.

## Wave 0 — release what is built (no new code)

**Status 2026-10-18: shipped as v0.14.0.**
- Merged to main as ed533c7; main CI is green; deploy-dojo ran; dojo `/version` = 0.14.0.
- **#238** is closed. Its install path is proven from a plain install.
- **#252** auto-closed.
- **#240** is reopened: production is deployed and the route is live, but a real user push to production is not yet observed.
- **#184** is not started. It is a code change: write `sync_status` or retire it.

| Step | Closes |
|---|---|
| Merge `develop` → `main` (97 commits; "Closes #N" commits auto-close) | — |
| `make bump` + release, which ships the mods, `list_cost_usd`, `partial` and `file_metric_rows` | #238, after install verification (C-13) |
| `deploy-dojo` + live check of user-scoped metrics | #240 |
| Write `dojo_memberships.sync_status` from the sync, or retire it; drop the 2026-08-29 held row | #184 |

## Wave 0b — release from main tags (user, 2026-10-22) — **done**: proven in v0.19.0 (deploy-worker built and deployed Worker version 29f5754b from the tag after deploy-dojo; live builtAt matches). The user removes the Cloudflare git connection.

Every release bumps the version (patch or minor): one tag, one release run. A tagged release ships from
**main**; develop is for scanning, coverage and tests. Mirrors rokkit/kavach.
1. `make bump` refuses unless on a clean `main` equal to `origin/main`; bumps, tags, pushes main + tag, then
   merges main back into develop.
2. `release.yml` verifies the tag is on main (fail closed) before any publish, deploy or DB job.
3. The dōjō Worker deploys from the tag in `release.yml` after the DB job — **user step:** repo secrets
   `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID`, and remove the `dojo` Worker's Cloudflare git build triggers
   (also clears the failing "Workers Builds: dojo" PR check).
4. Scanning stays on develop, main and PRs.
5. CLAUDE.md release section, the release skill and the dojo-worker-deploy memory follow.

## Wave 0c — CodeQL's open alerts on main (2026-10-22) — done

*Done 2026-10-22.* The critical SSRF is fixed with tests (9381ec2e; alert #1 closes when it reaches main). Every
other alert was triaged against the code and dismissed in GitHub with its reason: the path ones are the local API's
feature or exact-DB-guarded; the allocation ones are sized by an already-parsed body or clamped to 500; the logging
ones print ids and names in test assertions. The screen-shell alerts closed with 11602156 and 35efa6d4. Open: 1 (#1).

PR #324 was merged red on CodeQL (user decision, so the dōjō Worker matched its migrated database). Every
alert already existed on main. Each one is fixed with a test, or dismissed in GitHub with a written reason;
no alert is left open without one.
- **#1 critical** SSRF, `indexer/lib_indexer.rs:272` (first).
- **Path traversal ×5:** `api/handlers/workspace.rs` ×4, `libraries.rs:79`.
- **Uncontrolled allocation ×3:** `knowledge.rs`, `observatory.rs`, `tasks/ticker.rs`.
- **Cleartext logging:** in `dojo_client/session.rs` ×3, `app/src-tauri/log_collector.rs`,
  `tools/session-report/retro.rs`, and the pg_store tests (`dojo_tests.rs`, `workspaces_tests.rs`).
- **Improper code sanitization:** `app/e2e/tests-inspect/libraries-inspect.spec.ts:22`.
- The screen-shell spec alerts are fixed on develop (11602156, 35efa6d4); they close when that code reaches main.

## Wave 1 — Settings → Workspaces / Repositories (#271, #269, #144)

**Status 2026-10-19: built and verified on live data.**
- **Data:**
  - A checkout keeps one repository row across rescans (43b7cb6e).
  - The 13,346 orphan keyless rows were pruned on the live DB.
- **Quality role:** test-only repositories are tagged `quality` (650ae365).
- **UI:** repository cards, the icon, roles saved with rollback, a sticky header, search, and a note on
  workspaces with no repository (7f533892).
- **Derived roles:** a repository card shows the roles the scan inferred for its packages, so a
  monorepo no longer reads "Unassigned" (030592ee).
- **Modules stay modules:** the watcher's rescan wrote every subfolder as `folder` and demoted a
  package workspace's members. It now writes the pipeline's kind, from one shared rule (ffc79fc6).
- **#144 moved to wave 3** (the workspace metrics view, not Settings) and **closed 2026-10-20** there.
- **Clearing a role:** a `null` role clears it; before, it was a 200 that changed nothing (e777d692).
- **Live inspect** (`app/e2e/tests-inspect/settings-projects-inspect.spec.ts`):
  - 3 workspaces placed, 1 repository each.
  - 130 collapsed as having no repository on this machine.
  - Search narrows the list.

Measured 2026-10-18:
- `sensei.repositories` holds **13,504 rows for 2 indexed checkouts**.
- 13,346 of them have no `repo_key` and no folder.
- 5,618 of those are still linked into workspaces, e.g. `fitness-assistant` has 596.
- They are left over from folders pruned when watch roots were removed. Their repository rows and
  `repositories_in_projects` links were never pruned.

The settings screen lists them as repositories.

1. **Data (red first):**
   - Prune repository rows that no folder references and that have no `repo_key`, together with
     their workspace links.
   - Make the folder prune take its repository row with it, so this cannot recur.
   - What a workspace whose folders are all gone becomes was decided 2026-10-19: neither. They stay
     collapsed until the full re-index (wave 9) brings their checkouts back.
2. **API:** a workspace lists only repositories and subtrees that have a live checkout.
3. **UI (#271 items 2–7):**
   - repository cards
   - the icon rendered as an icon, not a path
   - tag persistence (`setRole` only mutates local state)
   - a sticky header
   - a search filter
4. **Quality role (#269, #271 item 3):** add it, auto-assigned to test-only repositories.
5. **#144:** a workspace with no repository says that `scope=repo` metrics need git history.

## Wave 2 — decisions only (no code; each closes or narrows an issue)

**Status 2026-10-19: all decided.** Each issue carries its decision.

| Issue | Decision needed |
|---|---|
| #141 | **Closed.** Graph reachability is trusted only at Rust's bar (Rust today); CLAUDE.md is updated. |
| #149 | Re-measured: 7 communities hold 52% of nodes. **Experiment:** drop `parent_id` adjacency, keep it if the giants break up (wave 3). |
| #138 | **Repository rung first.** Prune the stale `project:` namespaces after wave 9 (wave 8). |
| #230 | **Closed.** The declared layering is #277 (wave 8). |
| #90 | **Closed.** The review is #278: an `unknown` verdict, and action tools judged by outcome (wave 3). |
| #50, #150 | **Kept open** with the `deferred` label; each names the trigger that reopens it. |
| #120 | **Allowlist** with reasons and a review date (wave 4). |
| #44 | The Benchmark runner moves to deferred epic #279, to be re-designed on transcript history. #44 keeps the extensions browser and the editors (wave 8). |

## Wave 3 — small remainders (each under about half a day; closes the issue)

- ~~**Hard-rule violation, first:** #104. The dōjō inbox renders a relay failure as an empty list.~~ **Closed 2026-10-19**: error and partial states and a skeleton, verified on the local stack.
- ~~**Local dōjō setup:** #185 and #186~~. **Closed 2026-10-19**: the dev server binds `127.0.0.1:5173`, plus
  `make dojo-local` and the runbook in `dojo/CLAUDE.md`. The daemon logs `dojo_sync: cycle complete`.
- ~~**Metric registry cluster:** #153, #154, #177, #157~~. **Closed 2026-10-19**: ratings of 0 for
  `time_to_useful_result` fell from 173 to 42 of 197, and a catalog guard test is in place.
- ~~**#280**~~ **Closed 2026-10-19**: transcript reading on by default; per-turn linkage (`turn_usage`); one
  writer for `sessions.tokens_*`, now `bigint`; Claude transcripts streamed past the 64 MB cap; the metrics
  screen names assistants whose history is not read. Also fixed: `/api/metrics/correlations` 500 (c30ae5bb).
- **Operations:**
  - ~~#181~~ **Closed 2026-10-19**: `SKIP_DB_BACKUP`, prune after writing (12 → 8 GB), a failed export fails
    the backup.
  - #194: **fixed and deployed (b3dfaf7b)**; open until a live retryable failure shows one row per task.
    Pending a user decision: the 145k/day `task_started`/`task_completed` rows duplicate `task_executions`.
  - ~~#179~~ **Closed 2026-10-19**: every pre-bind exit records why; `senseid status` shows it.
- **Gates:**
  - #187: e2e typecheck in `bun run check`; **gate in and errors fixed (18eb0cde)**, closing once the changed
    specs pass against the app.
  - ~~#213~~ **Closed 2026-10-19**: commit-msg and `--stdin` scanning, the right line number, and the
    self-test in every commit.
- **App:**
  - #133: **slices 1–3b live** (the `ui.glyphs` switch by context, both rails, `ROLES`, the default workspace
    mark, Instruments, all 22 screen headers, a 106-kanji vocabulary, every mark through `<Kanji>`). The app
    is complete; the dōjō follows a `dojo-glyphs` choice everywhere, from the shared `packages/glyphs`
    vocabulary (140 kanji). **Implementation complete**; open on two decisions: the dōjō default (kanji, as
    shipped on develop, or icons) and workspace glyphs in icon mode. Solar Bold Duotone icons beside every kanji, one preference,
    `fields` derived in one place, across the app and the dōjō (scope expanded 2026-10-19).
  - ~~#135~~ **Closed 2026-10-19**: the menu item opens `/help`; the page lists only keys that are bound.
  - ~~#96~~ **Closed 2026-10-19**: next run, avg, last error, and a per-worker log tail (a
    `scheduled{worker}` span per pass; a row toggle). The page no longer shows `[]` for a failed read.
  - #284 (found via #96): blank inspect captures — **cause found** (Screen Recording grant lost per ad-hoc
    rebuild); the harness reports blank/unpainted captures, the e2e bundle is signed stably. Closes after the user
    runs `make dev-signing-cert` and grants Screen Recording once.
  - #283 (found via #96): **parts 1–3 live** — `running_since` + "running for", a per-pass limit that records
    "still running after Ns", and a daemon that never prompts (`NeedsApproval`; only sign-in/out and router-key
    saves may ask), and an "Approve keychain access" action. Approved by the user 2026-10-20, but it does not
    stick: `senseid` is signed ad hoc, so each rebuild drops the grant. Needs `make dev-signing-cert` (the user's),
    a reinstall, then "Always Allow". Also to do: the approve endpoint should log its outcome.
  - ~~#285~~ **Closed 2026-10-20**: `KeyStanding` replaces `has_key`; a withheld router key reads
    `configured: null` + `key_error`, and image generation answers 503 "unreadable", not 400 "not configured".
  - ~~#286~~ **Closed 2026-10-20**: `tryListGatewayRouters`; a failed read is the Providers error state.
  - ~~#85~~ **Closed 2026-10-20**: Overview hotspots and patterns in use; the overview/patterns reads fail as
    errors, not as an all-quiet workspace.
  - ~~#287~~ **Closed 2026-10-20**: rework is placed by path inside the workspace's repositories and counted once per event (617 → 371 rows, none outside). The Patterns view folds the unreviewed rows into one "rework hotspots" anti-pattern, grouped by directory and ranked by peak re-edits (618 anti-patterns → 2 live).
  - ~~#288~~ **Closed 2026-10-20**: every app read is the daemon's answer or an error the screen shows. 72 `get(path, fallback)` reads → 0, the legacy `get` is deleted, and a spec guards it. Found and fixed along the way: a fabricated 404, a fabricated warning dot, an Accept that skipped its artifact, Settings saving defaults over the user's settings, a scan marked done by a silent daemon, a refused role committed as done, and three endless re-fetch loops. Each screen was verified live.
  - ~~#291~~ **Closed 2026-10-20**: "Tools used" moved to workspace Instruments (its metric, `unused_tools`, was retired). Live: 24 tools = daemon.
  - ~~#144~~ **Closed 2026-10-20**: a repo-less workspace (130 of 133 here; zero metric rows each) says why it has
    no metrics.
  - #149: **experiment run 2026-10-20, criterion not met** — without `parent_id` the >50-file giants grew
    (22,484 → 24,258 nodes); they are dense call/reference cores. Reverted. Awaiting a decision: Leiden/Louvain
    (recommended) or hierarchy grouping.
  - #278: **both defects fixed and live** (`unknown`; action tools by outcome). Open to re-measure after #289.
  - ~~#290~~ **Closed 2026-10-20**: a file is where declarations are. Atlas shows 2,096 modules and 11,823 exports (was 0 / 0), and `rework_density` is written again (371/2096 = 0.177, its first row since 2026-10-07). Traceability says "no documents indexed" instead of "no drift".
  - #292 (found via #290): indexer v2 has no markdown adapter, so doc drift never scans a document. **Decided 2026-10-20:** a markdown LanguageAdapter emitting `doc` nodes (with sections) and code mentions as references; an unplaced mention is drift.
  - #289 (found via #278): 26% of assistant events are duplicates (non-idempotent staging import, now fixed).
    Removing the existing 167,932 awaits the user's OK.
- **Dōjō:**
  - #132: steps 1–2 done. No literal `font-size` remains in the dōjō (a spec guards it); sign-in verified at 13/11px. Step 3 (mockup compare) started: the sign-in left pane now follows the mockup, replacing an "Acme Corp" fixture (#241). The right pane's extra password form and placeholder remain.
  - **#293 screen structure (user, 2026-10-20):** every screen keeps presentation | state | load/save apart, uses the shared shell (fixed header over one scroll body) and rokkit before any custom duplicate. Audits filed as #293 with the ordered list. Step 0: `Screen` (with `fill` for list-and-detail screens) holds all 48 app screens and neither window `<main>` scrolls (522b8092, 47 probed live); `Screen` is now shared from `packages/ui` and holds all 22 dōjō screens (c9f1e0d7); the dōjō routes and AppShell are next, then the dōjō shell, then separation and rokkit. Step 1 (separation): app instruments (c290f38e), workspace memories (6c815eea), consolidation (ac1da700), workspace impact (3541ee2f), settings/* (5a1cb297), metric detail (e749b380) and the sessions pages → one screen (bae2126f) done — app step 1 complete; four hid a failed write, the metrics loaders a failed registry read (ee2b0f4c). Dōjō step 1: `you/[section]` (9f11df08), `org/[slug]/[section]` (785279a6), the run deep link (a0b8aaf4), inbox answers really sent (#305, bf16a437 — on develop; live on the local dōjō, production after the next release) and one run view (3d443b1b) — **step 1 done for both apps**. Next: step 2 (rokkit first); step 3 dead code adds `segmentsToPlan`, `planProgress`.
  - #304 (found via #293): 26 app writes (of 28) still use the swallowing `post`/`put`/`del` helpers; converted screen by screen, then the helpers deleted with a guard (as #288 did for `get`).
  - #294 (found 2026-10-20 connecting `jerrythomas`): one pending PKCE verifier per daemon, so two sign-ins clobber each other (`bad_code_verifier` + `no sign-in in progress`). Key pending flows by state.
  - ~~#295~~ closed 2026-10-20 (06482d85): `sensei auth status` lists every identity, keychain verdict first; `sensei auth approve` added. Live.
  - ~~#296~~ closed 2026-10-20: the default identity follows the global git identity (e341e26c). Live on 0.15.0: `/api/auth/personas` default = `jerrythomas`.
  - #299 S3 done on develop (99cddc18): a run names its workspace by slug + constitution; per-recipient classification removed. Closes with the live sync evidence (#301).
  - ~~#302~~ **Closed 2026-10-21**: one keychain item per identity (`credentials.<slot>`, 17302c40); the migration was seen live after one approval (old items gone, `credentials.<slot>` present).
  - **#301 (user, 2026-10-20): sign in → sync must just work.** Auto-elect a connected identity's repositories (opt-out, not opt-in); keep the subscription gate for private org repos (decided); surface an unreadable credential first (#283, #295); Team-ID signing (#282); workspaces from sync (#299); sign-in in the system browser with an account guard (#300, decided). Live: the first pass that reached the dōjō today mapped 22, allowed 0. Fixed so far: personal tenants get their owner's forge connection (f111fe41) — all 34 personal repos were `no_connection`; a user's repositories are elected by default, opt-out holds (2d92a632; local: 31 of the user's repos sync). Released in **v0.15.0** with #299 S1/S2 and #300 (1a132009, 5d776bee); the dōjō production reshape is deployed and `dbd diff` is clean (run 39496401704). Live proof waits on keychain approval + one dōjō sign-in + a sync pass.
  - ~~#298~~ **Closed 2026-10-25** (merged c90d0fbf): tool calls pair on stored `tool_use_id`/`duration_ms`; workspace Instruments 6–8.4s → 0.58s. Needed dbd 0.27.6 (dbd#104), released and pinned.
  - #297: the dōjō sign-in page drops Supabase's `#error` fragment (the user saw a blank form after a linking failure).
  - ~~#106~~ **Closed 2026-10-23**: no view reads a fixture or mock module (guard spec, mutation-probed). Your constitution, Plan & billing, the org constitution (`…/rules`), scopes, the triage detail and the org workspace preview now show real data or a stated absence; live-checked on the local dōjō.
  - ~~#335~~ **Closed 2026-10-23**: every org screen shows a failed read (`LoadError`, role=alert) in place of its body; a spec checks that each loader error reaches the page. Live with injected 500s.
  - #191: kavach 1.2.0.
- **Code health:**
  - #199: `has_key` part done via #285. Left: Linux roundtrip tests in CI, and a decision on headless Linux
    (an encrypted-file store, or stop publishing Linux daemon artifacts).
  - ~~#201~~ **Closed 2026-10-25** (1bda2c18): one fqn minter. All six v1 call sites were on code that cannot run; the path and its encoder are deleted (−1,642 lines), guarded by a source-scan test. Live: 0 of 50,251 nodes v1-shaped.
  - #139: rokkit material/frosted focus guard (upstream rokkit).

## Wave 4 — CI and test reliability (07-quality D1/D2, F)

- **Pool and isolation:** ~~#204~~ **closed 2026-10-25** (5656955c): 198 tests that passed silently on a failed DB connect now fail; test threads are capped at 16 so the budget fits `max_connections` (checked by a test; peak 78 → 69, suite 328s → 184s). ~~#183~~ **closed 2026-10-25**: a database per test (user decision); it exposed two tests relying on siblings' rows, and all 15 gates are removed. Suite 3,838 green, 184s → 392s.
- **Preflight:** ~~#142~~ **closed 2026-10-25** (8e20c3b5): the suite refuses a template behind the DDL tree, by its stamped fingerprint; `make test-template` provisions exactly as CI does.
- **Dependency cluster:** ~~#172~~ ~~#195~~ **closed 2026-10-25**, #120 updated. Rust 16 → 0 actionable
  advisories, the Tauri lockfile 10 → 0, the JS trees' critical/high cleared (one dev-only undici under the
  dōjō's wrangler). `cargo audit` gates rust.yml over all three lockfiles (develop run 40244782241 green);
  exceptions in `.cargo/audit.toml`, each with its proof.

## Wave 5 — production-readiness defects that need real work

- ~~#334~~ **Closed 2026-10-23** (91e2eb52, ships next release): any web page could write to the daemon — the
  origin guard covered only tracker routes and CORS allowed every origin. Now `caller_guard` on the whole router
  (loopback `Host` always; an app `Origin` or none for writes) and CORS for the app's origins only. Live on a
  throwaway daemon: the original repro → 403 with nothing added; the app's origin and the CLI still write.
- ~~#178~~ (B1): **closed 2026-10-25**, verified on v0.20.2 (live probe recorded `oversized`). Landed 2e5c6c1f: a file over 2 MiB or with a line over 10,000 bytes is skipped
  as `oversized` (new enum value), by the one classifier the scan and `process_file` share. Live DB
  reconciled; over the 61,349-file corpus it refuses exactly the 137 measured. Closes on the released
  daemon's first `oversized` rows.
- ~~#188~~ (B2): **closed 2026-10-25**, verified on v0.20.2 (`sensei doctor` Watchdog). Landed d5bfdc7d: `GET /api/tasks/abandoned` (per day and kind, ten worst by
  kind and path) and a Watchdog section in `sensei doctor`. Live: 204 in 14 days. Closes when the released
  doctor prints it.
- #336: **landed 2026-10-25** (e5bf2e9d): `ingest_capture` was abandoned 117× in 14 days on one 1.28 GB
  transcript. Claude's `parse_file` read the file five times, 5 s per pass. Now it reads once, 25.2 s → 5.1 s
  on that file. Closes after a week of the released daemon with no `ingest_capture` abandonment.
- ~~C1~~: `public.logs` size — #180, closed.
- #125: **landed 2026-10-25** (17e81b4b). 72 orphan turn sets: 53 are from untracked folders, 1 is a Zed thread
  with no cwd, 18 have no transcript left. Ingest is now scoped to folders under the watch roots (user
  decision); a trigger re-opens the skips when a repository is registered. On v0.20.2: 72 → 22. The 4 fresh
  orphans with hook events are fixed in 5dea36e3 (next release), then 18, which have no transcript.
- #161: typed rows, remainder. **2026-10-25** (98e2c9cf): `adopt_node_by_identity` (12 args) takes `FqnNodeRow`
  (adding a field fails the build, verified). A guard test pins the positional writers: none at 8+, and the 50 at
  5+ are listed by name and may only shrink. 558b90fd: the three MCP writers take rows (50 → 47), with
  round-trip tests. Then check runs, consolidated rulesets and workflow state (47 → 44); `merge_function`
  deleted and `callee_row` typed (→ 42); `insert_edge_with_props` takes `EdgeRow` (→ 41). Next: `upsert_folder`,
  then the `insert_edge` wrapper, then the `too_many_arguments` allows.
- #123: VS Code adapter A5/A6, Track B.
- #281: Zed threads over 64 MB, parsed straight from the zstd stream into typed messages.
- #282: releases are ad-hoc signed, so keychain and folder grants reset on every upgrade. Developer ID
  signing, notarization, and a first-start Permissions step. Needs the user's Apple Developer account.
  Local builds are fixed already (8d09be73, `make dev-signing-cert`).
- #190: read `design.yaml` through dbd's `project::survey`, and load a workspace design with
  `Workspace::load`. Since dbd 0.27, a repository with a `modules:` design cannot be imported.

**dbd 0.27.3 (2026-10-19).**
- All four pins move from v0.26.0.
- 0.27.0 adds workspace designs (`modules:`):
  - The hand-rolled reader in `adapters/manifest/dbd.rs` does not know them.
  - `indexer/dbd_import.rs`'s `Design::from_config` refuses them.
  - So #190's `project::survey` step, together with `Workspace::load`, moves up to wave 5.
- #258 gains a `module` field to persist, alongside identity, generated, predicate and checks.

## Wave 6 — indexer quality cluster

#146, #152 (external/local classifier and the local `mod` import resolver) close together and feed
#156 step 3. Then #147, #148, #174, #192, #229, #272, #246, #258.

## Wave 7 — metrics confidence chain

#159 → #160 → #158 (#155 was closed into #160), then #156's remaining metrics.

## Wave 8 — epics and features

#43, #44, #45, #46, #47, #81, #91, #102, #105, #108, #111, #114, #115, ~~#116/#119~~ (closed 2026-10-21: #116's create flow is retired; invites and GitHub-org provisioning shipped; R5 is #308), #121, #122,
#131, #239, #241, #266, #268 and #39. Order these by product priority, not by closure count.

## Wave 9 — the full re-index (after waves 0–8)

Held back until most of the issues above are resolved, because a full scan is slow.
- It brings back the workspaces with no checkout on this machine (130 on 2026-10-19).
- **Before running it:** time a full scan stage by stage and set a budget, so the wait is
  measured rather than guessed.

## Wave 10 — language parity: Java, C#, Python (#275)

Bring them to the compiler-grade resolution Rust and TypeScript have.
- **How:** one SCIP emitter per language behind the shared reader (`indexer/lang/compiler.rs`):
  scip-dotnet, scip-java, scip-python.
- **Measured 2026-10-19:** calls resolved: Rust 100%, TypeScript 52.9%, C# 8.9%, Java not
  indexed here, Python not parsed. Rust is the bar (decided 2026-10-19), so TypeScript is in scope too.

| Issue | What |
|---|---|
| #276 | Python files recorded but never parsed or skipped (a blocker) |
| #237 | C# is 52% indexed |
| #254 | The next ORM after EF Core (JPA) |
| #275 | The epic: scip-dotnet, scip-java, scip-python |

## Wave 11 — the website follows what ships (after the dōjō waves)

The user updated the website mocks on 2026-10-21 with the features added since: diagrams and lenses,
workspaces with issue-tracker integration, planning, release tracking. Taken up **after** the dōjō
work (#307 and the gaps #309–#321).
- **Dōjō work, 2026-10-22:** R8 (lead team metrics) merged; R9 (trackers by level, #318 D23) built and reviewed
  and merged; R5b (Azure) is next, then the release that ships R5a, R8 and R9. Status per item:
  docs/roadmap/09-dojo-expansion.md.
- **2026-10-23:** R5b merged (Azure DevOps through Entra); R5c (sign in with Microsoft) in progress; then **#330**
  (Terms of Service and Privacy Policy pages at /terms-of-service and /privacy-policy: the Entra app registration
  and the OAuth consent screens ask for both URLs; the legal wording is the user's, drafted from the code), then
  the release.
- **How:** build the website from the new mocks. Each feature reads *coming soon* until it ships;
  flip it to *available* in the change that releases it, not afterwards.

## Expected closures

| Wave | Issues closed | Rough effort |
|---|---|---|
| 0 | 2–3 | a release |
| 1 | 2 (#271, #269; #144 moved to wave 3) | 2–3 days |
| 2 | 6–9 | decisions only |
| 3 | ~20 | ~1–2 weeks of small items |
| 4 | ~6 | ~1 week |

Waves 0–4 close roughly **40 of the 89** open issues.
