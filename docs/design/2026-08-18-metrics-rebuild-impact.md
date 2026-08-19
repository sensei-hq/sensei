---
type: design
status: draft
date: 2026-08-18
covers: docs/spec/2026-08-18-repo-grain-metrics-watermark-engine.md (P-A + P-B)
---

# Metrics rebuild — impact analysis (P-A / P-B)

Pre-flight impact map for the repo-grain / watermark / user-attributed-quality rebuild
([spec](../spec/2026-08-18-repo-grain-metrics-watermark-engine.md)). Produced from four
read-only code mappers + a live `sensei` DB snapshot (2026-08-18). Every surface the auto-run
will touch is enumerated here with `file:line`, plus the gaps the spec missed and the
decisions that must be made before code starts. Target: **consistency, clean code, zero errors.**

---

## 0. Spec corrections (the spec is wrong here — fix first)

The mappers found the spec asserts things that don't match the code. These are corrected in
the spec and restated here so the auto-run doesn't build on false premises:

| Spec claim | Reality | Source |
|---|---|---|
| §4.1 "`walk_for_git` already descends into nested `.git` dirs" | **FALSE** — `walk_for_git` recurses only into *non-git* dirs; on a `.git` hit it pushes and stops (`scan_logic.rs:67-71`). Nested checkouts are never discovered. The recursion halt is the *core* D15 defect, not just `.is_dir()`. | scanner |
| D15 "builds on `FolderKind::{Git,Standalone,Subtree}`" | The Rust enum has **only `Git` + `Standalone`** (`scan_logic.rs:22-29`). `subtree`/`workspace_member` are **DB-only** strings the scanner never emits. | scanner |
| §10/§12 box/violin "upstream #143/#144 (pending)" | **Already shipped** in pinned `@rokkit/chart@1.3.13` — `Plot.Box`/`Plot.Violin` export at runtime. Only wiring needed. But the `Plot` subpath **types** omit them → TS-error risk under zero-errors; import `GeomBox`/`GeomViolin` (typed) or augment. | read/app |
| "7 files consume the `repositories` VIEW" | **Zero** shipped consumers read the view; all 7 call `list_repositories()` which hits `folders` directly (`folders.rs:507-514`). The collision is a pure DB name clash — dropping the view is safe. | scanner + ddl |
| §13 "backfill `repository_id` from `folders.repository_id`" | Covers only the 1043 folder-scoped + 486 session rows. The **15,697 project-scope rows have `folder_id IS NULL`** → no derivable repository. Needs an explicit decision (§4 G-A). | ddl |
| "6 views incl. `project_ftr_metrics`" | Only **5** view DDLs exist; `project_ftr_metrics` has no DDL source (live-DB drift — exists as a view but unmanaged). Don't migrate against it. | read |

---

## 1. Critical invariant the spec omitted — the pruner guard (must-fix)

`planner.rs:117 authorizes_capture()` + `day_keyed_task_names()` feed the **activity-pruner's
capture-before-reclaim `EXISTS` guard** — the guard added after the 164GB `nodes`-bloat /
data-loss incident ([[project_nodes_bloat_unguarded_bulk_update]] and the phantom-session
work). Retiring `DayKeyedGroup`/the planner without migrating this predicate into the new
engine **re-opens the data-loss bug.** The rebuild MUST carry the capture-scope guard forward
into `ComputeGroupMetrics`. This is invariant **I20** (added to the spec).

---

## 2. Per-surface impact

### 2.1 Scanner + folder/repository model
Key files: `scan_logic.rs` (classifier, `walk_for_git`), `scan.rs` (`scan_root`,
`read_git_remotes`), `watcher/root_watcher.rs` (`process_batch`, `is_branch_switch`),
`db/pg_store/folders.rs` (`list_repositories`, `repo_root_for_path`, `resolve_repo_anchor`),
`db/pg_store/mod.rs:688` + `projects.rs` (nested-standalone heal), `index_audit.rs`,
`function/sensei/repo_anchor_for.ddl`.

- **Reclaim the name (safe):** drop `view/sensei/repositories.ddl` (zero code consumers), create
  the canonical table with the same name; add `sensei.repo_folders` compat view only if a psql
  convenience surface is wanted. `list_repositories()` keeps reading `folders`.
