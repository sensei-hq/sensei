# 01 — Daemon (senseid)

The core engine. Rust binary running as a background service on `:7744`. Owns the graph store, event store, indexer, task queue, and three capability sub-layers.

**Traces to:** [ideas/08](../../ideas/08-codebase-intelligence.md), [ideas/07](../../ideas/07-metrics-analytics.md), [blueprints/02](../../blueprints/02-system-architecture.md)

## Core

| Doc | Description |
|-----|-------------|
| [architecture.md](./architecture.md) | Overall daemon architecture and module layout |
| [indexing-architecture.md](./indexing-architecture.md) | Language adapters, AST parsing, graph building pipeline |
| [task-queue.md](./task-queue.md) | Task queue: scan → repo → folder → file → resolve → connect |
| [enhancements.md](./enhancements.md) | What needs to be built — graph enrichment, events, state, metrics, patterns |
| [llmspec.md](./llmspec.md) | `.sensei/llmspec.yaml` output format |
| [local-model-indexer.md](./local-model-indexer.md) | Optional Ollama adapter for local inference |

## Capability sub-layers

These are built on the daemon's data layer. If the graph or event store is wrong, these don't work.

| Sub-layer | Docs | Description |
|-----------|------|-------------|
| [intelligence/](./intelligence/) | 5 | Context delivery, compression, patterns, metadata, response cache |
| [traceability/](./traceability/) | 3 | Drift detection, doc tools, traceability matrix |
| [analytics/](./analytics/) | 3 | Benchmarking, telemetry, project memory |
