---
name: Metrics chart brief — hand-off for chart/graph redesign
type: design-brief
status: current
updated: 2026-09-08
audience: design agent (chart + dashboard redesign)
source_of_truth: sensei.metrics registry (live DB), not the prose catalog
---

# Metrics chart brief

Everything sensei computes for a repository, as it actually exists in the
database on 2026-09-08 — plus real sample data to design against.

**Read this, not `docs/features/metrics/catalog.md`.** That catalog is prose from
2026-08-08 and has drifted: it documents 13 metrics that were never registered
(`reopen_rate`, `cost_of_rework`, `cache_hit_ratio`, `cost_per_ftr`,
`effective_velocity`, `feature_completion`, `drift_mttr`, `quality_delta`,
`recall_hit`, `repeat_mistake`, `guidance_adherence`, `outcome_utility`,
`registry_coverage`, `leak_scan_pass`, `resume_success`, `autonomy_ratio`,
`regression_rate`, `tokens_per_session`) and misses 13 that ARE registered and
computing (`spec_depth`, `spec_deviation_rate`, `context_pressure_rate`,
`refuted_finding_rate`, `incomplete_analysis_rate`,
`incomplete_analysis_llm_rate`, `coverage`, `cost_per_result`, `cache_reuse`,
`session_duration`, `time_to_useful_result`, `tokens_*`). The registry
(`sensei.metrics`) is authoritative and is exported here as `registry.json`.

## Headline numbers

| | count |
|---|---|
| Metrics registered | 29 |
| Active (inside their `effective_from`/`effective_until` window) | 27 |
| Active **and** have ever produced a row | 26 |
| Retired as stored rows | 2 — `project_health` (2026-08-12, **but see below — the capability is live as a view**), `unused_tools` (2026-08-20) |
| Active but never computed | 1 — `false_crash_rate` |
| Families | 7 active: outcome, cost, velocity, quality, autonomy, knowledge, usage |
| Total metric rows in DB | 17,026 across 68 repositories (grows daily) |
| Deepest history | `churn_rate` / `churn_concentration` — 2016-02-01 → today, 2,484 distinct days |
| Shallowest | `coverage` — 3 repositories, 20 days |

## The data model, in one page

One table drives everything: `sensei.repository_metrics`. Each row is
**one metric × one repository × one day × one scope × (optionally) one commit**.

```
metric_id ──→ sensei.metrics        the definition (name, family, direction, rating_scale, weight)
repository_id                       WHICH repo — metrics are per-repository, not per-project
computed_on   date                  the day the value describes
grain         daily | session       every row today is `daily`
scope         repo | user           repo = all authors, whole tree
                                    user = the local user's own commits ∩ files they touched
identity      text                  the git email, when scope = user
commit_sha    text                  set only for commit-cadence metrics (duplication, maintainability)
value         numeric               the reading, in the metric's own raw unit
props         jsonb                 numerator/denominator, a pre-written `explainer` sentence,
                                    `low_n` flag, `n`, `commit`, `currency`/`plan`
source        measured | estimated | federated
computed_by   local | dojo          shared-compute cache provenance
```

Roll-up views already exist and should be preferred over re-aggregating in the
client: `project_metric_daily` → `_weekly` → `_monthly` → `_quarterly`, plus
`project_metric_trend` (adds `prior` + `delta`) and `metric_rating_facts` (adds
the 0–5 `rating` and `weighted`). Note the roll-ups aggregate **type-aware**:
ratios/pcts re-derive from summed numerator/denominator (never a mean of means),
counts sum, durations average.

### The rating: the only cross-metric scale you have

`rating` = **how many of the metric's five `rating_scale` thresholds the reading
passed**, evaluated direction-aware (`>=` for `higher_better`, `<=` for
`lower_better`). Integer 0–5. It is `null` when the metric is `neutral`, has
`weight = 0`, or has no scale.

This matters for design: 19 of 29 metrics have a scale, 10 do not. The raw values
are **unplottable together** — `tokens_per_day` reaches 2.34 × 10⁹ while
`module_quality` lives inside 0–0.0053, six orders of magnitude apart. The 0–5
rating is the shared axis, and the existing radar already uses it.

