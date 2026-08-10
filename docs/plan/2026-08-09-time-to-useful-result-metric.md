# Plan — `time_to_useful_result` metric

> Status: **IMPLEMENTED 2026-08-10** with definition (B) (see §2). Registry seed,
> computer (`session_outcomes`), `duration` roll-up arm, and tests all landed;
> verified read-only against live data. Deploy note in §6.
> Created 2026-08-09. Emerged from a metrics discussion (three candidate metrics);
> two are derivable and dropped, this one is genuinely new. See §5 for the other two.

## 1. Why this metric

Nothing we ship measures **latency / speed-to-value**. FTR is a *rate*, throughput a
*count* — neither answers "how long until the user got something usable." It is **not
derivable** from existing metrics but **is computable** from data we already store
(`activity.sessions.started_at` + per-turn timestamps in `activity.turns`).

- family: `velocity` (enum value exists)
- type: `duration` (enum value exists — no new enum needed)
- direction: `lower_better`
- unit: `seconds` (roll-ups take median; see §3)
- task_name / computer group: **`session_outcomes`** (it already reads
  `activity.sessions` per project over the rolling window — this metric slots in
  beside `ftr` / `rework_ratio` / `throughput`).

## 2. Open design decision — what "useful result" means

`activity.sessions` has **no `ended_at`** (confirmed: only `started_at`, `outcome`,
`ftr`, `corrections`). `activity.turns` has `started_at`, `ended_at`, `duration`,
`is_correction`. Two viable definitions — pick one before implementing:

- **(A) Session wall-clock (cheapest, recommended default).** For each *completed*
  session (`outcome IS NOT NULL`): `max(turns.ended_at) − sessions.started_at`.
  Median over the window. Fully backed by current data.
  *Caveat:* inflated by idle time (user steps away mid-session).

- **(B) Time-to-first-useful-turn (truer to the name).** Session start → `ended_at`
  of the **first non-correction turn** (`turns.is_correction = false`). Measures
  speed to the first usable output, not total session length. Slightly more SQL;
  needs a per-session "first useful turn" pick.

Recommendation was **(A)** first; **Jerry chose (B)** — truer to "useful result". The
shipped computer implements (B): the inner `JOIN LATERAL ... WHERE is_correction =
false ORDER BY turn_number LIMIT 1` picks the first usable turn; sessions whose only
turns are corrections (or that have no turns) produce nothing and are excluded — never
a fabricated 0.

## 3. Implementation steps (TDD)

1. **Registry seed** — add a row to `database/import/staging/metrics.jsonl`
   (key `time_to_useful_result`, family `velocity`, type `duration`, direction
   `lower_better`, unit `seconds`, task_name `session_outcomes`, plus
   `name`/`description`/`purpose`/`how_to_read`/`formula`, `weight` default,
   optional `target`). Re-import via the staging `import_metrics` procedure (dbd
   import — timestamp-guarded, non-destructive). Confirm it lands in
   `sensei.metrics` as active.
2. **Computer** — in `crates/senseid/src/tasks/handlers/metrics/session_outcomes.rs`
   add a `daily_time_to_useful` query mirroring `daily_session_aggregates`:
   ```sql
   SELECT date_trunc('day', s.started_at)::date AS day,
          percentile_cont(0.5) WITHIN GROUP (
            ORDER BY EXTRACT(EPOCH FROM (te.max_end - s.started_at))
          ) AS median_seconds
     FROM activity.sessions s
     JOIN sensei.folders f ON f.id = s.folder_id           -- project scope (as existing)
     JOIN LATERAL (SELECT max(t.ended_at) AS max_end
                     FROM activity.turns t
                    WHERE t.session_id = s.id
                      AND t.ended_at IS NOT NULL) te ON true
    WHERE f.project_id = $1
      AND s.outcome IS NOT NULL
      AND s.started_at >= now() - make_interval(days => $2::int)
      AND te.max_end IS NOT NULL
    GROUP BY 1
   ```
   (Definition (B) instead filters to the first `is_correction = false` turn.)
   Write daily rows via `PgStore::upsert_project_metric` under the existing
   key→metric_id resolution. **Never-fabricate:** a day with no completed
   sessions writes no row (honest-empty), and every DB call propagates `Err` —
   match the module's existing contract (see its header).
3. **Roll-up** — `duration` type: confirm the `project_metric_*` views aggregate
   `duration` sensibly. `metric_type` comment says count/currency sum, value/score
   take period-end; **`duration` is not listed** — check the view CASE arms and add
   a `duration` arm (median or period aggregate) if missing. This is a real gap to
   verify, not assume.
4. **Tests (TDD)** — extend `session_outcomes` tests: seed completed sessions with
   known `started_at` + turns with known `ended_at`, assert the median equals the
   hand-computed value; seed an in-flight session (`outcome IS NULL`) and assert it
   is excluded; assert honest-empty when no completed sessions.
5. **Read path** — generic; the registry/project/series endpoints already serve any
   active metric. Confirm `time_to_useful_result` appears in `GET /api/metrics`
   registry and `GET /api/projects/{id}/metrics` after a compute run.
6. **project_health weight** — decide whether it contributes to the composite
   `project_health` score (it has a `weight` column). Default: `weight = 0`
   (surfaced but not scored) until validated, to avoid perturbing the composite.

## 4. Verification (done-gate)

- Registry row active; a `ComputeMetrics` run for a project writes
  `time_to_useful_result` daily rows to `sensei.project_metrics`.
- Series endpoint returns a median-seconds series; value matches a manual
  `percentile_cont` query on the same window (verify against live data, not just
  the code path).
- Unit tests green (median math, in-flight exclusion, honest-empty).

## 5. The other two candidates — decided NOT to add (derivable)

- **Accuracy improvement** → don't add. "Accuracy" ≈ FTR (or 1−rework_ratio);
  "improvement" = its trend/delta, already computed by `project_metric_trend`
  (prior + delta per window) and by the recommendation impact loop
  (`inference.recommendations.baseline_ftr → current_ftr → verdict`). Surface the
  existing FTR trend rather than a new metric.
- **Revisions needed** → don't add. Already the corrections/rework family:
  `sessions.corrections` (raw), `rework_ratio`, `rework_density`, and FTR itself
  (= zero-revision rate). A `corrections_per_session` mean would be cosmetic and
  redundant with `rework_ratio`.

## 6. Deploy status (as of 2026-08-10)

Landed in code + tested; `sensei_test` seeded and the 3 roll-up views applied there.
**Not yet live in the running daemon.** To activate in production:
1. `dbd import metrics` against the live `sensei` DB (loads the new `metrics.jsonl`
   row via `import_metrics()` — timestamp-guarded upsert) and apply the 3 updated
   `project_metric_*` views.
2. Redeploy the daemon (`make install-service`) so the `session_outcomes` computer
   ships the new query. The daily metrics scheduler then backfills on its next tick.
Verified read-only against live data before deploy (kavach ~2367s, sensei ~711s,
rokkit ~37s median first-useful latency — plausible).
