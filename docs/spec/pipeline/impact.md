# 果 · Pipeline · Impact

**Owner files:**
- Measurement: `crates/senseid/src/tasks/handlers/measure_verdicts.rs`
- Persistence: `crates/senseid/src/db/pg_store.rs` (`sensei.impact_verdicts` table)
- Read endpoints: `crates/senseid/src/api/handlers/impact.rs`
- Regression alert: same table + a scheduled comparison task

## Purpose

Impact is the receipt. When the user `Apply`s a recommendation, the
system makes a claim — "this will move FTR by +X%". Impact is the
measurement that either confirms or contradicts the claim. It is
the accountability layer over [[pipeline/insights]] and
[[pipeline/memory]]: no claim without a receipt.

Two direction-critical outputs:

- **Positive verdicts** — evidence that an applied change worked.
  Reinforces the memory / promotes the pattern / earns a "kept"
  chip on the recommendation.
- **Negative verdicts (regressions)** — evidence that an applied
  change made things worse. Surfaces on the **Impact › Regressions**
  screen (the safety screen from the journey map). The user needs
  to see these; the mockup gives them a dedicated nav entry so they
  can't hide behind the summary.

Kanji is 果 — *result / fruit*.

## Data invariants

- `sensei.applied_recommendations` — join row when a recommendation
  is applied:
  - `id` uuid, `recommendation_id` uuid, `project_id` uuid,
    `applied_at` timestamptz, `applied_by` uuid,
    `measurement_window_days` int (default 7),
    `baseline_snapshot` jsonb (`{ ftr_14d, ftr_7d, sessions_7d, corrections_7d }` at apply time)
- `sensei.impact_verdicts` — one row per completed measurement:
  - `id` uuid, `applied_recommendation_id` uuid,
    `measured_at` timestamptz,
    `before` jsonb (== baseline_snapshot),
    `after` jsonb (`{ ftr_14d, ftr_7d, sessions_7d, corrections_7d }` at measurement time),
    `delta` jsonb (per-metric diff),
    `verdict` enum `positive | neutral | negative | insufficient_data`,
    `confidence` numeric 0..1,
    `note` text (optional — the "why" behind the verdict, model-generated)
- Verdict decision rule:
  - `positive` if `delta.ftr_7d >= +0.05` AND `after.sessions_7d >= MIN_SAMPLES`
  - `negative` if `delta.ftr_7d <= -0.05` AND `after.sessions_7d >= MIN_SAMPLES`
  - `neutral` if the change is within ±0.05 with sufficient samples
  - `insufficient_data` when `after.sessions_7d < MIN_SAMPLES`
    (default 3) — measurement re-schedules for another window
- `MIN_SAMPLES` is the low-signal guard. Without it a 1-session
  window would produce a swing that isn't real.

## Measurement task

`MeasureVerdicts` is a global (not per-project) task that runs on
the analyzer tick when there's an applied recommendation due for
measurement (`applied_at + measurement_window <= now`).

For each due applied-recommendation:

1. Fetch `after` snapshot — same shape as `baseline_snapshot`
   from `sensei.project_ftr_metrics` scoped to the project.
2. Compute `delta`.
3. Apply the verdict rule.
4. Generate the `note` via [[pipeline/narration-cache]] with
   `kind = impact_verdict_positive | impact_verdict_neutral |
   impact_verdict_negative | impact_verdict_insufficient`.
5. Insert into `sensei.impact_verdicts`.
6. Downstream effects:
   - `positive` → reinforce the underlying memory (+1 to
     `reinforced` counter); if the underlying signal is a pattern,
     bump `pattern_effectiveness.observed_ftr_delta`.
   - `negative` → challenge the underlying memory (or flag the
     pattern as anti-pattern candidate); create an
     Impact-Regression alert (see below).
   - `insufficient_data` → do not touch counters; re-queue with
     a longer window (double up to `max_measurement_window`).

## Regression alerts

When a verdict is `negative`, the pipeline creates an alert:

- `sensei.impact_regressions` row with `applied_recommendation_id`,
  `verdict_id`, `severity` (`low | med | high`).
