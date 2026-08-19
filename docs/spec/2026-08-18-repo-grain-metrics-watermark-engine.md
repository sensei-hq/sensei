# Repo-grain metrics + watermark engine + user-attributed quality

Status: **DRAFT — decisions D1–D13 locked. No code until phase P-A starts.**
Date: 2026-08-18
Supersedes: the metric half of the `PlanMetricDays → ComputeMetrics{group,day} → ComputeHealth`
chain and the planner `covered_days`/`effective_from` coverage logic (commit `81c49c2d`).
Related: `docs/spec/2026-08-18-project-repo-session-model.md` (P0–P5 repo-anchor model this builds on).

---

## 1. Problem

Metrics are computed at **project** level by scanning a **single repo** (`project_root_path`
= shortest git/standalone folder, `LIMIT 1`). Consequences observed live:

- **Multi-repo blind spot.** 6 projects already have >1 repo; only one is measured. Grows
  as collection-grouping (P3b) lands.
- **No structural-change trigger.** Scan/reconcile/clone/create/move enqueue no metric
  compute; only the daily scheduler tick, the manual backfill endpoints, and
  transcript-driven analyze do. A new repo waits up to 24h — or forever for git-only
  metrics if no session ever lands.
- **Coverage logic is fragile.** The planner diffs `covered_days` with `effective_from`
  (the OR→AND→effective-from saga). A global `metrics.last_run` watermark is one clock for
  all repos.
- **Quality is polluted / empty.** `qlty` scans the whole tree at a commit → in a team repo
  the number reflects teammates' code, not the user's. Duplication double-counts across
  authors. Locally `module_quality = 0` because the sampled historical commits predate (or
  differ from) today's in-tree `.qlty`.
- **Local identifiers can't federate.** `folder_id`/`abs_path` differ per machine; there is
  no portable repo identity to aggregate a team on.

## 2. Objective

Measure whether **AI sessions maintain / degrade / improve quality**, per repo, per user,
over the observed window of AI activity — with a resumable, watermarked engine that only
touches active repos, a clean **me vs team** comparison correct by construction (attribute at
derivation, never at the aggregator), and a portable repo identity so a team consolidates
without redundant compute.

---

## 3. Decisions (locked)

- **D1 — Repo-grain storage.** Metric values are stored at repo grain keyed on the stable
  `repository_id` (D10) alone — **no `project_id` in the metric identity**. A repository is
  computed once and surfaces in every project that contains it.
- **D2 — Project = a group of repositories (view).** `project ↔ repository` is M:N derived
  through the `folders` junction; a project metric is a **view** aggregating its repositories.
  Ratios pool `Σnumerator/Σdenominator`; counts sum; never average-of-averages. No stored
  project-scalar rows.
- **D3 — Project-orchestrated run, one frozen `as_of`.** A parent task stamps
  `as_of = now()` once at kickoff and hands the same timestamp to every group child, owns
  retry / fill / watermark advance, and emits the health barrier.
- **D4 — Per-(repo × group) watermark.** Watermark *logic* lives in the parent; the *value*
  is keyed `(repository_id, metric_group)` so cadences seal independently and a failing group
  can't stall or re-do healthy ones. Retires `covered_days`/`effective_from` and the global
  `metrics.last_run`.
- **D5 — Cadence per metric.** `day` (session_outcomes, autonomy, tool, knowledge, health)
  reopens the trailing/open day; `commit` (quality, churn) is immutable-once-scanned and only
  adds new commits. Cadence is a registry enum `metrics.cadence ('commit','day')` (data-driven,
  handles intra-group variance); `rework_density` splits out of `churn` into its own day-cadence
  group so every group is single-cadence for the watermark. **No stored `session`-grain rows** —
  per-session values are *derived on demand from granular activity data* via a view (G11); the
  drill-down re-sources from there, not from a stored metric row.
- **D6 — Quality/churn attributed to the local user.** Enumerate only commits authored by
  the local user's git identity; within each, scope the metric to the files that commit
  touched (file-level for v1; line/hunk intersection is a v2 refinement). Numerator **and**
  denominator move together onto that surface. The reported value pools the user's surface
  over the watermark window (kills low-N noise).
- **D7 — Dual `scope ∈ {user, repo}`, one scan → two derivations.** A single qlty scan of a
  commit yields both the whole-tree author-agnostic number (`scope='repo'`) and the
  user-attributed number (`scope='user'`). No second qlty run.
