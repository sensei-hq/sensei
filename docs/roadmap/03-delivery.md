# Roadmap — Delivery plane (M14–M22)

Getting signal back to the human and the assistant: MCP/context (M14),
Observatory (M15), workspace window (M16), Code map (M17), instruments (M18),
insights/impact (M19), planner (M20), relay (M21), agentic execution (M22).
Status legend: [`README.md`](README.md).

> **Status populated 2026-11-03.** The cells follow the readiness table in
> [`../review/2026-11-03-orientation.md`](../review/2026-11-03-orientation.md) §2 and the capability
> audit in [`../review/2026-10-27-roadmap-vs-state.md`](../review/2026-10-27-roadmap-vs-state.md) §1.3
> and §2. The last column names what proves each cell; a status is a snapshot, so move it when the
> module moves.

| ID | Module | One-line "done" | Status | Evidenced by |
|---|---|---|---|---|
| M14 | MCP & Context Delivery | The assistant gets the right context on the first turn | 🟡 **Partial** | 56 tools in `crates/mcp/src/lib.rs`, `query.rs context_pack`, and `search` fuses ILIKE with embeddings by RRF; the L0–L3 resolution levels and the per-session dedup log are unbuilt, and `search` still has no full text (D-8) |
| M15 | Observatory | The daily user sees *today's one thing*, with its receipt | 🟡 **Partial** | `GET /api/observatory/today` and the consolidation routes/screen are built (the M15 README calling consolidation mock-only is stale), but the hero's receipt is empty live (`source:""`, `impact:null`), there is no `/insights/{id}/reasoning`, and `GET /api/sessions` hard-codes `toolUsage: []` and `benchmarkPairs: []` (`handlers/sessions.rs:65`, `get_sessions_stub`) |
| M16 | Workspace Window | A user trusts what sensei learned about one workspace | 🟡 **Partial** | Every screen under `(workspace)/workspace/[id]/` is built (Overview, Intake, Metrics, Tickets, Integrations, Memories, Impact, Traceability, Patterns, Diagrams, About); the Tasks section and the multi-repository roll-up wait on M20's plan tables, and the dōjō binding is a POST with no GET and no audit |
| M17 | Code map | The code graph is legible at every level — birds-eye to symbol | 🟡 **Partial** | `…/diagrams/codemap`, `/codemap/{module,file,cochange}` and `/diagrams/zones` have route tests, `packages/codemap/` renders seven lenses, and an e2e test draws each lens against the real daemon — the readiness table puts local diagrams at ~85%; the dōjō embed and the daemon-pushed map are 0%, the S4 2c pooling double count is open, S6 history is a proposal, and Layers still freezes the page |
| M18 | Instruments | The user can see and replay what the assistant actually did | ✅ **Built** | Playground lists every discovered MCP server and calls `tools/call` through it (`POST /api/instruments/mcp-servers/{id}/call`, hand-registered servers included), Replay is `/api/sessions/{id}/replay`, verdicts carry `unknown` (#278), the tools-health grid runs (84 registered, 20 invoked in 14 days) and 31 of 32 read-only tools match an independent oracle; the benchmark runner is deferred, not dropped (#279), and the 24 write/spend tools are unverified |
| M19 | Insights & Impact | A recommendation is applied, measured, and reinforced | 🟡 **Partial** | `verdicts.rs`, `/api/workspaces/{id}/impact`, `impact_verdicts.ddl` and accept/reject are built; `applied_recommendations`, `impact_regressions` and `GET /api/impact` do not exist, so applied → measured → reinforced never closes, and `ActionItems.svelte` has 0 buttons or links (the readiness table's metrics-engine gap) |
| M20 | Planner & Baseline | A goal becomes phases → features → acceptance criteria + gates | 🟡 **Partial** | `/api/planner/generate`, `plan_graph.rs` and `/api/runs/plan` are built; the plan tables, the baseline FTR and the ratchet are not |
| M21 | Relay | A long run is supervised from anywhere, without leaking code | 🟡 **Partial** | `advance_run.rs drive_run`, `dojo/gate.rs` (36 tests) and the dōjō's relay screens are built; `SENSEI_RUN_DRIVE` defaults off, `relay_drivers/mod.rs driver_for` always returns `ClaudeDriver` so ACP and the fallbacks are unreachable, and the mobile relay is unverified in production (D27) |
| M22 | Agentic Execution | sensei drives a multi-model Planner→Builder→Judge run | 📄 **Designed** | Design only — no `agents/` directory and no `agent_runs` DDL |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| ~~G-1~~ | ~~graph/layered view is unreadable at granular scale — no L0/L1/L2 level of detail~~ — **closed 2026-11-03:** `packages/codemap/` renders L0 (`ModuleMap.svelte`), L1 (`FileColumns.svelte`) and L2 (`SymbolView.svelte`) over `…/diagrams/codemap`; what is still missing is the dōjō embed, not the map (M17) | — |
| C-12 | inference-pipeline review not scoped as a task (M5/M14 - M14 consumed by delivery) | P1 |
| D-8 | MCP `search` tool is symbol-only (lexical + no FTS), `node_fts`/`hybrid.rs` absent | P2 |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

M17's L0/L1/L2 (reorg §9.7) is the prerequisite for reading architecture
metrics and for the manager's roll-up. M19's loop closing (accept → verdict →
reinforce) is what makes the Observatory's promises true. M21 (Relay) rides the
Dōjō's team infra (T1).
