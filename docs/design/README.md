---
name: Design Documentation
description: Technical design docs organized by system layer — how things work, not what they do
---

# Design Documentation

Architecture, implementation details, and recorded decisions for the sensei project. Design docs answer *how* and *why*. Feature docs (in `docs/features/`) answer *what*.

## Layers

| # | Layer | Description |
|---|-------|-------------|
| 01 | [Daemon](./01-daemon/) | The core engine — indexer, graph store, task queue, and three capability sub-layers |
| | [├── intelligence/](./01-daemon/intelligence/) | Compression, context delivery, pattern store, metadata model, response cache |
| | [├── traceability/](./01-daemon/traceability/) | Drift detection, doc tools, traceability matrix |
| | [└── analytics/](./01-daemon/analytics/) | Benchmarking, telemetry resilience, project memory |
| 02 | [MCP](./02-mcp/) | Tool contracts and workflow tools — the AI's interface to the daemon |
| 03 | [Marketplace](./03-marketplace/) | Commands, skills, hooks, plugin packaging — the Claude Code integration layer |
| 04 | [Desktop](./04-desktop/) | UX design, dashboard views, SSE event patterns |
| 05 | [Platform](./05-platform/) | Architecture overview, multi-coordinator support |
| 06 | [CLI](./06-cli/) | Command-line interface |
| 07 | [Configuration](./07-configuration/) | Templates, config schemas |

### Why intelligence, traceability, and analytics are under daemon

These are not independent systems — they are **capabilities built on the daemon's data layer**. The graph schema, event store, and query surface must be designed together. If the daemon gets it wrong, these layers have nothing to work with. MCP, desktop, and marketplace are consumers; the daemon is the producer.

## Cross-cutting

| Doc | Description |
|-----|-------------|
| [roadmap.md](./roadmap.md) | Implementation roadmap — 66 items across 6 waves |
| [decisions/](./decisions/) | Architecture Decision Records (ADRs) |

## Archive

[_archive/](./_archive/) — superseded and stale docs from prior eras (Supabase, SaaS, pre-Rust). Preserved for historical reference.