- **D8 — Dōjō: per-user by membership + repo-overall deduped by `(repository, sha)`, never
  summed.** Team number for session-family = pooled per-user (disjoint → clean); for
  git-family = the deduped whole-tree `repo_metrics` (summing double-counts + omits untouched
  code). Members collectively cover the commit DAG; dōjō unions their reports.
- **D9 — Peer visibility: private + team aggregate (default).** A member reads their own
  `member_metrics` + the team aggregate + `repo_metrics`; peers' individual numbers stay
  private. Attribution (`membership_id`) is always stored, so member-vs-member is a future RLS
  toggle (`engagement.metric_visibility = transparent`), not a migration.
- **D10 — Repositories are the canonical, global entity + remote-URL seam.** `folder_id`/
  `abs_path` are local and never cross the boundary. The portable identity is `repo_key` = the
  **normalized remote URL** (`git@host:Org/Repo.git` and `https://host/Org/Repo` →
  `host/org/repo`; strip scheme/creds/port/`.git`, lowercase host). `sensei.repositories` is
  **global** (`unique(repo_key)`, no owning project); **repo-kind `folders` reference it N:1**
  (`folders.repository_id`), so multiple checkouts collapse to one identity — a **standalone
  clone and a subtree embedding of the same remote are one repository**, as are two worktrees
  or two branches (D14). **Metrics key on `repository_id`**, so history survives a
  re-checkout/prune and a repo shared across projects is measured once. `dojo.repositories` is
  the canonical `(tenant, repo_key)`. A repo with no remote (`repo_key` null) is **never
  federated** — local views only. Tightens the P1 `repo_key = "remote/abs_path"` definition
  (the abs_path fallback was the leak). Only a checkout's **root** (repo-kind) folder carries
  `repository_id`/`branch`; nested subfolders reference their parent and resolve the repository
  via the nearest ancestor (`repo_anchor_for`) — folders *in* a checkout, not checkouts.
  **Name-collision resolved (decision):** `sensei.repositories` is currently a VIEW
  (folders-as-repos, 7 consumers) — **drop it; the canonical TABLE takes the name**, and a new
  view (e.g. `sensei.repo_folders`) preserves the old folders-shaped projection for the existing
  consumers (some may instead move to the canonical table — per the DDL impact map).
- **D11 — Two-way sync, repo-in-dōjō enrollment, compute-only-if-missing.** A repo syncs to a
  dōjō only if enrolled (`dojo.repositories(tenant_id, repo_key)`) — bounds what publishes and
  to whom. **Push** `sensei → dōjō`: individual `member_metrics` + own-commit `repo_metrics`.
  **Pull** `dōjō → sensei`: consolidated team aggregate + repo-overall, cached locally
  `source='federated'`. **Compute-avoidance:** a device scans only commits it authored (the
  me-number is local by construction); repo-overall for teammates' commits is pulled from
  `repo_metrics(repository, sha)`, never recomputed; session-family team is always pulled.
  Dōjō is the shared compute cache — no leader election. Miss = honest-empty until sync
  catches up.
- **D12 — Views are the only consumer contract.** Every consumer (app, API, MCP, dōjō console)
  reads **views**, never base tables. One normalized point view + rollup (daily/weekly/monthly)
  + comparison + distribution views on each plane. Base-table shape can evolve without breaking
  consumers.
- **D13 — Distributions only where readings are many.** Box/violin/quantile views are defined
  only for families whose period holds many independent readings (commit-grain quality/churn,
  session-grain latency/throughput). An already-pooled daily ratio has no distribution — no
  box plot is emitted for it (honest, not a fabricated spread).
- **D14 — Branch lives on the folder, not the repository.** A `folder` is a checkout (path +
  `branch`/worktree); the repository is branch-agnostic. `develop` and `main` are two folders
  sharing one `repository_id`; the commit-walk keys on **sha**, so commits on both branches
  dedupe into one repository series. Branch-scoped metric views ("main vs develop") are a
  possible future (§17), not a metric-identity dimension now. **Scope boundary:** `branch` is a
  metrics-only seam here — it does NOT make the indexer or code graph branch-aware. Metrics are
  already safe against a branch/worktree deletion (folder delete → session `SET NULL`; the
  repository survives, D10), but the folder-keyed **code graph is not** — full branch-awareness
  is a separate post-metrics program (§17).
