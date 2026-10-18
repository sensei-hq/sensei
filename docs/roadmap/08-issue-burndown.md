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

| Step | Closes |
|---|---|
| Merge `develop` → `main` (97 commits; "Closes #N" commits auto-close) | — |
| `make bump` + release, which ships the mods, `list_cost_usd`, `partial` and `file_metric_rows` | #238, after install verification (C-13) |
| `deploy-dojo` + live check of user-scoped metrics | #240 |
| Write `dojo_memberships.sync_status` from the sync, or retire it; drop the 2026-08-29 held row | #184 |

## Wave 1 — Settings → Projects / Repositories (#271, #269, #144)

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
   - Decide what a project whose folders are all gone becomes: archived, or deleted.
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

| Issue | Decision needed |
|---|---|
| #141 | Is the graph earning its keep? Calls now resolve at 77.7%. Narrow the CLAUDE.md MCP-first rule. |
| #149 | Keep label propagation, drop `parent_id` from its adjacency, or replace it with hierarchy grouping (re-measure first). |
| #138 | The project/namespace model: add a `repository:` rung. |
| #230 | (a) is done. Close it, and file the declared layering (b) separately. |
| #90 | The verdict classifier is delivered. Close it, and file the 75% `ignored` review. |
| #50, #150 | Deferred by design. Close as not-now, or keep as a parking lot. |
| #120 | Allowlist the GTK3 advisories in qlty, or keep them visible. |
| #44 | The Benchmark runner was cut by D-SCREENS. Drop it from the issue. |

## Wave 3 — small remainders (each under about half a day; closes the issue)

- **Hard-rule violation, first:** #104. The dōjō inbox renders a relay failure as an empty list.
- **Local dōjō setup:** #185 (vite `server.host: '127.0.0.1'` + note) and #186 (runbook / `make dojo-local`).
  The steps were exercised on 2026-10-15/18.
- **Metric registry cluster:**
  - #153: the churn caption.
  - #154: `time_to_useful_result` rating scale in seconds.
  - #177: a fresh-tokens metric plus `how_to_read`.
  - #157: generate or drop the catalog prose.
- **Operations:**
  - #181: `SKIP_DB_BACKUP`, and prune after writing (07-quality C2).
  - #194: retryable failures not logged as error on every attempt (part of C1).
  - #179: a pre-bind exit writes the startup error (07-quality B3).
- **Gates:**
  - #187: e2e typecheck in `bun run check`, 10 errors (07-quality D4).
  - #213: commit-msg leak hook, plus the off-by-one line number.
- **App:**
  - #135: Keyboard Shortcuts → `/help`.
  - #96: render `last_error`, `next_run_at`/`avg_ms`, log tail.
  - #85: Overview hotspots and patterns-in-use.
- **Dōjō:**
  - #132: 16 inline font sizes.
  - #106: last fixture imports, plus a vitest guard.
  - #191: kavach 1.2.0.
- **Code health:**
  - #199: `has_key` reports `Unavailable`.
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

## Wave 6 — indexer quality cluster

#146, #152 (external/local classifier and the local `mod` import resolver) close together and feed
#156 step 3. Then #147, #148, #174, #192, #229, #272, #237, #246, #254, #258.

## Wave 7 — metrics confidence chain

#159 → #160 → #158 (#155 was closed into #160), then #156's remaining metrics.

## Wave 8 — epics and features

#43, #44, #45, #46, #47, #81, #85, #91, #102, #105, #108, #111, #114, #115, #116/#119, #121, #122,
#131, #133, #239, #241, #266, #268 and #39. Order these by product priority, not by closure count.

## Expected closures

| Wave | Issues closed | Rough effort |
|---|---|---|
| 0 | 2–3 | a release |
| 1 | 3 | 2–3 days |
| 2 | 6–9 | decisions only |
| 3 | ~20 | ~1–2 weeks of small items |
| 4 | ~6 | ~1 week |

Waves 0–4 close roughly **40 of the 89** open issues.
