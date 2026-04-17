---
name: Design Documentation
description: Technical design docs organized by system layer — how things work, not what they do
---

# Design Documentation

Architecture, implementation details, and recorded decisions for the sensei project. Design docs answer *how* and *why*. Feature docs (in `docs/features/`) answer *what*.

## Layers

| # | Layer | Docs | Description |
|---|-------|------|-------------|
| 01 | [Daemon](./01-daemon/) | 6 | Indexer, graph store, events, task queue, metrics |
| 02 | [MCP](./02-mcp/) | 2 | Tool contracts, workflow tools |
| 03 | [Marketplace](./03-marketplace/) | 5 | Commands, skills, hooks, plugin packaging |
| 04 | [Desktop](./04-desktop/) | 3 | UX design, dashboard views, SSE patterns |
| 05 | [Intelligence](./05-intelligence/) | 5 | Context delivery, compression, patterns, metadata |
| 06 | [Traceability](./06-traceability/) | 3 | Drift detection, doc tools, traceability matrix |
| 07 | [Analytics](./07-analytics/) | 3 | Benchmarking, telemetry, project memory |
| 08 | [Platform](./08-platform/) | 2 | Architecture overview, multi-coordinator |
| 09 | [CLI](./09-cli/) | 1 | Command-line interface |
| 10 | [Configuration](./10-configuration/) | 1 | Templates, config schemas |

## Cross-cutting

| Doc | Description |
|-----|-------------|
| [roadmap.md](./roadmap.md) | Implementation roadmap — 66 items across 6 waves |
| [decisions/](./decisions/) | Architecture Decision Records (ADRs) |

## Archive

[_archive/](./_archive/) — superseded and stale docs from prior eras (Supabase, SaaS, pre-Rust). Preserved for historical reference.