- **D15 — Scanner: a checkout is any `.git` (dir OR file), at every depth, incrementally.**
  Repo-root detection today is `.git`.is_dir()-only (`scan_logic::walk_for_git`) — it **misses
  worktrees + submodules** (`.git` is a *file*), which D14 relies on, and can mask a nested
  clone as a plain subdir. Rules: (a) a checkout = a `.git` dir **or** file; identity via
  `git -C <p> remote get-url origin` → `repo_key`; (b) nested checkouts are their own
  repository roots (different remote ⇒ different repository; a worktree/submodule of the same
  remote collapses via `repo_key`); (c) **incremental-safe** — a create/update under a watched
  tree must re-detect a newly-appeared `.git` (a clone masked as a subdir update) and
  **reconcile** (create the repository, re-parent + re-anchor the subtree), never fold it into
  the parent; (d) detect `.git` **before** ignore/prune (never prune a checkout via a parent's
  ignore rules); (e) `index_audit::detect_nested_standalone_roots` extends to nested-**git** as
  a backstop. True git-subtree (merged, no nested `.git`) → standalone-repo mapping stays
  content-level (§17).
- **D16 — Sync enrollment & isolation (reuse federation auth; enforce server-side).** No second
  auth model. Connecting sensei to a dōjō = **optional login** (kavach magic-link/password/
  GitHub; `authenticated_via ∈ {sso, github_oauth, device_code}`) → a `sensei.dojo_memberships`
  row (the authorized membership) + a device credential in the **OS Keychain** (`credential_ref`
  — never in PG/logs). A user may hold MANY memberships (multi-dōjō); a repo routes to one by
  `org_slugs` (`repo_key` owner slug ∈ membership) with `kind` breaking ties, recorded on
  `sensei.repositories.dojo_id`. **All sync is daemon → Worker with the device credential; the
  Worker enforces isolation server-side** (validates credential → membership → tenant, and that
  the repo is enrolled in that tenant) and NEVER trusts a daemon-asserted tenant/membership.
  Client-direct RLS (`owns_membership`/`is_repo_member`) is defense-in-depth for the phone/
  console read path. Sync is OPT-IN; local-only / unenrolled repos never sync; pulled rows are
  tagged with their source tenant so multi-dōjō data never cross-renders.

---

## 4. The engine

Replaces the three-task chain with a parent → cadence-aware children → barrier shape (the
house pattern the old chain already gestured at).

```
ComputeProjectMetrics { project_id }              -- parent / orchestrator
  as_of := now()                                   -- one frozen clock for the whole run
  repos := repositories(project)                   -- canonical repos (D10)
  for each metric_group:
      wm := watermark(repository, group)           -- read per-(repo × group)
      spawn ComputeGroupMetrics { repos, group, wm, as_of }
  await all; retry failed groups (bounded) against the SAME as_of
  advance watermark(repository, group) only for groups that succeeded
  -> ComputeHealth (barrier; combines groups at the same as_of)

ComputeGroupMetrics { repos, group, wm, as_of }    -- child, owns cadence
  cadence = day  -> fill [reopen(wm) .. as_of.date]; reopen pulls back to the open day
                    and to any day with late-arriving sessions (min(wm, earliest_late-1))
  cadence = commit -> walk the LOCAL USER's commits in (last_sha(wm) .. as_of];
                    for each: pull repo-overall from dōjō if present (D11), else scan
                    (1st full, rest cache-accelerated); derive scope=user + scope=repo
  write repo rows (repository_id); ratios carry props.numerator/denominator
```

**Enqueue side.** The scheduler selects **active repositories**
(`max_activity_day > sealed_through`) and enqueues one `ComputeProjectMetrics` per project
with active repos. The structural-change trigger (P-C) enqueues it for a new/moved repo (unset
watermark → full fill from `min_date`). Transcript-ingest continues to fire it for the
just-active project so evening work appears without waiting for the daily tick.

### 4.1 Scanner — the enqueue precondition (D15)

