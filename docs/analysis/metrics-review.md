# Analysis — Project metrics screens review

**Status:** in progress · **Owner task:** #22 (+ #20 redesign) · **Last updated:** 2026-08-11

Self-contained checkpoint from a live review of the project metrics screens
(dbd). Each item: symptom → root cause → course of action → exact files → status.
Chart-history shortness is largely the history-recovery problem —
see [history-recovery.md](./history-recovery.md).

## File map (so refs don't get mixed up)

**Frontend (SvelteKit)** — `app/src/`
- Pure mapper + types + tests: `lib/metrics/metric-view.ts`, `lib/metrics/metric-view.spec.ts`
- Shared sparkline: `lib/components/MetricSparkline.svelte`
- API client: `lib/api.ts` (`getProjectMetrics`, `getMetricsRegistry`,
  `getProjectMetricSeries`, `getProjectSessions`, `getSessionsDigest`, and the
  overview FTR `/api/projects/{id}/ftr`)
- Landing screen: `routes/(project)/project/[id]/metrics/+page.svelte` + `+page.ts`
- Detail (master-detail) screen: `routes/(project)/project/[id]/metrics/[key]/+page.svelte` + `+page.ts`
- Screen-local components (same `metrics/` dir): `HealthHero.svelte`,
  `MoverCard.svelte`, `SignalGridCell.svelte`, `SignalLegend.svelte`,
  `SignalRail.svelte`, `DetailChart.svelte`

**Daemon (Rust)** — `crates/senseid/src/`
- Metric computers: `tasks/handlers/metrics/{session_outcomes,churn,duplication,autonomy,knowledge,tool,health}.rs` + `mod.rs` (dispatch + `compute_health`)
  - **FTR, throughput, rework, quality** → `session_outcomes.rs`
  - **health composite** → `health.rs`
  - **tool relevance (N of M)** → `tool.rs`
- Read endpoints: `api/handlers/metrics.rs` (`get_project_metrics`,
  `get_project_metric_series` [grain], `get_metrics_registry`)
- **Overview FTR (the disagreeing source):** `/api/projects/{id}/ftr` — search
  `ftr14d` in `api/handlers/{project_detail,observatory,sessions}.rs`
- Grain roll-up **views**: `database/ddl/view/sensei/project_metric_{daily,weekly,monthly,quarterly}.ddl` + `project_metric_trend.ddl`
- Registry (definitions): `database/ddl/table/sensei/metrics.ddl`, seeded from `features/metrics/catalog.md`
- Narrative: `analysis/insight_copy.rs`, `analysis/metric_narrative.rs`

---

## A. Data-correctness bugs

### A1. FTR shows two different values
- **Symptom:** metrics grid FTR = 100% (`n` empty); overview = 91%
  (`/api/projects/{id}/ftr` → `ftr14d=0.909`).
- **Root cause:** two independent FTR computations. The metric is computed in
  `session_outcomes.rs`; the overview endpoint computes its own `ftr14d`. Windows
  and denominators differ; the metric row's `n` prop is empty (suspicious).
- **Course:** pick ONE canonical FTR (same window, same denominator) and have both
  the metric and the overview read it. Fix the empty `n` on the metric row.
- **Files:** `metrics/session_outcomes.rs`; the overview `ftr14d` handler
  (`api/handlers/project_detail.rs` / `observatory.rs`).

### A2. "Sessions in this period" is a fixed set of 3
- **Symptom:** the detail panel always shows the same 3 sessions; disagrees with the
  headline `n` (e.g. ttur headline `n=2` vs panel `3`).
- **Root cause:** the detail loader calls `getProjectSessions(id, 5)` (recent 5,
  sliced to 3) — **not** scoped to the selected metric's window/period.
- **Course:** scope the panel to the metric's actual period — either the series
  date-range or the metric window — via `getSessionsDigest(range, project)`; label
  it with that range; keep it consistent with the headline `n`.
- **Files:** `metrics/[key]/+page.ts` (loader), `metrics/[key]/+page.svelte`
  (sessions panel), `lib/api.ts`.

### A3. Chart point-counts inconsistent + some series stale
- **Symptom:** per-metric series differ (churn 8 pts from 7/28; ttur **4 pts ending
  8/04 — stale**; health/dup/rework/memory/unused 2 pts from 8/09).
