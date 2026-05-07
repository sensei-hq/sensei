---
name: Daemon Architecture
description: Core engine architecture — indexer, graph store, task queue
type: design
---

# Daemon Architecture

The daemon is the core engine of sensei. It indexes codebases, stores the code graph, and serves intelligence to AI assistants via MCP.

## Components

- **HTTP API** — REST endpoints for the desktop app and CLI
- **Task queue** — scan, index, resolve, connect pipeline
- **Graph store** — Kuzu embedded graph database
- **Inference** — local model routing via Ollama