`repo_anchor_for` can only anchor to a repository the **scanner created** — so a masked
checkout ⇒ misattributed metrics (the nested subtree's work credited to the outer repo). The
detector must therefore be robust *before* any compute:

- **Detect** a checkout by a `.git` dir **or** file at every depth; derive `repo_key` from the
  resolved origin remote. ⚠️ `walk_for_git` today **halts recursion at the first `.git`**
  (`scan_logic.rs:67-71`) so nested checkouts are never discovered — fixing the recursion halt
  (and the depth-3 bound, `scan.rs:53`) is the *core* fix; `.git`-file support alone is not enough.
- **Incremental reconcile:** a file event under a watched tree re-checks for a `.git` newly
  appearing at/below the event path; if found, promote it to a repository root and re-anchor
  its subtree — do **not** merely mark the subfolder updated.
- **Backstop:** `index_audit` extends its nested-root detection/repair to nested-git, so a
  masked checkout that slipped through is caught and repaired (mirrors the existing
  nested-standalone repair).

Builds on `scan_logic::walk_for_git` + `resume.rs` `.git` validation. NOTE: the Rust
`FolderKind` enum has only `Git`/`Standalone` (`scan_logic.rs:22-29`) — `subtree`/
`workspace_member` are DB-only strings the scanner never emits. And `index_audit`'s
`detect_nested_standalone_roots` is a DB-shape query that **cannot** see a nested *git*
checkout (no row exists) — D15(e) needs a **new disk-walking** nested-git detector.

## 5. Watermark model

New table: `sensei.metric_watermarks (repository_id, metric_group, sealed_through date,
last_sha text, updated_at)`.

- `min_date(repo)` = `least(min(sessions.started_at anchored to repo),
  min(assistant_events.ts for repo))`. None → honest-empty no-op.
- Unset watermark → fill from `min_date`.
- Each run fills the tail `(sealed_through .. as_of.date]`; **today is never sealed**
  (`sealed_through := as_of.date - 1`), so the open day is always recomputed.
- **Late/backdated data:** events dated before `sealed_through` → roll back to
  `earliest_late - 1` only.
- Commit cadence stores `last_sha`; a scanned commit is immutable, so only commits after
  `last_sha` are added.

## 6. Cadence

| Group | Cadence | Iterated unit | Recompute rule |
|---|---|---|---|
| session_outcomes (ftr, throughput, rework_ratio, time_to_useful_result, context_pressure_rate) | Day | sessions of that day | reopen trailing/open day + late days |
| autonomy (false_crash_rate, interruption_rate, run_completion) | Day | runs/sessions | reopen trailing/open day |
| knowledge, tool | Day | — | reopen trailing/open day |
| churn | Commit | user's commits | immutable; new commits only |
| quality (module_quality, duplication_ratio) | Commit | user's commits | immutable; new commits only |
| health | Day (derived) | rolls up the above | barrier at `as_of` |

## 7. Quality/churn commit-walk

- **Config pinning.** Inject one fixed `.qlty` config into each worktree before scanning (do
  not trust the commit's in-tree config). Makes pre-config history measurable and every commit
  measured against the *same ruler* — the precondition for a comparable trend.
- **Cache-accelerated, not additive.** First scan = full; subsequent point every worktree
  scan at one shared qlty content-cache so only changed files recompute. Each commit stays a
  *true full reading*; duplication (a global signal) stays correct. Never do arithmetic
  `prev + Δ` (unsound for duplication). Confirm qlty's cache-dir flag at build; fall back to
  full scans on a bounded commit set if absent.
- **Commit budget.** Default = the user's last 7 commits; full backfill (opt-in) = all the
  user's commits since `min_date`.
- **Attribution (D6).** `git log --author ∈ {local identities}`; post-filter qlty SARIF to
  the commit's touched files (v1); numerator + denominator both on that surface; pool over the
  window. `props.author_identity` for provenance.
- **Compute-sharing (D11).** Skip the scan for a commit whose `scope='repo'` value already
  exists in dōjō `repo_metrics(repository, sha)` — pull it (`source='federated'`). A device
  only ever scans its own commits.

## 8. DB — local plane

Canonical repository entity + folder reference + metric key move onto it:

```sql
-- DROP the existing sensei.repositories VIEW first; the canonical table takes the name.
-- Existing folders-as-repos consumers move to a new view `sensei.repo_folders` (same
-- projection over folders where kind in git/standalone) — see the DDL impact map.
drop view if exists sensei.repositories;

create table sensei.repositories (            -- GLOBAL, canonical, keyed on remote (D10)
  id uuid primary key default gen_random_uuid()
, repo_key   text            -- normalized remote (host/path); null = local-only (never federated)
, remote_url text            -- raw origin
, name       text
, dojo_id    uuid            -- dōjō repository linkage (like sensei.projects.dojo_id); null = not enrolled
, created_at timestamptz not null default now()
, updated_at timestamptz not null default now()
, unique (repo_key));        -- one canonical repository per remote; no owning project

-- ONLY the root (repo-kind) folder is a CHECKOUT: it carries repository_id + branch (N:1, D10/
-- D14). Nested subfolders leave both NULL and resolve their repository via the nearest ancestor
-- (repo_anchor_for). folders is the project↔repository junction (folder already carries project_id).
alter table sensei.folders
  add column repository_id uuid references sensei.repositories(id) on delete set null
, add column branch        text;   -- root-folder only; repository stays branch-agnostic

-- metrics connect to the repository, not the folder or project (D1); + scope/identity/commit
alter table sensei.project_metrics
  add column repository_id uuid references sensei.repositories(id) on delete cascade
, add column scope         metric_scope not null default 'user'   -- enum {user, repo}
, add column identity       text                                   -- git identity for scope=user; null for scope=repo
, add column commit_sha     text;                                  -- commit-cadence rows; null for day/session

drop index sensei.project_metrics_identity;   -- manual (dbd can't drop) then recreate
create unique index project_metrics_identity on sensei.project_metrics
  (metric_id, repository_id, scope, identity, commit_sha, computed_on, grain)
  nulls not distinct;                         -- NO project_id — repository is the grain (D1)
```

- New enum `sensei.metric_scope {user, repo}`; `metric_source` gains `federated` (enum add).
- `repository_id` supersedes `folder_id`/`project_id` as the metric grain; `project_metrics.
  project_id` becomes nullable/derived (a project's metrics = a view over its repositories via
  the folders junction) — physical column kept for lookup convenience, out of the identity.
- Session-family metrics stay `scope='user'` (no whole-tree twin) — the default leaves them
  untouched.

## 9. DB — dōjō plane

Canonical repo + enrollment, then the two metric tables reference it:

```sql
create table dojo.repositories (              -- enrollment: repo-in-dōjō
  id uuid primary key default gen_random_uuid()
, tenant_id uuid not null references dojo.tenants(id)
, repo_key text not null, remote_url text, name text
, enrolled_at timestamptz not null default now()
, unique (tenant_id, repo_key));

create table dojo.member_metrics (            -- per-user attributed; PRIVATE to owner
  id uuid primary key default gen_random_uuid()
, membership_id uuid not null references dojo.memberships(id)
, repository_id uuid not null references dojo.repositories(id) on delete cascade
, metric_key text not null
, commit_sha text, computed_on date not null, grain metric_grain not null
, value numeric not null, numerator numeric, denominator numeric, props jsonb not null default '{}'
, created_at timestamptz not null default now(), updated_at timestamptz not null default now()
, unique (membership_id, repository_id, metric_key, commit_sha, computed_on, grain));

create table dojo.repo_metrics (              -- whole-tree, author-agnostic; team-readable
  id uuid primary key default gen_random_uuid()
, repository_id uuid not null references dojo.repositories(id) on delete cascade
, metric_key text not null
, commit_sha text, computed_on date not null, grain metric_grain not null
, value numeric not null, numerator numeric, denominator numeric, props jsonb not null default '{}'
, created_at timestamptz not null default now(), updated_at timestamptz not null default now()
, unique (repository_id, metric_key, commit_sha, grain));   -- deterministic → any member upserts identically
```

RLS (D9 default; service_role writes bypass, like `relay_sessions`):

```sql
create policy member_metrics_select_own on dojo.member_metrics
  for select to authenticated using (dojo.owns_membership(membership_id));
create policy repo_metrics_select_team on dojo.repo_metrics
  for select to authenticated using (dojo.is_repo_member(repository_id));
```

New helper `dojo.is_repo_member(rid)` (the P6 team-visibility extension the relay_sessions
comment foreshadowed): `exists(select 1 from memberships m join repositories r on
r.tenant_id = m.tenant_id where r.id = rid and m.user_id = auth.uid())`, `security definer`,
granted to `authenticated`. Same drop-if-exists idempotency + explicit `grant select` as the
relay tables.

## 10. Sync & compute-sharing (D11)

- **Enrollment.** Local `sensei.repositories.dojo_id` links a repo to its dōjō row; publish
  only enrolled repos, only to that tenant.
- **Push (sensei → dōjō)**, via the existing Worker service_role seam (like relay):
  `member_metrics` (my `scope='user'`) + `repo_metrics` (my own-commit `scope='repo'`).
- **Pull (dōjō → sensei):** consolidated team aggregate + repo-overall for enrolled repos →
  cached locally as `scope='repo'`/team rows, `source='federated'` (I1 still holds: a miss is
  honest-empty, never a defaulted 0).
- **Compute-sharing:** each device scans only its own commits; the union across members (D8)
  builds the full repo-overall series in dōjō; everyone pulls the rest.

### 10.1 Enrollment & security (D16)

- **Enrollment (opt-in).** Sensei.app "Connect to dōjō" → kavach login (magic-link/password/
  GitHub) → dōjō associates this device → daemon writes a `sensei.dojo_memberships` row + stores
  the device credential in the OS Keychain (`credential_ref`). One row per membership (multi-dōjō).
- **Routing.** A repo's `repo_key` owner slug selects the membership whose `org_slugs` covers it
  (`kind` breaks ties) → `sensei.repositories.dojo_id`. Unmatched / local-only repos never sync.
- **Write gate (server-side, every write).** The Worker accepts a write for (repo R, membership
  M, tenant T) only if: M's credential validates ∧ `R.owner_slug ∈ M.org_slugs` (or R enrolled
  in M.tenant) ∧ `T = M.tenant`. The daemon's asserted tenant/membership is never trusted.
- **Read isolation.** Daemon pulls are Worker-mediated under the same checks; phone/console read
  client-direct under RLS (`owns_membership`, `is_repo_member`). No service_role read path is
  exposed to a client.
- **Credentials.** Device token in the OS Keychain only (never PG/logs); per-device, revocable
  (`enabled`/`disabled_at`), rotated by re-pairing.
- **Local tagging.** Federated/pulled rows carry their source tenant/membership; multi-dōjō data
  never cross-renders.

## 11. Aggregation semantics

`team = Σ users` **only when the underlying events are partitioned by user.**

| Family | Events | Team number | Cleanliness |
|---|---|---|---|
| session_outcomes, autonomy, tool, knowledge | partitioned per device/user | pool of per-user series | disjoint → clean |
| quality, churn (git) | shared tree, cross-author duplication | separate whole-tree scan, deduped by `(repository, sha)` | can't sum → compute it |

## 12. Views & analytics layer (D12/D13)

Consumers read only these. Every view carries `(repository_id/project_id, metric, scope,
period, grain, value, numerator, denominator, source)` so the app can label provenance.

**Local (`sensei`) — NO `v_` prefix; match the existing `project_metric_*` house naming:**
- `metric_point` — atomic reading normalizer over `project_metrics`.
- `metric_me` / `metric_repo` — scope filters (`user` with local identity / `repo`).
- `project_metric_daily` / `_weekly` / `_monthly` / `_quarterly` — the **existing** rollup views,
  *re-keyed* to aggregate repo-grain rows per `(repository, metric, scope)` then pool to project
  (Σnum/Σden for ratios, sum for counts). The project number is derived here — **no stored
  project-scope rows** (D2). `project_metric_trend` re-keyed likewise; `session`-grain values are
  a view over granular activity data (D5), not stored.
- `metric_compare` — me vs repo (+ team when federated rows present), aligned on period →
  the overlay/comparison UI.
- `metric_distribution` — quantiles `(min, q1, median, q3, max, outlier-fence)` via
  `percentile_cont` over the raw commit-/session-grain readings in a window. **Defined only
  for D13 families** (many readings per period); empty for pooled daily ratios.

**Dōjō (`dojo`) — same no-`v_` convention:**
- `team_session_metric` — pooled per-user (session-family) across the tenant.
- `repo_metrics` — the git-family team series directly (no view needed).
- `member_vs_team` — a member's series vs the team aggregate (D9-gated: own + team only).
- `repo_snapshot_weekly` / `repo_snapshot_monthly` — team rollups for console/phone.
- `metric_distribution_team` — quantiles across members/commits for team box/violin (D13).

**Wrappers (API/MCP).** Metric read endpoints take a uniform shape:
`scope ∈ {me, repo, team}`, `period ∈ {daily, weekly, monthly}`, `view ∈ {series, snapshot,
compare, distribution}`. One contract for local and federated reads.

**Chart mapping (rokkit).**
- `series` → line/area (DetailChart); `count` → bars + moving-average.
- `snapshot` → pooled weekly/monthly points.
- `distribution` → **Plot.Box / Plot.Violin** (already ship in pinned `@rokkit/chart@1.3.13` at
  runtime — wiring only; import `GeomBox`/`GeomViolin` for types, the `Plot` subpath types lag) —
  per-commit quality spread,
  per-session latency spread, me-vs-team distribution. Rendered **only** where the period holds
  many independent readings (D13); never over an already-pooled daily ratio.

## 13. Migration

- Add `sensei.repositories` + `folders.repository_id` + backfill (one repository per repo-kind
  folder, `repo_key` = normalized origin; multiple checkouts of one remote collapse).
- Add `project_metrics` columns + enums + watermark table + dōjō `repositories`/`member_metrics`/
  `repo_metrics` (dbd reconcile for additive cols; manual index drop/recreate — dbd can't drop).
- **Delete the old-grain rows, then repopulate at repo grain.** The existing project-scope
  (15,697), per-module (1,043), and session (486) rows are superseded — and once the index drops
  `project_id`, two projects' same-metric/same-date rows with `repository_id = NULL` **collide**
  under the new unique index (build fails). So DELETE them **before** the index swap. Project /
  module / session numbers become **views** (D2/D5, no project-scope recompute); repo-grain
  *history* repopulates via the watermark engine's normal `min_date` fill — source data (sessions,
  git) is intact → **lossless**. Never stamp a fabricated primary repo (no-fabrication rule).
- Identity-index swap is manual (`create index if not exists` silently no-ops on a name match):
  `DROP INDEX` old, create the new (recommend `project_metrics_identity_v2`), and update the Rust
  `upsert_project_metric` `ON CONFLICT` list **in lockstep** or the upsert throws at runtime.
- Replace `PlanMetricDays`/`ComputeMetrics{group,day}` with `ComputeProjectMetrics`/
  `ComputeGroupMetrics`; keep `ComputeHealth`. **Migrate the pruner capture-guard (I20) first.**
- Delete planner `covered_days`/`effective_from` (incl. `81c49c2d`) and the global
  `metrics.last_run` — the watermark is coverage.
- First engine run per repo fills from `min_date` (unset watermark).

## 14. Invariants

- **I1** No data → no row (honest-empty; never a defaulted 0). `source='federated'` still obeys
  this — a sync miss is honest-empty.
- **I2** Every ratio row carries `props.numerator` + `props.denominator`.
- **I3** A repo-derivable metric is stored at `repository_id`; project value is a view.
- **I4** `scope='user'` rows carry `identity`/`membership_id`; `scope='repo'` rows carry none.
- **I5** Today is never sealed; the open day is recomputed every run.
- **I6** A commit is scanned at most once per (repo, metric) — immutable.
- **I7** Quality/churn `scope='user'` counts only the local user's commits ∩ touched files.
- **I8** Dōjō `repo_metrics` is unique per `(repository, metric, sha, grain)` — never summed.
- **I9** A member reads only their own `member_metrics` (D9) + tenant `repo_metrics` + team views.
- **I10** `repository_id` is the repo-grain key; `folder_id`/`abs_path` never cross the
  daemon→dōjō boundary. `repo_key` is the only portable identity.
- **I11** `repo_key` = normalized remote; two clones of the same remote (ssh/https) yield the
  same key; `repo_key` null ⇒ never federated.
- **I12** A device scans only commits it authored; teammates' repo-overall is pulled, not
  recomputed.
- **I13** A distribution view emits rows only for many-reading families (D13).
- **I14** One `repository` per `repo_key`; a standalone clone and a subtree/worktree of the
  same remote share it and are measured once (surfacing in every containing project).
- **I15** Branch is a `folder` attribute; two branch-checkouts share the repository and their
  commits dedupe by sha into one series.
- **I16** Only a checkout's root (repo-kind) folder carries `repository_id`/`branch`; a nested
  subfolder carries neither and resolves its repository via the nearest ancestor.
- **I17** A checkout is any `.git` dir OR file at any depth (worktrees/submodules included); a
  nested checkout is its own repository, and an incrementally-appearing `.git` reconciles into a
  new repository (+ re-anchor), never a masked subdir update.
- **I18** A metric write is accepted only when the credential validates ∧ the repo is enrolled
  in ∧ `tenant = membership.tenant` — enforced server-side; a daemon-asserted tenant is ignored.
- **I19** A user reads only their own `member_metrics` + `repo_metrics` for member tenants; a
  repo's data reaches only enrolled tenants; credentials live in the OS Keychain (never PG/logs);
  federated rows are tenant-tagged and never cross-rendered.
- **I20** The activity-pruner's capture-before-reclaim guard (today `planner.rs authorizes_capture`
  / `day_keyed_task_names`) is preserved through the engine refactor — reclaim never runs before
  the metric that consumes the data has captured it. (Retiring the planner must NOT drop this — it
  is the guard from the 164GB data-loss incident.)