Weights: `3` = ftr, coverage, duplication_ratio, module_quality. `2` = spec_depth,
spec_deviation_rate. `1` = most. `0` = descriptive-only (churn_concentration,
session_duration, tokens_in/out/per_day, false_crash_rate) — these are context,
never scored.

### Project health is live — as a view, not a stored metric

The `project_health` **metric row** is retired and produced zero rows. The
**capability is live**, composed from the ratings above:

| view | what it gives |
|---|---|
| `sensei.metric_ratings` | latest daily reading per (project, metric) with its 0–5 `rating` |
| `sensei.project_health_score` | `round(20 × Σ(weight × rating) / Σ(weight))` → 0–100, plus `rated_metrics`, `total_weight`, and a `components` jsonb carrying each metric's `rating`/`weight`/`name` |
| `sensei.project_health_trend` | the same score per `grain` × `period`, plus **`coverage`** — the share of total model weight that backed it |

Served by `GET /api/projects/{id}/health`. **67 projects scored**; the trend has
1,806 daily / 449 weekly / 228 monthly rows back to 2018. This is the dial +
radar + component-breakdown input, and it is real data you can design against
today. Live examples: `kavach` 66 (18 rated metrics), `rokkit` 65 (18), `dbd` 59 (17).

⚠️ **But the score is not gated on coverage, and it inverts.** Projects with 1
rated metric average **98**; projects with 18 average **58**. 50 of 67 scored
projects rest on 1–2 metrics, so a repo sensei knows almost nothing about looks
perfect. `project_health_trend` carries `coverage` and `project_health_score`
does not, and neither gates on it. Tracked as
[#155](https://github.com/sensei-hq/sensei/issues/155). **Any dial you design
must show its coverage** — "65, from 18 of 21 metrics" — never a bare number.
`weightCoverage()` in `health-radar.ts` already exists for exactly this.

## The 26 live metrics

Direction: ▲ higher is better · ▼ lower is better · ● neutral/descriptive.
`w` = weight. `scale` = has a 0–5 rating scale.

### outcome — was the work right? (6)

| key | name | type | dir | w | scale | what it measures |
|---|---|---|---|---|---|---|
| `ftr` | First-turn resolution | pct | ▲ | 3 | ✓ | Sessions finished without a correction turn. **The north star.** |
| `rework_ratio` | Rework share | ratio | ▼ | 1 | ✓ | Share of all tool-calls that came from corrected sessions. FTR's mandatory companion. |
| `spec_depth` | Spec depth | score 0–5 | ▲ | 2 | ✓ | LLM judgment of how complete the session's *own stated plan* was before building. Only sessions that planned are scored. |
| `spec_deviation_rate` | Spec deviation | pct | ▼ | 2 | ✓ | Planned sessions whose implementation departed from the stated plan. |
| `context_pressure_rate` | Context pressure | pct | ▼ | 1 | ✓ | Sessions that hit compaction / low-context warnings. |
| `run_completion` | Run completion | ratio | ▲ | 1 | ✓ | Autonomous runs reaching `done`. **Low-N: 5 rows, 1 repo.** |

### quality — is the code healthy? (9)

| key | name | type | dir | w | scale | what it measures |
|---|---|---|---|---|---|---|
| `duplication_ratio` | Duplication | ratio | ▼ | 3 | ✓ | Duplicated source lines ÷ total, from a **qlty scan on a git worktree at each sampled commit** — real history, ~weekly sampling. |
| `module_quality` | Maintainability | ratio | ▼ | 3 | ✓ | qlty non-duplication smell count ÷ total source lines, same worktree mechanism. **Project-level despite the name — not per-module.** |
| `coverage` | Coverage | ratio | ▲ | 3 | ✓ | lines-hit ÷ lines-found from an **ingested lcov report** (the daemon never runs your tests). Present-tense snapshot. |
| `churn_rate` | Churn rate | count | ▼ | 1 | ✓ | Distinct source files touched per commit-day, from `git log --numstat`. |
| `churn_concentration` | Churn concentration | pct | ● | 0 | — | Share of the day's line-churn absorbed by the busiest 20% of files (Pareto). |
| `rework_density` | Rework density | ratio | ▼ | 1 | ✓ | Files flagged `rework:` ÷ project files, from `inference.detected_patterns`. |
| `incomplete_analysis_rate` | Edit before read | pct | ▼ | 1 | ✓ | Sessions that modified a file before any read/search. Deterministic, from tool-event ordering. |
| `incomplete_analysis_llm_rate` | Shallow analysis | pct | ▼ | 1 | ✓ | Sessions showing a retraction-of-understanding ("I misread", "let me actually check"). The LLM companion to the above. |
| `refuted_finding_rate` | Refuted findings | pct | ▼ | 1 | ✓ | Sessions where the assistant reversed its own earlier finding. Distinct from a user correction. |

### velocity (3)

| key | name | type | dir | w | scale | what it measures |
|---|---|---|---|---|---|---|
| `throughput` | Throughput | count | ▲ | 1 | ✓ | Completed sessions per day. Target 5. Only meaningful paired with FTR. |
| `time_to_useful_result` | Time to useful result | duration | ▼ | 1 | ✓ | Median seconds from session start to the first non-correction turn. **See defect #1 below.** |
| `session_duration` | Session duration | duration | ● | 0 | — | Average gap-aware active working time per session. Descriptive, not good/bad. |

### usage — token / context volume (5)

| key | name | type | dir | w | scale | what it measures |
|---|---|---|---|---|---|---|
| `tokens_per_result` | Tokens per result | ratio | ▼ | 1 | ✓ | Output tokens ÷ completed sessions. Not inflated by input cache. |
| `cache_reuse` | Context reuse | pct | ▲ | 1 | — | Mean **per-session** share of input tokens served from cache. Spread comes from session *length*: long sessions 96–99%, 2–3-turn sessions ~83%. |
| `tokens_per_day` | Tokens per day | count | ● | 0 | — | Total input (incl. cache) + output. Dominated by cache re-reads. |
| `tokens_in_per_day` | Input tokens per day | count | ● | 0 | — | Input side. |
| `tokens_out_per_day` | Output tokens per day | count | ● | 0 | — | Generation side — the cleaner "work produced" proxy. |

### cost (1)

| key | name | type | dir | w | scale | what it measures |
|---|---|---|---|---|---|---|
| `cost_per_result` | Cost per result | currency | ▼ | 1 | — | Trailing-30-day subscription fee ÷ (merged runs + accepted recommendations). **Deliberately not a token metric** — under a flat subscription the marginal cost of a token is zero. Blank until a plan is configured. |

### autonomy (2), knowledge (1)

| key | name | type | dir | w | scale | what it measures |
|---|---|---|---|---|---|---|
| `interruption_rate` | Interruptions | ratio | ▼ | 1 | ✓ | Stop events ÷ UserPromptSubmit. The babysitting signal. |
| `false_crash_rate` | False crashes | ratio | ▼ | 0 | — | **Never computed — 0 rows.** Design an empty state, not a chart. |
| `memory_promotion` | Memory promotion | ratio | ▲ | 1 | ✓ | Memories created ÷ eligible patterns/corrections. **≈0 is the signal** — it proves the distill loop is silent. |

## Coverage against the three themes you named

### 1. Code quality — well covered (9 metrics)

Two genuinely strong ones: `duplication_ratio` and `module_quality` run a real
`qlty` scan against a `git worktree` checked out to each sampled commit, so they
backfill over actual history rather than snapshotting today. `churn_rate` and
`churn_concentration` come from `git log --numstat` and reach back to 2016.
`coverage` ingests lcov. Design opportunity: these four have the densest, longest,
most trustworthy series in the system and are currently under-used visually.

### 2. Architectural robustness — NOT COVERED. Zero metrics.

This is the gap. The code graph is populated and substantial:

- `sensei.edges` — **811,071 edges** (`calls`, `imports`, `references`, `extends`, `implements`)
- `inference.communities` — **32,495 architecture clusters**
- `sensei.nodes`, `sensei.symbols`, `sensei.call_graph`, `sensei.edge_resolution_class` — all live

**No registered metric reads any of it.** There is no coupling, cohesion,
fan-in/fan-out, dependency-cycle, layering-violation, module-boundary, or
instability metric. `module_quality` sounds architectural but is a project-level
qlty smell density — it is neither per-module nor structural.

There is also a measurable *indexing* quality signal that no metric exposes.
Edge resolution across the graph:

| resolution_class | edges | share |
|---|---|---|
| resolved | 474,925 | 58.6% |
| no-local-name | 248,613 | 30.7% |
| name-collision-n | 50,017 | 6.2% |
| name-collision-1 | 37,522 | 4.6% |

By kind, `imports` resolves at ~100% while `references` resolves at only 34%.
That 58.6% overall figure is a first-class caveat for any architecture chart —
and arguably a metric in its own right ("graph confidence"), since an
architecture diagram built on 58.6%-resolved edges must say so.

Two further facts, both measured, that bound what this surface can show:

- **`folder_id` is repo-grained, not module-grained.** Only 179 of 9,378 folders
  hold nodes, and those are repo roots. Cross-folder edge count is literally 0 for
  all five edge kinds. Module boundaries have to be derived from `nodes.file_path`
  — sensei resolves to 93 modules at a 2-segment prefix, 246 at 3.
- **Cross-module edges are far too sparse today.** Collapsed to distinct
  module→module pairs, sensei's whole repository yields **13 dependency edges**
  (2 in a cycle), all TypeScript — Rust contributes none because call resolution
  is broken (#146/#151/#152).

So the module-level metrics (coupling, cycles, instability) are **blocked on the
resolver, not on metric design**. Three signals *are* computable today and were
prototyped successfully: graph confidence (58.6%), public-surface ratio (43.8% of
sensei's symbols exported), and symbol-size distribution (26 functions ≥200 lines).

**Recommendation for the design agent:** treat architectural robustness as a
*new surface to design*, not an existing one to restyle, and design it with a
visible confidence/resolution indicator built in from the start. Design against
the three Tier-1 signals, which have real values; leave placeholders for the
module-level ones. **Do not invent values** — nothing computes the blocked ones yet.

Full proposal with prototype queries, per-metric feasibility, and a forward-only
build sequence: [`docs/features/metrics/architecture-metrics.md`](../../features/metrics/architecture-metrics.md),
tracked as [#156](https://github.com/sensei-hq/sensei/issues/156).

### 3. Overall quality of development with AI — best covered

This is where sensei is genuinely differentiated. `ftr` (w=3) is the north star,
and the process-quality cluster is unusual: `spec_depth` and
`spec_deviation_rate` are LLM judgments over the session's own stated plan;
`incomplete_analysis_rate` (deterministic, from tool-event ordering) and
`incomplete_analysis_llm_rate` (semantic) triangulate "dived in before
understanding"; `refuted_finding_rate` catches the assistant flip-flopping on its
own conclusions. Add `interruption_rate`, `context_pressure_rate`,
`time_to_useful_result`, `cache_reuse` and `memory_promotion` and you have a
coherent story about *how* the work was done, not just what shipped.

Design opportunity: these are currently scattered across families (outcome,
quality, autonomy, knowledge) and read as unrelated tiles. As a set they answer
one question and could be composed as one panel.

## Eleven data properties that constrain any chart

Derived from the real data in this bundle. Each one has broken a naive chart
before.

1. **Two radically different time densities on one axis.** Git-sourced metrics
   are per *commit-day* and dense (`churn_rate`: 6,176 rows, 2,484 distinct days,
   back to 2016). Session-sourced metrics only exist on days you worked in that
   repo (`ftr`: 180 rows total across 17 repos — **29 points over 3 months** for
   sensei). Put both on a shared date axis and the session metrics render as
   scattered dots. They need different treatments, or explicit gap handling.

2. **Sparse means not-measured, never zero.** A missing row is "no data", and the
   pipeline deliberately writes no row rather than a fabricated one (non-git
   project, absent `qlty`, no lcov, zero denominator). **Never interpolate,
   never plot 0 for a gap, never `fillna`.** This is a hard project rule, not a
   preference.

3. **Per-tile staleness varies by weeks.** The "latest reading" row for sensei
   spans `2026-08-12` → `2026-09-02`. A KPI row that shows one global "as of"
   date lies about three-quarters of its tiles. Each tile needs its own as-of,
   and a visibly degraded state past some age.

4. **Scales are six orders of magnitude apart.** `tokens_per_day` max
   2,338,956,188. `module_quality` max 0.0053. Any multi-metric visual must use
   the 0–5 rating, per-metric normalisation, or small multiples with independent
   axes — never a shared linear value axis.

5. **Commit-cadence metrics have many rows per day.** `duplication_ratio` and
   `module_quality` carry `commit_sha`. 2026-08-31 alone has 11 commits × 2
   scopes = 22 rows for that single day, values 0.0170–0.0179. A daily chart must
   pick (latest commit) or aggregate (mean), and say which.

6. **`scope` is a real dimension, not a detail.** `repo` = all authors,
   `user` = your commits only. Some repos have only one: `slips` is `repo`-only
   for churn (1,022 rows each) with no user-scope churn at all. A chart that
   silently mixes them compares your work against the whole team's.

7. **`props` already carries the drilldown, including prose.** Every reading
   carries `numerator`/`denominator` and a pre-written `explainer` sentence
   ("value 0.9912, up 0.0393 from the prior day"). Commit metrics carry `commit`;
   duration/mean metrics carry `n`; cost carries `plan` + `currency`. Tooltips
   and detail panels should render these rather than recompute — the explainer is
   generated copy meant to be shown verbatim.

8. **`low_n: true` is an explicit flag in `props`.** `interruption_rate` and
   `run_completion` set it. These readings need a distinct visual state, not a
   confident-looking point.

9. **Ratios are not always bounded by 1.** `interruption_rate` reaches **5.0**
   (Stop ÷ UserPromptSubmit), `memory_promotion` reaches 4.0. Do not hard-code a
   `[0,1]` domain for anything typed `ratio`.

10. **Ten metrics have no rating and cannot be scored.** `churn_concentration`,
    `session_duration`, `tokens_in/out/per_day`, `cache_reuse`, `cost_per_result`,
    `false_crash_rate` (+ 2 retired). Six of those are `weight = 0` context
    metrics. They belong in a chart, but not in a scored grid or a radar spoke —
    and the design needs a home for "descriptive, deliberately unscored".

11. **`neutral` direction means no colour semantics.** `churn_concentration`,
    `session_duration`, `tokens_*` are `neutral` — up is not bad and down is not
    good. Applying a red/green trend arrow to them is actively misleading; the
    registry direction must drive the tone, never a hardcoded per-key rule.

## What the app renders today

`app/src/routes/(project)/project/[id]/metrics/` — the redesign target.

Components: `HealthHero`, `HealthRadar` (0–5 rating spokes, weight-ordered),
`SignalRail` / `SignalGridCell` / `SignalLegend`, `MoverCard`, `CorrelationCard`,
`DetailChart`, `ActionItems`, `DatapointDrilldown`, `ToolBubbles`, `AboutMetric`.
Shared: `MetricCard`, `MetricSparkline`, `FtrStrip`, `ChartCanvas`.

Derivation layer (presentation logic lives here, components are pure templates —
see `app/CLAUDE.md`): `app/src/lib/metrics/metric-view.ts` (1,096 lines —
formatting, grading A–F, trend tone, y-domains, densify, moving average),
`health-radar.ts`, `correlation-view.ts`, `metric-kanji.ts`.

Wire API: `GET /api/projects/{id}/metrics`, `/metrics/{key}`,
`/metrics/{key}/sessions`, `/health`, `/correlations`.

Two existing decisions worth preserving: the radar plots **ratings, not raw
values** (documented reason: raw scales are unplottable together), and the
daemon owns the rating so a spoke and the score cannot disagree. Any redesign
should keep the "never re-derive a rating client-side" invariant.

## Three data defects — do not design around these, they are filed and being fixed

**1. `time_to_useful_result` rating scale is in the wrong unit.**
([#154](https://github.com/sensei-hq/sensei/issues/154))
`unit = "seconds"`, values are seconds (avg 57,885 s ≈ 16 h), but
`rating_scale = [60, 30, 15, 8, 4]`. Distribution of the 173 readings:

| bucket | readings | share |
|---|---|---|
| ≤ 60 s (scores 1–5) | 22 | 12.7% |
| 1–10 min | 57 | 32.9% |
| 10–60 min | 59 | 34.1% |
| > 1 h | 35 | 20.2% |

**87.3% of readings score 0.** The thresholds read as *minutes* (4–60 min to
first useful output is plausible; 4 *seconds* is not), so the scale is off by
60×. Until fixed, this metric contributes a near-constant 0 to the composite
score and should not be presented as a graded signal. Source:
`database/import/staging/metrics.jsonl`.

**2. `churn_rate`'s registry caption describes a retired source.**
([#153](https://github.com/sensei-hq/sensei/issues/153))
`unit = "executions/day"`, `formula = "count(process_file task_executions) per
file per day"`, `description = "process_file executions/day per source file"` —
all describing the old file-*indexing* feed. The handler
(`crates/senseid/src/tasks/handlers/metrics/churn.rs`) is explicit that this was
replaced by `git log --numstat`, and the values confirm it. A chart that renders
`unit` or `description` verbatim will caption a git metric as an indexing metric.
Real unit is *files changed per day*.

Also worth knowing: `churn_rate` has legitimate extreme outliers from bulk
commits — 15,975 files in one day (`spec` repo, 2026-01-22), 6,952 (`sensei`,
2026-05-06) against a median of 6. A linear axis over full history will be one
spike and a flat line. Log scale, clipping with an explicit marker, or a
winsorised view with the raw value in the tooltip.

**3. The composite health score is not gated on coverage, and it inverts.**
([#155](https://github.com/sensei-hq/sensei/issues/155))
`project_health_score` divides by the weight it happened to find, not by the
weight of the model, so absent metrics are excluded rather than counted as
unknown. Across 67 scored projects:

| rated_metrics | projects | avg score |
|---|---|---|
| 18 | 3 | 58 |
| 16–17 | 6 | 70 |
| 9–14 | 8 | 68 |
| **2** | **25** | **79** |
| **1** | **25** | **98** |

75% of scored projects rest on 1–2 metrics. The clearest illustration is in
`sample-health.json` — rokkit's weekly trend jumps from **36 to 100** between
2026-03-02 and 2026-03-16, because `coverage` *fell* from 0.31 to 0.034. The
score doubled by measuring less.

**Design consequence:** never render a bare health number. Show the coverage
alongside it ("65, from 18 of 21 metrics"), and suppress the dial below a floor.
`weightCoverage()` in `app/src/lib/metrics/health-radar.ts` already exists for
this and is currently fed only from the trend view.

## Files in this bundle

| file | contents |
|---|---|
| `registry.json` | All 29 definitions: key, name, family, type, unit, direction, cadence, weight, target, `rating_scale`, description, purpose, `how_to_read`, formula, effective window, retired flag. **The authoritative definition list.** |
| `coverage.json` | Per metric: row count, repos with data, first/last day, distinct days, scopes present, whether commit-keyed, min/p50/max, and the `props` keys that actually appear. **Use this to judge which metrics can carry which chart.** |
| `sample-latest-sensei.json` | Latest reading per active metric for the sensei repo, with computed 0–5 rating, weight, rating scale, per-tile `as_of`, and full `props`. Composite score 43.8 over 18 rated metrics. **The KPI-row / radar / scorecard input.** |
| `sample-series-sensei.json` | Daily series 2026-06-01 → 2026-09-02 for all 25 metrics that have data for sensei, each point carrying its `props`. Point counts range 5 → 66. **The trend-chart input, including the realistic sparsity.** |
| `sample-cross-repo.json` | Latest reading per metric across 6 repos (sensei, rokkit, dbd, kavach, gateway, slips), 121 rows with ratings. **The portfolio heatmap / small-multiples / ranked-bar input. Includes `slips`, which has only 1 metric — the ragged-coverage case.** |
| `sample-long-history.json` | `slips` churn, `scope = repo`: 1,022 daily points over 2016–2024, rolled to 398 weeks. **The decade-scale dense-and-spiky series case.** |
| `sample-health.json` | Composite health from the live views: all 67 scored projects, the 8 well-covered ones with full `components` breakdowns, rokkit's 16-point weekly trend with `coverage`, and the coverage-inversion table. **The dial / radar / component-breakdown input.** |

All values are real readings from the live daemon database on 2026-09-08. None
are synthetic.

## Related

- `registry.json` — the authoritative definitions (supersedes `docs/features/metrics/catalog.md`)
- `app/CLAUDE.md` + `docs/architecture/frontend-svelte-guidelines.md` — token system, type scale, state/component separation. Non-negotiable for any implementation.
- `docs/mockups/Sensei/screenshots/` — current visual reference
