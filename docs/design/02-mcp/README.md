# 02 — MCP (sensei-mcp)

The AI's interface to the daemon. Rust binary (stdio transport) that translates MCP tool calls to daemon HTTP requests. The AI never calls the daemon directly — it only sees MCP tools.

**Traces to:** [blueprints/01](../../blueprints/01-workflow-engine.md), [blueprints/02](../../blueprints/02-system-architecture.md)

| Doc | Description |
|-----|-------------|
| [tool-contracts.md](./tool-contracts.md) | Existing code intelligence tools — search, get_callers, get_lib_docs, etc. |
| [workflow-tools.md](./workflow-tools.md) | New workflow tools — log_event, get_workflow_state, update_phase, get_metrics, pattern queries |
