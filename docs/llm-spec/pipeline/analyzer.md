# 分 · Pipeline · Analyzer scheduler + enrichment

**Owner files:**
- `crates/senseid/src/tasks/analyzer_scheduler.rs` — the long-lived tokio scheduler
- `crates/senseid/src/tasks/handlers/analyze.rs` — the per-project handler (L0 enrichment + L1 signal derivation)
- `crates/senseid/src/tasks/handlers/tool_insights.rs` — the aggregate insights writer
- `crates/senseid/src/tasks/handlers/generate.rs` — recommendation generation (L2)
- `crates/senseid/src/tasks/handlers/consolidate.rs` — memory consolidation

**Task kinds involved:** `AnalyzeProject`, `AggregateCorrections`, `MeasureVerdicts`, `AggregateToolInsights`, `DetectCommunities`

## Purpose

The analyzer is what turns raw captured activity into the numbers and
copy every screen depends on. Without it, `sensei.projects` has rows
but nothing to say about them — no FTR, no signals, no memories, no
model preference. With it, every session that lands enriches the
project, and every project that grows updates its downstream signals.

The scheduler is deliberately **incremental** — a persisted per-project
watermark ensures a daemon restart doesn't re-analyze the world.
Genuinely new activity is due; nothing else is. A daily full refresh
guarantees time-based insights (maturity, effectiveness, ranking,
communities) stay fresh even for projects that had no new sessions.

Kanji is 分 — *to divide/analyse*.

## Data invariants

### The scheduler loop

- Wakes every `DEFAULT_INTERVAL_SECS` (3600s = 1h).
- Reads the watermark from `sensei.config['analyzer.watermark']`
  (JSON: `{project_uuid: rfc3339_ts}`).
- Reads the last-refresh timestamp from
  `sensei.config['analyzer.last_full_refresh']`.
- Two paths:
  - **Incremental**: `projects_due(activity, &mut watermark)` returns
    projects whose latest `activity.sessions.started_at` > watermark;
    enqueues `AnalyzeProject` for each; advances watermark.
  - **Full refresh**: if `now - last_refresh > full_refresh_secs`
    (default 86400s = 24h), enqueues `AnalyzeProject` for every
    active project + `DetectCommunities` per indexed folder;
    updates `last_refresh`.
- After either path, if ≥ 1 project was queued, runs two global
  aggregators: `AggregateCorrections` (cluster corrections across
  projects) and `MeasureVerdicts` (before/after FTR on accepted
  recommendations).
- `AggregateToolInsights` runs on the same tick as `AnalyzeProject`
  so per-tool signals stay in sync with the DDL persistence.

### `analyze_project` handler

1. Reads project's sessions via `list_project_sessions(pid)`.
2. For each session where `analyzed_at IS NULL` OR the session events
   have changed:
   - Runs `enrich_session(pid, sid)` → walks assistant events,
     computes turns, corrections, FTR, outcome, sets
     `sessions.{ftr, corrections, outcome, analyzed_at}`.
3. If any sessions were enriched (or `affected` is non-empty), runs
   `derive_signals(ctx, pid, affected)` — L1 — which produces
   `inference.detected_patterns` rows for the affected folders.
4. Runs `generate` (recommendation candidates), `consolidate`
   (memory promotion), `model_insight` (per-project model
   preference), `rank` (ordering).
5. Errors from downstream steps are **logged but not fatal** — the
   session enrichment must land; the downstream is best-effort.

### Watermark math (why it matters)

- `activity` list is `[(project_id, MAX(sessions.started_at))]` per
  project.
- A project is "due" iff `latest > watermark[pid]` OR
  `watermark[pid]` is missing.
- `projects_due()` **mutates the watermark** so a re-run without new
  activity is a no-op. This is the source of incrementality.
- Restart safety: watermark persisted after each tick; malformed
  entries in the JSON are dropped (not fatal).

## Signals produced

The analyzer doesn't produce user-facing signals directly — it
populates the tables the screens read:

| Screen / endpoint | Table / view read | Analyzer step that fills it |
|---|---|---|
| Today FTR chip | `sensei.project_ftr_metrics` view | `enrich_session` writes `sessions.ftr` |
| Today insights | `inference.recommendations` | `generate` |
| Today adopted | `sensei.memories` | `consolidate` |
| Health signals | `sensei.tool_insights` | `AggregateToolInsights` |
| Model preference | `inference.model_insights` | `model_insight` |
| Project signals | `inference.detected_patterns` | `derive_signals` |
| Community view | `inference.communities` | `DetectCommunities` (daily full-refresh only) |
| Impact verdicts | `sensei.impact_verdicts` | `MeasureVerdicts` |

## Done gate

- On a warm daemon with new sessions since the last tick, the
  scheduler enqueues `AnalyzeProject` for exactly those projects
  (not all).
- On a warm daemon with **no** new sessions, the scheduler does
  nothing per tick — no queue writes, no aggregator runs — until
  the daily full-refresh window opens.
- After a daemon restart, the watermark is restored and only projects
  with activity newer than the persisted watermark are queued
  (verified by `analyze_project_enriches_sessions_and_writes_turns`
  + `unchanged session is skipped` tests).
- The full-refresh timestamp advances by ≈ `full_refresh_secs` each
  daily pass; not per tick.
- No session ever ends up with `analyzed_at IS NOT NULL` AND
  (`ftr IS NULL` OR `corrections IS NULL`) — enrichment writes them
  atomically.
- Downstream failures (generate / consolidate / model_insight)
  produce a `tracing::warn!` with the project id and error, but do
  not block the tick.

Optional check:
```
# what's in the queue right now?
curl -s http://localhost:7744/api/tasks?status=queued | jq 'length'

# when did the scheduler last do a full refresh?
psql -A -t -c "select value from sensei.config where key='analyzer.last_full_refresh'" -d sensei

# is any active project un-analyzed?
psql -A -t -c "select count(*) from activity.sessions
                where analyzed_at is null and started_at > now() - interval '7d'" -d sensei
# expected: small; if large, the scheduler isn't catching up
```

## Wrong gate

- **Every tick re-analyzes every project.** Watermark not persisting
  or `projects_due` not advancing it.
- **Daemon restart re-analyzes the world.** Watermark serialization
  broken (JSON invalid on read).
- **`AggregateToolInsights` never runs.** Not wired into the tick,
  or gated on a condition that never fires.
- **`MeasureVerdicts` never enqueued.** Regression of the fix that
  scheduled it in the first place. Verdicts staying empty is the
  read-side symptom.
- **`inference.detected_patterns` grows unbounded.** `derive_signals`
  is not scoped to `affected` folders — every tick re-runs the
  churn/correction rollup over the whole project.
- **Session enriched but `ftr` still NULL.** `enrich_session` failed
  silently — should surface as a warn log and leave `analyzed_at`
  NULL so the next tick retries.
- **Downstream step's warning suppresses the overall handler
  result.** `analyze_project` returns Ok(0) when actually enrichment
  happened — a "count enriched" gap that hides progress from the UI.
- **Full-refresh runs on every tick.** Compare on `full_refresh_secs`
  is inverted; last_refresh never advances.

## Related

- [[pipeline/capture]] — feeds `activity.sessions` and `activity.assistant_events`
- [[pipeline/ftr]] — depends on `enrich_session` for `sessions.ftr`
- [[pipeline/signals]] — depends on `AggregateToolInsights`
- [[pipeline/memory]] — depends on `consolidate`
- [[pipeline/impact]] — depends on `MeasureVerdicts`
- [[pipeline/insights]] — depends on `generate` + `rank`
- [[screen/observatory-today]] — the flagship consumer of every table