- **Root cause:** metrics were activated on different days (real staggered data) and
  some stopped computing (ttur's last `computed_on` is 8/04). Short overall history
  is the history-recovery problem.
- **Course:** (a) historical snapshot computation (see history-recovery.md §5);
  (b) a **staleness guard** — surface/refresh metrics whose latest `computed_on`
  lags today, and investigate why ttur stopped (`session_outcomes.rs`).
- **Files:** `metrics/session_outcomes.rs`, `tasks/metrics_scheduler.rs`.

### A4. Grain toggle appears to filter, not bucket
- **Symptom:** weekly/monthly look like they filter to the current week/month rather
  than showing week-by-week / month-by-month across all history.
- **Root cause (to verify):** the roll-up **views** should bucket per period
  (Σnum/Σden). With only ~2 weeks of data, monthly = 1 bucket and weekly = ~2,
  which *reads* like "current period only". Confirm the views actually bucket the
  full history (not a WHERE-filter to now); the perception should resolve once
  history is backfilled.
- **Course:** verify `project_metric_{weekly,monthly,quarterly}.ddl` bucket over all
  periods; confirm `get_project_metric_series` passes grain through to the right
  view; re-check after historical backfill.
- **Files:** `database/ddl/view/sensei/project_metric_*.ddl`,
  `api/handlers/metrics.rs::get_project_metric_series`,
  `metrics/[key]/+page.ts` (grain param), `+page.svelte` (grain toggle).

---

## B. UI / layout

### B1. Grid top-rule too thick/bright in dark mode — **DONE**
- Changed `border-t-2` → `border-t` in `SignalGridCell.svelte`. (Colour tokens
  already flip; thickness was the issue.)

### B2. Large durations should read as days — **DONE**
- `formatDuration` in `metric-view.ts` now emits `d/h` above 24h
  (e.g. "2025m" → "1d 9h"). Applies wherever durations render.

### B3. Detail: rail + content must scroll independently — **pending**
- **Symptom:** long signal rail pushes content; half the metrics hidden on entry.
- **Course:** make the master-detail card fill height (`h-full flex flex-col`,
  card `flex-1 min-h-0`) and give the rail + content each `overflow-y-auto min-h-0`.
- **Files:** `metrics/[key]/+page.svelte`, `SignalRail.svelte`.

### B4. Per-page header locked + content scrollable (consistency) — **pending**
- **Symptom:** whole page scrolls; header should stay put.
- **Course:** page = `h-full flex flex-col`, header `shrink-0`, content
  `flex-1 overflow-y-auto`. Apply to metrics landing + detail; then adopt as the
  section-page pattern app-wide (ties to audit #21; `project/[id]/+layout.svelte`
  `<main data-component="project-main">` is `overflow-y-auto`).
- **Files:** `metrics/+page.svelte`, `metrics/[key]/+page.svelte`,
  `routes/(project)/project/[id]/+layout.svelte`.

---

## C. Metric semantics + new metrics (design decisions)

### C1. Throughput redesign — **DECIDED (2026-08-11): FTR-weighted turns**
- **Symptom:** throughput = sessions/day is counterintuitive (stop working → it
  drops). dbd = 1.
- **Decision:** output unit = **FTR-weighted turns** — credit turns/interactions
  weighted by first-time-right, normalized per session-*time* (per session-hour),
  so a short high-yield session scores well and churny rework doesn't inflate it.
  Ties throughput to the FTR north star (see C4).
- **Open (implementation):** confirm the per-turn FTR signal source (session `ftr`
  bool vs per-turn correction markers) and the time denominator (session
  `duration`, populated once C3/backfill land).
- **Files:** `metrics/session_outcomes.rs` (throughput lives here), `health.rs`
  (composite weight), `features/metrics/catalog.md`.

### C2. Total duration spent — **new metric**
- Add a `total_duration` metric (Σ session durations over the window) — straightforward
  once durations are available on sessions.
- **Files:** new computer under `metrics/` + `features/metrics/catalog.md` + registry.

### C3. Token usage metrics — **DECIDED (2026-08-11): backfill from transcripts**
- **Findings (verified 2026-08-11):**
  - `activity.assistant_events` has **no** token columns (payload is raw jsonb),
    and Claude Code's PreToolUse/PostToolUse/Stop hook payloads don't carry
    per-turn token counts — so the live event stream never had tokens to capture.
    ("Hooks were supposed to capture" is a mis-expectation; hooks can't.)
  - A session-level seam already exists but is **unfed**: `complete_session(…,
    tokens_in, tokens_out)` reads `body["tokensIn"]/["tokensOut"]` on the
    session-complete hook (`api/handlers/sessions.rs:236`), and `activity.sessions`
    already has `tokens_in`, `tokens_out`, `duration` columns.
  - Claude transcripts DO carry per-turn usage (`input_tokens`, `output_tokens`,
    `cache_creation_input_tokens`, `cache_read_input_tokens`), but
    `transcript/claude.rs` does **not** parse it today.
- **Decision:** transcripts are the authoritative source — **yes, backfill from
  transcripts.** Extend the parser to read each turn's `usage` block and sum it
  onto the session (`tokens_in`/`tokens_out` + a cache-tokens field) during
  synthesis; this backfills history AND fills the gap the live hook leaves.
  Follow-up: feed `tokensIn/Out` on the session-complete hook for live sessions.
  Then add token metrics (tokens/session, tokens/useful-result, cost).
- **Files:** `transcript/claude.rs` (parse `usage`), `transcript/mod.rs` (persist
  onto session at synthesis), maybe `database/ddl/table/activity/sessions.ddl` (add
  a cache-tokens column), new `metrics/` computer, `catalog.md`; follow-up on the
  session-complete hook script.

### C4. Health score → qlty.sh grades — **DECIDED (2026-08-11): dropped; FTR is the north star**
- **Decision:** do **not** integrate qlty.sh. FTR is the single north-star headline
  metric (throughput now derives from it — see C1). No qlty data path, no letter-
  grade badge UI.
- **Composite health score:** left as-is for now (secondary), not the headline;
  revisit only if it proves misleading in practice.
- **Files:** n/a (no work) — `HealthHero.svelte` keeps rendering the existing composite.

---

## D. Narrative prompt in the metrics table (agreed enhancement)

Move the **per-metric insight instruction** from code into the registry so it's
data-driven (no rebuild to tune), keeping all guardrails in code.

- Add a **nullable** `insight_prompt text` to `sensei.metrics`
  (`database/ddl/table/sensei/metrics.ddl`) + `features/metrics/catalog.md` per
  metric (author only where the generic prompt underperforms).
- Generator uses it as the `<task>` line; **fallback → the generic
  `MetricSignalInsight` instruction** when null (new metrics still work).
- Keep `VOICE_CHARTER`, `<limits>` (char caps, banned words), `<format>`,
  `voice_ok` validation, never-fabricate **in code** (`insight_copy.rs`).
- **Cache-invalidation:** include the prompt (or its hash) in the `facts` so a
  prompt edit changes `facts_hash` and re-warms — else stale copy is served.
- Headline prompt stays code (project-level, not per-metric).
- **Files:** `analysis/insight_copy.rs` (task_line/build_prompt),
  `analysis/metric_narrative.rs` (facts + prompt threading),
  `api/handlers/metrics.rs` (expose `insight_prompt` on the served row),
  `metrics.ddl`, `catalog.md`.

---

## Done this session (landed with this checkpoint)
- `SignalGridCell.svelte`: `border-t-2` → `border-t` (B1).
- `metric-view.ts` `formatDuration`: days tier (B2).
- Live DB: 3 folder-rename aliases + backfill triggered (see history-recovery.md).

Remaining UI (B3/B4 + A2) ships in the next metrics-UI batch.

## Suggested resume order
1. History recovery §5 (historical snapshots) + verify backfill — unlocks real charts.
2. A1 (FTR single-source) + A2 (sessions-in-period scoping) + A4 (grain verify).
3. B3/B4 (scroll + locked header) — batch with B1/B2.
4. C3 (token backfill from transcripts) + C1 (FTR-weighted throughput) — decisions
   locked 2026-08-11; C4 dropped (FTR is the north star, no qlty.sh).
5. D (prompt-in-table).