- **`.git` dir OR file:** shared pure `is_checkout(p) = p.join(".git").is_dir() || .is_file()`
  used in `walk_for_git:67`, `is_inside_git_repo:101`, `walk_dirs:130` (worktrees/submodules).
- **Fix the recursion halt (the real D15 fix):** `walk_for_git` must continue descending into a
  found checkout (bounded), + lift the depth-3 bound (`scan.rs:53`). `.git`-file support alone is
  insufficient.
- **Detect-before-prune:** test `is_checkout` before the `IGNORED_DIRS`/dotfile skip
  (`scan_logic.rs:64-65`).
- **Incremental reconcile:** in `process_batch` (`root_watcher.rs:484-494`) detect a `Create`
  introducing a `.git` at/below the event path → targeted repository-create + subtree re-anchor,
  not fold-into-nearest-root. Note `.git` is in the watcher ignore list except the `.git/HEAD`
  special-case (`root_watcher.rs:19,357`) — fragile single trigger (G-S3).
- **`index_audit` (D15e) needs a NEW disk-walking nested-git detector** — the existing
  `nested_standalone_candidates` (`mod.rs:688`) is a DB-shape query that cannot see an
  unregistered nested checkout (no row exists).
- **Reuse existing ancestry:** `folders` already has `root_id`/`parent_id`/`remote_urls`; add only
  `folders.repository_id` (root folder only, I16) + `folders.branch`. Subfolders resolve via
  `repo_anchor_for` — **define `repository_id` resolution in terms of `repo_anchor_for`, never a
  parallel walk** (the "three divergent resolvers" failure the anchor fn header warns against).

### 2.2 Metric compute engine
Key files: `tasks/mod.rs` (kinds), `executor.rs:133-135`, `metrics_scheduler.rs`,
`handlers/metrics/{planner,mod,session_outcomes,churn,quality,autonomy,knowledge,tool,health}.rs`,
`db/pg_store/metrics.rs` (`upsert_project_metric`).

- **New parent `ComputeProjectMetrics{project_id}`** (replaces `PlanMetricDays`): freeze one
  `as_of`; resolve a **new `repositories(project)` resolver** (NOT `project_root_path` — it's the
  relay run cwd, `advance_run.rs:490`/`runs.rs:91`); read per-`(repository,group)` watermark; spawn
  cadence-aware children; retry failed groups against the same `as_of`; advance watermark only for
  succeeded groups; keep the `ComputeHealth` barrier.
- **New child `ComputeGroupMetrics`**: owns cadence; reuse the existing per-group fetch/aggregate/
  git-walk/qlty-scan bodies verbatim inside it.
- **Repoint grain:** session_outcomes = join `session→folder→repository`, `GROUP BY repository`,
  `scope='user'`; churn/quality iterate `repositories(project)` (kills the `LIMIT 1`
  `project_root_path` blind spot) + add `git log --author ∈ {local identities}` (churn is
  **author-agnostic today** — `churn.rs:97-146`, no `--author`; D6/D7 is net-new).
- **DELETE:** planner `covered_days:255`/`plan_days:318`/`data_days:154` + `DayKeyedGroup` machinery;
  scheduler global clock (`LAST_RUN_KEY:54`, `due_for_run:83`, `next_watermark:95`); the
  `PlanMetricDays`/`ComputeMetrics` task kinds. **But migrate `authorizes_capture` first (§1).**
- **Phantom:** `false_crash_rate` is declared in the registry but **never computed**
  (`autonomy.rs:212`) — implement or formally retire; don't carry it.
- **churn is mixed-cadence:** `rework_density` is a day snapshot inside the commit group
  (`churn.rs:350-387`) → **split it into its own day-cadence group** so every group is
  single-cadence (required for the per-group watermark).

### 2.3 DDL / registry / migration
Key files: `table/sensei/{metrics,project_metrics,folders,dojo_memberships}.ddl`,
`enum/sensei/metric_{source,scope}.ddl`, `view/sensei/repositories.ddl` + the 5 rollup views,
`table/staging/metrics.ddl` + `procedure/staging/import_metrics.ddl` + `import/staging/metrics.jsonl`,
`design.yaml`, `table/dojo/*`, `function/dojo/owns_membership.ddl`.