## 15. Rollout phases

- **P-A — grain + rollup foundation.** `sensei.repositories` + `folders.repository_id`;
  `scope`/`identity`/`commit_sha`/`repository_id` on `project_metrics`; enums; watermark table;
  churn + session_outcomes at repository grain; project = aggregation view;
  `ComputeProjectMetrics`/`ComputeGroupMetrics` with frozen `as_of` + watermark; retire
  `covered_days` + global clock; the local `v_*` views.
- **P-B — quality commit-walk.** Config pinning + shared cache + commit budget + `scope` dual
  derivation + user attribution (file-level).
- **P-C — triggers + dōjō sync + UI.** Structural-change trigger (clone/create/move → targeted
  recompute); dōjō `repositories`/`member_metrics`/`repo_metrics` + RLS; **enrollment & auth**
  (reuse `sensei.dojo_memberships` + Keychain credential, D16) + Worker push/pull with
  server-side tenant enforcement; two-way sync + compute-sharing; dōjō team/compare/distribution
  views; app me-vs-team overlay + commit x-axis + box/violin where D13 applies. **This phase is
  the dōjō-dependent slice — deferrable past a v1 that ships P-A/P-B locally (see phases.md).**

## 16. Atomic tests

1. `min_date` = earliest of session/event for a repo; none → no-op.
2. Watermark fill `(sealed_through .. as_of]`; today recomputed; `sealed_through = as_of-1`.
3. Late event before watermark → reopen to `earliest_late-1` only.
4. Commit cadence scans each new commit once; re-run adds no duplicate row (I6/I8).
5. Frozen `as_of` shared across all group children in a run.
6. Failed group keeps its watermark; healthy groups advance (retry isolation).
7. Config pinning: a commit predating in-tree `.qlty` still scans against the pinned config.
8. One qlty scan → both `scope='repo'` and `scope='user'` rows (D7).
9. Attribution: a teammate-authored commit contributes no `scope='user'` row locally (I7).
10. Ratio scoping: `scope='user'` numerator and denominator both on the touched surface.
11. Project view pools `Σnum/Σden` across repositories (not mean-of-ratios).
12. `repo_key` normalization: ssh + https clone of one remote → identical key (I11).
13. Metrics + folders reference `repository_id`; a folder prune keeps repository + its
    metric history (I10).
