# Analyzer & Signals · M7

**Plane:** insight · **Status:** [roadmap/02-insight.md](../../../../roadmap/02-insight.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

The analyzer is what turns raw captured activity into the numbers and copy every
screen depends on. Without it, `sensei.projects` has rows but nothing to say about
them — no FTR, no signals, no memories, no model preference. With it, every session
that lands enriches the project, and every project that grows updates its downstream
signals — Today's "one thing", the health-signal cards, tool insights, the
recommendation queue, and the memory pipeline all wake up. The scheduler runs on a
persisted per-project **watermark**: genuinely new activity is due, nothing else, and
a daemon restart never re-analyzes the world. A daily full refresh keeps the
time-based insights fresh. The user feels this module as *sensei seems to know*, the
day after setup.

## In scope / out of scope

- In: the scheduler loop (incremental tick + daily full refresh, watermark persisted
  in `sensei.config`); `analyze_project` enrichment (turns · corrections · FTR ·
  outcome, written atomically); the derived surface (FTR at four granularities,
  rework hotspots, pattern-adherence correlation, god-node detection, module
  correction clusters, recommendation candidates L2); `AggregateToolInsights` health
  signals with the per-MCP curation contract; the narration-cache copy pipeline that
  voices every derived card.
- Out: the FTR **metric aggregation** store (`sensei.project_metrics`) — M8 owns the
  registry, the store, and the health score; the analyzer writes `sessions.ftr`, the
  metric computer rolls it up. Memory formation/reinforcement is M9; pattern
  detection *machinery* is M10; drift scanning execution is M11. Insights *surfacing*
  (the Today/Insights UI) is the Delivery plane.

## Surfaces

- **Daemon** — `tasks/analyzer_scheduler.rs` (long-lived tokio scheduler),
  `tasks/handlers/analyze.rs` (enrichment + `derive_signals`),
  `tasks/handlers/tool_insights.rs` (aggregate insights writer),
  `tasks/handlers/generate.rs` (recommendation candidates),
  `tasks/handlers/consolidate.rs` (memory consolidation hookup).
- **API** — `GET /api/observatory/tool-signals`; `GET /api/tasks?status=queued`;
  `GET /api/projects/{id}/ftr` / `/api/observatory/ftr` (read the M7-derived
  `sessions.ftr` line through the M8 store).
- **App** — Today (FTR chip + sparkline + insights), Insights triage, Health L1/L2,
  memory adopted lane, communities view.

## Data invariants

- **Watermark.** `sensei.config['analyzer.watermark']` holds `{project_uuid:
  rfc3339_ts}`; a project is due iff its latest `activity.sessions.started_at` >
  watermark. Enqueueing **advances the watermark** in the same pass — a re-run with
  no new activity is a no-op. Watermark persisted each tick; malformed entries are
  dropped, not fatal.
- **Full refresh** fires when `now - last_refresh > full_refresh_secs` (default 24h),
  and only then — every `AnalyzeProject` + a `DetectCommunities` per indexed folder.
- **Atomic enrichment.** No session ever has `analyzed_at IS NOT NULL` AND
  (`ftr IS NULL` OR `corrections IS NULL`). A failed `enrich_session` leaves
  `analyzed_at` NULL so the next tick retries.
- **Downstream best-effort.** `generate`/`consolidate`/`model_insight` failures are
  `tracing::warn!` with the project id — never fatal to the tick.
- **Scoped signal derivation.** `derive_signals` only touches `affected` folders —
  `inference.detected_patterns` must not grow unbounded on every tick.
- **`sensei.tool_usage_stats`** — one row per tool ever called (`call_count` ·
  `error_count` · `avg_duration_ms` · `last_used_at` · derived `error_rate`).
  `sensei.tool_insights` is **append-only**, read via `DISTINCT ON (tool_name)
  ORDER BY tool_name, computed_at DESC`.
- **Wire contract.** `GET /api/observatory/tool-signals` →
  `{ signals: [{ tool_name, variant, title, detail, action? }], source: "derived" }`;
  `tool_name` is `""` for summary cards; `action` is an *omittable* JSON field
  (`Option::is_none` skip).
- **One thresholds struct.** `SignalThresholds::default()` is the single source for
  warn/opportunity/unused/win triggers — writer and endpoint must reach for the same.
- **No final copy authored here.** Every visible string goes through
  narration-cache (`title_max_chars` 60 / `detail_max_chars` 180); static text is the
  labelled `fallback:`, never the default.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Today FTR chip + sparkline | per-session FTR rolled into the M8 store; 14d trend inline | `ftr14d: 0.82`, arrow vs `ftr14dPrev` |
| Health signal cards (`warn`) | `call_count ≥ 50 && error_rate ≥ 0.05` — per-tool action card | "get-embedding: 61% failure rate" |
| Health signal cards (`opportunity`) | `call_count ≥ 10 && error_rate ≥ 0.05` | "search: room to improve" |
| Summary cards (`unused`/`win`) | >1 dormant/workhorse collapse to ONE card each | "5 tools dormant · {short}…" |
| Today insights / adopted | recommendation queue + memory adoption lane | "promote pattern?" · "adopted: use dbd for migrations" |
| Community view | `DetectCommunities` daily full-refresh only | cluster colouring in Atlas |
| Model preference chip | `model_insight` per-project | "Opus 4.6 chasing 0.82 here — try it" |

## Done gate

- On a warm daemon with new sessions since the last tick, exactly those projects are
  enqueued — not all. With no new sessions, **nothing** is written per tick until the
  daily full-refresh window opens.
- After restart, the watermark is restored; only projects newer than the persisted
  watermark queue (pinned by `analyze_project_enriches_sessions_and_writes_turns`
  and the "unchanged session skipped" tests).
- `GET /api/observatory/tool-signals` on a ≥20-tool registry where most are dormant
  returns ≤4-5 entries, at most one `unused` and at most one `win`, sorted
  warn → opportunity → unused → win.
- Every signal card title carries the tool's short name — never the verbatim fallback
  
  ("Dormant tool").
- `Sessions` never carry `ftr = null` on a session with `analyzed_at IS NOT NULL`.

```bash
curl -s http://localhost:7744/api/observatory/tool-signals \
  | jq '.signals | group_by(.variant) | map({variant: .[0].variant, n: length})'
# expected: warn/opportunity per-tool; unused/win at most one summary each
curl -s http://localhost:7744/api/tasks?status=queued | jq 'length'
```

## Wrong gate

- **Every tick re-analyzes every project** — watermark not advancing.
- **Daemon restart re-analyzes the world** — watermark JSON broken on read.
- **`ftr14d` reads 0 despite FTR-true sessions** — `session_outcomes` never wrote the
  daily rows or the window filter misfired.
- **N dormant cards, one per tool** — curation regressed; raw `derive_signals` output
  leaked to the wire.
- **`inference.detected_patterns` grows unbounded** — `derive_signals` not scoped.
- **Session enriched but `ftr` NULL** — `enrich_session` failed silently.
- **A prompt like "actually let's keep it" tanks FTR** — `correction_signal` is
  precision-favouring; add it to the false-positive regression list.
- **A new assistant that emits no UserPromptSubmit shows FTR = 1.0 always** — no
  signal must read `ftr = null` with a UI note, not `true`.

## Scenarios

- [`scenarios/analyzer.feature`](scenarios/analyzer.feature) — scheduler,
  enrichment, FTR derivation, scoping.
- [`scenarios/health-signals.feature`](scenarios/health-signals.feature) — tool
  signals, curation, copy contract.

## Open gaps

- No M7 rows in [06-gaps.md](../../../../roadmap/06-gaps.md) today; the plane's P1s
  are M8's (M-7/M-8/M-9) and the unbuilt feeds M10 and #156 note. Watch: the "low
  signal" chip on thin projects (median turns < 3) and per-model FTR decision aids
  are specified but not shipped.

## Related

- [spec/pipeline/analyzer](../../../../spec/pipeline/analyzer.md) — scheduler +
  enrichment detail (legacy depth, folds here).
- [spec/pipeline/signals](../../../../spec/pipeline/signals.md) · [spec/pipeline/ftr](../../../../spec/pipeline/ftr.md) —
  tool-signal derivation, FTR precision rules.
- [spec/pipeline/narration-cache](../../../../spec/pipeline/narration-cache.md) — the
  mentor-voice copy pipeline (cross-module: M9 adoption blurbs, M11 drift copy,
  M19 koan + insight copy).
- [spec/pipeline/memory](../../../../spec/pipeline/memory.md) (consolidate hookup) ·
  [spec/pipeline/impact](../../../../spec/pipeline/impact.md) (`MeasureVerdicts`).
- [features/03-observatory.md](../../../../features/03-observatory.md) — the
  observatory data half (capture → map → analysis → knowledge) and section statuses.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/tasks/{analyzer_scheduler.rs,handlers/*}`;
  `crates/senseid/src/api/handlers/tool_signals.rs`.