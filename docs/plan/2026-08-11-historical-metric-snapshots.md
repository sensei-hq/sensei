# Plan — Historical metric snapshots (unified backfill + incremental)

**Status:** planned · **Owner:** #22 (metrics) / history-recovery #23 · **Created:** 2026-08-11
**Analysis:** [../analysis/history-recovery.md](../analysis/history-recovery.md) §5,
[../analysis/metrics-review.md](../analysis/metrics-review.md) A3/A4.

## Goal

Project-metric charts render **months** of history, not ~2 weeks — and the same
mechanism serves both the **first-install full backfill** and the **daily
incremental** run. No separate one-off backfill path: backfill and incremental are
the same operation applied to different day-sets.

## The model (agreed 2026-08-11)

Every metric value is computed **for a specific `computed_on` day** and upserted
idempotently per `(metric_id, project_id, folder_id, session_id, computed_on,
grain)`. The persistence + roll-up views are **already** date-parameterized (the
write takes `computed_on: NaiveDate`; the daily/weekly/monthly views bucket on
`computed_on` with no recent-window filter). The only backfill blockers are the
source-row filters (`>= now() - window`) and the snapshot day source
(`SELECT current_date`).

Three processors:
1. **Transcript importer** (EXISTS — `BackfillTranscripts` → `BackfillTranscriptFile`):
   synthesizes sessions/events with historical `started_at`/`ts`.
2. **Timeline → day-task planner** (NEW): per project, derive the data-day span from
   the feed and diff it against the `computed_on` days already present; enqueue one
   `ComputeMetrics{as_of=D}` per **(project, group, missing/stale day)**. First
   install → all days (backfill); daily → just today (incremental); gaps self-heal.
3. **Per-day computer** (MODIFY the 6 computers + health): a task computes its target
   day `as_of` and upserts `computed_on=as_of`.

## Honesty constraints (firm — never fabricate)

- Anchor day = **true occurrence time** (`sessions.started_at`, event client `ts`,
  `runs.started_at`, `task_executions.started_at`), NEVER `created_at` (insert-time
  = now for synthesized rows). Event-anchored computers keying `created_at` switch to
  `ts`.
- **Backfill source per metric, honestly** — a metric backfills as far as *its
  source's real timestamps* reach, and the source (and its cost) differ by metric:
  - **Activity/event feed** (cheap): session/event-derived metrics from
    `sessions.started_at` / event client `ts` / `runs.started_at`. Delivers the
    months-spanning delivery charts (FTR/throughput/ttur) — dbd back to 2025-06.
  - **Git commit history** (medium): churn is fundamentally files-changed-over-time —
    derive `churn_rate`/`churn_concentration`/`rework_density` from `git log
    --name-only` bucketed per day (true historical churn), NOT the `task_executions`
    indexing proxy (whose timestamps are only recent).
  - **git worktree @ past-date commit + `qlty` scan** (code-quality family — reuse,
    don't reinvent): for each sampled past date D, `git worktree add` a detached
    worktree at the commit as-of D, run a `qlty` scan, and capture
    **maintainability, duplication, coverage** scores → upsert with `computed_on=D`,
    then remove the worktree. Do NOT build a custom time-travel indexer; qlty already
    computes these. Sampled cadence (weekly / per-release / per-N-commits), NOT every
    day, to bound scan cost.
    - *Reconciles with C4:* C4 dropped qlty as the **north-star health grade** (FTR
      stays the headline). Using qlty here as a **historical data source** for a
      code-quality metric family is compatible — it's a supplementary family, not the
      headline. Supersedes the current own-graph `duplication_ratio`.
    - *Dependencies:* `qlty` CLI available (add to `sensei-bootstrap` prereqs); git
      worktree plumbing; a per-project git root that resolves a commit as-of a date
      (`git rev-list -1 --before=<D>`).
  - **Current inference state** (genuinely forward-only): the parts with no historical
    source — `memory_promotion`'s eligible denominator (current
    `detected_patterns`/`corrections`), tool-relevance registry. These accrete
    forward; the planner enqueues them **today-only**, and a computer that receives a
    historical `as_of` for a forward-only metric **skips** (no row) — never a
    fabricated historical value.