14. Enrollment: a non-enrolled repo publishes nothing to a tenant (D11).
15. Compute-sharing: repo-overall present in dōjō → local scan skipped, row `source='federated'`
    (I12).
16. Dōjō `repo_metrics` upsert idempotent for two members writing same `(repository, sha)`.
17. RLS: member reads own `member_metrics`, not a peer's (D9); reads tenant `repo_metrics`.
18. Team views: session-family = pooled per-user; git-family = `repo_metrics`.
19. Distribution view emits quantiles for commit-grain quality; emits nothing for a pooled
    daily ratio (I13/D13).
20. Subtree + standalone checkout of one remote → one `repository`; metric computed once,
    appears in both containing projects' views (I14).
21. Two folders on `develop` and `main` share the repository; a commit on both branches yields
    one metric row (dedupe by sha) (I15).
22. A nested subfolder has null `repository_id`; a path inside it resolves to the ancestor
    root folder's repository (I16).
23. A worktree/submodule (`.git` file) is a checkout root; a `git clone` into a watched tree
    (incremental) reconciles into a new repository, not a subdir update (I17).
24. Server-side write gate: a membership in tenant A writing a repo enrolled in tenant B is
    rejected; a daemon-asserted tenant ≠ membership.tenant is ignored (I18).
25. Read isolation: a user reads only their tenants' rows; a local-only/unenrolled repo
    publishes nothing; credential absent → sync no-op (opt-in, honest) (I19).