- **dbd 0.10.4** (FK support now exists; old 0.8.21 caveat retired). Additive cols → `dbd reconcile`.
  **Cannot drop cols/indexes/views → manual `psql`.** `create index if not exists` **silently skips
  on name-match with different columns** → the identity-index swap MUST be a manual `DROP INDEX` then
  recreate (recommend new name `project_metrics_identity_v2` to avoid a silent no-op).
- **Enums:** new `metric_scope('repo','user')`; `metric_source += 'federated'` (live = out-of-txn
  `ALTER TYPE … ADD VALUE`; alphabetical materialization is safe — referenced by name).
- **Registry column ripple:** any new `metrics` column must be edited in **4 places** — `metrics.ddl`,
  `staging.metrics`, both column lists in `import_metrics()`, and every `metrics.jsonl` row (the
  `effective_from`-omission footgun, `import_metrics.ddl:18-19`).
- **dōjō enum dep (G-D2):** `dojo.member_metrics`/`repo_metrics` use `metric_grain` (a `sensei` enum);
  `deps: include` can't reach enum column types → add `sensei.metric_grain` to the `dojo` scope
  `includes:` or `dbd deploy --scope dojo` fails.
- **`sensei.dojo_memberships` (D16)** already exists with `org_slugs`/`kind`/`role`/`credential_ref`/
  `sync_status`/`enabled` — confirm no additive columns needed for enrollment.
- **dōjō RLS** follows the `relay_sessions` template; new `dojo.is_repo_member()` mirrors
  `owns_membership` (`security definer` + `grant execute to authenticated` + drop-if-exists).

### 2.4 Read / API / app / views
Key files: `db/pg_store/metrics.rs` (read getters), `api/handlers/{metrics,observatory}.rs`,
`api/routes.rs`, the 5 `project_metric_*` views, `app/src/lib/metrics/metric-view.ts`,
`app/src/lib/api.ts`, `app/src/routes/(project)/project/[id]/metrics/*`.

- **Views:** `folder_id IS NULL` no longer means "project scope" (a `scope` column does). Rewrite
  `project_metric_daily` → `v_metric_point`/`v_metric_me`/`v_metric_repo`/`v_project_metric`;
  re-key weekly/monthly to `(repository, metric, scope)`; **`project_metric_trend` and
  `quarterly` are not in spec §12 → gaps** (trend chip + series allowlist depend on them).
- **Read getters** (`metrics.rs` `get_project_metrics:480`, `get_project_metric_series:564`,
  `get_project_metric_trend:525`, `get_ftr_daily:177`, `get_project_ftr*:717/763`,
  explainer `:290-403`) are all `WHERE project_id` + `folder_id IS NULL` → re-source through the
  new views + a `scope` arg. **`get_holistic_ftr:779` reads `activity.sessions` directly** — a D12
  violation needing its own holistic view.
