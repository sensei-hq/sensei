# Roadmap — Delivery plane (M14–M22)

Getting signal back to the human and the assistant: MCP/context (M14),
Observatory (M15), project window (M16), Atlas (M17), instruments (M18),
insights/impact (M19), planner (M20), relay (M21), agentic execution (M22).
Status legend: [`README.md`](README.md).

> Per-module statuses are filled in Phase 3 from the legacy coverage map
> (`../history/features/coverage-map.md`) and the plan gap analysis
> (`../history/plan/`). Structure is final.

| ID | Module | One-line "done" | Status |
|---|---|---|---|
| M14 | MCP & Context Delivery | The assistant gets the right context on the first turn | 🚧 Phase 3 |
| M15 | Observatory | The daily user sees *today's one thing*, with its receipt | 🚧 Phase 3 |
| M16 | Project Window | A user trusts what sensei learned about one project | 🚧 Phase 3 |
| M17 | Atlas | The code graph is legible at every level — birds-eye to symbol | 🚧 Phase 3 |
| M18 | Instruments | The user can see and replay what the assistant actually did | 🚧 Phase 3 |
| M19 | Insights & Impact | A recommendation is applied, measured, and reinforced | 🚧 Phase 3 |
| M20 | Planner & Baseline | A goal becomes phases → features → acceptance criteria + gates | 🚧 Phase 3 |
| M21 | Relay | A long run is supervised from anywhere, without leaking code | 🚧 Phase 3 |
| M22 | Agentic Execution | sensei drives a multi-model Planner→Builder→Judge run | 🚧 Phase 3 |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| G-1 | graph/layered view is unreadable at granular scale — no L0/L1/L2 level of detail (M17) | P1 |
| C-12 | inference-pipeline review not scoped as a task (M5/M14 - M14 consumed by delivery) | P1 |
| D-8 | MCP `search` tool is symbol-only (lexical + no FTS), `node_fts`/`hybrid.rs` absent | P2 |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

M17's L0/L1/L2 (reorg §9.7) is the prerequisite for reading architecture
metrics and for the manager's roll-up. M19's loop closing (accept → verdict →
reinforce) is what makes the Observatory's promises true. M21 (Relay) rides the
Dōjō's team infra (T1).