## 17. Deferred

- Line/hunk-level attribution (v2, tighter than file-level).
- `engagement.metric_visibility = transparent` (member-vs-member view — data already supports).
- Fork/upstream mapping (a fork's origin differs from upstream → distinct `repo_key`; optional
  fork→upstream alias).
- A repo enrolled in more than one dōjō (multi-tenant publish).
- Session cadence as a first-class stored grain (if a per-session-stored metric appears).
- True git-subtree (merged, no nested `.git`) → standalone-repo content-level mapping (D15).

### Branch-awareness program (separate, sequenced AFTER metrics)

Today the system does not track branches; D14 adds only the `branch` seam. Making the system
branch-aware is a large-blast-radius program — each item is its own follow-up with its own
spec, tackled after this metrics work lands:

- **Indexer** — stop blind upsert/delete: a branch (or worktree) deletion must NOT delete the
  repository's index/nodes that remain valid on other branches. Reconcile by branch, not by
  folder existence.
- **Code graph** (nodes / functions / calls / interfaces) — scope by the active branch; the
  graph currently reflects whatever branch last scanned.
- **Scanning changes** — branch-level diff/scan (what changed on this branch vs the base).
- **MCP** — `search` / `get_callers` / `get_callees` / interfaces become branch-scoped.
- **UI** — the graph view scopes to the active branch.
- **Libraries** — indexed lib docs may differ by branch (dependency changes).