- Each metric backfills only as far as **its own source's real timestamps** reach.
  Honest per-metric horizons.
- Idempotent upsert; a day's compute failure propagates `Err` (task retries), never a
  fabricated value; honest-empty day → no row.

## Per-metric classification

| group | metric(s) | backfill source | mode |
|---|---|---|---|
| session_outcomes | ftr, throughput, rework_ratio, time_to_useful_result | activity feed — `sessions.started_at` | per-day ✅ (cheap; months-spanning win) |
| autonomy | interruption_rate | event `ts` (was `created_at`) | per-day ✅ (after anchor fix) |
| autonomy | run_completion | `runs.started_at` | per-day ✅ |
| churn | churn_rate, churn_concentration, rework_density | **git log `--name-only`** per day (was `task_executions` proxy) | per-day ✅ (medium) |
| code-quality (NEW) | maintainability, duplication, coverage | **git worktree @ past commit + `qlty` scan** | sampled cadence ✅ (heavy) — supersedes own-graph `duplication_ratio` |
| knowledge | memory_promotion | current inference state (no historical source) | forward-only |
| tool | unused_tools / relevance | event `ts` (usage) + current registry | usage backfillable; registry/relevance forward-only |
| health | project_health | derived from component daily rows | backfill deferred (once components have history) |

## Phases (forward-only; each ships green + independently testable)

### Phase 1 — `as_of` plumbing + session_outcomes per-day (the template)
- Add `as_of: Option<chrono::NaiveDate>` to `Task` (constructor/retry/`with_*`), and
  thread it through `ComputeMetrics` dispatch (`metrics/mod.rs::compute`) into the
  computer signatures. `None` = today (current incremental behavior preserved).
- Refactor `session_outcomes` day-keyed queries: when `as_of=Some(D)`, filter
  `date_trunc('day', s.started_at)::date = D` (single day) instead of the rolling
  window; emit `computed_on=D`. When `None`, keep the window (unchanged behavior).
- **TDD:** seed a session dated 60 days ago; assert a `computed_on = that day` daily
  row is written for `ftr`/`throughput` (RED first — today-only window omits it).
- Gate: `cargo test -p senseid` green; `cargo clippy` clean.

### Phase 2 — autonomy per-day (+ `created_at → ts` anchor fix)
- Apply the `as_of` single-day refactor to `autonomy`: `run_completion` (anchor
  `runs.started_at`) and `interruption_rate` — **switch `ae.created_at` → `ae.ts`** so
  synthesized events bucket into their true day.
- **TDD:** a historical-day source row → a `computed_on=that-day` row; seed a
  synthesized event with historical `ts` but `created_at=now` and assert it lands on
  the historical day (guards the anchor fix).
