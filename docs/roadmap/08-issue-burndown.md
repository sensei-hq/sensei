# Issue burndown: what to do first to close the most issues soonest

Written 2026-10-18 from the audit of all open issues. Each kept issue carries a status comment
listing what is done and what remains. 109 issues were open; 20 were closed as fixed, superseded or
duplicate, leaving 89. The production-readiness items
([`07-quality.md`](07-quality.md) A–H) map onto these issues and are folded in below.

## How it is ranked

1. **Ship what is already done.** A merge and a release close issues with no new code.
2. **Answer the decision-only issues.** Some issues need a decision, not code; recording it closes them.
3. **The user-visible break first:** Settings → Projects.
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

## Wave 1 — Settings → Projects / Repositories (#271, #269, #144)

**Status 2026-10-19: built and verified on live data.**
- **Data:**
  - A checkout keeps one repository row across rescans (43b7cb6e).
  - The 13,346 orphan keyless rows were pruned on the live DB.
- **Quality role:** test-only repositories are tagged `quality` (650ae365).
- **UI:** repository cards, the icon, roles saved with rollback, a sticky header, search, and a note on
  projects with no repository (7f533892).
- **Derived roles:** a repository card shows the roles the scan inferred for its packages, so a
  monorepo no longer reads "Unassigned" (030592ee).
- **Modules stay modules:** the watcher's rescan wrote every subfolder as `folder` and demoted a
  workspace's packages. It now writes the pipeline's kind, from one shared rule (ffc79fc6).
- **#144 stays open** and moves to wave 3. It is about the project metrics view, not Settings.
- **Clearing a role:** a `null` role clears it; before, it was a 200 that changed nothing (e777d692).
- **Live inspect** (`app/e2e/tests-inspect/settings-projects-inspect.spec.ts`):
  - 3 projects placed, 1 repository each.
  - 130 collapsed as having no repository on this machine.
  - Search narrows the list.

Measured 2026-10-18:
- `sensei.repositories` holds **13,504 rows for 2 indexed checkouts**.
- 13,346 of them have no `repo_key` and no folder.
- 5,618 of those are still linked into projects, e.g. `fitness-assistant` has 596.
- They are left over from folders pruned when watch roots were removed. Their repository rows and
  `repositories_in_projects` links were never pruned.

The settings screen lists them as repositories.

1. **Data (red first):**
   - Prune repository rows that no folder references and that have no `repo_key`, together with
     their project links.
   - Make the folder prune take its repository row with it, so this cannot recur.
   - What a project whose folders are all gone becomes was decided 2026-10-19: neither. They stay
     collapsed until the full re-index (wave 9) brings their checkouts back.
2. **API:** a project lists only repositories and subtrees that have a live checkout.
3. **UI (#271 items 2–7):**
   - repository cards
   - the icon rendered as an icon, not a path
   - tag persistence (`setRole` only mutates local state)
   - a sticky header
   - a search filter
4. **Quality role (#269, #271 item 3):** add it, auto-assigned to test-only repositories.
5. **#144:** a project with no repository says that `scope=repo` metrics need git history.

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
  - #133: **slices 1–3b live** (the `ui.glyphs` switch by context, both rails, `ROLES`, the default project
    mark, Instruments, all 22 screen headers, a 106-kanji vocabulary, every mark through `<Kanji>`). The app
    is complete; the dōjō follows a `dojo-glyphs` choice everywhere, from the shared `packages/glyphs`
    vocabulary (140 kanji). **Implementation complete**; open on two decisions: the dōjō default (kanji, as
    shipped on develop, or icons) and project glyphs in icon mode. Solar Bold Duotone icons beside every kanji, one preference,
    `fields` derived in one place, across the app and the dōjō (scope expanded 2026-10-19).
  - ~~#135~~ **Closed 2026-10-19**: the menu item opens `/help`; the page lists only keys that are bound.
  - ~~#96~~ **Closed 2026-10-19**: next run, avg, last error, and a per-worker log tail (a
    `scheduled{worker}` span per pass; a row toggle). The page no longer shows `[]` for a failed read.
  - #284 (found via #96): some inspect screenshots capture a blank webview.
  - #283 (found via #96): **parts 1–3 live** — `running_since` + "running for", a per-pass limit that records
    "still running after Ns", and a daemon that never prompts (`NeedsApproval`; only sign-in/out and router-key
    saves may ask), and an "Approve keychain access" action. Open only for the user's click to be seen working.
  - ~~#285~~ **Closed 2026-10-20**: `KeyStanding` replaces `has_key`; a withheld router key reads
    `configured: null` + `key_error`, and image generation answers 503 "unreadable", not 400 "not configured".
  - #286 (found via #285): the app's router list turns a failed read into "no routers".
  - #85: Overview hotspots and patterns-in-use.
  - #144: on a project with no repository, say once that `scope=repo` metrics need git history.
  - #149: the community experiment (drop `parent_id` adjacency, measure).
  - #278: the verdict heuristic (`unknown`, and action tools by outcome).
- **Dōjō:**
  - #132: 16 inline font sizes.
  - #106: last fixture imports, plus a vitest guard.
  - #191: kavach 1.2.0.
- **Code health:**
  - #199: `has_key` part done via #285. Left: Linux roundtrip tests in CI, and a decision on headless Linux
    (an encrypted-file store, or stop publishing Linux daemon artifacts).
  - #201: 6 `languages::fqn` call sites.
  - #139: rokkit material/frosted focus guard (upstream rokkit).

## Wave 4 — CI and test reliability (07-quality D1/D2, F)

- **Pool and isolation:** #204 (pool cap and test threads), then #183 (isolation). Both are needed before CI can be trusted.
- **Preflight:** #142, test DB behind the DDL tree.
- **Dependency cluster:** #172, #195, #120.
  - Bumps: quick-xml, h2, yanked crates; JS: vite, esbuild, vitest 4, undici and others.
  - Add `cargo audit`.
  - Re-check Dependabot: it reports 0 alerts while the lockfiles are vulnerable.

## Wave 5 — production-readiness defects that need real work

- #178 (B1): bound the parse stack.
- #188 (B2): surface watchdog abandonment. Measured 300–1,800 a day in early October.
- C1: `public.logs` size (no issue yet; with #194).
- #125: 68 orphaned transcript turn sets.
- #161: typed rows, remainder.
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

#43, #44, #45, #46, #47, #81, #85, #91, #102, #105, #108, #111, #114, #115, #116/#119, #121, #122,
#131, #239, #241, #266, #268 and #39. Order these by product priority, not by closure count.

## Wave 9 — the full re-index (after waves 0–8)

Held back until most of the issues above are resolved, because a full scan is slow.
- It brings back the projects with no checkout on this machine (130 on 2026-10-19).
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

## Expected closures

| Wave | Issues closed | Rough effort |
|---|---|---|
| 0 | 2–3 | a release |
| 1 | 2 (#271, #269; #144 moved to wave 3) | 2–3 days |
| 2 | 6–9 | decisions only |
| 3 | ~20 | ~1–2 weeks of small items |
| 4 | ~6 | ~1 week |

Waves 0–4 close roughly **40 of the 89** open issues.
