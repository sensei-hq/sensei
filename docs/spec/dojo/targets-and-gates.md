# Targets: eligibility, store, resolution, evaluation, delivery, acceptance

Written 2026-11-03 by L0 for lane L3 (Claude Code B), with slices for L7, L2 and L4. Read first:
`CLAUDE.md`, `docs/plan/2026-11-03-parallel-lanes.md` §1–§3, `docs/decisions.md` (D47–D79),
T3 (`docs/prd/modules/04-team/03-dojo-workspace-planning/README.md`), M14
(`docs/prd/modules/03-delivery/14-mcp-context-delivery/README.md`) and its
`scenarios/work-delivery.feature`, `docs/prd/03-personas.md` §6 ("what must be fair", :100).

Vocabulary (D47): a **target** is a metric threshold. A target in mode `block` makes its miss
**blocking**; the UI says "target missed (blocking)". "Gate" is relay's approval word and never
names a target in code, copy or wire fields. This file keeps its agreed name.

## 1. What exists today (verified 2026-11-03)

**No target exists anywhere.** No table, enum, function, route, MCP tool or screen.
`rg -n "target_results|dojo\.targets|get_missed_targets" --no-ignore -g '!target'` over `crates/`,
`database/ddl`, `dojo/src` and `app/src` finds nothing. `sensei.metrics.target`
(`database/ddl/table/sensei/metrics.ddl:18`, comment :70) is a health-score normalisation bound, not
target configuration.

**Metric catalogue.** One table, `sensei.metrics`, on both planes (`metrics.ddl:2–25`; the dōjō reads
it through `dojo.metric_catalogue`, grants at :96, RLS at :103–105). It already carries `direction`
(`:10`, enum `sensei.metric_direction` = `higher_better | lower_better | neutral`,
`database/ddl/enum/sensei/metric_direction.ddl`) and `unit` (`:9`). Seeded from
`database/import/staging/metrics.jsonl` (34 rows) by `staging.import_metrics()`
(`database/ddl/procedure/staging/import_metrics.ddl`), which upserts on `key` with a `modified_at`
guard. The dōjo scope includes the metric enums and the staging pair by name
(`database/design.yaml`, `scopes.dojo.includes`, the "Metric CATALOGUE" block).

**Readings that exist for the seven targets** (T3 table, "The seven targets"):

| # | Target | Catalogue key today | Local data a reader can use |
|---|---|---|---|
| 1 | Hard-rule violations | none | `sensei.rule_check_runs` (`ddl/table/sensei/rule_check_runs.ddl:15–23`: rule, verdict, `ran_at`; no commit sha), written by `checker.rs:87 run_checkers` |
| 2 | Coverage on guarded paths | `coverage` (ratio, higher_better, task `coverage`) | lcov ingest in `tasks/handlers/metrics/coverage.rs` (`parse_lcov` :109 sums LH/LF across every record; no per-file split) |
| 3 | New layer violations | none | computed on demand, not persisted: `crates/senseid/src/analysis/layering.rs` (Tarjan over module/structure edges; under derived layering every violation is a cycle edge, doc :36–60), so target 3 = the Up-edge count. Declared layers do not exist. #156 (L2) persists it |
| 4 | New dependency cycles | none | the same `layering.rs` cycle computation, on demand; #156 (L2) persists a cycle count |
| 5 | Modules in the zone of pain | none | `sensei.component_zones(p_workspace_id)` (`ddl/function/sensei/component_zones.ddl:17`), current graph only |
| 6 | Changed functions over complexity 10 | none | `sensei.file_complexity` (`ddl/table/sensei/file_complexity.ddl:3–12`): per FILE `complex` count at HEAD; no per-function row |
| 7 | New high-severity anti-patterns | none | none. `inference.detected_patterns` has `is_anti_pattern` and `severity` (`detected_patterns.ddl:10–11`), but the only production insert (`pg_store/patterns.rs:810`) never writes `severity`, and the live rows (727) are session patterns (`rework:`, `correction-prone:`) with null severity. No code anti-pattern producer exists |

L2's #156 list (`parallel-lanes.md` L2 step 3) is cycle count, largest cycle, zone-of-pain module
count, hidden-coupling pairs and complexity outliers. Target 3's Up-edge count comes from the same
`layering.rs` analysis and needs only persisting (an L2 item to request). **No lane plans a code
anti-pattern detector**, so target 7 has no producer.

**Index completeness.** `sensei.folder_completeness.subtree_complete`
(`ddl/view/sensei/folder_completeness.ddl:9–74`) is per folder (one checkout): true only when every
file was parsed or deliberately skipped against the walk's own count, recursively. It reads
`parsed_at`, because `files.indexed_at` is `NOT NULL DEFAULT now()` and so a tautology (view comment
:16–21). It has no production reader today; only tests read it
(`pg_store/tests/graph_tests.rs:865, 953, 1001`).

**Tracked branches and environments (D18).** The dōjō side is built:
`dojo.repository_graph_branches.label` with check `('production','testing','development')`
(`ddl/table/dojo/repository_graph_branches.ddl:16, 20–21`), `dojo.set_graph_branch(…, p_label)`
(`set_graph_branch.ddl:24, 61–63`), and the Worker route accepts a label
(`dojo/src/routes/v1/you/repositories/graph-branches/+server.ts:41–52`, WP3 d71a79ceb). **Missing:
the label is not on the plan wire.** `GraphBranch` is `{ branch, is_default }`
(`dojo_client/user_plane.rs:59–62`, `dojo/src/lib/client-data.ts:146–149`), so the daemon cannot see
which branch is `development`.

**Metric gate (not a target).** `MetricGate` (`tasks/handlers/metrics/mod.rs:382–424`) is the
"compute or skip" switch read from `sensei.metric_deactivations`. It fails open. It is unrelated to
targets; do not extend it.

**Copy pipeline.** The lane plan says "insight_copy". It was renamed: the code is
`crates/senseid/src/analysis/narration_cache.rs` (table `sensei.narration_cache`,
`ddl/table/sensei/narration_cache.ddl`). `InsightKind` (:98–143), `FallbackCopy` (:271),
`copy_or_warm` (:697, never blocks the wire), `read_cached_copy` (:612), `warm` (:717). The file is
L2's (`crates/senseid/src/analysis/**`).