- (`churn` is NOT here — it changes *source* to git history, so it's Phase 7.)

### Phase 3 — forward-only guard
- The 5 forward-only computers: when `as_of` is `Some(D)` and `D != today`, **skip**
  (return 0, no row). `as_of=None`/today → unchanged snapshot behavior.
- **TDD:** calling a forward-only computer with a historical `as_of` writes 0 rows
  (never a fabricated historical snapshot).

### Phase 4 — the timeline → day-task planner (pure diff + enqueue)
- New `TaskKind::PlanMetricDays` (project id in `folder_path`). Handler: for each
  active group, read the project's **data-day set** (distinct
  `date_trunc('day', anchor)` over the group's source) and the **covered-day set**
  (distinct `computed_on` in `project_metrics` for the group's metrics); enqueue
  `ComputeMetrics{as_of=D}` for each `data − covered` day (plus today).
- Keep the day-set diff a **pure function** (tested without a DB), like
  `metrics_scheduler::base_task_names` / `next_watermark`.
- **TDD:** pure diff (data − covered = to-enqueue, incl. today, excl. settled days);
  enqueue test against a real `TaskQueue` (one task per missing day per group).

### Phase 5 — wiring: daily scheduler + first-install + on-demand
- The metrics scheduler enqueues `PlanMetricDays` per project (replaces the direct
  per-group enqueue) so the daily run is "plan → compute today's + any gap".
- First-install / boot: after the initial transcript backfill, enqueue
  `PlanMetricDays` for all projects once (full backfill). Guard with
  `queue.has_pending_kind`.
- On-demand endpoint `POST /api/metrics/backfill` (mirrors transcripts/backfill) for
  manual re-plan.
- **TDD:** scheduler enqueues one `PlanMetricDays` per project; the honest-empty +
  fail-closed watermark contract is preserved.

### Phase 6 — ship + verify against live data
- `zero-errors`: `cargo clippy` clean, `cargo test -p senseid` green, svelte-check 0
  (no FE change expected), atlas/vitest unaffected. Never merge on red.
- `make bump v=patch`; **`make install`** (full: service binaries + desktop `.app` —
  the charts live in the app, so a service-only overlay wouldn't surface them);
  stop-daemon only if a DDL change is needed (none expected — table/views already
  support `computed_on`).
- Trigger `PlanMetricDays` for dbd/torii/gateway; **verify against live data**: dbd
  daily `ftr` series `min(computed_on)` reaches 2025-06; torii reaches 2026-03; the
  metrics chart renders months (not ~2 weeks). Done = the rows exist AND the chart
  shows them, per the "done = verified against live data" rule.

## Notes / risks

- **Task volume:** first-install backfill bursts ~(data-days × day-keyed groups ×
  projects) tasks (dbd ≈ 430 × 2–3). Bounded, idempotent, and the queue already
  handled 303 transcript tasks. Steady-state = ~1 day/group/project/run. If the burst
  is a problem, the planner can chunk days per task — but per-day granularity is the
  requested design (resumable; one bad day can't block the rest).
- **No DDL change expected** — `project_metrics.computed_on` + `project_metrics_identity`
  + the roll-up views already key/bucket on `computed_on`. Confirm before shipping.
- **`Task.as_of` serialization:** if the queue persists tasks across restarts, include
  `as_of` in that (de)serialization so an interrupted backfill resumes.

## Post-build findings (2026-08-12) — BLOCKED before ship

Workflow `wf_c77252c3` built Phases 1–5 (5 stages, all modules report green + clippy
clean). NOT shippable yet — blockers:

- **BLOCKER 0 (root cause of the "vanished history") — retention vs. backfill.**
  The daemon's `activity_pruner` (`crates/senseid/src/tasks/activity_pruner.rs:56`)
  runs `prune_activity(activity.retention_days)` **daily**, default **30 days**,
  deleting analyzed sessions with `started_at` older than the window (+ their
  turns/events/transcript_turns). Backfilled sessions are historical-dated + get
  analyzed → the pruner reclaims them within ~a day. This is why `backfilled_total`
  went from ~18 → **0** and dbd/torii lost their recovered history. It is EXPECTED
  retention, NOT corruption and NOT the tests (tests hit a separate `sensei_test`
  DB). `prune_activity` touches only `activity.*` — **`sensei.project_metrics` is
  the durable history store and is untouched.**
  - **Implication (corrects item-1 + this plan):** session-level backfill is
    TRANSIENT. The durable recovery IS the metric snapshot. So the feature must
    **capture-before-reclaim**: compute a day's snapshots before the pruner deletes
    that day's sessions, else history is lost before capture (exactly what happened).
    Earlier "dbd → 2025-06 / torii → 2026-03, verified" was true at that instant but
    not durable — the pruner reclaimed it. Resolution pending (see the retention
    decision).

- **BLOCKER 1 (F1, HIGH — real regression the change introduced).** The per-day
  planner (`planner.rs`) forces only `today` and permanently excludes covered days,
  and enqueues today's compute as a SINGLE in-progress day. So each day's aggregate
  is computed once mid-day and then frozen — the current day converges to a
  partial-day sliver, and a session enriched after that day's first compute is
  dropped. The old `as_of=None` path recomputed the full rolling window daily, so
  recent days converged to their COMPLETE value. Fix: also recompute the last N
  (unsettled) days each pass, or gate "covered" on day-is-over (not day-touched).

- **BLOCKER 2 (cargo-gate RED).** `api::routes::tests::metrics_pipeline_end_to_end`
  fails deterministically (`unused_tools=3` expected, got 0). Build agents claim it's
  pre-existing at HEAD — **NOT yet independently confirmed; must verify per
  zero-errors before ship.** Also the full `cargo test -p senseid` HANGS (scheduler
  tests stuck >60s) — a test-infra/DB-contention issue to resolve so CI can gate.

- **DESIGN NOTE (Stage 5).** The planner became the COMPLETE per-project graph owner
  (day-keyed backfilled per-day; snapshot groups enqueued today-only; ComputeHealth
  barrier blocked on TODAY's computes only). Confirm this split is intended.

### Corrected Phase-6 verification targets (from live-data readiness, 2026-08-12)

The plan's original targets were wrong — corrected against live data:
- **torii** floor = **2026-07-17** (NOT 2026-03; the 2026-03/06 rows were the
  re-pointed sessions the pruner then reclaimed). Session-anchored ftr/throughput
  recover ~10 days (2026-07-17 .. 07-26).
- **gateway** session-anchored = **no-op** (floor 2026-07-30 already inside the
  window).
- **dbd** session-anchored = **no-op** (floor 2026-07-30). The REAL observable win
  for dbd is **`interruption_rate`** (event `ts` anchor) extending to **2026-07-04**
  (events exist back to 07-04; currently window-clipped at 07-28).
- **sensei** floor = 2026-07-13.
- Forward-only snapshots (duplication/health/etc.) stay at 08-09/08-10 (correct).
- NOTE: all of the above assume the sessions are **re-backfilled first** (they were
  pruned) AND captured into snapshots before the next prune (BLOCKER 0).

### Retention decision (2026-08-12): capture-before-reclaim (gate the pruner)

`prune_activity` gains a guard: a session is prune-eligible only when its day is
already captured in `sensei.project_metrics` for that project, OR it is older than a
hard backstop (`activity.capture_backstop_days`, default e.g. 2× retention) so
nothing lingers forever if metrics never compute. Existing prune tests must be
updated to reflect capture-before-reclaim. This makes backfilled history durable
regardless of prune/compute timing. Fix F1 (recompute last N unsettled days) +
confirm the pre-existing `metrics_pipeline_end_to_end` red before ship.

### BLOCKER 3 (live-verify 2026-08-12) — plan runs before data is measurable

Deployed v0.7.2+code and drove the live backfill. Sessions re-synthesized fine
(dbd → 2025-06-05, 179 backfilled), and the planner's data-day discovery is correct
(dbd: **16 measurable session-days + 30 event-days, both back to 2025-06**). BUT the
daily snapshots stayed clipped (dbd `ftr` floor 07-30, `interruption_rate` 07-28) —
**no historical days computed**. Root cause is ORCHESTRATION ORDERING, not logic:
- The boot sequence enqueues `PlanMetricDays` on a fixed **30s sleep** after the
  transcript backfill *starts* — but re-synthesis takes ~5 min, and `session_outcomes`
  days are only "measurable" after the **analyzer** sets `outcome`. So the plan ran
  before the data was plan-ready and enqueued only recent days.
- Manual `POST /api/metrics/backfill` re-triggers are **overlap-guarded**
  (`has_pending_kind(PlanMetricDays)`) → `enqueued=0`, so they can't force a fresh wave.
- The real dependency chain is **synthesizer → analyzer → PlanMetricDays** (a sleep is
  not a dependency; the queue's `depends_on`/`blocked_by` is the right tool). A trigger
  on *synthesis* completion still misses `ftr` (analyzer hasn't set `outcome`); it must
  hang on **analysis** completion.
- FIX (pending): drive `PlanMetricDays(project)` off **analysis completion** (guarded
  against storms), keeping the daily scheduler as the self-heal backstop. Then re-verify
  live. Holding bump/merge until historical snapshots land AND survive a prune cycle.

### BLOCKER 3 RESOLVED + live-verify status (2026-08-12)

Fixes landed (committed on develop): analysis-completion → PlanMetricDays trigger
(synthesizer→analyzer→metrics; enqueue_unique-guarded; boot 30s-sleep removed);
activity-pruner boot-tick delayed (capture beats reclaim; backstop kept). Deployed
via `make install`.

**Live verification (decisive):**
- ✅ MECHANISM PROVEN: `interruption_rate` (autonomy) durably backfilled
  **2025-06-05 → 2026-08-11 (31 daily points, 14 months)** — the per-day
  plan→compute→snapshot chain works end-to-end on live data AND survives prune
  (project_metrics untouched by the pruner).
- ✅ pruner-delay works: dbd's deep sessions survived (16 measurable session-days
  back to 2025-06, not re-pruned on boot).
- ⏳ `ftr`/`throughput` (session_outcomes) NOT yet showing months: NOT a logic bug.
  A PlanMetricDays(dbd) ran post-analysis (data_days=16, covered=6 → ~10 historical
  days planned) and enqueued the historical computes, but they are STARVED behind
  the boot code-graph re-index (`build_connections`/`extract_deps` monopolize the
  worker pool on this 130-project machine — no session_outcomes compute for dbd ran
  in 30 min). They land once the re-index drains; the daily scheduler +
  analysis-hook re-plan are the self-heal. session_outcomes' data-days require the
  analyzer's `outcome` (autonomy's event-ts days don't), which is why interruption
  won its computes in an earlier, less-contended window.

**FOLLOW-UP (operational, pre-existing, not this feature's defect):** the boot
re-index starves metric computes → historical backfill is slow-to-appear on a large
install and re-delays on every restart. Consider giving `ComputeMetrics`/
`PlanMetricDays` priority over bulk indexing, or deferring the boot re-scan. This is
a queue-priority change (affects everything) — raise separately, do not fold in here.

### Retention window + vacuum (2026-08-12, user request)

- **`activity.retention_days` default 30 → 90** (`activity_pruner.rs`
  `DEFAULT_RETENTION_DAYS`). Consequence: `capture_backstop_days` default
  = `max(90, 2×retention)` = **180**. No live config override exists, so the new
  default takes effect on redeploy. Longer retention also widens the durable-session
  window, easing capture-before-reclaim timing.
- **Daily `VACUUM (ANALYZE)` after the prune.** After the activity prune tick
  commits, run `VACUUM (ANALYZE)` on the tables the prune deletes from
  (`activity.sessions`, `activity.turns`, `activity.assistant_events`,
  `activity.transcript_turns`) to reclaim dead tuples + refresh planner stats after
  the bulk delete. Constraints: **runs OUTSIDE a transaction** (VACUUM cannot run in
  one — a dedicated autocommit connection/statement); **plain `VACUUM (ANALYZE)`,
  NEVER `VACUUM FULL`** (FULL takes ACCESS EXCLUSIVE + rewrites → blocks the daemon).
  Autovacuum still runs; this is an explicit post-bulk-delete reclaim, once per day
  after the prune ("after all processing").
- SEQUENCING: apply both AFTER the in-flight finalize agent completes — it currently
  owns `activity_pruner.rs` + `pg_store.rs::prune_activity`; editing them concurrently
  would conflict.
