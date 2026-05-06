# 03 — Observatory (Project Dashboard)

> Route: `/p/{id}` — the core of the desktop app.

## Purpose

This is the answer to "How is this product doing?" Aggregated metrics across all repos in a project. Shows quality trends, recent sessions, system topology, and quality signals. Surfaces problems and connects them to actions.

This page is what makes sensei more than an MCP server. Without it, sensei provides tools. With it, sensei provides insight.

## What the User Sees

### Metric Cards (top row)

```
Sessions    FTR         Turns/Session    Tokens
  24        82% ▲         4.2 avg          2.1M
  +6        was 70%       -0.8 ▼           -400k ▼
```

Time range selector: this week / 7d / 30d / all.

- **Sessions** — how many AI coding sessions happened across this project
- **FTR (First Try Right)** — % of sessions that completed without corrections/rework
- **Turns per session** — average conversation length (lower is more efficient)
- **Tokens** — total token consumption (cost indicator)

### FTR Trend (sparkline or small chart)

30-session FTR trend. Visual: are things getting better or worse?

When FTR dips, the chart annotates what changed: "new skill enabled", "rule added", "module refactored". This connects cause to effect.

### Quality Signals

```
✓ Zero-errors policy: 22/24 sessions passed first compile
✓ Pattern compliance: 92% of new code follows established patterns
⚠ Doc drift: 3 functions changed since their docs were last updated
✗ Test coverage: dropped 2% this week (now 66%)
```

Each signal is a pass/warn/fail indicator with a one-line explanation. Clicking drills into detail (sessions page, architecture page, etc.).

### Connection Diagram

Visual topology of the project's repos and their relationships:

```
┌─────────┐   REST    ┌──────────┐
│ acme-ui │─────────▶│ acme-api │──────▶ [PostgreSQL]
│ React   │           │ Express  │
└────┬────┘           └────┬─────┘
     └─────────┬───────────┘
               ▼
         ┌──────────┐
         │  design  │ ← also used by Internal Tools
         │  system  │
         └──────────┘
```

Derived from: cross-repo imports, shared dependencies, infra files (Dockerfile, docker-compose, k8s). User can add manual connections.

**Why this matters:** The AI sees one repo at a time. This diagram shows the developer (and informs the AI via session context) how repos connect. When editing `acme-api`, the AI should know it serves `acme-ui` and shares `design-system` with another project.

### Recent Sessions

Last 5-10 sessions across all repos in the project. Each shows:
- Repo name, timestamp, duration
- Outcome: completed / blocked / partial
- Correction count (0 = first try right)
- One-line summary

Clicking a session opens the session detail page.

### Sensei Efficiency

What sensei contributed to this project:

```
Context packs served:  142  (avg 3.2k tokens each)
Tokens saved by packs: ~680k  vs loading full files
Pattern reuses:        18   (4 unique patterns)
Drift alerts caught:    3   before they hit PR
Skills triggered:      87   (top: session-mgmt 24)
```

This proves the value of sensei — visible to the developer, their lead, and anyone reviewing the tooling investment.

## Insights → Actions

| What you see | What it means | What to do |
|---|---|---|
| FTR dropping in one module | AI is struggling with that area | Add a persona for that module, extract patterns from successful sessions |
| High correction count in auth sessions | Auth rules/patterns are insufficient | Review auth sessions, add missing patterns to rules |
| Doc drift warnings increasing | Code is changing faster than docs | Enable drift detection skill, prioritize doc updates |
| Token consumption spiking | Context packs may be too large, or sessions are unfocused | Review what session-start loads, tune context |
| Zero-errors failing | AI is producing code that doesn't compile | Check if rules are clear, verify test patterns exist |

## Tabs

The project layout has tabs. The dashboard is the first tab. Other tabs:

| Tab | Route | Purpose |
|-----|-------|---------|
| Overview | `/p/{id}` | This page — metrics, topology, quality |
| Architecture | `/p/{id}/arch` | Code graph, structural view |
| Traceability | `/p/{id}/trace` | Requirements → design → code → tests chain |
| Sessions | `/p/{id}/sessions` | All sessions for this project |
| Repos | `/p/{id}/repos` | Manage repo membership and roles |
| Profiles | `/p/{id}/profiles` | Mindsets, personas, rules, agents |

## What's Built

The project overview page exists with: stats row, cross-repo analysis, graph preview, recent sessions. The tabbed layout exists with: Overview, Architecture, Traceability, Sessions, Skills.

## What Needs to Be Built

Everything in the "metrics" and "insights" layer:

| Feature | Blocked by |
|---------|-----------|
| FTR metric | `log_event()` MCP tool in daemon — sessions need outcome tracking |
| FTR trend chart | `/api/metrics/:repo` daemon endpoint |
| Quality signals | Daemon-side computation from events + index data |
| Connection diagram | Infra file detection in daemon (Dockerfile, compose, k8s parsing) |
| Sensei efficiency | Context pack usage tracking in MCP tools |
| Correction analysis | Event type `revision_requested` capture in hooks |
| Token tracking | Blocked by ACP — Claude Code doesn't expose token counts per session yet |

**Build order:** `log_event()` tool → event capture in hooks → metrics API → this page. The connection diagram can be built independently (infra detection in indexer).