**Dōjō access helpers** (all SECURITY DEFINER, `search_path` pinned, execute granted to
`authenticated`): `current_principal_id` (`ddl/function/dojo/current_principal_id.ddl:28`),
`tenant_role_at_least(uuid, member_role)` (`tenant_role_at_least.ddl:3`, enum order
`contributor < maintainer < lead < admin`, `ddl/enum/dojo/member_role.ddl`),
`repo_role_at_least(uuid, member_role)` (`repo_role_at_least.ddl:6`, override else forge role, only
with an active membership), `administers_workspace(uuid)` (`administers_workspace.ddl:6`),
`can_read_repository(uuid, uuid)` (`can_read_repository.ddl:13`), `can_read_workspace(uuid)`,
`is_member(uuid)`, `is_workspace_viewer(uuid)`, `leads_team_member(uuid, uuid)`
(`leads_team_member.ddl`: caller is `lead`+ in the tenant and shares a team),
`can_read_repository_metric(repo, scope, principal, tenant)` (`can_read_repository_metric.ddl:18–42`:
own user rows; repo rows for a role holder; admin; maintainer+ for another's rows; a team lead).

**Entitlement verdict.** `dojo.all_my_repositories` (`ddl/view/dojo/all_my_repositories.ddl:64`), one
row per (repository, member), service_role only (:458–459), read by definer functions.
`entitlement_refusal` (:216–243) is the capture refusal first (`forge_visibility_unknown` |
`forge_visibility_stale`, :105–109, :217: the forge's visibility is not read yet or is out of date),
then `null` for public, else `unclaimed` (org only) / `not_subscribed` / `subscription_expired`; `may_share = entitlement_refusal is null` (:340).
**Today a personal private repository is refused `not_subscribed`** (note at :205–209). L1's WP8
(`docs/plan/dojo-sync-handoff.md` WP8) adds `when mr.origin = 'personal' then null`. After WP8 the
verdict is exactly D52: personal free, org public free, org private needs a subscription.

**Writer and filer patterns.** `set_metric_activation` (`set_metric_activation.ddl:16–58`): verdict
row of the caller, `no_data_found` (404) for unknown, `insufficient_privilege` (403) when not theirs.
`file_metric_rows` (`file_metric_rows.ddl:23–52`): principal from the session, every row's
repository must be the caller's with `sync_enabled` in `all_my_repositories`, else the **whole batch**
is refused. This is the contract TG4 copies. `file_graph_slices` (`file_graph_slices.ddl:28–75`)
differs: it refuses the whole call for a bad repository or branch, but counts an older slice
(`refused_older`) instead of failing the batch.

**Definer gate test.** `database/tests/dojo/definer_functions_gate_the_caller.sql`: the `listed`
array (:62–73) must name every definer function granted to `authenticated` (:77–85 fails on an
unlisted one); bob, a member of nothing, calls each predicate (:140–162), read (:164–180) and write
(:191–217); a fingerprint of every dojo table must not change (:271–284); positive controls for
alice/carol (:224–267).

**Worker route pattern.** `dojo/src/routes/v1/t/[origin]/[org]/workspaces/[id]/metrics/+server.ts`:
`resolveCaller` first (:32), then the id check (:33, a local `UUID_RE` :22; WP4 round 2 F8 moves it to
a shared `isUuid`, not on develop yet; L1 owns that replacement), then `resolveTenantId` (:35), then the
read with `callerDb(caller)` (:37). `workspaces-data.ts:129–169`: `callFn` gate (403),
`requireInTenant` (404), `readAllPages` with a total order (:144–154; `repositories.ts:244`).
`db-refusal.ts` maps `P0002→404`, `55000→409` (:10), `42501→403`, `22023→422`, `23503/23505→409`,
else 500.
Tenant-floored routes use `resolveTenantCaller(origin, org, request, ACCESS.member)` (e.g.
`routes/v1/t/[origin]/[org]/rules/resolved/+server.ts`).

**Resolved rules pull.** Not a sync and not cached: at `get_rules` time
`api/handlers/knowledge.rs:1284–1308 resolve_adopted_pack_raws` goes folder → workspace → bound
membership → `DojoClient::resolved_pack_rules` (`dojo/client.rs:311–342`,
`GET /v1/t/{tenant}/rules/resolved?ns=…`) and is best-effort. The local replica is
`sensei.rule_packs` (`resolve_local_pack_raws`). `resolve_repo_ruleset` (`knowledge.rs:1354`,
private) folds local + remote; `resolve_folder` (`:1314`, `pub(crate)`) maps a folder or workspace to
a folder id. Ordering is `enforcement desc, scope level desc` (`db/pg_store/governance.rs:61–63`,
`:339–341`; D57).

**Daemon → dōjō.** `tasks/dojo_sync.rs`: `tick` (:172) loops `pg.signed_in_personas()`
(`pg_store/personas.rs:79`), token by `persona_token` (:235, private, wraps
`api/handlers/auth.rs:956 live_access_token`), plan, push (`push_allowed` :899). The plan's
`MappedRepo` carries `repo_key`, `tenant` (`{origin}/{slug}`, :32), `tenant_id` and `repo_id`
(`dojo_client/user_plane.rs:25–36`). Sync verdicts go to `sensei.sync_state` with enum
`sensei.sync_entity` (`ddl/enum/sensei/sync_entity.ddl`).

**A repository has one workspace per tenant**: `dojo.repositories_in_workspaces` is
`unique (repository_id, tenant_id)` (`ddl/table/dojo/repositories_in_workspaces.ddl:19`), and a
repository belongs to one tenant. So a repository determines its workspace; no resolver needs a
workspace argument.

**Tracker (daemon path, Jira only).** `sensei.trackers` (`workspace_id`, `kind` = `jira` only,
`level`), `sensei.sprints` (`external_id`, `name`, `state`, `start_at`, `end_at`),
`sensei.tracker_tasks` (`key`, `summary`, `issue_type`, `status`, `current_sprint_id`; no assignee,
description or acceptance criteria), `sensei.tracker_task_sprints`, `sensei.session_task_links`
(`session_id, task_id, source link_source, matched_key, mentions, linked_at`, pk
`(session_id, task_id, source)`, `ddl/table/sensei/session_task_links.ddl:5–13`). `link_source` =
`branch | prompt | commit` (`commit` reserved, never written; `ddl/enum/sensei/link_source.ddl`). Rust:
`TrackerAdapter` (`tracker/mod.rs:254–288`: `validate`, `fetch_snapshot`, `match_links`,
`unknown_keys`, `fetch_tasks`), `adapters()` (:290, Jira only), `LinkSource { Branch, Prompt }`
(:198). **Sync relinks by full replacement**: `DELETE FROM sensei.session_task_links WHERE task_id IN
(… tracker_id = $1)` (`db/pg_store/trackers.rs:590–596`), then `write_links` (fn :305, its INSERT :331). Any link written
by another path is deleted on the next sync. `GET /api/workspaces/{id}/tracker-tasks/{key}`
(`api/routes.rs:63`, `api/handlers/trackers.rs:294`) reads the mirror, then fetches a missing key
from the board (`discover_on_miss` :354).

**Sessions.** `activity.sessions.client_session_id` is the assistant's own session id (unique when
set; NULL for MCP-created sessions; `ddl/table/activity/sessions.ddl:57–60`, comment :88).

**MCP.** Tools are declared inline in `crates/mcp/src/lib.rs`: `daemon_request_for` (:99) shapes the
daemon request per tool (`get_rules` → `GET /api/knowledge/rules`, :232), `handle_list_tools` holds
the declarations (`get_rules` :745), `tool()` (:998) builds the schema, `EXPECTED_TOOLS` (:1390)
pins the catalogue. The binary forwards; logic lives in the daemon.

**Notice.** `packages/ui/Notice.svelte` and `packages/ui/notice.js` (`noticeFromReason`,
`toneForReasonKind`; tones `fault | refusal | action | normal`), exported from `packages/ui/index.js`.

## 2. Decisions this spec makes (each from the code above, none new to the product)

1. **`metric_gating` holds no `direction` or `unit`.** The brief listed them; `sensei.metrics`
   already owns both (`metrics.ddl:9–10`). The comparator is derived from `metrics.direction`:
   `higher_better → '>='`, `lower_better → '<='`. A threshold of 0 with `<=` displays as `= 0`.
   `neutral` is never eligible.
2. **The resolver takes no workspace.** The repository decides it (`repositories_in_workspaces.ddl:19`).
3. **One resolver, in SQL.** `dojo.targets_resolve` is the only implementation of the ladder
   (D48 with D80). The daemon caches its answers; it never re-ranks rungs in Rust.
4. **Narrower rungs may only tighten (D80, user, 2026-11-03).** A narrower rung may raise the
   bar (a stricter threshold) or the mode (`warn → block`, `off → warn|block`). It may never lower
   either unless the caller holds the role that sets the wider rung it loosens. Loosening means a
   laxer threshold, `block → warn`, or `→ off`. Example: T3's legacy `AUTH-420 ≤ 4` against a
   workspace `≤ 2` needs an admin. This matches D57: a repository can tighten a dōjō rule but cannot
   weaken it. Resolution therefore picks **the strictest of the applicable rungs**. The one exception
   is an authorised loosening (TG2 step 2b, `loosens = true`), which replaces the wider value it was
   authorised against.
5. **Dōjō routes live under `/v1/t/[origin]/[org]/targets/**`**, the path L3 owns. T3's done-gate
   curl (`/workspaces/$WS/targets`) becomes `/targets?workspace=$WS`.
6. **Every L3 database object starts with `target`** so it falls in L3's owned glob
   (`database/ddl/**/{sensei,dojo}/target*`): `target_set`, `targets_resolve`, and so on.
7. **The three enums live in the `sensei` schema** and deploy to both planes, the way
   `metric_direction` does: `sensei.target_level`, `sensei.target_mode`, `sensei.target_verdict`.

8. **The former open questions Q1–Q5 are accepted as written (D81, user, 2026-11-03):**
   - **Q1. Target 1 is absolute.** It counts failing hard rules on the change's tree, `= 0`.
     `rule_check_runs` has no commit sha, so "new at this change" cannot be computed.
   - **Q2. A hard rule** is a checker-backed rule whose enforcement, resolved exactly as `get_rules`
     resolves it (including the per-adoption override, `rule_pack_adoptions.ddl:16`), is
     `mandatory` or `required`. Today `resolve_local_checker_rules` (`pg_store/governance.rs:164–190`)
     returns only `(statement, checker_ref)` from the local `rule_pack_*` replica. It reads neither
     memories nor the dōjō packs that `get_rules` folds in through `resolve_adopted_pack_raws`
     (`knowledge.rs:1284`). So a checker-backed rule adopted only in the dōjō is never run by
     `run_checkers`. TG4 handles that case.
   - **Q3. The baseline is per (repository, metric, paths).** It is the code's own history, and one
     healthy repository cannot hide another.
   - **Q4. `subject_key`** is the tracker's sprint `external_id` (with `subject_label` = its name)
     or the raw `tracker_tasks.issue_type`, compared case-insensitively. There is no invented
     issue-type → kind mapping.
   - **Q5. `min_coverage`** is 0.5 for coverage and 1.0 for graph-derived targets (the
     repository's root folder `subtree_complete`). Each value is a data row in
     `metric_gating.jsonl`.

## 4. Work packages

Rules for every package (parallel-lanes §1): failing test first, watched red locally, committed
with its implementation when green. Zero errors. Gate on `cargo test -p senseid` (full),
`make test-fast` (includes `check-sql-in-layer`, `check-ddl-grants`, `check-ddl-comments`),
`cd dojo && bun run check && bun run test`, `make test-db`. Production SQL only in
`crates/senseid/src/db/pg_store/` (new file `pg_store/targets.rs`; one `mod` line in
`pg_store/mod.rs`). Reconcile the shared Supabase only while holding
`~/.sensei/locks/supabase`. Never fabricate on a failure path: a failed read is an error the caller
sees, never `[]`, `0` or "met".

Order (forward-only; no package depends on a later one): **TG1 → TG2 → TG3 → TG4 → TG5 → TG6 →
TG7.** TG5's `get_ticket` half depends only on TG1-free tracker work and may run beside TG2–TG4.

---

### TG1 · Gating eligibility (both planes)

Which catalogue metrics may be a target, how each is read, and when its reading is too thin. Its own
table and import, so L2's `metrics.jsonl` lines are never edited.

**Schema** (`sensei` schema; deploys `--scope default` and, through `design.yaml`, `--scope dojo`):

- `database/ddl/table/sensei/metric_gating.ddl`:
  ```sql
  create table if not exists metric_gating (
    metric_id     uuid        primary key references sensei.metrics(id) on delete cascade
  , eligible      boolean     not null
  , delta_scoped  boolean     not null            -- true: reads what a change introduced; false: an absolute reading, ratchetable
  , min_coverage  numeric     not null check (min_coverage > 0 and min_coverage <= 1)
  , note          text        not null            -- why eligible or not, in one sentence
  , modified_at   timestamptz not null default now()
  );
  ```
  Comments on the table and every column. Grant `select` to `authenticated, service_role`; RLS on,
  one `for select to authenticated using (true)` policy, as `metrics.ddl:96–105`.
- `database/ddl/table/staging/metric_gating.ddl` (flat: `metric_key text`, the four values,
  `modified_at`) and `database/ddl/procedure/staging/import_metric_gating.ddl`. The procedure joins
  `sensei.metrics` on `key`, upserts with the `modified_at` guard of `import_metrics`, and **raises**
  when a staging row names an unknown key or marks a `neutral` metric eligible. Fail loudly: a
  gating row for a metric L2 has not landed yet stays out of the file until it lands.
- `database/import/staging/metric_gating.jsonl`, first import:

  | metric_key | eligible | delta_scoped | min_coverage | note |
  |---|---|---|---|---|
  | `coverage` | true | false | 0.5 | target 2; ratchetable; needs `paths` for "guarded" |
  | `ftr` | false | false | 1.0 | an insight, never a target (T3 "Insights") |

  Rows to add **when their metric exists** (each its own commit, after L2 appends the catalogue row):
  - `hard_rule_violations` (target 1, eligible, delta_scoped false per Q1, 1.0). **No lane owns
    this catalogue row**; L3 lists it in its handoff and L2 appends it.
  - L2's zone-of-pain module count (target 5, eligible, false, 1.0).
  - L2's cycle count (target 4, eligible, true, 1.0) and Up-edge count (target 3, eligible, true,
    1.0), each from `layering.rs` once #156 persists them.
  - Complexity outliers (target 6) stay **not eligible** until a per-function read exists.
  - Target 7: no row. There is no code anti-pattern producer (§1).

  **The scheduler and target-only metrics.** A `task_name` the scheduler does not know is skipped:
  `metrics_scheduler.rs:69–71` and `planner.rs:410–419` log a warning and return `Ok(0)`. So a
  target-only catalogue row (`hard_rule_violations`, read by TG4's reader, not by a metric task)
  costs one no-op task and a warning per pass. **Recommend** giving such rows a `task_name` the
  scheduler excludes on purpose (add it to the scheduler's exclusion beside the health barrier, with
  a test `a_target_only_metric_schedules_no_task`). If that edit cannot land with TG1, accept the
  warning and say so in the handoff.
- `database/design.yaml`, `scopes.dojo.includes`: add `sensei.metric_gating`,
  `staging.metric_gating`, `staging.import_metric_gating` (shared hotspot: list it in the handoff;
  L0 merges).

**Tests first:**
- SQL `database/tests/dojo/target_metric_gating.sql`: `an_unknown_metric_key_fails_the_import`,
  `a_neutral_metric_cannot_be_eligible`, `reimporting_unchanged_rows_changes_nothing`,
  `an_insight_is_present_and_ineligible` (`ftr` row, `eligible = false`).
- Rust `pg_store/tests/metric_rows_tests.rs` (or a new `targets_tests.rs`):
  `gating_reads_eligible_metrics_with_their_comparator` (`coverage → '>='`, from `metrics.direction`),
  `an_absent_gating_row_is_not_eligible`.

**API:** `PgStore::metric_gating(&self) -> Result<Vec<GatingRow>, String>` in `pg_store/targets.rs`;
`GatingRow { metric_key, eligible, delta_scoped, min_coverage, comparator, unit }`, comparator
computed in SQL from `metrics.direction`.

**Accept (live, after L0 reconciles):** `select m.key, g.eligible, g.delta_scoped from
sensei.metric_gating g join sensei.metrics m on m.id = g.metric_id` returns the two rows on the
daemon DB **and** on the local dōjō (`:54322`). `dbd import` run twice reports the same row count.

**Do not:** add columns to `sensei.metrics` or edit any line of `metrics.jsonl`; copy `direction` or
`unit`; mark a session/behaviour metric eligible (FTR, rework, throughput, AI share are insights).

---

### TG2 · Target store (dōjō)

**Schema** (`--scope dojo` for the table; the enums are `sensei` schema and listed in
`design.yaml` like `metric_direction`):

- `database/ddl/enum/sensei/target_level.ddl`:
  `create type target_level as enum ('dojo','workspace','repository','sprint','task_type','task');`
  (D48; the order IS the specificity, least to most).
- `database/ddl/enum/sensei/target_mode.ddl`: `('block','warn','off')`.
- `database/ddl/table/dojo/targets.ddl`:
  ```sql
  create table if not exists dojo.targets (
    id             uuid               primary key default gen_random_uuid()
  , tenant_id      uuid               not null references dojo.tenants(id) on delete cascade
  , level          sensei.target_level not null
  , workspace_id   uuid               references dojo.workspaces(id) on delete cascade
  , repository_id  uuid               references dojo.repositories(id) on delete cascade
  , subject_key    text               -- sprint external_id | issue type | ticket key (Q4)
  , subject_label  text               -- display only (a sprint's name)
  , metric_id      uuid               not null references sensei.metrics(id)
  , paths          text[]             -- guarded globs, e.g. {auth/**}; null = the whole repository
  , threshold      numeric            not null
  , mode           sensei.target_mode not null
  , ratchet        boolean            not null default false
  , set_by         uuid               not null references dojo.principals(id)
  , set_at         timestamptz        not null default now()
  , note           text
  , loosens        boolean            not null default false  -- D80: set only when the setter held the wider rung's role
  , constraint targets_level_shape check (
        (level = 'dojo'       and workspace_id is null     and repository_id is null     and subject_key is null)
     or (level = 'workspace'  and workspace_id is not null and repository_id is null     and subject_key is null)
     or (level = 'repository' and workspace_id is null     and repository_id is not null and subject_key is null)
     or (level in ('sprint','task_type','task')
                              and workspace_id is not null and repository_id is null     and subject_key is not null))
  , constraint targets_one_per_rung unique nulls not distinct
        (tenant_id, level, workspace_id, repository_id, subject_key, metric_id, paths)
  );
  ```
  Index `(tenant_id, metric_id)`. A comment on the table and every column, including that the
  comparator is not stored (§2.1).
- `database/policies/dojo/targets.sql`: RLS on; `for select to authenticated using
  (dojo.is_member(tenant_id))`; no insert/update/delete policy (writes go through the definer
  functions). Revoke table writes from `authenticated`.
- `database/ddl/function/dojo/target_set.ddl`:
  `dojo.target_set(p_tenant_id uuid, p_level sensei.target_level, p_workspace_id uuid,
  p_repository_id uuid, p_subject_key text, p_subject_label text, p_metric_key text,
  p_paths text[], p_threshold numeric, p_mode sensei.target_mode, p_ratchet boolean, p_note text)
  returns setof dojo.targets`, plpgsql, volatile, SECURITY DEFINER, `set search_path = dojo, pg_temp`.
  In order:
  1. The workspace or repository named must be in `p_tenant_id` → else `no_data_found`.
  2. **Role (D54):** `dojo` / `workspace` → `tenant_role_at_least(p_tenant_id,'admin')` (workspace:
     `administers_workspace(p_workspace_id)`); `repository` →
     `repo_role_at_least(p_repository_id,'maintainer')`; `sprint` / `task_type` / `task` → exists a
     repository in `repositories_in_workspaces` for `p_workspace_id` with
     `repo_role_at_least(r,'maintainer')`. A lead who is not a maintainer is refused (D58). Refusal:
     `insufficient_privilege`.
  2b. **Tighten only (D80, §2.4).** Compute the value the row would override: `targets_resolve`
     for the same metric and paths, with this rung removed. A **looser** new row is one with a laxer
     threshold under the comparator, or a lower mode (`block > warn > off`). It is allowed only if the
     caller also passes `target_may_set` for the rung that supplied the overridden value. The row
     records this in a column `loosens boolean not null default false` (true only when allowed this
     way). Otherwise refuse with `insufficient_privilege` and the message prefix `loosens_wider:`,
     naming the rung and who may loosen it.
  3. **Eligibility:** a `metric_gating` row with `eligible` → else `invalid_parameter_value`
     (`'<key> is not a target metric: <note>'`). `p_ratchet` with `delta_scoped` → refused the same way.
  4. **Entitlement (D52, D75), repository level only:** the caller's `all_my_repositories` row for
     `p_repository_id`, mapped by `entitlement_refusal`:
     - `unclaimed`, `not_subscribed`, `subscription_expired` → `insufficient_privilege` with the
       message prefix `needs_pro:` and the text "Targets on private repositories need Pro." The Worker
       maps the prefix to `reason_code: 'needs_pro'`.
     - `forge_visibility_unknown`, `forge_visibility_stale` → `object_not_in_prerequisite_state`
       (409) with the prefix `visibility_unknown:` and the text "The forge's visibility for this
       repository has not been read yet. Resync, then try again." **Never "needs Pro"**: nobody knows
       yet whether the repository is private.
     - Any other code passes through raw as its own reason. Dōjō, workspace and ticket rungs are not refused here; the
     resolver marks each unentitled repository (TG3).
  5. Upsert on `targets_one_per_rung`; `set_by = current_principal_id()`, `set_at = now()`.
- `database/ddl/function/dojo/target_clear.ddl`: `dojo.target_clear(p_target_id uuid) returns
  boolean`; the same role rule for the row's level; `no_data_found` for an unknown id or one the
  caller cannot see.
- `database/ddl/function/dojo/targets_for_workspace.ddl`: `dojo.targets_for_workspace(p_workspace_id
  uuid)` → every row of the workspace's tenant at `dojo`, the workspace's rows, its repositories'
  rows, plus per row `may_edit boolean` (the step-2 rule as a predicate) and, for repository rows,
  `entitled`, `reason_code`. Gate: `can_read_workspace(p_workspace_id)` or raise
  `insufficient_privilege`. This is the Targets tab's read (TG7). Shared predicate: put the role rule
  in **one** function, `dojo.target_may_set(p_tenant_id, p_level, p_workspace_id, p_repository_id)`,
  called by `target_set`, `target_clear` and here. The same mapping for `entitled` / `reason_code`
  as step 4 lives in one helper, `dojo.target_entitlement(p_repository_id)`, which returns
  `(entitled, reason_code)` and is used by `target_set`, here, and TG3.
- Every function: `revoke all … from public; grant execute … to authenticated;` and a comment. Add
  all five names (`target_entitlement` too) to the `listed` array of
  `definer_functions_gate_the_caller.sql` (:62–73), bob's write calls (:191–217: `target_set` at alice's workspace, `target_clear` of alice's target), bob's
  predicate call (`target_may_set`), bob's read (`targets_for_workspace`), and alice's positive
  controls. Seed one `dojo.targets` row for alice in the fixture.

**Tests first:**
- SQL `database/tests/dojo/target_store_access.sql`, one block per row of T3's role outline:
  `admin_sets_a_workspace_target`, `a_maintainer_in_the_workspace_cannot_set_a_workspace_target`,
  `a_maintainer_sets_a_repository_target_on_their_repository`,
  `a_maintainer_on_another_repository_is_refused`,
  `a_maintainer_on_any_workspace_repository_sets_a_sprint_target`,
  `a_lead_who_is_not_a_maintainer_is_refused_a_task_type_target`, `a_viewer_sets_nothing`;
  plus `an_ineligible_metric_is_refused_with_its_note`, `ratchet_on_a_delta_metric_is_refused`,
  `an_org_private_repository_without_a_subscription_is_refused_needs_pro`,
  `a_repository_with_unknown_visibility_is_409_visibility_unknown_not_needs_pro`,
  `narrower_rung_cannot_loosen_a_wider_target` (D80),
  `wider_role_may_loosen` (D80 positive control: an admin sets `AUTH-420 ≤ 4` under a workspace
  `≤ 2`, and the row has `loosens = true`), `a_narrower_rung_may_tighten_freely`,
  `an_org_public_repository_target_is_free`, `a_personal_private_repository_target_is_free`
  (red until WP8 lands; see Do not), `setting_the_same_rung_twice_updates_one_row`,
  `a_workspace_from_another_tenant_is_not_found`, `a_member_reads_targets_a_non_member_reads_none`.
- The definer gate test, extended as above, still green.
- vitest `dojo/src/lib/server/targets-data.spec.ts`: `set_maps_needs_pro_to_403_with_reason_code`,
  `set_maps_an_ineligible_metric_to_422`, `a_failed_read_is_500_never_empty`.

**Route** (`dojo/src/routes/v1/t/[origin]/[org]/targets/`, data layer `dojo/src/lib/server/targets-data.ts`):
- `GET  /targets?workspace=<uuid>` → `targets_for_workspace` (`readAllPages`, ordered by `level,
  metric_key, id`). Caller first, then uuid check (400), then tenant (404), then read as the caller
  (403/500 kept apart), exactly as `workspaces/[id]/metrics/+server.ts:24–43`.
- `PUT  /targets` body `TargetInput` (TG7) → `target_set`. 422 on a bad body before the database.
  The refusal prefixes `needs_pro:`, `visibility_unknown:` and `loosens_wider:` become a
  `reason_code` beside the status.
- `isUuid`: reuse the shared helper if it exists at build time. If not, create
  `dojo/src/lib/server/is-uuid.ts` (one regex, one test) and list it for L1, who owns the WP4
  replacement. Do not copy `UUID_RE` again.
- `DELETE /targets/[id]` → `target_clear`.
- Route spec `targets-routes.spec.ts`: `get_without_a_session_is_401_before_the_tenant_is_resolved`,
  `a_malformed_workspace_id_is_400`, `an_unknown_org_is_404`, `put_as_a_viewer_is_403`,
  `put_on_an_unentitled_repository_answers_needs_pro`.

**Accept (live, local dōjō):** as an admin of `sensei-hq`, `PUT` a workspace coverage target
(`≥ 0.8`, `block`, `paths {crates/senseid/src/api/**}`); `GET /targets?workspace=…` returns it with
`may_edit = true`; as a contributor the same `PUT` is 403 and the row is unchanged.

**Do not:** store a comparator; let a team own a row (D13: there is no team column); hide the
option on an unentitled repository (the refusal names Pro, D52); merge before WP8 — without it every
personal private repository answers `needs_pro`, which D52 forbids. If WP8 has not merged, land
TG2 with `a_personal_private_repository_target_is_free` marked as the one known gap in the handoff.

---

### TG3 · Resolution (dōjō) and the daemon's replica

**Schema (dōjō):**
- `database/ddl/function/dojo/targets_resolve.ddl`:
  `dojo.targets_resolve(p_repository_id uuid, p_sprint_key text, p_task_type text, p_task_key text)
  returns table (target_id uuid, level sensei.target_level, metric_key text, paths text[],
  threshold numeric, comparator text, mode sensei.target_mode, ratchet boolean, delta_scoped boolean,
  min_coverage numeric, unit text, source_label text, set_at timestamptz, note text,
  entitled boolean, reason_code text)`, SQL, stable, SECURITY DEFINER.
  - Gate: `can_read_repository(p_repository_id, r.tenant_id)` else no rows (it is a read; the route
    answers 403 first through `can_read_repository`, so the empty set is never a masked refusal).
  - Candidates: the repository's tenant at `dojo`; its workspace (from
    `repositories_in_workspaces`) at `workspace`; the repository at `repository`; the workspace at
    `sprint` with `subject_key = p_sprint_key`, at `task_type` with
    `lower(subject_key) = lower(p_task_type)`, at `task` with `subject_key = p_task_key`.
  - **The strictest applicable rung wins (D80).** Among the candidate rows for one
    `(metric_id, coalesce(paths,'{}'))`, take the strictest threshold under the comparator and the
    highest mode (`block > warn > off`). On a tie, the most specific rung is reported as the source.
    The exception is a row with `loosens = true`: it replaces every wider row, because its setter
    held the wider role (TG2 step 2b). So a broader rung tightened later still wins over an older
    narrower row that was not authorised to loosen. `off` can win only through `loosens = true`.
  - `comparator` from `metrics.direction` (§2.1); `delta_scoped`, `min_coverage` from
    `metric_gating`; a target whose metric has lost eligibility is dropped.
  - `entitled` / `reason_code` from `target_entitlement` (TG2: `needs_pro` for the three billing
    codes, `visibility_unknown` for the two capture codes). `source_label` = the rung and its subject, e.g.
    `Repository github.com/sensei-hq/sensei`, `Task type migration`.
- `database/ddl/function/dojo/targets_resolve_batch.ddl`: `dojo.targets_resolve_batch(p_contexts
  jsonb)` → the same columns plus `context_index int`, a lateral call of `targets_resolve` per array
  element `{repository_id, sprint_key, task_type, task_key}`. One database call per request whatever
  the batch size (the WP9 lesson: Workers cap subrequests per request). Refuse more than 500
  contexts with `invalid_parameter_value`.
- Both into the definer gate test (bob's reads return nothing; alice's control returns her target).

**Tests first (SQL `database/tests/dojo/targets_resolve.sql`):**
`all_six_rungs_are_candidates`, `a_rung_with_no_value_inherits_the_nearest_broader`,
`a_task_type_override_applies_only_to_matching_tasks`,
`the_strictest_applicable_rung_wins`, `a_narrower_tightening_wins`,
`an_authorised_loosening_replaces_the_wider_value`,
`a_broader_rung_tightened_later_overrides_an_older_narrower_row`,
`an_authorised_off_at_a_narrower_rung_turns_a_block_off`, `paths_resolve_independently`
(`auth/**` and `payments/**` are two targets), `a_team_link_adds_no_rung`,
`another_dojo_s_targets_never_apply` (D53: a second tenant's dōjō-rung row is ignored),
`an_unentitled_repository_is_marked_needs_pro`, `unknown_visibility_is_marked_visibility_unknown`,
`a_reader_without_access_gets_nothing`,
`the_batch_answers_the_same_rows_as_one_by_one`.

**Route:** `POST /v1/t/[origin]/[org]/targets/resolve` body `{ contexts: TargetContext[] }` →
`targets_resolve_batch`. `resolveTenantCaller(…, ACCESS.member)`; a context naming a repository
outside the tenant is 422. (POST because the batch does not fit a query string.) Route spec:
`resolve_is_one_database_call_for_a_batch`, `a_foreign_repository_in_the_batch_is_422`.

**Daemon replica (`crates/senseid/src/targets/sync.rs`, SQL in `pg_store/targets.rs`):**
- Schema (`--scope default`) `database/ddl/table/sensei/target_resolutions.ddl`:
  `repository_id uuid not null references sensei.repositories(id) on delete cascade, sprint_key text,
  task_type text, task_key text,` the resolver's columns, `fetched_at timestamptz not null`, unique
  nulls not distinct on `(repository_id, sprint_key, task_type, task_key, metric_key, paths)`. A
  context is replaced whole in one transaction (delete its rows, insert the answer), so a target
  removed in the dōjō disappears locally.
- New scheduled task `targets_sync` (one row appended to `database/import/staging/schedules.jsonl`,
  `interval_secs 3600`; one line in `tasks/mod.rs` — both shared hotspots, list them). Per
  signed-in persona (`pg.signed_in_personas()`), token from `live_access_token`
  (`api/handlers/auth.rs:956`, `pub(crate)`; do not copy `dojo_sync::persona_token`), plan from
  `user_plane::sync_plan` (read-only use of L1's client). For each tenant in `plan.allowed`, build
  contexts: per repository the bare context, plus one per ticket of that workspace's active and
  future sprints (`sprints.state`, `tracker_task_sprints`, `tracker_tasks.issue_type`, read-only).
  One `POST …/targets/resolve` per tenant, batches of ≤ 500.
- Record the outcome under two new `sync_entity` values, `resolved_targets` (key = `repo_key`) and
  `target_result` (TG4) — `ddl/enum/sensei/sync_entity.ddl` is shared; list the change. A failed
  fetch writes `error` and **leaves the previous rows and their `fetched_at` in place**; nothing is
  deleted on failure, and nothing reads them as fresh (every consumer shows `fetched_at`).
- On demand: `targets::resolve_now(pg, repo, context)` for a context not in the replica (a ticket
  fetched before the next tick). A failure here is an error to the caller, never "no targets".

**Tests first (Rust, scripted plane like `dojo_sync.rs` tests):**
`a_tick_replaces_each_context_whole`, `a_target_removed_in_the_dojo_is_removed_locally`,
`a_failed_fetch_keeps_the_previous_rows_and_records_error`,
`a_repository_not_in_the_plan_is_not_asked`, `contexts_cover_every_ticket_of_the_active_sprint`,
`resolve_now_for_an_unknown_ticket_calls_the_dojo_once`,
`resolve_now_failure_is_an_error_not_an_empty_set`.

**Accept (live):** after a tick, `select r.repo_key, t.metric_key, t.level, t.mode, t.fetched_at
from sensei.target_resolutions t join sensei.repositories r on r.id = t.repository_id` shows the
TG2 coverage target for `github.com/sensei-hq/sensei` with `level = workspace`; `sync_state
(resolved_targets, 'github.com/sensei-hq/sensei')` is `synced`. Add a repository-rung override in
the dōjō; the next tick shows `level = repository` and the new threshold.

**Do not:** rank rungs in Rust; resolve from another dōjō than the repository's (D53); treat an
expired replica as current; extend `dojo_sync.rs` (L1's file).

---

### TG4 · Evaluation (daemon) and the result push

**What is evaluated.** Three subjects, one evaluator:
- **change** — the working tree against its base (MCP `get_missed_targets`, TG5);
- **ticket** — each branch of the sessions linked to the ticket (`session_task_links` →
  `activity.sessions` branches), head = the branch tip, base = its merge-base with the development
  branch;
- **sprint** — the development branch at the sprint's `start_at` (base) and now / `complete_at`
  (head), per repository of the workspace, daily and once at sprint close (T3: sprint targets read
  the development branch, D18).

**Base branch.** Use an explicit `base` argument if given. Otherwise use the repository's tracked
branch labelled `development` (`repository_graph_branches.label`, §1) when it has one, and its
default branch when it does not. The label is not on the plan wire yet (`GraphBranch { branch,
is_default }`, `user_plane.rs:59–62`, `client-data.ts:146–149`). **Adding `label` to the plan wire
is an L1 handoff item**: L1 owns `dojo_client/**` and `v1/you/**`. Until it lands, the daemon has
only `is_default`, so the base is the default branch. Ratcheted targets need no base. Do not guess a
branch name such as `develop`.

**Readers** (`crates/senseid/src/targets/readers/`, one file per metric key, a
`trait TargetReader { fn metric_key(&self) -> &'static str; async fn read(&self, ctx: &ReadCtx) ->
Result<Reading, ReadError>; }`, a registry like `tracker::adapters()`):
- `Reading { value: Option<f64>, coverage: f64, findings: Vec<Finding> }`, `Finding { identity:
  String, kind: FileRef|FunctionRef|EdgeRef|RuleRef, location: String, detail: String }`.
- `coverage`: lcov at head, records filtered by the target's `paths`; `value = Σ LH / Σ LF` over the
  matched records; `coverage` = matched files with a record ÷ tracked files under `paths`
  (`git ls-files`); findings = the matched files ordered by uncovered lines, top 10. Refactor
  `coverage.rs:109 parse_lcov` into `parse_lcov_by_file` returning per-record sums, with
  `parse_lcov` summing it (one parser; `coverage.rs` has no lane owner — list the edit).
- `hard_rule_violations` (when its catalogue row exists), per D81 Q2:
  - **Hard-rule set.** Resolve the folder's ruleset exactly as `get_rules` does:
    `resolve_repo_ruleset` (`knowledge.rs:1354`), which includes memories, local packs, dōjō packs
    and the per-adoption enforcement override. Keep the checker-backed rules of `mandatory|required`
    enforcement.
  - **Runs.** Run `checker::run_checkers` for the folder. Extend `resolve_local_checker_rules` to
    return the enforcement, so the two sets join on the statement.
  - **Reading.** The reading is the count of failing hard rules. Findings are each rule's statement,
    command and `output_tail`.
  - **Known limit.** A checker-backed hard rule adopted only in the dōjō is never run by
    `run_checkers`. Its result is `not_measured` with reason `checker_not_run`, and it is listed
    by name, never counted as passing. If every hard rule is in that state, the whole target is
    `not_measured`.
- Graph readers (zone of pain; cycles and Up-edges once #156 persists them from `layering.rs`):
  `coverage` = 1.0 when the repository's root folder has `sensei.folder_completeness.subtree_complete`
  true, else 0. The not_measured reason is then `index_incomplete`. This is the view's first
  production reader (today only `pg_store/tests/graph_tests.rs:865, 953, 1001` read it); its SQL
  goes in `pg_store/targets.rs`.
- **Delta-scoped** readers read head and base and report `findings(head) − findings(base)` by
  `identity` (a removed violation does not pay for an added one); `value` = that count.

**Verdict (pure, `targets/verdict.rs`):**
1. `mode = off` → not evaluated, no row.
2. `entitled = false` → no row (the surfaces show the Pro line from the resolution, not a result).
3. The read failed, `value` is `None`, or `coverage < min_coverage` → `not_measured` with a reason
   code (`read_failed`, `no_reading`, `thin_coverage`, `index_incomplete`, `no_development_branch`).
4. Else compare `value` against the **effective threshold**: for a ratcheted target with a
   baseline, `min(threshold, baseline)` for `>=` and `max(threshold, baseline)` for `<=` (it never
   asks for more than the target, and never lets the reading fall below the code's own history);
   otherwise `threshold`. → `met` or `missed`.

**Ratchet (`targets/ratchet.rs`, pure):** the baseline starts at the median of the first ≤ 3
sprint-close readings; at each sprint close it advances to the trailing 3-sprint median **only when
that median beats it** (V3); a single spike never advances it. Stored in
`database/ddl/table/sensei/target_baselines.ddl` (`repository_id, metric_key, paths, baseline,
sprints_seen, advanced_at`, unique nulls not distinct on the first three).

**What to fix.** Each `missed` result carries `fix_items` (the findings, as is) and `fix_text`
(`title`, `detail`). The deterministic `FallbackCopy` is built from the findings ("target missed
(blocking): coverage 0.77 on auth/** — add tests for login.ts, session.ts"; `warn` says "target
missed"). Model copy comes through `narration_cache::copy_or_warm` with a new
`InsightKind::TargetMissed` — `analysis/narration_cache.rs` is L2's file: L3 writes the variant,
its `as_str` (`"target_missed"`) and task line in its handoff and uses the fallback until it lands.

**Schema (daemon, `--scope default`)** `database/ddl/enum/sensei/target_verdict.ddl` =
`('met','missed','not_measured')`; `database/ddl/table/sensei/target_results.ddl`:
`id uuid pk, repository_id uuid not null fk, target_id uuid not null` (the dōjō id), `subject_kind
text check in ('change','ticket','sprint'), subject_key text` (ticket key / sprint external id /
null for a change), `head_sha text not null, base_sha text, metric_key, paths, level, mode,
threshold, effective_threshold, comparator, reading numeric, baseline numeric, coverage numeric,
verdict sensei.target_verdict not null, reason_code text, fix_items jsonb not null default '[]',
fix_title text, fix_detail text, persona_id uuid` (whose change), `evaluated_at, shared_at`. Unique
nulls not distinct on `(target_id, repository_id, subject_kind, subject_key, head_sha)`; a
re-evaluation of the same head replaces its row.

**Push.** In `targets_sync` (TG3), after resolving: rows with `shared_at is null` for repositories
in `plan.allowed`, `subject_kind in ('ticket','sprint')` (a working-tree change is local), batches
of ≤ 200, `POST /v1/t/[origin]/[org]/targets/results`, then mark exactly the accepted rows shared
(the `push_allowed` contract, `dojo_sync.rs:899`).

**Schema (dōjō):** `database/ddl/table/dojo/target_results.ddl`: `id, tenant_id, repository_id,
target_id references dojo.targets(id) on delete cascade, principal_id references dojo.principals,
subject_kind, subject_key, head_sha, verdict sensei.target_verdict, mode, reading, threshold,
effective_threshold, reason_code, fix_items jsonb, fix_title, fix_detail, evaluated_at, pushed_at`,
unique on `(target_id, repository_id, subject_kind, subject_key, head_sha)` nulls not distinct.
Policy `database/policies/dojo/target_results.sql`: select using
`dojo.can_read_repository_metric(repository_id, 'user', principal_id, tenant_id)` — a result is one
person's reading, so the metric rule applies unchanged (own, maintainer+, team lead, admin).
`database/ddl/function/dojo/target_results_file.ddl`: `dojo.target_results_file(p_rows jsonb)
returns integer`, the `file_metric_rows` contract (not `file_graph_slices`'s, which counts older slices rather than refusing): principal from the session, every row's
repository must be the caller's with `sync_enabled`, every `target_id` must belong to that
repository's tenant, else the **whole batch** is refused (`insufficient_privilege` /
`invalid_parameter_value`); upsert on the unique key; `principal_id = current_principal_id()`.
`database/ddl/function/dojo/target_results_for_workspace.ddl`: `(p_workspace_id uuid, p_since date)`
→ detail rows the caller may read (same predicate) **plus**, for a workspace viewer, aggregates per
(repository, metric, day): `met`, `missed`, `not_measured` counts and `contributors`, **withheld when
`contributors < 2`** (F4, D68), never a principal. Shape and branches as
`workspace_metric_series.ddl:51–116`. That function has **no** ≥ 2 withholding on develop yet: its
F4 `having count(*) >= 2` is pending in L1's lane (375f63175). So this function implements
`having count(distinct principal_id) >= 2` itself. Once F4 lands, check that the two state the floor
the same way, and leave a note in the handoff (not in code) to fold them into one predicate if L1
extracts one. Both functions into the definer gate test.

**Tests first:**
- Rust `targets/verdict.rs`: `off_is_not_evaluated`, `thin_coverage_is_not_measured_never_met`,
  `a_failed_read_is_not_measured_with_read_failed`, `delta_counts_new_findings_by_identity`,
  `a_removed_violation_does_not_offset_an_added_one`, `the_ratchet_never_asks_more_than_the_target`,
  `the_ratchet_never_accepts_less_than_the_baseline`, `an_unentitled_target_produces_no_result`.
  `targets/ratchet.rs`: `the_baseline_advances_on_a_sustained_3_sprint_median`,
  `a_single_spike_does_not_advance_it`.
  Reader: `coverage_reads_only_the_guarded_paths`, `coverage_with_no_lcov_is_no_reading`,
  `parse_lcov_by_file_sums_to_parse_lcov` (conservation: Σ per-file = the old total on a real
  report from this repo).
  Push: `only_allowed_repositories_are_pushed`, `a_rejected_batch_marks_nothing_shared`,
  `a_change_result_is_never_pushed`.
- SQL `database/tests/dojo/target_results_access.sql`: `other_tenant_target_refuses_the_batch`,
  `an_unsynced_repository_refuses_the_batch`, `refiling_the_same_head_changes_one_row`,
  `a_contributor_reads_their_own_results_not_a_peers`, `a_lead_reads_teammates_results`,
  `a_viewer_reads_aggregates_only`, `a_viewer_aggregate_over_one_contributor_is_withheld`.
- Mutation probe (memory: probe the mutation): flip the comparator in `verdict.rs` and the
  `coverage < min_coverage` test; each must turn a named test red.

**Route:** `POST /targets/results` (daemon push, `resolveTenantCaller(…, ACCESS.member)`, one
`rpc` per request, answers `{accepted, rejected: [{index, reason}]}`); `GET
/targets/results?workspace=<uuid>&days=N` → `target_results_for_workspace`.

**Accept (live):** on `github.com/sensei-hq/sensei` with the TG2 coverage target: a ticket's branch
whose `crates/senseid/src/api/**` coverage reads below 0.8 yields one `missed` row locally with
`fix_items` naming files; after a tick the same row is in `dojo.target_results` and `shared_at` is
set; deleting the lcov report and re-evaluating yields `not_measured / no_reading`, never `met`.

**Do not:** compute a reading on the dōjō; record `met` for a reading not made; push working-tree
results; send a per-person result to a viewer; write a model call into the evaluation path (copy is
warmed off the wire).

---

**TG4 addendum (L0, 2026-11-03, from L2's #156 review): how architecture findings are diffed.**
- **Target 3, layer violations:** judged by the **change in the Up-edge count** between base and change. Up-edge
  identity is not stable: adding `c→b` to `a→b, b→c, c→a` changes which edge is "Up". So a delta by
  identity would blame an edge that was already in the base.
- **Target 4, cycles:** judged by **cycle identity = the sorted set of its member modules**, which is stable
  under unrelated edits. "Largest cycle" is a property of `dependency_cycles`, never a summed count.
- Both identities are still stored, because the screen shows them. Only the verdict uses the rule above.
- **The stored contract (#156, scope `repo`, one row per key per repository, never summed across
  repositories):**
  - `dependency_cycles`: value = the number of cycles; props `{cycles: [[sorted members]…], largest, cross_repository_deps}`.
  - `layer_up_edges`: value = the Up-edge count; props `{up_edges: ["s→t"…], cross_repository_deps}`.
  - `zone_of_pain_modules`: value = the count; props `{modules: […]}`.
  - `hidden_coupling_pairs`, `complexity_outlier_modules`: value only, props `{}`.

  An all-zero reading still stores all five rows (a clean repository is "met", not "not measured"). A reading
  on an incomplete index stores no rows.

### TG5 · MCP: `get_ticket`, `get_targets`, `get_missed_targets`

**Tracker-agnostic read (L7 implements; L3 consumes).** In `tracker/mod.rs` (L7):
```rust
pub struct TicketDetail {
    pub key: String, pub title: String, pub description: Option<String>,
    pub acceptance_criteria: Vec<String>, pub status: Option<String>, pub issue_type: Option<String>,
    pub sprint: Option<String>, pub assignee: Option<String>, pub tracker: TrackerKind, pub url: String,
}
#[async_trait::async_trait]
pub trait TicketReader: Send + Sync {
    /// Live read with the person's own access (D71). Ok(None) = no such ticket.
    async fn read_ticket(&self, key: &str) -> Result<Option<TicketDetail>, TrackerError>;
}
```
Jira: a reader over the existing adapter, transport and board credential. GitHub Issues: a reader
using the person's own forge sign-in, read-only (D71). The token is the Keychain
`provider_token.<persona>` (`dojo_client/session.rs:38`). Its scopes are `read:org repo`
(`api/handlers/auth.rs:724`); `repo` covers reading issues. It expires in about 8 hours and the
`forge_token` task renews it (`dojo_client/forge_token.rs`). An expired or missing token is a 502
`forge_token_unavailable`, never an empty ticket. `TrackerKind` gains `GithubIssues` only with its
adapter (`tracker_kind.ddl` comment). VERIFY (not answerable from the repo): where a given Jira site
keeps acceptance criteria. Make the field id configurable per tracker
(`trackers.params.acceptance_criteria_field`). When it is unset or the field is absent,
`acceptance_criteria` is `[]` and `acceptance_criteria_source` says why (`"not_configured"` |
`"field_absent"`). It is never an invented list. Choice of reader: a key matching
a workspace tracker's key prefix → that tracker; `#<n>` or `<owner>/<repo>#<n>` → GitHub Issues of
the cwd repository; else 404 `no_tracker_for_key`.

**Start of work (L7 owns the files; L3 writes the tests in its handoff if L7 is not there yet):**
- `link_source.ddl`: add `'fetch'` (the assistant fetched the ticket through Sensei, D63).
- `session_task_links.ddl`: add `started_at timestamptz` (set only on a `fetch` link).
- `db/pg_store/trackers.rs:590–596`: the relink delete gains `AND source <> 'fetch'`. Test:
  `a_sync_relink_keeps_fetch_links`.
- `LinkSource::Fetch` (`tracker/mod.rs:198`) with `as_str() = "fetch"`; the enum-labels test stays
  green.

**Daemon routes** (handlers in `crates/senseid/src/targets/http.rs`; one hunk in `api/routes.rs`):
- `POST /api/tickets/{key}/fetch` body `{ session_id?, folder }` → live `read_ticket`; on `None`
  → 404 and **no link**. On a hit: find the mirror row (`tracker_task_detail`, else the existing
  `discover_on_miss` path), map `session_id` → `activity.sessions.id` by `client_session_id`, and
  `insert … source 'fetch', started_at now(), mentions 1 on conflict do nothing` (a second fetch
  records nothing new). Session not found → the ticket is still returned with `start_of_work:
  { recorded: false, reason: "session_not_found" }`. Response also carries the `get_targets`
  payload, so one call starts the work (M14).
- `GET /api/targets?folder=<cwd>[&key=]` → the repository from the folder (`resolve_folder`,
  `knowledge.rs:1314`), the context from `key` (sprint, issue type from the mirror), the replica
  (`resolve_now` on a miss), plus **the ticket's rules** (D73) = the repository's resolved rules
  (`resolve_repo_ruleset`, `knowledge.rs:1354`, made `pub(crate)` — one-line visibility change in an
  unowned file, listed) and the ticket's acceptance criteria (live). No ticket level in governance.
- `POST /api/targets/evaluate` body `{ folder, key?, base? }` → TG4 over the working tree; stores
  `subject_kind = 'change'` rows; returns every result, missed first.

**MCP (`crates/mcp/src/lib.rs`, one hunk):** `daemon_request_for` arms, declarations in
`handle_list_tools`, names in `EXPECTED_TOOLS`, a non-empty `defaults` (M14 lead invariant):
- `get_ticket` (required `key`; optional `session_id`) → `POST /api/tickets/{key}/fetch` with
  `folder = cwd`. Description: "Fetch a tracker ticket by key and start work on it. Returns the
  ticket, its rules and the targets that apply. Call this first when the user hands you a ticket."
- `get_targets` (optional `key`) → `GET /api/targets?folder=<cwd>&key=`.
- `get_missed_targets` (optional `key`, `base`) → `POST /api/targets/evaluate`. Description says a
  `block` miss reads "target missed (blocking)" and that `not_measured` is not a pass.

**Tests first:**
- `crates/mcp/src/lib.rs` tests: `get_ticket_shapes_a_post_with_the_cwd`,
  `get_targets_defaults_to_the_cwd`, `get_missed_targets_passes_base_when_given`; the catalogue test
  with the three names.
- Daemon (router-level, scripted `TicketReader`): `a_fetch_links_the_session_and_records_start`,
  `a_second_fetch_records_nothing_new`, `an_unknown_key_is_404_and_writes_no_link`,
  `a_fetch_without_a_known_session_returns_the_ticket_and_says_not_recorded`,
  `the_ticket_s_rules_and_targets_arrive_with_it`, `each_target_names_its_rung`,
  `a_reader_failure_is_502_not_an_empty_ticket`, `key_matching_still_links_a_gh_issue_view_session`
  (fallback unchanged), `evaluate_reports_thin_readings_as_not_measured`.

**Accept (live):** in a Claude Code session in this repository, `get_ticket AUTH-…` (a real synced
key) returns the ticket and `select source, started_at from sensei.session_task_links where
source = 'fetch'` shows one row; a Jira sync afterwards leaves it in place; `get_missed_targets`
after lowering coverage on a guarded path returns that target as "target missed (blocking)" with
files.

**Do not:** store description, assignee or acceptance criteria (D71); write to the tracker;
return a placeholder ticket; call any result a gate.

---

### TG6 · Accepting a deviation (dōjō)

**Schema (dōjō):**
- `database/ddl/table/dojo/target_acceptances.ddl`: `result_id uuid primary key references
  dojo.target_results(id) on delete cascade, tenant_id, accepted_by references dojo.principals,
  accepted_at timestamptz default now(), note text not null check (length(btrim(note)) > 0),
  follow_up_title text not null, follow_up_body text not null, follow_up_state text not null check
  (follow_up_state in ('drafted','filed','written')) default 'drafted', follow_up_key text`. A
  separate table, so a re-push of the result (TG4 upsert) never erases an acceptance.
  Policy `database/policies/dojo/target_acceptances.sql`: select where the caller may read the
  result (join, same predicate as `target_results`).
- `database/ddl/function/dojo/target_deviation_accept.ddl`: `dojo.target_deviation_accept(p_result_id
  uuid, p_note text) returns dojo.target_acceptances`. Gate: `tenant_role_at_least(tenant,'lead')`
  (lead or admin, D61; a maintainer is refused) and `can_read_repository(repo, tenant)`. The result
  must be `mode = 'block'` and `verdict = 'missed'` (else `invalid_parameter_value`: a `warn` miss
  needs no acceptance, D72). Empty note → `invalid_parameter_value`. Already accepted →
  `unique_violation` (409). The follow-up draft is built here from `fix_title` / `fix_items`.
- `database/ddl/function/dojo/targets_needing_acceptance.ddl`: `(p_tenant_id uuid)` → for a
  lead or admin, the latest result per `(target_id, repository_id, subject_kind, subject_key)` that is
  `block` + `missed` with no acceptance; a later `met` for the same subject clears it. A caller below
  lead → `insufficient_privilege` (the Home says the queue is not theirs, rather than an empty list).
- `database/ddl/function/dojo/target_follow_up_record.ddl`: `(p_result_id uuid, p_state text,
  p_key text)` → `filed` (the lead filed the draft by hand) or `written` (a writer created it); lead+.
- All into the definer gate test.

**Follow-up ticket (D50, D61).** Always drafted. **Written only when the workspace opted into
writeback** — L7's flag (L7 step 1) — **and** a writer exists. Neither exists today: the flag is L7's
current work, and a dōjō-side tracker write needs the dōjō-owned connection (D49, #310). So this
package ships draft + `filed`; `written` is reachable only through `target_follow_up_record`, which
L7's writer will call. State it as the remaining item in the handoff.

**Tests first (SQL `database/tests/dojo/target_acceptance.sql`):**
`a_lead_accepts_a_blocking_miss_with_a_note`, `an_admin_accepts`, `a_maintainer_cannot_accept`,
`a_viewer_cannot_accept`, `an_empty_note_is_refused`, `a_warn_miss_cannot_be_accepted`,
`a_met_result_cannot_be_accepted`, `accepting_twice_is_409`, `a_repush_keeps_the_acceptance`,
`the_queue_lists_only_standing_block_misses`, `a_later_met_clears_the_queue`,
`a_warn_or_off_never_enters_the_queue`, `the_queue_below_lead_is_refused`,
`the_draft_names_the_fix_items`. vitest: `accept_maps_409_and_422`, `needs_you_403_below_lead`.

**Route:** `GET /targets/needs-you` (tenant-floored, member; the function decides 403);
`POST /targets/results/[id]/accept` body `{ note }`; `POST /targets/results/[id]/follow-up` body
`{ state: 'filed', key }`.

**Accept (live):** with a TG4 `missed` block result in the local dōjō, the lead's
`GET /targets/needs-you` lists it; `POST …/accept` with a note removes it from the list and
`dojo.target_acceptances` holds the note, the accepter and a draft whose body names the files; a
contributor's `POST` is 403 and changes nothing.

**Do not:** lock a merge, a sprint or the tracker (D61); let a `warn` miss into the queue (D72); write
to a tracker from this package; send email or Slack (D69).

---

### TG7 · Surfaces (contracts only; L4 builds the screens, L3 the state and load)

Types go at the end of `app/src/lib/types.ts` and in `dojo/src/lib/server/targets-data.ts`
(shared-hotspot rule: append, never rename). Field names are snake_case on the wire.

```ts
type TargetLevel = 'dojo' | 'workspace' | 'repository' | 'sprint' | 'task_type' | 'task';
type TargetMode = 'block' | 'warn' | 'off';
type TargetVerdict = 'met' | 'missed' | 'not_measured';

interface TargetInput {            // PUT /v1/t/{o}/{org}/targets
  tenant_id: string; level: TargetLevel; workspace_id: string | null; repository_id: string | null;
  subject_key: string | null; subject_label: string | null; metric_key: string;
  paths: string[] | null; threshold: number; mode: TargetMode; ratchet: boolean; note: string | null;
}
interface TargetRow extends TargetInput {   // GET /targets?workspace=
  id: string; comparator: '>=' | '<='; unit: string | null; set_by: string; set_at: string;
  may_edit: boolean; entitled: boolean | null; reason_code: string | null;  // 'needs_pro'
}
interface ResolvedTarget {         // daemon replica; MCP get_targets; desktop "What applies here"
  target_id: string; level: TargetLevel; source_label: string; metric_key: string;
  paths: string[] | null; threshold: number; comparator: '>=' | '<='; mode: TargetMode;
  ratchet: boolean; delta_scoped: boolean; unit: string | null; note: string | null;
  entitled: boolean; reason_code: string | null; fetched_at: string;
}
interface FixItem { kind: 'file' | 'function' | 'edge' | 'rule'; location: string; detail: string; }
interface TargetResult {
  id: string; repository_id: string; repo_key: string; target_id: string; level: TargetLevel;
  metric_key: string; paths: string[] | null; mode: TargetMode;
  subject_kind: 'change' | 'ticket' | 'sprint'; subject_key: string | null; head_sha: string;
  reading: number | null; threshold: number; effective_threshold: number; baseline: number | null;
  coverage: number | null; verdict: TargetVerdict; reason_code: string | null;
  fix_items: FixItem[]; fix_title: string | null; fix_detail: string | null;
  principal_id: string | null; evaluated_at: string;
  acceptance: { accepted_by: string; accepted_at: string; note: string;
                follow_up_state: 'drafted' | 'filed' | 'written'; follow_up_key: string | null } | null;
}
interface TargetAggregate {        // viewer (D68): never a principal; withheld below 2 contributors
  repository_id: string; repo_key: string; metric_key: string; day: string;
  met: number; missed: number; not_measured: number; contributors: number;
}
interface NeedsYouItem { result: TargetResult; workspace_id: string; workspace_name: string; }
```

Surfaces and their sources:
- **Desktop workspace "Metrics & targets"** (`app/src/routes/(workspace)/workspace/[id]/metrics/**`,
  L3): `GET /api/workspaces/{id}/targets/results?days=` (daemon, `TargetResult[]` from
  `sensei.target_results` joined to the replica). Each series card shows its target line and the
  latest verdict; WP5 left the slot (L1 step 4).
- **Desktop notice on a miss**: one `@sensei/ui` `Notice` (tone `action`, eyebrow "target missed
  (blocking)" or "target missed", the fix title, the fix items, a "hand to the assistant" action that
  copies the `get_missed_targets` prompt). Built from `TargetResult` by a pure
  `noticeFromTargetResult` in the app (L4 decides whether it belongs in `notice.js`). Do not fork
  `Notice`.
- **Desktop "What applies here"** (D67, D76; L4's `(observatory)/dojo/**`):
  `GET /api/workspaces/{id}/targets/applies` → `{ rules: ResolvedRule[], targets: ResolvedTarget[],
  fetched_at }`, each with its source rung.
- **Dōjō Targets tab** (L4): `TargetRow[]`; edit only where `may_edit`; an unentitled repository
  row shows "Targets on private repositories need Pro." and links to Settings › Plan & billing; never
  hidden (D52, D75).
- **Dōjō Home "needs you"** (D69): `NeedsYouItem[]`; accept needs a note; the draft shows with
  "File it" (writeback off) — "Open in tracker" appears only once `written` exists.
- **Viewer shell** (D68): `TargetAggregate[]` beside the metrics, never per person.
- **Lead**: `TargetResult[]` with `principal_id`, filtered by the database (D38).

**Tests first:** vitest for each `*-state` / `*-view` module the screen lanes add, plus
`noticeFromTargetResult`: `a_block_miss_reads_target_missed_blocking`,
`not_measured_reads_as_not_measured_not_met`, `a_warn_miss_has_no_accept_action`.

**Accept:** L4's screens render the live TG2–TG6 data; the words "gate" and "gated" appear in no
target string (`rg -n -i "\bgate" app/src dojo/src -g '*target*'` empty).

**Do not:** compute a verdict, a comparator or an inheritance in TS; show a per-person result to a
viewer.

## 5. Ownership

| WP | Lane | Owned paths it adds or edits | Cross-lane items to list in the handoff |
|---|---|---|---|
| TG1 | L3 | `ddl/table/{sensei,staging}/metric_gating.ddl`, `procedure/staging/import_metric_gating.ddl`, `import/staging/metric_gating.jsonl`, `pg_store/targets.rs` | `design.yaml` dojo includes (L0); catalogue rows for target metrics (L2 appends to `metrics.jsonl`) |
| TG2 | L3 | `ddl/enum/sensei/target_{level,mode}.ddl`, `ddl/table/dojo/targets.ddl`, `ddl/function/dojo/target_*.ddl`, `targets_for_workspace.ddl`, `policies/dojo/targets.sql`, `routes/v1/t/[origin]/[org]/targets/**`, `lib/server/targets-data.ts` | definer gate test lines (L3 adds; the file is shared by every dōjō lane); WP8 must merge first (L1) |
| TG3 | L3 | `targets_resolve*.ddl`, `ddl/table/sensei/target_resolutions.ddl`, `crates/senseid/src/targets/{sync,resolve}.rs` | `sync_entity` values, `schedules.jsonl` row, `tasks/mod.rs` line |
| TG4 | L3 | `targets/{verdict,ratchet,readers/*}.rs`, `target_verdict.ddl`, `target_results.ddl` (both planes), `target_baselines.ddl`, `target_results_*.ddl`, `policies/dojo/target_results.sql` | `coverage.rs` `parse_lcov_by_file` (unowned); `InsightKind::TargetMissed` (L2); graph readers wait on #156 rows from `layering.rs` (L2); `label` on the plan wire `GraphBranch` (L1) |
| TG5 | L3 + **L7** | L3: `targets/http.rs`, MCP hunk, `api/routes.rs` hunk. L7: `TicketReader`, `TicketDetail`, Jira and GitHub Issues readers, `link_source 'fetch'`, `started_at`, relink exclusion, `LinkSource::Fetch` | `knowledge.rs` `resolve_repo_ruleset` → `pub(crate)` |
| TG6 | L3 | `target_acceptances.ddl`, `target_deviation_accept.ddl`, `targets_needing_acceptance.ddl`, `target_follow_up_record.ddl`, policy | `written` waits on L7's writeback flag and a writer (D49/#310) |
| TG7 | L4 (screens) + L3 (state/load of `workspace/[id]/metrics/**`) | wire types appended to `app/src/lib/types.ts` | Notice helper placement (L4) |
| #156 metrics | **L2** | catalogue rows + computation (cycles, zone of pain, complexity outliers) | each lands with its `metric_gating` row from L3 |

## 6. What this spec does NOT do

- Targets owned by teams (D13, D48), an Epic rung (D48), release targets over a change set (T5).
- Any hard stop: nothing locks a merge, a sprint or the tracker (D61).
- Writing to a tracker, the dōjō-owned tracker path (D49, #310), the writeback flag itself (L7).
- Storing ticket description, assignee or acceptance criteria (D71).
- An anti-pattern metric (target 7): no code anti-pattern producer exists (`detected_patterns` holds session
  patterns with null severity). Target 3 (layer violations) is **not** in this list: L2's #156 produces
  it as the Up-edge count under derived layering (§1, TG1).
- A per-function complexity store (target 6 stays ineligible).
- D18's environment labels already exist in the dōjō (§1). This spec does not add `label` to the
  plan wire; that is L1's change (TG4 base branch).
- Slack hooks, email, the sprint scorecard screen beyond the wire types, the quality headline goal
  line (V2), the memory proposal on two misses (D66; the learnings lane), the planner (C-1).
- Changing governance precedence (D57) or `MetricGate`.

## 7. Context you need

- dōjō local: `make dojo-local`; schema with `../scripts/deploy-dojo-db.sh` (pre-v1,
  `--allow-destructive`), **deployed twice** (empty, then over itself).
- Daemon DB: lanes never run dbd against `localhost:5432/sensei`; L0 reconciles after merge. A
  `default`-scope DDL change needs `make test-template` before `cargo test -p senseid`.
- Tests spawning `git` (readers, base resolution) must clear every `GIT_*` variable.
- `service_role` bypasses RLS: every Worker read is a definer function with its own gate or an
  invoker read through `callerDb(caller)`.
- Supabase `fake-dojo-db.ts` treats `null === null`; prove the `nulls not distinct` keys in SQL.

## 8. VERIFY items (collected)

Open:
1. `isUuid`: TG2 reuses the shared helper if it exists at build time, else creates
   `dojo/src/lib/server/is-uuid.ts`. L1 owns the WP4 replacement.
2. Where a given Jira site keeps acceptance criteria. The field id is configurable per tracker; an
   absent field gives an empty list with its reason.
3. That WP8 has merged before TG2 merges (personal private entitlement). It has not merged as of
   2026-11-03.

Answered (2026-11-03):
- An unknown `task_name` is skipped with a warning (`metrics_scheduler.rs:69–71`,
  `planner.rs:410–419`). TG1 recommends excluding target-only rows.
- Index completeness is `sensei.folder_completeness.subtree_complete`.
- The development branch is the dōjō's `label`. Putting it on the plan wire is L1's change.
- GitHub token: Keychain `provider_token.<persona>`, scopes `read:org repo`, renewed by
  `forge_token`.
- Hard-rule enforcement: D81 (Q2). The dōjō-only checker gap is a known limit, reported as
  `not_measured`.
