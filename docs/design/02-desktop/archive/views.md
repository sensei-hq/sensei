---
name: Desktop Views
description: Dashboard pages needed in the Tauri desktop app for quality analysis, pattern management, and guided coaching
date: 2026-04-17
type: design
traces:
  - ideas/10-visualization.md
  - ideas/07-metrics-analytics.md
  - ideas/15-pattern-store.md
  - blueprints/02-system-architecture.md
---

# Desktop Views

## Overview

The desktop app (Tauri + SvelteKit) exists with basic views. It needs quality dashboards, pattern management, event logs, and guided coaching views. All data comes from daemon HTTP API — desktop is read-only for workflow state.

---

## Views to build

### Quality dashboard

- **FTR trend** — line chart over time (7d, 30d, all)
- **Turn count per task** — histogram, average trend
- **Rework rate** — percentage trend
- **Tool adherence** — MCP vs built-in usage ratio
- **Locate accuracy** — files identified vs files modified match rate
- Drill into any metric → filtered event log

**Data source:** `GET /api/metrics/:proj?range=30d`

### Phase timeline

- Visual timeline of phase transitions with duration bars
- Click a phase → see events within that phase
- Current phase highlighted

**Data source:** `GET /api/phases/:proj`

### Event log

- Filterable by event type, date range, issue
- Each event expandable to show full JSON data
- Correction events highlighted (for FTR analysis)

**Data source:** `GET /api/events/:proj?limit=50&type=&since=`

### Pattern catalog

- All detected patterns with instance count and conformance status
- Click pattern → detail view: interface, invariants, instances, recent violations
- Pattern coverage heatmap: files in patterns vs ad-hoc
- Duplication clusters with suggested extractions

**Data source:** `GET /api/patterns/:proj` (new endpoint from pattern detection)

### Guided coaching

- System-generated recommendations based on metric trends
- "FTR dropped in module X — common correction: missing Y pattern"
- Actionable: "Add guardrail" button, "Generate prompt" button
- Prompt preview: copy-pasteable prompt for next coding session

**Data source:** `GET /api/metrics/:proj` + event analysis logic

### Active work

- Current phase, task, issue from workflow state
- Links to GitHub issue
- Remaining issues in current milestone/wave

**Data source:** `GET /api/state/:proj`

---

## Testing

Desktop views tested via Playwright against dev server with seeded test data.
