# Metrics & Health · M8

**Plane:** insight · **Status:** [roadmap/02-insight.md](../../../../roadmap/02-insight.md) ·
**Serves:** every persona — the read most teams care about; every other plane renders
what this module derives (Delivery), and Team gates (T3) rely on its data not lying.

## Purpose

Give every repository a small set of measured, first-party, paired numbers and their
trends, computed on a schedule from data sensei already captures, so a developer can
see whether the pairing is improving and act on it. The metric set is
**data-driven**: the `sensei.metrics` registry says what to compute, how to describe
it, and which scheduled task produces it; values are stored **once** at
metric × repository × day grain in `sensei.repository_metrics`; all aggregation —
week/month/quarter, trends, ratings, the **health score** — is derived on read as
views over that one store. Nothing here fabricates a value: **no data ⇒ no row** —
absence reads as "not yet measured," never a defaulted 0. The user feels this module
as *numbers that don't lie, and say what they're for*.

## In scope / out of scope

- In: the registry (`sensei.metrics` — key, name, family, type, unit, direction,
  purpose, how_to_read, formula, task_name, cadence, capture_source, weight, target,
  rating_scale, derives_from, effective window); the value store
  (`sensei.repository_metrics`, repository-grained); the read-side view model
  (`metric_facts`, `project_metric_daily` → weekly/monthly/quarterly,
  `project_metric_trend`, `metric_ratings`/`metric_rating_facts`, `metric_status`);
  the derived health score (`project_health_score` + `project_health_trend`);
  the registry-driven scheduler + per-metric compute handlers; the metrics API family;
  the **master-detail model + gating eligibility** (2026-10-13 decision 8,
  reorg §9.6 — the spec this module authors) and the per-metric design/coverage record
  (M-9); coverage-gated presentation (#155 generalised — M-8); the retired-feeds
  accounting (#153 `churn_rate`, #154 `time_to_useful_result` — D-6).
- Out: the analyzer that produces per-session `sessions.ftr` (M7 — this module rolls
  it up); surfacing in the apps (overview/impact chips, health dial, radar — Delivery
  plane); DORA delivery module (C-3; prereq deploy/release detector, deferred); the
  architecture-metrics family (#156) is spec'd separately and unblocks T3's gates.

## Surfaces

- **API** — `GET /api/metrics/registry` (self-describing catalog),
  `GET /api/metrics/status?repo=` + `GET /api/metrics/status/summary` (per-repo
  computation state and the whole estate — "never computed" judged by stored VALUES,
  not by the registry), `GET /api/projects/{id}/metrics` (+ per-metric series and
  day-stage sessions), `GET /api/projects/{id}/health` (score + rated_metrics +
  components), `POST /api/metrics/backfill`. The legacy name-keyed
  `GET /api/metrics/{project}` remains a landmine (it fabricated zeroes) — consumers
  repoint; see Wrong gate.
- **Daemon** — `tasks/metrics_scheduler.rs` (compute-project wave per repository,
  watermark-frozen as_of) + `tasks/handlers/metrics/*.rs` (per-family compute:
  session_outcomes, autonomy, churn, coverage, quality, cost, usage, knowledge,
  architecture, session_process, planner).
- **DB** — DDL under `database/ddl/` (table `sensei/repository_metrics.ddl`,
  `sensei/metrics.ddl`; views `sensei/{project_metrics,metric_facts,
  project_metric_{daily,weekly,monthly,quarterly,trend},metric_ratings,
  metric_rating_facts,metric_status,project_health_score,project_health_trend}.ddl`).
  Seeding via dbd staging imports (`metric_watermarks.ddl` + the staging
  `import_metrics` pattern), timestamp-guarded.

## Data invariants

- **The store is `sensei.repository_metrics`.** One row = one metric × one
  repository × one day × one scope (× optional commit). It was **renamed from
  `project_metrics`** — a misnomer, not a mismodel: the identity index keyed on
  `repository_id` since D1/D2 (measured pre-rename: repo_id NULL = 0, folder/session
  columns held nothing, and no row disagreed with the folders mapping). `repository_id`
  is NOT NULL; `sensei.project_metrics` survives as a **read-compat VIEW** restoring
  project_id so every dependent view and read path is unchanged.
- **Identity.** Unique index `repository_metrics_identity
  (metric_id, repository_id, scope, identity, commit_sha, computed_on, grain)` with
  `nulls not distinct` — the upsert target compute tasks conflict on, which makes a
  re-run idempotent instead of doubling history.
- **Attribution.** `scope` is `repo | user`; `identity` is the **raw git author
  email** (never replaced); `persona_id` is a nullable *derived* resolution so
  re-attribution stays a re-derivation, not a destructive edit; `commit_sha` only for
  commit-cadence values; `computed_on` is the date the value is **FOR**, not when it
  was computed. `computed_by` (`local | dojo`) is the loop-breaker for
  pull-else-compute; `shared_at` records the Dōjō push.
- **Read-side model.** `metric_facts` (joined facts) → `project_metric_daily`
  (Σ num/Σ den day-pool per project) → weekly/monthly/quarterly; `project_metric_trend`
  adds `prior` + `delta` via `lag()`, with `direction` travelling so the UI can colour
  without a registry re-join.
- **Health is derived on read, never stored.** `project_health_score` (latest) and
  `project_health_trend` (period-grain) =
  `round(20 · Σ(weight·rating) / Σ(weight))` over the **rated** metrics of the period
  — scaling 0–5 to 0–100. `having sum(weight) > 0` is the honest-empty rule: a period
  with nothing rated gets **no row** — a gap in the chart means "not measured," a bar
  at 0 means "measured and failing." `rated_metrics`, `total_weight`, `components`,
  and — on the trend — **`coverage`** (the share of the full model's weight that
  backed the score, 0–1) are exposed because "3 of 21 metrics" must not draw like
  "21 of 21." Retuning a scale re-reads all of history under the current model — no
  recompute-and-backfill job, no stored score to drift. Replaces the **retired**
  `project_health` composite metric (2026-08-12).
- **Registry ↔ code contract.** `task_name` maps to a **compiled `TaskKind`** — no
  string dispatch; a blocked metric is not seeded until its handler ships (the
  scheduler can never enqueue a handlerless task). `cadence` (commit | day) drives
  watermark groups (a mixed-cadence task like churn splits); `capture_source`
  (session | git | snapshot) says whether this metric **authorizes the
  activity-pruner** to reclaim a day's raw sessions (I20) — defaults to snapshot,
  fail-closed; `derives_from` is the correlation-suppression list so a ranking is not
  topped by arithmetic.
- **Never-fabricate, applied.** No data ⇒ no row; ratios/pcts carry
  `props.numerator`/`denominator` so roll-ups re-derive (sum of parts, never
  mean-of-daily-ratios); estimates tagged `source = 'estimated'` never render as
  truth; money-facing metrics write no row on a price miss (**fail closed**).
- **Ratings.** A metric's 0–5 `rating` = how many of its five `rating_scale`
  thresholds the reading passed, direction-aware (≥ for higher_better, ≤ for
  lower_better); null for neutral / weight-0 / scale-less metrics. The daemon owns the
  rating — a spoke and the score can never disagree; never re-derive client-side.
- **Retirement.** `effective_until` in the past (with `retire_reason`) stops new rows;
  history is kept. Standing retirements to keep honest: `project_health` (plain mean —
  not meaningful) and `unused_tools` (verdict rested on fragment-overlap, not outcome).
- **FTR retirement is done.** The legacy `ftr_daily`/`project_ftr_metrics` views were
  retired in Phase 8; FTR now reads `project_metric_daily` only — one number, one
  source.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Registry row | what it computes, how to read it, what "good" is | `ftr`: pct, higher_better, purpose + how_to_read (#gotcha) |
| Metric status | per-repo computation state — live / thin / never | `false_crash_rate`: "never computed" (0 values), not a chart |
| Latest value + series | per metric, grain-selectable | `ftr` daily: Σnum/Σden with session_count |
| Trend arrow | `lag()` prior + delta, direction-coloured | weekly ftr 0.75 → 0.82 (▲) |
| Health dial + radar | `project_health_score`; radar plots **ratings**, never raw values | 82, from 18 of 21 metrics |
| Coverage readout | `rated_metrics`/`total_weight` + trend `coverage` | "65, from 18 of 21 metrics" — never a bare number |
| Pareto / heatmap | churn concentration, module friction map | rework_density per module |

## Done gate

- Registry self-describes: `curl -s localhost:7744/api/metrics/registry | jq
  '[.[]|select(.purpose==null or .direction==null or .formula==null)]|length'` → `0`.
- The sensei repo has a daily `ftr` with `props.numerator/denominator` (+ companion
  `rework_ratio`) and `GET /api/projects/{id}/health` returns a score with
  `rated_metrics` + `components` — all `source=measured`.
- FTR reads one number everywhere — `/ftr`-family endpoints, `get_ftr_daily`, the
  MCP tool, and the store agree (pin `ftr_parity_store_vs_views`).
- A weekly value re-derives a ratio from summed parts (3 of 4 → 0.75, never the mean
  of daily ratios); an absent metric is null, not 0, on every surface.
- Setting `effective_until` to yesterday stops new rows next run; a retired metric
  never computes (pin `project_health_retired_compute_writes_no_row`) and never
  renders.
- **Master-detail + gating (decision 8):** every advertised metric has a master-detail
  record with explicit gating eligibility (M-7); a per-metric design/coverage record
  ties it to its chart and LOD (M-9); thin metrics render behind a coverage gate, not
  as strong numbers (M-8, #155 generalised).

```bash
curl -s http://localhost:7744/api/metrics/status/summary \
  | jq '.by_status'   # live / thin / never — decided by stored VALUES
curl -s http://localhost:7744/api/projects/sensei/health \
  | jq '{score: .health_score, rated: .rated_metrics, weight: .total_weight}'
```

## Wrong gate

- **FTR computed/served more than one way** — the three-way conflict this module
  exists to end.
- **A weekly/monthly value that is the arithmetic mean of daily ratios.**
- **A fabricated 0** where data is absent (the legacy `/api/metrics/{project}`
  handler surviving) or a health score for a repository with nothing rated.
- **A handlerless `task_name` seeded/enqueued**, or a metric past `effective_until`
  still computed.
- **Cost written with a defaulted price on a miss** — must fail closed.
- **`scope=user` rows attributed to the wrong author** — `identity`/`persona_id`
  resolution skipped.
- **A rating re-derived client-side** (spoke and score disagree), or a **neutral /
  weight-0 metric rendered with good/bad colour**.
- **`coverage` ignored** — a repo sensei knows almost nothing about reading as
  perfect (#155 inversion: 1 metric → 98).
- **Health rows stored** — the composite must stay a view; storing it reintroduces
  drift and backfill.
- **Metric defected but still advertised** — #153 `churn_rate` (caption describes the
  retired indexing feed) and #154 `time_to_useful_result` (rating scale in minutes,
  value in seconds) are the standing examples (D-6).

## Scenarios

- [`scenarios/metrics.feature`](scenarios/metrics.feature) — registry, store,
  aggregation, health, gating.

## Open gaps

- [M-7](../../../../roadmap/06-gaps.md) — no master-detail model + `gating` field;
  gating eligibility undefined per metric (P1 — closed by decision 8's spec).
- [M-8](../../../../roadmap/06-gaps.md) — coverage-gated presentation absent (#155
  generalised; P1).
- [M-9](../../../../roadmap/06-gaps.md) — no per-metric design/coverage record tying
  a metric to its chart and LOD (P1).
- D-6 — metric doc defects (#153 `churn_rate`, #154 `time_to_useful_result`; P2, see
  [02-insight.md](../../../../roadmap/02-insight.md)). The #156 architecture-metrics
  family is WIP in a separate backlog session (decision 8) and unblocks T3's gates.

## Related

- [spec/pipeline/metrics](../../../../spec/pipeline/metrics.md) — registry, store,
  views, scheduler, FTR retirement (legacy depth, folds here; its `project_metrics`
  storage must be read through the `repository_metrics` rename).
- [features/metrics/catalog.md](../../../../features/metrics/catalog.md) ·
  [features/metrics/feature.md](../../../../features/metrics/feature.md) ·
  [features/metrics/architecture-metrics.md](../../../../features/metrics/architecture-metrics.md) —
  the catalog + master-detail story + #156 family (dossiers stamped superseded).
- [design/metrics-chart-brief/](../../../../design/metrics-chart-brief/) — the
  per-metric chart/design coverage record (registry.json is authoritative over the
  prose catalog; coverage.json shows live coverage).
- DDL: [`database/ddl/table/sensei/repository_metrics.ddl`](../../../../../database/ddl/table/sensei/repository_metrics.ddl) ·
  [`database/ddl/table/sensei/metrics.ddl`](../../../../../database/ddl/table/sensei/metrics.ddl) ·
  [`database/ddl/view/sensei/project_health_score.ddl`](../../../../../database/ddl/view/sensei/project_health_score.ddl).
- [spec/pipeline/ftr](../../../../spec/pipeline/ftr.md) — FTR source lives here ·
  [spec/pipeline/analyzer](../../../../spec/pipeline/analyzer.md) — scheduler sibling.
- [reorg.md §9.6](../../../../reorg.md) — the master-detail + gating decision.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/tasks/{metrics_scheduler.rs,handlers/metrics/*.rs}`.