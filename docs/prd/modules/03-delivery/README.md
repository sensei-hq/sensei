# Delivery plane — getting signal back to the human and the assistant

> Delivery = M14–M22. This plane closes the loop: context to the assistant, a
> daily view for the human, workspace understanding, the **graph UI surface** (the Code map),
> instruments, insights, planning, relay and autonomous execution. Status:
> [`../../../roadmap/03-delivery.md`](../../../roadmap/03-delivery.md).

| ID | Module | One-line "done" | Spec |
|---|---|---|---|
| M14 | MCP & Context Delivery | The assistant gets the right context on the first turn | [`README`](14-mcp-context-delivery/README.md) · [`mcp-surface`](14-mcp-context-delivery/scenarios/mcp-surface.feature) · [`context-delivery`](14-mcp-context-delivery/scenarios/context-delivery.feature) · [`search`](14-mcp-context-delivery/scenarios/search.feature) |
| M15 | Observatory | The daily user sees *today's one thing*, with its receipt | [`README`](15-observatory/README.md) · [`observatory-today`](15-observatory/scenarios/observatory-today.feature) · [`observatory-insights`](15-observatory/scenarios/observatory-insights.feature) · [`observatory-surfaces`](15-observatory/scenarios/observatory-surfaces.feature) |
| M16 | Workspace Window | A user trusts what sensei learned about one workspace | [`README`](16-workspace-window/README.md) · [`project-overview`](16-workspace-window/scenarios/workspace-overview.feature) · [`project-sections`](16-workspace-window/scenarios/workspace-sections.feature) |
| M17 | **Code Map** | The code graph is legible at every level — birds-eye to symbol | [`README`](17-code-map/README.md) · [`code-map`](17-code-map/scenarios/code-map.feature) · [`diagrams`](17-code-map/scenarios/diagrams.feature) |
| M18 | Instruments | The user can see and replay what the assistant actually did | [`README`](18-instruments/README.md) · [`instruments`](18-instruments/scenarios/instruments.feature) · [`instruments-health`](18-instruments/scenarios/instruments-health.feature) |
| M19 | Insights & Impact | A recommendation is applied, measured, and reinforced | [`README`](19-insights-and-impact/README.md) · [`insights`](19-insights-and-impact/scenarios/insights.feature) · [`impact`](19-insights-and-impact/scenarios/impact.feature) |
| M20 | Planner & Baseline | A goal becomes phases → features → acceptance criteria + gates | [`README`](20-planner-and-baseline/README.md) · [`planner`](20-planner-and-baseline/scenarios/planner.feature) · [`baseline`](20-planner-and-baseline/scenarios/baseline.feature) |
| M21 | Relay | A long run is supervised from anywhere, without leaking code | [`README`](21-relay/README.md) · [`relay`](21-relay/scenarios/relay.feature) · [`relay-drivers`](21-relay/scenarios/relay-drivers.feature) |
| M22 | Agentic Execution | sensei drives a multi-model Planner→Builder→Judge run | [`README`](22-agentic-execution/README.md) · [`agentic-execution`](22-agentic-execution/scenarios/agentic-execution.feature) |

## What this plane owns

The surfaces the human and the assistant actually meet: MCP tool surface and
context delivery (M14), the Observatory's daily view and its module loops (M15),
the per-workspace window (M16), **the Graph UI Surface — the Code map (M17)**
rendering the core code graph built by M4 (Indexer) with L0/L1/L2 level-of-detail
(reorg §9.7), instruments and replay (M18), insights and impact verdicts (M19),
the planner and baseline gates (M20), Relay's away-from-keyboard supervision (M21),
and agentic execution (M22).

## Unified Graph Architecture

```
M4 (Indexer) → Core Graph (`nodes`/`edges`/`communities`) → M17 (Code map) → UI
                                    ↘ M16 (Workspace Window: Code map embedded)
                                    ↘ M18 (Instruments: replay overlay)
                                    ↘ M19 (Impact: drift overlay)
```

- **M4 (Foundation)** builds the convergent core graph: `sensei.nodes` / `sensei.edges` + `inference.communities` (FQN symbol-table, idempotent edges, community durability).
- **M17 (Delivery)** is the **Graph UI Surface** — the Code map: one canvas at three levels (L0 modules · L1 files · L2 symbols) with seven lenses (Structure · Wheel · Zones · Dependency matrix · Cycles · Hidden coupling · Complexity) — all consuming the same core graph API (`/api/graph/*`, `/api/projects/{id}/diagrams/*`).
- **M16, M18, M19** embed Code map views as sub-surfaces (Workspace Window, Instruments Replay, Impact drift overlay).

## Reading order

M14 first (the assistant channel) → M15/M16 (the human channels) → **M17 (the Graph UI Surface)** → M18 → M19 → M20 → M21 → M22. The **graph level-of-detail** (M17 L0/L1/L2, reorg §9.7) is also the manager's birds-eye view — it is a Delivery surface with an Insight (M8) dependency.

## Customer

The humans in [`../../03-personas.md`](../../03-personas.md) — solo and team
developers daily, and the **manager / delivery-lead** persona through M17's L0
and M20's gates.

## Legacy sources

Folded from `features/03-observatory.md` (UX half), `features/04-project.md`,
`features/06-relay.md`, `features/front-door/`, `spec/screen/*`,
`spec/pipeline/{mcp-surface,context-delivery,semantic-search,impact,
agent-execution,insights,workspace-icon,testability,benchmarks}`,
`design/{assistants,automated-run,local-agent-coordinator,workers,remote}`,
`architecture/{mcp,marketplace,app}`.