# Metric rating scales + weighted 0–5 health score

Status: **DRAFT — decisions R1–R9 locked (Jerry, 2026-08-20). No code until phase P-A.**

## 1. Problem

Metrics live on incomparable scales (ftr 0–1, throughput a count, maintainability
~0.002 smells/line, time-to-useful-result minutes), so they can't be read together
or rolled into one honest health number. The `project_health` composite is currently
**retired** — the only prior roll-up was a crude linear `normalize(type,direction)` →
`[0,1]` weighted mean that never landed as a usable, explainable score, and gave no
per-metric read you could compare at a glance.

## 2. Objective

Map **every** rate-able metric onto a common **0–5 rating** (0 = worst, 5 = best) via
a **per-metric banded scale**, then:
- surface the per-metric rating (the spokes of a **radar / spider** diagram — the
  health visual), and
- roll the ratings into a **weighted 0–5 health score** = `Σ(weightᵢ · ratingᵢ) /
  Σ(weightᵢ)`, reviving `project_health`.

The radar component itself is **out of scope here** — rokkit has no polar geom
([jerrythomas/rokkit#154](https://github.com/jerrythomas/rokkit/issues/154)); Jerry
builds the viz separately. This spec delivers the **rating-scale data + the score**
(the backend + API), so the radar has clean 0–5 ratings + a health number to plot.

## 3. Decisions (locked)

- **R1 — 0–5 rating, 0 worst / 5 best.** Every rate-able metric maps its latest reading
  to an integer-ish `rating ∈ [0,5]` on a scale where 5 is always "best" regardless of
  the metric's polarity. This is the common scale the radar + the score share.
- **R2 — Per-metric BANDED scale, stored on the registry, tunable.** Each metric carries
  a `rating_scale` (a JSON array of 5 thresholds, in IMPROVEMENT order) on
  `sensei.metrics`. `rating` = how many thresholds the value has "reached". Bands (not a
  linear ×5) so a tiny lower-better ratio (maintainability ~0.0024) grades meaningfully
  instead of bunching at an extreme. Tunable by editing the datafile + re-import — no
  redeploy, no code change. Generalizes the maintainability grade already shipped
  (`25beb609`).
- **R3 — Direction-aware rating from the metric's own `direction`; 0–5 over 5
  thresholds; HALF-OPEN bands with OPEN ends.** The scale is 5 thresholds (the entry
  values for ratings 1,2,3,4,5). `rating = count of thresholds "reached"`:
  `higher_better` (thresholds ascend) → `count(t ≤ value)`; `lower_better` (thresholds
  descend) → `count(t ≥ value)`. So the bands are half-open `[tᵢ, tᵢ₊₁)` and the ENDS
  are open — the bottom band (rating 0) runs down to the worst possible value and the
  top band (rating 5) runs up to the best (100% / 0). Worked coverage
  `[0.40,0.55,0.70,0.80,0.90]`: `x<0.40 → 0`, `[0.40,0.55) → 1`, `[0.55,0.70) → 2`,
  `[0.70,0.80) → 3`, `[0.80,0.90) → 4`, `0.90 ≤ x → 5`.
  **Floor is 0, not 1 — deliberately** (R6 corollary): a *measured-and-failing* metric
  reads **0** (a radar spoke at the center — an honest "this is failing"), which is
  distinct from *not-measured-at-all* (EXCLUDED from the score, R6). A `neutral`-direction
  metric has no "better", so it is NOT rated (excluded — see R7).
  (Alternative under consideration: a 1–5 floor — 4 thresholds, `rating = 1 + count` —
  if a metric should never read 0; rejected as default because it blurs failing vs
  unmeasured.)
- **R4 — Health = weighted mean of ratings, on the 0–5 scale.** `project_health` is
  revived as `round1(Σ wᵢ·ratingᵢ / Σ wᵢ)` over the INCLUDED metrics, `type = score`,
  domain `[0,5]` (a "3.8 / 5"). (A 0–100 presentation is just ×20 in the UI; the stored
  score stays 0–5 to match the radar.)
- **R5 — Weight prop drives aggregation; core metrics weigh more.** `sensei.metrics.weight`
  (already exists) is the aggregation weight. Starting weights (TUNABLE — Jerry's "until
  we conclude what works"): **core = 3** (coverage, ftr, module_quality, duplication_ratio),
  **standard = 1** (rework_ratio, run_completion, interruption_rate, memory_promotion,
  rework_density, throughput, context_pressure_rate, time_to_useful_result, churn_rate),
  **excluded = 0** (false_crash_rate — declared-but-uncomputed; churn_concentration —
  neutral direction).
- **R6 — Honest-empty is EXCLUDED, never rated 0.** A metric with no reading (no row /
  null) drops out of BOTH the radar and the weighted mean — a missing metric must not
  fabricate a bad rating that drags health down. `Σw` is over the metrics that actually
  have a rating this period (same never-fabricate rule the old health rollup used).
- **R7 — Neutral metrics are informational, not rated.** `churn_concentration`
  (direction = neutral) has no "good" pole, so it carries no `rating_scale`, no rating,
  and no weight — it stays a readable number but is not a radar spoke or a score input.
- **R8 — Rating lives in ONE place (a shared fn), reused by the score + the API.**
  `rate(value, direction, scale) → 0..5` is computed once (daemon-side, in the health
  computer's roll-up AND exposed per-metric via the metrics API) so the radar and the
  score never disagree.
- **R9 — Scales are seed data on the registry (jsonl), like weights.** Adding/tuning a
  scale is a `database/import/staging/metrics.jsonl` edit + `dbd import` (the same path
  weights/retirement use), not a migration.

## 4. Schema

- `sensei.metrics += rating_scale jsonb null` — 5 thresholds in improvement order, e.g.
  `[0.5,0.65,0.75,0.85,0.9]` (higher_better) or `[25,12,6,3,1.5]` (lower_better,
  per-KLOC). NULL = not rated (neutral / non-core-yet). Additive.
- `import_metrics` proc + `staging.metrics` gain the `rating_scale` column (the 4-place
  registry ripple: metrics.ddl already has `weight`; add `rating_scale` to staging +
  the import col-list + all jsonl rows that get a scale).
- `project_health` revived: clear `effective_until`/`retire_reason`; `type = score`,
  `direction = higher_better`, `weight` irrelevant (it's the composite, not an input);
  `task_name = health` (the existing `ComputeHealth` barrier computes it).
- `weight` values updated per R5 (jsonl).

## 5. Proposed rating scales (thresholds in improvement order → ratings 1..5)

Tunable starting points; each is "the value at which you reach rating 1, 2, 3, 4, 5".

| metric | dir | weight | scale (→1,2,3,4,5) | notes |
|---|---|---|---|---|
| **coverage** | higher | 3 | `0.40, 0.55, 0.70, 0.80, 0.90` | % lines covered |
| **ftr** | higher | 3 | `0.50, 0.65, 0.75, 0.85, 0.92` | first-try rate |
| **module_quality** (maintainability) | lower | 3 | `25, 12, 6, 3, 1.5` | smells / 1,000 lines (×1000 of the ratio) |
| **duplication_ratio** | lower | 3 | `0.20, 0.10, 0.06, 0.03, 0.015` | duplicated-line share |
| rework_ratio | lower | 1 | `0.60, 0.45, 0.30, 0.18, 0.10` | |
| run_completion | higher | 1 | `0.50, 0.65, 0.78, 0.88, 0.95` | |
| interruption_rate | lower | 1 | `0.60, 0.45, 0.30, 0.18, 0.10` | |
| memory_promotion | higher | 1 | `0.10, 0.25, 0.45, 0.65, 0.85` | |
| rework_density | lower | 1 | `0.40, 0.25, 0.15, 0.08, 0.03` | |
| context_pressure_rate | lower | 1 | `0.60, 0.45, 0.30, 0.18, 0.08` | |
| throughput | higher | 1 | `1, 3, 6, 10, 16` | measurable sessions / window |
| time_to_useful_result | lower | 1 | `60, 30, 15, 8, 4` | minutes (map from the duration unit) |
| churn_rate | lower | 1 | `40, 25, 15, 8, 4` | files/day; high churn = churny |
| churn_concentration | neutral | 0 | — | R7: informational, not rated |
| false_crash_rate | lower | 0 | — | declared-but-uncomputed; scale when it computes |

> The scale *shape* (5 ascending/descending thresholds) is uniform; only the numbers
> differ. `time_to_useful_result` is stored in the duration unit — the rating fn converts
> to the scale's unit (minutes) before banding (documented on the scale, or the scale is
> in the stored unit; decide at build).

## 6. Rating computation

`rate(value: f64, direction: &str, scale: &[f64; 5]) -> u8` (0..5), a pure fn:
- `higher_better`: `scale` ascends; `rating = scale.iter().filter(|t| value >= **t).count()`.
- `lower_better`: `scale` descends; `rating = scale.iter().filter(|t| value <= **t).count()`.
- `neutral` or no scale → `None` (excluded).

Pure + unit-tested against the table above (a coverage 0.86 → 4; a maintainability
0.0024 → 2.4/kloc → 5; a duplication 0.07 → 3; etc.).

## 7. Health computer rework (`health.rs`)

Replace the linear `normalize` with the scale-based `rate`:
1. For each active NON-composite metric with a `rating_scale` + a latest daily reading:
   `rating = rate(value, direction, scale)`; skip (R6) when no reading / no scale.
2. `score = round1(Σ(weightᵢ · ratingᵢ) / Σ(weightᵢ))` over the included metrics;
   `Σweight = 0` (nothing rated) → NO row (honest-empty, never a fabricated score).
3. Write `project_health` (`type = score`, value = the 0–5 score, `props.components` =
   `{key → {rating, weight}}` so the radar + drill-down read the per-metric ratings from
   the same row).

## 8. API / read path

- `project_health.props.components` carries every included metric's `rating` (0–5) +
  `weight` — the radar reads the spokes + the score from this one row.
- (Optional) extend the metrics read so each metric row can also carry its `rating` +
  `rating_scale` (for a per-metric badge, reusing the shipped grade badge for the core 4).

## 9. Phases

- **P-A — scale data + rating fn + health rework** (this spec's core): schema
  (`rating_scale`), the `rate` fn, `health.rs` rework, weights + scales seeded, revive
  `project_health`, tests, deploy. Backend only — no viz.
- **P-B — radar viz** (Jerry / rokkit#154): the spider component consuming
  `project_health.props.components`. Tracked separately.
- **P-C — tuning** (ongoing): adjust scales/weights in the jsonl as real data accrues.

## 10. Invariants

- I1: a metric with no reading is excluded, not rated 0 (R6).
- I2: a neutral-direction / scale-less metric is never a rating or a score input (R7).
- I3: `rate` is monotonic in the improvement direction (a better value never lowers the rating).
- I4: no included metrics ⇒ no `project_health` row (never a fabricated score).
- I5: the radar's per-metric rating == the score's per-metric rating (one `rate` fn, R8).
- I6: scales/weights are seed data — a tune is a jsonl edit + `dbd import`, never code (R9).

## 11. Tests

1. `rate` bands each metric per §5 (higher + lower + boundary values); neutral/no-scale → None.
2. health = weighted mean of ratings; core weight-3 dominates; excluded (0-weight / neutral) don't count.
3. honest-empty: a project missing a metric excludes it (Σw shrinks), doesn't zero-rate it.
4. no rated metrics ⇒ no project_health row.
5. `props.components` carries {rating, weight} per included metric.
6. registry: `rating_scale` seeds from jsonl; a scale edit re-imports (timestamp-guarded).
