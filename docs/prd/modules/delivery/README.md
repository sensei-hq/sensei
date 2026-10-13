# Delivery plane — getting signal back to the human and the assistant

> Delivery = M14–M22. This plane closes the loop: context to the assistant, a
> daily view for the human, project understanding, the graph, instruments,
> insights, planning, relay and autonomous execution. Status:
> [`../../../roadmap/03-delivery.md`](../../../roadmap/03-delivery.md).

| ID | Module | One-line "done" | Spec |
|---|---|---|---|
| M14 | MCP & Context Delivery | The assistant gets the right context on the first turn | 🚧 Phase 2 |
| M15 | Observatory | The daily user sees *today's one thing*, with its receipt | 🚧 Phase 2 |
| M16 | Project Window | A user trusts what sensei learned about one project | 🚧 Phase 2 |
| M17 | Atlas | The code graph is legible at every level — birds-eye to symbol | 🚧 Phase 2 |
| M18 | Instruments | The user can see and replay what the assistant actually did | 🚧 Phase 2 |
| M19 | Insights & Impact | A recommendation is applied, measured, and reinforced | 🚧 Phase 2 |
| M20 | Planner & Baseline | A goal becomes phases → features → acceptance criteria + gates | 🚧 Phase 2 |
| M21 | Relay | A long run is supervised from anywhere, without leaking code | 🚧 Phase 2 |
| M22 | Agentic Execution | sensei drives a multi-model Planner→Builder→Judge run | 🚧 Phase 2 |

## What this plane owns

The surfaces the human and the assistant actually meet: MCP tool surface and
context delivery (M14), the Observatory's daily view and its module loops (M15),
the per-project window (M16), the code graph visualisation with its three
legible levels of detail L0/L1/L2 (M17, reorg §9.7), instruments and replay
(M18), insights and impact verdicts (M19), the planner and baseline gates (M20),
Relay's away-from-keyboard supervision (M21), and agentic execution (M22).

## Reading order

M14 first (the assistant channel) → M15/M16 (the human channels) → M17 (the
graph) → M18 → M19 → M20 → M21 → M22. The **graph level-of-detail** (M17) is
also the manager's birds-eye view (§9.7) — it is a Delivery surface with an
Insight dependency.

## Customer

The humans in [`../../03-personas.md`](../../03-personas.md) — solo and team
developers daily, and the **manager / delivery-lead** persona through M17's L0
and M20's gates.

## Legacy sources

Folded from `features/03-observatory.md` (UX half), `features/04-project.md`,
`features/06-relay.md`, `features/front-door/`, `spec/screen/*`,
`spec/pipeline/{mcp-surface,context-delivery,semantic-search,impact,
agent-execution,insights,project-icon,testability,benchmarks}`,
`design/{assistants,automated-run,local-agent-coordinator,workers,remote}`,
`architecture/{mcp,marketplace,app}`.