# Metrics pipeline — implementation plan (registry · value store · roll-ups · scheduler · FTR consolidation)

> **For agentic workers:** every task is TDD (failing test → confirm fail →
> implement → confirm green → `zero-errors-policy` → commit). Forward-only: no
> phase depends on a later one. Never mark done on a masked/piped exit code
> (`cargo … | tail` reports the pipe's status). Verify against live data, not the
> code path.

**Spec:** [`docs/spec/pipeline/metrics.md`](../spec/pipeline/metrics.md) ·
**Feature:** [`docs/features/metrics/feature.md`](../features/metrics/feature.md) ·
per-metric detail [`catalog.md`](../features/metrics/catalog.md).

**Goal.** A data-driven metrics layer: a `sensei.metrics` registry + a single
`sensei.project_metrics` value store (project+session+date grain) + roll-up views
+ a derived health score + a daily scheduler that runs grouped compute tasks —
and consolidate FTR onto this store, retiring `ftr_daily`/`project_ftr_metrics`.

**Architecture.** One value store, all aggregation as views. The registry
describes/enables/schedules metrics as data; `task_name` dispatches to a single
`TaskKind::ComputeMetrics` handler (grouped by source) — a genuinely new
computation adds a match arm + handler (code), a new *instance* of an existing
computation is a registry row. Health is a barrier task (`blocked_by` the base
tasks). Never fabricate: no data ⇒ no row; estimates tagged; cost fails closed.

**Tech stack.** Rust (`crates/senseid`); Postgres DDL via `dbd` (full-file, no
ALTERs; lowercase keywords, leading commas, `set search_path`, `comment on`);
`cargo test -p senseid` + `cargo clippy --all-targets`; seed via staging + import
procedure.

**Sequencing (forward-only):**
`0 DDL (enums+tables)` → `1 views` → `2 registry seed (staging+import)` →
`3 value-store DB layer` → `4 scheduler + ComputeMetrics skeleton` →
`5 compute handlers` → `6 health` → `7 API endpoints` →
`8 FTR consolidation + retire views` → `9 live deploy + verify`.

---

## Phase 0 — DDL: enums + tables

**Precondition (verified):** read sources exist (`activity.sessions.ftr/outcome/
corrections/turns/started_at/module`, `activity.turns.tool_calls/is_correction`,
`activity.task_executions.folder_path`, `activity.assistant_events`, `activity.runs`,
`sensei.nodes`, `sensei.memories`, `inference.detected_patterns/corrections`,
`sensei.assistant_tools`, `sensei.tool_call_verdicts`, `sensei.projects/folders`,
`sensei.folder_path_aliases`). `nodes.ddl` is the leading-comma table exemplar.

- [ ] **0.1** Enum DDL (one file each, `database/ddl/enum/sensei/`): `metric_family`
  (outcome·cost·velocity·quality·autonomy·knowledge·tool·composite), `metric_type`
  (ratio·pct·count·duration·currency·value·score), `metric_direction`
  (higher_better·lower_better·neutral), `metric_grain` (session·daily),
  `metric_source` (measured·estimated). Follow `database/ddl/enum/sensei/node_kind.ddl`.
- [ ] **0.2** `database/ddl/table/sensei/metrics.ddl` — the registry, exactly the
  spec's columns incl. `weight`, `target`, `effective_from/until`, `retire_reason`,
  `modified_at`; unique on `key`; `metrics_task_idx` on `task_name`. Leading-comma
  style; `comment on table`/columns.
- [ ] **0.3** `database/ddl/table/sensei/project_metrics.ddl` — the value store;
  `project_metrics_identity` partial-free unique index `(metric_id, project_id,
  folder_id, session_id, computed_on, grain) nulls not distinct`; `project_metrics_lookup`.
- [ ] **0.4** `dbd apply` to `sensei_test`. Verify: `psql sensei_test -c "\d+
  sensei.project_metrics"` shows the columns + the identity index; the 5 enums exist.
- [ ] **0.5** Commit `feat(db): metrics registry + project_metrics value store + enums`.

**Acceptance:** the two tables + 5 enums materialize on `sensei_test`; the identity
index treats NULL folder_id/session_id as equal (`nulls not distinct`).

---

## Phase 1 — Roll-up views

**Precondition:** Phase 0. Style exemplar: `database/ddl/view/sensei/ftr_daily.ddl`
(lowercase keywords, leading commas, `set search_path`, `comment on view`).

- [ ] **1.1** `project_metric_daily.ddl` — base: `(project_id, metric, date, value,
  props, type, direction)` from `grain='daily' and folder_id is null` joined to
  `metrics`.
- [ ] **1.2** `project_metric_weekly.ddl` — `date_trunc('week',date)`, inline
  `case` by `type`: ratios/pcts `sum(num)/nullif(sum(den),0)`, counts/currency
  `sum(value)`, else `(array_agg(value order by date desc))[1]`. Same for
  `_monthly.ddl`, `_quarterly.ddl`.
- [ ] **1.3** `project_metric_trend.ddl` — weekly + `lag()` prior + delta + direction.
- [ ] **1.4** `dbd apply` to `sensei_test`; seed a handful of `project_metrics`
  rows by hand and verify a 3-of-4 ratio week reads `0.75` (Σnum/Σden), NOT the mean
  of daily ratios. Remove the hand rows.
- [ ] **1.5** Commit `feat(db): project_metric roll-up + trend views`.

**Acceptance:** weekly/monthly re-derive ratios from numerator/denominator; a
point-in-time metric takes the period end; trend carries prior+delta.

---

## Phase 2 — Registry seed (staging + import procedure)

**Precondition:** Phase 0. Exemplar: `database/ddl/table/staging/scopes.ddl` +
`database/ddl/procedure/staging/import_scopes.ddl` (timestamp-guarded upsert).

- [ ] **2.1** `database/ddl/table/staging/metrics.ddl` — staging table mirroring
  the registry (jsonb or typed columns per the import contract).
- [ ] **2.2** `database/ddl/procedure/staging/import_metrics.ddl` — upsert
  `staging.metrics → sensei.metrics` on `key`, timestamp-guarded; **only rows whose
  `task_name` handler exists are imported** (blocked groups like `cost` are omitted
  from the seed file, per the spec's "not seeded until handler ships").
- [ ] **2.3** Seed data file for the v1 metrics (the computable-now set from
  [`catalog.md`](../features/metrics/catalog.md)): `ftr`, `rework_ratio`,
  `throughput`, `churn_rate`, `churn_concentration`, `rework_density`,
  `duplication_ratio`, `interruption_rate`, `false_crash_rate`, `run_completion`,
  `memory_promotion`, `unused_tools`, `project_health` — each with family/type/unit/
  direction/purpose/how_to_read/formula/task_name/weight. **Seed an explicit
  `target` for the `count`-type metrics** (`throughput`, `churn_rate`,
  `unused_tools`) so they contribute to `project_health` from day one (a null
  `target` correctly excludes a count metric from the composite — fine, but here
  it's unintended). No `design.yaml` change is needed — `import.staging` already
  covers the whole `staging` schema (`design.yaml:83-85`); the new staging table +
  import proc are picked up automatically.
- [ ] **2.4** `dbd import` (or the seed path) into `sensei_test`; verify
  `select count(*) from sensei.metrics` = the seeded set and every row has non-null
  family/type/direction/purpose/how_to_read/task_name.
- [ ] **2.5** Commit `feat(db): metrics registry seed via staging + import_metrics`.

**Acceptance:** the v1 registry seeds idempotently; blocked metrics (cost, reopen)
are absent; a re-import doesn't clobber an edited row (timestamp guard).

---

## Phase 3 — Value-store DB layer (`pg_store`)

**Precondition:** Phases 0–2.

- [ ] **3.1 Failing test** `upsert_project_metric_is_idempotent`: two upserts of the
  same `(metric, project, folder=NULL, session=NULL, date, grain=daily)` yield ONE
  row, second updates value + `modified_at`. FAIL → implement
  `PgStore::upsert_project_metric(metric_id, project_id, folder_id, session_id,
  computed_on, grain, value, props, source)` (`on conflict … do update`). PASS.
- [ ] **3.2 Failing test** `active_metrics_excludes_retired_and_future`: seed 3
  registry rows (active; `effective_until`=yesterday; `effective_from`=tomorrow) →
  `active_metrics()` returns only the active one. FAIL → implement
  `PgStore::active_metrics()` + `active_task_names()`. PASS.
- [ ] **3.3 Failing test** `resolve_folder_from_path_uses_aliases`: a
  `task_executions.folder_path` that matches a `folder_path_aliases` old path
  resolves to the current `folder_id`/`project_id`. FAIL → implement
  `PgStore::resolve_folder_by_path(folder_path) -> Option<(folder_id, project_id)>`
  (join `folders.abs_path`, fall back to `folder_path_aliases`). PASS.
- [ ] **3.4 Failing test** `get_project_metrics_reads_views`: after inserting daily
  rows, `get_project_metrics(project)` returns latest-per-metric + the catalog
  facets (purpose/how_to_read/direction). FAIL → implement read over
  `project_metric_daily`/`_trend` + `metrics`. PASS.
- [ ] **3.5** Commit `feat(senseid): project_metrics DB layer (upsert, active
  registry, folder resolution, read)`.

**Acceptance:** idempotent upsert; active-registry filter honors `effective_*`;
folder-path resolution is alias-safe; reads carry the self-describing facets.

---

## Phase 4 — Scheduler + `ComputeMetrics` skeleton

**Precondition:** Phase 3. Exemplar: `crates/senseid/src/tasks/analyzer_scheduler.rs`
(persisted `sensei.config` watermark) + the task queue's `depends_on` barrier
(used by the scan pipeline).

- [ ] **4.1** Add `TaskKind::ComputeMetrics` + `TaskKind::ComputeHealth` to
  `crates/senseid/src/tasks/mod.rs` (Display, retry policy = retryable, watchdog =
  long bucket) + dispatch arms in `executor.rs`. The compute group travels in
  `Task.path` (the `task_name`). Tests: `task_kind_display` + retry/watchdog lists updated.
- [ ] **4.2 Failing test** `metrics_scheduler_enqueues_active_task_names_per_project`:
  with 2 projects + an active registry over task_names {session_outcomes, churn},
  the scheduler enqueues one `ComputeMetrics` per (project, task_name) + a
  `ComputeHealth` per project `blocked_by` those ids. FAIL → implement
  `metrics_scheduler.rs` (read `active_task_names`, enqueue per project, health as a
  barrier; window from `sensei.config` key `metrics.window_days` default 14). PASS.
- [ ] **4.3 Failing test** `compute_metrics_dispatches_by_task_name`: a
  `ComputeMetrics` task with `path='session_outcomes'` calls the session_outcomes
  computer; an unknown task_name is a logged no-op (never panics). FAIL → implement
  `handlers/metrics/mod.rs` dispatch (`match task.path { "session_outcomes" => …, … }`). PASS.
- [ ] **4.4** Wire the scheduler into boot (alongside the analyzer scheduler).
- [ ] **4.5** Commit `feat(senseid): metrics scheduler + ComputeMetrics dispatch`.

**Acceptance:** the daily scheduler fans out one task per (project, active
task_name); health is a barrier on the base tasks; unknown task_name is inert.

---

## Phase 5 — Compute handlers (one sub-task per group; TDD)

**Precondition:** Phase 4. Each handler recomputes session + date grain for the
window and upserts; **no row when no data**. Each sub-task: seed fixture rows in
`sensei_test` → run the computer → assert the exact `project_metrics` rows.

**⚠ `props` contract (cross-cutting — Phase 1.2's views depend on it).** Every
row of a `ratio` or `pct` metric MUST write `props.numerator` + `props.denominator`
(exact keys) — because `project_metric_weekly/_monthly` re-derive as
`sum((props->>'numerator')::numeric)/nullif(sum((props->>'denominator')::numeric),0)`.
The `value` is `numerator/denominator` at daily grain; bespoke display fields
(e.g. `session_count`, `correction_count`) are ADDITIONAL, never a substitute.
A `ratio`/`pct` row with a real denominator but zero numerator writes `0` (not
absent); a metric with no data (denominator 0 / no rows) writes nothing.

- [ ] **5.1 `session_outcomes`** — `ftr` (daily `value = numerator/denominator`,
  `props.numerator` = first-turn sessions, `props.denominator` = `session_count`,
  `props.correction_count` display), `rework_ratio` (`numerator` = corrected-session
  tool-calls, `denominator` = all tool-calls), `throughput` (`count`, sessions/day).
  Test `session_outcomes_writes_ftr_rework_throughput`: 4 sessions (3 ftr, 1
  corrected; the corrected has 6 tool-calls, the others 2 each = 12 total) → daily
  `ftr` value 0.75 with `props.numerator=3, denominator=4`; `rework_ratio` value 0.5
  with `props.numerator=6, denominator=12`; `throughput` value 4. Per-session `ftr`
  rows (value 1/0) too.
- [ ] **5.2 `churn`** — `churn_rate` (`count`, process_file execs/day per file),
  `churn_concentration` (`pct`, top-20%-files share, `numerator`/`denominator` =
  top-20% churn / total churn), `rework_density` (`ratio`, `numerator` = rework-
  flagged files, `denominator` = project files, from `detected_patterns`). Folder-
  attributed via `resolve_folder_by_path` (3.3), writing per-module (`folder_id`) +
  project rows. **On `None` (unresolved `folder_path`): log a warning and SKIP that
  execution entirely** — it cannot be attributed to a project/module, so it counts
  toward neither the module nor the project aggregate (never a mis-attributed row).
  Test `churn_attributes_to_project_via_folder_path`: an aliased folder path
  resolves; an unresolvable path is skipped + logged (assert it produced no row).
  *(Note: `churn_rate`/`concentration` counts are known-inflated by the rescan bug
  until the version-rescan debounce lands — catalog P0 #4; first live numbers carry
  that caveat, not a computation bug.)*
- [ ] **5.3 `duplication`** — `duplication_ratio` (`ratio`, `numerator` = new
  symbols with a duplicate match, `denominator` = new symbols) via
  `find_duplicates_scoped` (internal fn, not HTTP). Test
  `duplication_ratio_from_node_matches`: 5 new symbols, 2 with a match → value 0.4,
  `props.numerator=2, denominator=5`; per-module + project rows.
- [ ] **5.4 `autonomy`** — `interruption_rate` (`ratio`, `numerator` = Stop,
  `denominator` = UserPromptSubmit), `run_completion` (`ratio`, `numerator` = runs
  `done`, `denominator` = runs started), `false_crash_rate` (`ratio`, killed-at-cap-
  but-waiting ÷ non-done runs). Test `autonomy_metrics_from_events_and_runs`:
  24 Stop / 25 UserPromptSubmit → `interruption_rate` 0.96 (`numerator=24,
  denominator=25`); 5 done of 9 runs → `run_completion` 0.556 (`numerator=5,
  denominator=9`); `props.low_n=true` when denominator < 10.
- [ ] **5.5 `knowledge`** — `memory_promotion` (`ratio`, `numerator` = memories
  created, `denominator` = eligible patterns/corrections with `instance_count≥3`).
  Test `memory_promotion_rate`: 0 memories, 3 eligible → value 0.0 with
  `props.numerator=0, denominator=3` (a real 0, NOT absent — the denominator exists,
  and 0 is the signal that distillation is stalled).
- [ ] **5.6 `tool`** — `unused_tools` (`count`, registered tools with 0 outcome-
  positive calls in window) from `assistant_tools` + `tool_call_verdicts`. Test
  `unused_tools_count`: 3 registered tools, 1 with a positive verdict → value 2.
- [ ] **5.7** Commit per group (`feat(senseid): metrics — <group> computer`).

**Acceptance:** each group writes exactly the expected rows at the right grains;
every ratio/pct row carries `props.numerator`+`props.denominator` (so 1.2's views
re-derive, verified by re-running the Phase 1.4 check on real computed rows); folder
attribution is alias-safe and un-attributable executions are skipped+logged; a real
denominator with zero numerator writes `0`, no-data writes nothing.

---

## Phase 6 — Derived health score

**Precondition:** Phase 5.

- [ ] **6.1 Failing test** `project_health_normalizes_by_direction_and_weight`: two
  components — `ftr`=0.8 (higher_better, weight 2) + `rework_ratio`=0.3
  (lower_better → 0.7, weight 1) → health = round(100 × (2·0.8 + 1·0.7)/3) = 77.
  FAIL → implement `handlers/metrics/health.rs` (normalize per direction; ratios/pcts
  bounded; counts/durations vs `metrics.target`, excluded when `target is null`;
  weighted mean ×100). PASS.
- [ ] **6.2 Failing test** `project_health_absent_when_no_components`: a project with
  no component values writes NO health row (never a fabricated 100). PASS.
- [ ] **6.3 Failing test** `compute_health_waits_for_base` (barrier): `ComputeHealth`
  runs only after its `depends_on` base tasks complete (queue-level). PASS.
- [ ] **6.4** Commit `feat(senseid): project_health derived score`.

**Acceptance:** health is a direction-normalized weighted roll-up; absent on an
empty project; runs after the base tasks.

---

## Phase 7 — API endpoints

**Precondition:** Phases 3–6.

- [ ] **7.1 Failing test** `get_metrics_registry_endpoint`: `GET /api/metrics/registry`
  returns the active catalog; every row has purpose+direction. FAIL → add the route +
  handler (`api/handlers/metrics.rs`). PASS.
- [ ] **7.2 Failing test** `get_project_metrics_endpoint`: `GET /api/projects/{id}/
  metrics` returns latest-per-metric + trend + health, facets attached. FAIL →
  implement. PASS.
- [ ] **7.3 Failing test** `get_project_metric_series_endpoint`: `…/metrics/{key}?
  grain=weekly` returns the series from the weekly view. FAIL → implement. PASS.
- [ ] **7.4** Commit `feat(senseid): metrics read endpoints (registry, project, series)`.

**Acceptance:** the three endpoints read the views/registry; values carry their
self-describing facets; honest-empty where a metric has no rows.

---

## Phase 8 — FTR consolidation + retire the FTR-specific views

**Precondition:** Phases 5+7 (FTR now in `project_metrics`). This is the
supersede/retire from the spec — **repoint before dropping** (P4 blast-radius).

- [ ] **8.1 Failing test** `ftr_getters_read_project_metrics`: `get_ftr_daily`
  (reads `sensei.ftr_daily`) and `get_project_ftr`'s **headline** path (reads
  `sensei.project_ftr_metrics` — `ftr_14d`/`ftr_14d_prev`/`sessions_7d`,
  `pg_store.rs:~7439`) both re-source from `project_metrics` with the SAME response
  shape (`project_id, day, ftr_rate, session_count`; 14d/prev re-derived from the
  daily `props.numerator/denominator`). FAIL → re-source both. PASS.
  **Note the second read path:** `get_project_ftr`'s inline 14-day *trend* query
  (`pg_store.rs:~7446-7456`) reads `activity.sessions` **directly** (not a view), so
  the view retirement doesn't require touching it — re-source it to
  `project_metric_daily where metric='ftr'` for one source of truth (assert its
  numbers are unchanged), or leave it and say so explicitly.
- [ ] **8.2 Failing test** `no_fabricated_zero_ftr_in_metrics_surfaces`: BOTH the
  legacy HTTP route `GET /api/metrics/{project}` (`observatory.rs:575-599`) AND the
  **MCP tool arm `"get_metrics"`** (`mcp.rs:255-273`) independently do
  `if session_count>0 {..} else {0.0}` — a fabricated `0`. FAIL → replace both with
  the store-backed data (remove the `else {0.0}`); the HTTP route is superseded by
  the Phase-7 endpoints, the MCP tool re-sources from `project_metrics`. PASS.
- [ ] **8.3 `measure_verdicts` needs NO repoint (grounded correction).** The real
  handler is `verdicts.rs::measure_verdicts` → `PgStore::measure_pending_verdicts`
  (`pg_store.rs:~4730`), which computes "current" FTR **inline from
  `activity.sessions`** per recommendation (since `acted_at`, `having count(*)>=3`)
  and stores on `inference.recommendations.{baseline_ftr,current_ftr,verdict}`. It
  **does not read `ftr_daily`/`project_ftr_metrics`**, so retiring the views does
  not affect it — no repoint. (The spec's earlier "MeasureVerdicts re-sources here"
  was based on a stale `impact.md`; corrected.) Two **separate follow-up tickets**
  (out of scope for this plan, filed in `docs/backlog.md`): (a) `impact.md` is stale
  — wrong owner file (`measure_verdicts.rs` doesn't exist), wrong tables
  (`applied_recommendations`/`impact_verdicts` snapshots), wrong enum
  (`insufficient_data`); (b) a pre-existing **fabricated baseline** bug —
  `measure_pending_verdicts` reads `coalesce(r.baseline_ftr, 0)` but
  `accept_recommendation` never sets `baseline_ftr`, so verdicts likely compute
  against a `0.0` baseline (violates never-fabricate).
- [ ] **8.4 Parity check** `ftr_parity_store_vs_views`: for a seeded project, the
  store's daily FTR equals what the old `ftr_daily` view computed (`avg(ftr)` over
  the day) and the 14d headline equals `project_ftr_metrics`. PASS — proves no drift
  before dropping.
- [ ] **8.5** Delete `database/ddl/view/sensei/{ftr_daily,project_ftr_metrics}.ddl`;
  `grep -rE "ftr_daily|project_ftr_metrics" crates/ app/` is clean AND
  `grep -n "else 0.0\|else {0.0}" ` in the two metrics surfaces is gone. Update
  `spec/pipeline/ftr.md` to name `project_metrics` as the FTR source. (`impact.md`
  stays untouched here — its fix is the separate 8.3 ticket.)
- [ ] **8.6** Commit `refactor(senseid): consolidate FTR onto project_metrics; retire
  ftr_daily + project_ftr_metrics`.

**Acceptance:** one FTR number everywhere (store == `/ftr-daily` == `/projects/{id}/
ftr` == MCP); both fabricated-`0` sites (HTTP + MCP) are gone; the two FTR views are
dropped with zero dangling references; ftr.md updated; `measure_verdicts` left
correctly untouched (with the stale-doc + baseline-bug follow-ups filed).

---

## Phase 9 — Live deploy + verify (deploy-sensitive; own careful run)

**Precondition:** Phases 0–8 green on `sensei_test`; `cargo test -p senseid` +
`clippy --all-targets` clean; `make test-fast` green.

- [ ] **9.1** Deploy gate: on the live `sensei` DB (daemon stopped for the DDL
  window), `dbd reconcile --scope default` to add the enums + `metrics` +
  `project_metrics` + the new views, and drop the retired FTR views (additive except
  the two view drops, which 8.5 made safe). Then run `import_metrics` to seed the
  registry.
- [ ] **9.2** Install the new daemon (`make install-service`); the metrics scheduler
  runs on boot.
- [ ] **9.3 Live verify:** after a scheduler pass, `curl …/api/projects/sensei/metrics`
  shows real `ftr` (+ session_count/correction_count props), `rework_ratio`, and a
  `project_health` in 0–100; `…/ftr-daily` matches the store; `…/api/metrics/registry`
  lists the seeded metrics. HALT-ON-FAILURE — do not drop the FTR views on the live
  DB until parity (8.4) is confirmed there.
- [ ] **9.4** Commit any migration notes to `docs/backlog.md`.

**Acceptance:** live project metrics compute on schedule; FTR parity holds on the
live DB; health renders; no fabricated values.

---

## Final verification (whole plan)
- [ ] `cargo test -p senseid` green (every canonical test above); `clippy
  --all-targets` clean; `make test-fast` green.
- [ ] One FTR number: store, `/api/projects/{id}/ftr`, `/ftr-daily`, MCP agree.
- [ ] `grep -rE "ftr_daily|project_ftr_metrics" crates/ app/` clean.
- [ ] Neither metrics surface fabricates a `0`: the legacy HTTP route and the MCP
  `get_metrics` arm are both store-backed (no `else {0.0}`).
- [ ] Every ratio/pct row carries `props.numerator`+`props.denominator` (rerun the
  1.4 weekly-rollup check on real computed rows).
- [ ] A metric with no data writes no row; cost (when built) fails closed.

## Out of scope (separate specs/plans)
- Session-retro → insights → dynamic skill/agent/rule registry (threshold-gated;
  `session_retro.rs` partly exists) — its own spec.
- App surfacing (project overview/impact chips, trend surface, module heatmap,
  health dial) in sensei + dōjō — taken up after this lands.
- Blocked metrics (cost/tokens, reopen_rate, effective-velocity, per-module qlty
  quality) — seeded + implemented when their source instrumentation lands (catalog
  P0/P1).

## Separate follow-up tickets (filed in docs/backlog.md, not this plan)
- **`impact.md` is stale** — names a non-existent `measure_verdicts.rs` owner file,
  `sensei.applied_recommendations` + snapshot `impact_verdicts` tables that don't
  match reality, and an `insufficient_data` verdict that isn't in
  `recommendation_verdict`. Reality: `verdicts.rs::measure_verdicts` →
  `measure_pending_verdicts` on `inference.recommendations`. Doc-fix only.
- **Fabricated verdict baseline (pre-existing bug)** — `measure_pending_verdicts`
  reads `coalesce(r.baseline_ftr, 0)` but `accept_recommendation` never sets
  `baseline_ftr`; verdicts likely compute against a `0.0` baseline (never-fabricate
  violation). Fix: stamp `baseline_ftr` at accept time (from the store's current
  FTR). Separate bug ticket.
- **Dead `getMetrics()` app wrapper** — `app/src/lib/api.ts:648-649` wraps the
  legacy `/api/metrics/{project}`; unused in `app/src`. Delete once Phase 8.2 lands.
