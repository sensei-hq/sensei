# 録 · Observatory · Logs

**Segment:** 03 · Observatory — daily use
**Route:** `/logs`
**Source mockup:** [`lib/project-logs.jsx`](../../mockups/Sensei/lib/project-logs.jsx) → `ObsLogs`
**App file:** `app/src/routes/(observatory)/logs/+page.svelte`

## Purpose

Logs is the operator surface — daemon activity, background tasks,
scheduler heartbeats, capture events, adapter status. Not for
everyday use; for **debugging** when something looks off. The
user opens this when the numbers on Today don't correlate or a
signal isn't updating.

Kanji is 録 — *record*.

## Data invariants

- `GET /api/logs?level=…&source=…&since=…` returns log rows.
- Log rows carry `ts_ms`, `level` (`trace|debug|info|warn|error`),
  `source` (`scanner|watcher|analyzer|scheduler|capture|adapter:{name}|api|mcp`),
  `message`, `payload` jsonb.
- **Background task visibility** (user's ask from prior
  session): a strip at the top lists scheduled tasks — each with
  its schedule, last run, next run, and last outcome.

## Signals shown

| Element | Value |
|---|---|
| Task strip (top) | scheduled task list · last-run / next-run / last-outcome |
| Level filter | trace / debug / info / warn / error |
| Source filter | scanner / watcher / analyzer / scheduler / capture / adapter / api / mcp |
| Time-range chip | 15m / 1h / 24h / all |
| Log row | ts (relative) · level chip · source chip · message · expand for payload |
| Search | full-text over message |
| Follow toggle | auto-scroll to newest |

## Done gate

- Task strip shows every scheduled task (`AnalyzeProject`,
  `MeasureVerdicts`, `AggregateToolInsights`,
  `AggregateCorrections`, `DetectCommunities`); last-run and
  next-run populate from the scheduler state (see
  [[pipeline/analyzer]] watermark).
- Log rows stream in real time when Follow is on — a new
  scanner tick appears in the visible list within 2s.
- Level + source filters narrow correctly; combining `level=warn`
  with `source=scanner` shows only the intersection.
- Payload expand shows the raw jsonb without truncation.

Optional check:
```
curl -s "http://localhost:7744/api/scheduler/tasks" | jq
curl -s "http://localhost:7744/api/logs?level=info&since=1m" | jq 'length'
```

## Wrong gate

- **Task strip missing a task that's actually running.**
  Scheduler introspection incomplete.
- **Log rows arrive delayed by minutes.** Buffering issue.
- **Silent errors don't appear as `warn` / `error`.** See
  (memory: feedback_no_silent_errors) — this is the surface that
  reveals when the rule is violated.
- **Level filter set to `error` hides warns.** Should be
  inclusive-below or clearly labeled.

## Related

- [[pipeline/analyzer]] — scheduler state consumer
- [[pipeline/capture]] — scanner + watcher events
- (memory: feedback_no_silent_errors) — the failure mode this reveals