- **API (additive, back-compat):** `scope=me|repo|team` (default `me`), `view=series|snapshot|
  compare|distribution`; `period` collides with existing `grain` (decide alias + quarterly's fate).
  Registry response should expose `cadence`/`scope`/distribution-eligibility.
- **App:** `metric-view.ts` has no scope/repo/team/compare/quantile concept; `seriesDistribution:941`
  is min/mean/max, not quantiles. Add scope toggle + `view=compare` (two aligned series) +
  `DistributionChart.svelte` (box/violin, D13 families only). Adjacent `project_quality_signals`/
  `project_hotspots` stay project-scalar → will contradict the new repo/user numbers (align them).
- **Test blast radius:** `metric-view.spec.ts` (27K), `DetailChart.spec`, `DatapointDrilldown.spec`,
  e2e `project-window-metrics.spec.ts` — extend without breaking existing series assertions.

---

## 3. Consolidated gaps / additional surfaces (prioritized)

| # | Gap | Severity | Resolution |
|---|-----|----------|------------|
| G1 | **Pruner capture-guard** not migrated → data-loss re-opens | 🔴 blocker | Carry `authorizes_capture` scope into `ComputeGroupMetrics`; invariant I20 |
| G2 | `walk_for_git` doesn't recurse into checkouts → nested masked | 🔴 blocker | Fix recursion + `.git`-file + depth in P-A scanner step |
| G3 | 15,697 project-scope rows have no derivable `repository_id` | 🔴 resolved | Delete old-grain rows **before** index swap (else `NULL` collide); project = view, **no project-scope recompute**; repo-grain history via engine `min_date` fill (lossless) |
| G4 | Identity-index swap silently no-ops (`if not exists`) | 🔴 correctness | Manual `DROP INDEX` + new name `_v2`; update Rust `ON CONFLICT` in lockstep |
| G5 | `repo_key` normalizer doesn't exist; rename-match uses raw URLs | 🟠 | Shared pure `normalize_repo_key()`; back `find_live_root_by_remote` too |
| G6 | Cadence source unresolved (registry col vs code map) | 🟠 decision | **Registry `metrics.cadence` enum** + split `rework_density` (§4) |
| G7 | `project_metric_trend` + `quarterly` absent from spec views | 🟠 resolved | Re-key trend to `(repository,scope)`; keep quarterly as a **view**, `project_metric_quarterly` — **no `v_` prefix on any view** |
| G8 | `get_holistic_ftr` bypasses views (reads `activity.sessions`) | 🟠 | New holistic view or it never becomes repo/scope-aware |
| G9 | `dojo` scope missing `sensei.metric_grain` include | 🟠 | Add to `design.yaml` dojo `includes` before dojo deploy (P-C) |
| G10 | `index_audit` can't detect nested *git* (only standalone) | 🟠 | New disk-walking nested-git detector |
| G11 | 486 session rows collide under new identity (no `session_id`) | 🟡 resolved | **No stored session rows** — delete them; derive per-session from granular activity data via a view; drill-down re-sources |
| G12 | `false_crash_rate` declared-but-uncomputed phantom | 🟡 | Implement or retire |
| G13 | Box/violin types lag runtime in `@rokkit/chart@1.3.13` | 🟡 | Import `GeomBox`/`GeomViolin` typed; no `@ts-expect-error` |
| G14 | Adjacent `project_quality_signals`/`project_hotspots` stay project-scalar | 🟡 | Align to repo/user grain or they contradict |
| G15 | `.git/HEAD`-only reconcile trigger; submodule/worktree HEADs differ | 🟡 | Generalize `is_branch_switch` beyond literal `.git/HEAD` |
| G16 | ~30 `upsert_project_metric` call-sites + old-chain tests rewrite | 🟡 | Mechanical, compiler-caught; budget for it |

---

## 4. Decisions — RESOLVED (2026-08-18)

- **G3 / project-scope rows → RESOLVED.** Project-scope is a **view** (D2) — *no project-scope
  recompute*. Delete the old-grain rows (project-scope 15,697 + module 1,043 + session 486)
  **before** the identity-index swap — with `project_id` gone from the index, `repository_id = NULL`
  rows collide and the build fails. Repo-grain history repopulates via the watermark engine's
  `min_date` fill (source intact → lossless).
- **G6 / cadence → RESOLVED.** `metrics.cadence` registry enum `('commit','day')` (data-driven);
  **split `rework_density` out of `churn`** into its own day-cadence group so every group is
  single-cadence for the watermark.
- **G7 / quarterly → RESOLVED.** Keep a quarterly **view** (re-keyed to repo grain); **no `v_`
  prefix** — house naming (`project_metric_quarterly`). All new views drop the `v_` prefix.
- **G11 / session rows → RESOLVED.** **No stored session-grain rows.** Per-session values derive
  on demand from granular activity data (sessions/outcomes) via a view; the drill-down re-sources
  from there. Delete the 486 rows; drop `session_id` from the metric grain.
- **Canonical name → LOCKED.** `sensei.repositories` = the canonical table; drop the (consumer-less)
  view; `sensei.repo_folders` compat view only if a psql surface is wanted.

---

## 5. Phase readiness

- **P-A** is buildable once G1/G2/G3/G4/G6 decisions are locked (all above). Order within P-A:
  scanner recursion+`.git`-file+`repository_id` → repositories table + folders cols + enums →
  watermark table + engine refactor (carrying the pruner guard) → view rewrites → read-getter repoint.
- **P-B** (quality commit-walk) depends on P-A's `repositories(project)` resolver + `repository_id`
  grain + the local-identity set (`get_user_for_project` / `git_identity.rs`).
- **P-C** (dōjō sync) is post-v1; its DDL gaps (G9) are pre-noted.

Net: the spec is sound in shape; the rebuild's risk is concentrated in **G1 (pruner)**, **G2
(scanner recursion)**, **G3/G4 (migration of the 15,697 rows + index swap)** — all now explicit.