- Alerts surface on the **Impact › Regressions** screen at
  `/impact` (alert state) — a distinct nav entry, not buried.
- User actions on a regression alert:
  - **Revert** — mark the memory `archived` and the recommendation
    dismissed; log revert reason.
  - **Keep** — accept the trade-off (some negatives are worth it —
    e.g. more corrections but higher final quality). Record the
    reasoning; do not surface again unless materially worse.
  - **Investigate** — leave open; snooze the alert for a
    configurable window.

## Signals produced

| Signal | Consumer |
|---|---|
| `positive` verdicts | Impact screen "kept" list; reinforce memory / promote pattern |
| `negative` verdicts | Impact › Regressions alert; Today red banner when unacknowledged |
| Verdict notes (copy) | Impact detail pane |
| Delta plot | Impact chart (before/after FTR) |
| Insufficient-data flags | Impact "measurement pending" list |

## Done gate

- On Jerry's live data every applied recommendation older than the
  measurement window has an `impact_verdicts` row.
- Verdict distribution is bounded: no run should return >90%
  `insufficient_data` — that means the measurement window is too
  short for the project's cadence.
- Positive verdicts reinforce the underlying memory (memory row's
  `reinforced` incremented).
- Negative verdicts create a regression alert row and surface on
  the Impact › Regressions screen.
- Every verdict has a `note` in the mentor voice, generated by
  narration-cache or its fallback.
- Delta plot on the Impact detail pane matches the raw before /
  after values (no chart-vs-number drift).
- The Impact nav entry is always present (safety-screen
  discoverability from the journey map).

Optional check:
```
psql -A -t -c "select verdict, count(*)
                 from sensei.impact_verdicts
                 where measured_at > now() - interval '30 days'
                 group by verdict" -d sensei
# expected: rows for positive / neutral / insufficient_data;
#           any negative row → check Impact › Regressions

# Are any applied recommendations overdue for measurement?
psql -A -t -c "select count(*) from sensei.applied_recommendations a
                where not exists (select 1 from sensei.impact_verdicts v
                                    where v.applied_recommendation_id = a.id)
                  and a.applied_at + a.measurement_window_days * interval '1 day' < now()" -d sensei
# expected: 0 (or a small number if the tick is behind)
```

## Wrong gate

- **Applied recommendation sits without a verdict past its
  window.** `MeasureVerdicts` isn't scheduled or is failing
  silently. This is a recurring [[pipeline/analyzer]] regression
  ("`MeasureVerdicts` never enqueued").
- **Positive verdicts are logged but the underlying memory's
  `reinforced` doesn't advance.** Feedback loop broken; the
  measurement is decorative.
- **Every verdict is `positive`.** Delta rule inverted or
  denominator wrong; check the `>= +0.05` bound.
- **Regression alerts don't reach the Impact › Regressions nav
  entry.** The safety-screen story was the whole point.
- **Impact chart before/after values disagree with the raw
  `before` / `after` JSON.** Chart derivation is buggy — the
  chart should read the same numbers.
- **`insufficient_data` runs re-schedule forever without ever
  succeeding.** Cap on retries missing; a stale applied
  recommendation should eventually be closed as inconclusive.
- **Revert on a regression doesn't archive the memory.** Revert
  action is decorative.
- **Verdict `note` reads "measurement complete" every time.**
  Insight-copy fallback fired for every case; the model isn't
  being reached.

## Related

- [[pipeline/insights]] — where applied recommendations come from
- [[pipeline/memory]] — reinforcement / challenge feedback loop
- [[pipeline/analyzer]] — schedules `MeasureVerdicts`
- [[pipeline/ftr]] — before/after snapshots read from
  `sensei.project_ftr_metrics`
- [[pipeline/narration-cache]] — verdict notes
- [[screen/observatory-impact]] — primary consumer (includes
  the Impact › Regressions alert state)
- [[screen/project-impact]] — project-scoped verdicts view
- [[screen/observatory-today]] — surfaces unacknowledged
  regression banners
