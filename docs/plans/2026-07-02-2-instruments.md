---
name: 2026-07-02 — T2 Instruments rebuild
issue: TBD
epic: https://github.com/sensei-hq/sensei/issues/83
analysis: docs/analysis/2026-07-01-project-window-instruments-depmap-gap-analysis.md
mockups:
  - docs/mockups/Sensei/lib/instruments.jsx
  - docs/mockups/Sensei/lib/instruments-simple.jsx
  - docs/mockups/Sensei/lib/mcp-replay-insights.jsx
  - docs/mockups/Sensei/lib/mcp-signals-data.js
  - docs/mockups/Sensei/lib/instruments-data.js
---

# T2 — Instruments rebuild

Three-tab Instruments screen (Playground / Replay / Health) plus per-project
instruments view. Requires four new daemon capabilities (rich tool manifest,
`mcp_servers` persistence, replay aggregation, insights job) and a new
`mcp.svelte.ts` state slice.

**Assumes T1b landed** (adapter registry + ManifestAdapter shape). No Dōjō
coupling — user-scope preferences only.

## Success gate

- Playground tab renders the 14+ registered tools with `{kind, mcp, summary,
  inputs[], example_response}` from a single daemon call — no hardcoded
  metadata in the app.
- Replay tab shows a session's PreToolUse↔PostToolUse timeline with
  `used | partial | ignored` verdict per call.
- Health tab shows KPI strip + per-tool usage table + signal cards driven by
  a nightly insights job.
- Project-scoped Instruments page returns MCP tools joined with per-project
  call/FTR aggregation (not extensions — that endpoint is misnamed today).
- Playwright e2e verifies all three tabs render and switch without console
  errors; per-tool detail pane opens with `inputs[]` visible.

## Slices

Each slice is its own commit sequence + PR + merge cycle. E2E gate before
slice merges to `main`.

### Slice A — Rich tool manifest (daemon, 2 days)

**Deliverables**
- New DDL: `sensei.mcp_tool_manifests` (or JSONB column on existing table).
  One row per tool: `{name, mcp, kind ('action'|'query'), summary, inputs
  jsonb, example_response jsonb}`.
- `crates/senseid/src/api/handlers/mcp.rs` — replace hardcoded 14-tool list
  with a DB read. Seed via new startup task (`SeedMcpManifests`) that
  populates rows from a curated const table.
- New MCP tool `mcp.list_manifests` (server-side) returning the same shape.

**Tests**
- Unit: seed idempotency; kind classification (action vs query per tool).
- Integration: `GET /api/mcp/tools` returns the new shape; kind chip filter
  works over the response.

### Slice B — `mcp_servers` table + connection state (daemon, 1 day)

**Deliverables**
- DDL: `sensei.mcp_servers` (id, name, source, enabled, per-project scope).
- `sensei.project_mcp_scopes` (project_id, server_id, enabled) — for the
  per-project enable/disable in the playground.
- Seed from existing `.acp` scan on startup; add API for
  `GET /api/mcp/servers`, `PUT /api/mcp/servers/:id/scope/:project_id`.

**Tests**
- Seed picks up sensei's own MCP server + any user-installed servers.
- Per-project scope toggle round-trips through the endpoint.

### Slice C — Replay aggregation (daemon, 2 days)

**Deliverables**
- Pair PreToolUse ↔ PostToolUse events on `activity.assistant_events` — new
  view `sensei.tool_calls` (session_id, tool, request jsonb, response jsonb,
  duration_ms, verdict enum).
- Verdict logic: join with the next assistant turn to derive `used |
  partial | ignored` (used = the response influenced the next message;
  ignored = didn't; partial = influenced but not fully).
- `GET /api/sessions/:id/tool-timeline` returning the joined view.

**Tests**
- Verdict classifier unit tests over synthetic turn fixtures.
- Endpoint round-trips a session with 5 tool calls → returns 5 timeline
  entries with verdicts.

### Slice D — Insights aggregation task (daemon, 2 days)

**Deliverables**
- Task `AggregateToolInsights` — nightly. Reads `sensei.tool_calls`,
  computes per-tool usage-split %, 14d call trend, FTR delta (sessions
  calling the tool vs not), signal recommendations.
- Table `sensei.tool_insights` (tool, computed_at, metrics jsonb, signals
  jsonb).
- `GET /api/observatory/tool-usage` — return the latest snapshot.

**Tests**
- Deterministic aggregation over a fixed session fixture.
- Signal detection: dormant tool (0 calls 14d) → `unused` signal.

### Slice E — Frontend: `mcp.svelte.ts` state slice + three tabs (app, 3 days)

**Deliverables**
- `app/src/lib/state/mcp.svelte.ts` — Svelte 5 rune-based state owning
  connections, server list, tool manifests, cached recent responses. All
  screens read from this store; no per-component fetching.
- `app/src/routes/(app)/instruments/+page.svelte` — Tabs (Rokkit `Tabs`)
  hosting Playground, Replay, Health.
- Playground: rokkit `List` (grouped) for MCP + tool sidebar; two-pane
  list+detail with `inputs[]` form + `example_response` panel.
- Replay: session picker (rokkit `List` with FTR/correction badges) +
  timeline (bespoke — rokkit has no timeline).
- Health: KPI `StatBlock.svelte` strip + `SignalCard.svelte`
  (variant: warn/opportunity/unused/win/neutral) + per-tool usage
  `Table.svelte`.

**Tests**
- Vitest: state slice reducer over fixture events.
- Playwright: tab switching, tool detail pane opens, kind chip filter works.

### Slice F — Project-scoped Instruments (app + daemon, 1 day)

**Deliverables**
- `GET /api/projects/{id}/instruments` — CHANGE the return shape from
  extensions to MCP tools joined with per-project call/FTR aggregation.
  Existing consumers (extensions view) move to a new
  `/api/projects/{id}/extensions` endpoint.
- `app/src/routes/(project)/project/[id]/instruments/+page.svelte` — rebuild
  using the same primitives as global instruments, filtered to the project.

**Tests**
- Playwright: project instruments page shows scoped call/FTR per tool.

## Non-goals for T2

- Third-party MCP server ingestion beyond sensei's own registry — captured
  as a follow-up issue.
- The playground's "actually run this tool" button (execution vs. inspect
  only) — a T2.5 follow-up gated on the mcp registry maturing.

## Sequencing

A → B (parallelizable with A) → C → D → E → F. Slices A–D can ship
without frontend changes (daemon-only merges to `develop`, then
develop→main once E lands). Slice E is where the mockup fidelity gets
verified; Slice F closes the per-project gap.

Merge sub-chunk to `develop` after each slice; `develop → main` after
Slice F clears its Playwright gate.
