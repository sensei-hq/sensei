---
name: API Surface Overview
description: Screen-by-screen API design for the sensei daemon, following developer lifecycle order
date: 2026-04-27
type: design
traces:
  - ideas/26-bootstrap-and-dependencies.md
  - design/roadmap.md
---

# API Surface Overview

## Principle

Every daemon endpoint exists because a screen needs it. No speculative APIs. Each screen gets its own design doc with:
- What the screen shows
- What endpoints it calls
- Request/response shapes
- Verification scenarios

## Architecture

```
Desktop App (app repo)
├── SvelteKit frontend
│   ├── Tauri invoke → bootstrap commands (prereqs, lifecycle)
│   └── HTTP fetch → daemon API (all data)
└── src-tauri/
    └── depends on sensei-bootstrap crate (from daemon workspace)

Daemon Workspace (daemon repo)
├── crates/
│   ├── senseid/      — HTTP API, indexing, inference (binary)
│   ├── cli/          — sensei CLI (binary, renamed from sensei-cli/)
│   ├── mcp/          — MCP server (binary, renamed from sensei-mcp/)
│   └── bootstrap/    — prereq checks, hardware detection (library)
└── database/         — DDL schema
```

### Bootstrap crate responsibilities

The `bootstrap` crate is a library — no binary, no database access, no daemon dependency. It knows how to check and fix prerequisites on the host system.

Consumers:
- **Tauri app** — imports as dependency, exposes as Tauri commands
- **sensei CLI** — uses for `sensei doctor` and `sensei init`

Modules:
- `homebrew` — detect/install brew, check/install/upgrade formulae
- `service` — check/start postgresql, ollama, daemon
- `database` — check db exists, schema version, pgvector (via psql/pg_isready, not sqlx)
- `hardware` — RAM, GPU, core count → model tier recommendation
- `models` — ollama model detection, pull progress
- `types` — ComponentStatus, HardwareInfo, BootstrapResult, ModelTier

### Data flow

```
Bootstrap (no daemon needed):
  UI → Tauri invoke → bootstrap crate → shell commands → UI

Data requests (daemon running):
  UI → HTTP GET/POST → senseid API → PostgreSQL → response
```

## Screen order (developer lifecycle)

| # | Screen | Daemon repo | App repo | Design doc |
|---|--------|-------------|----------|------------|
| 0 | **Bootstrap** | `bootstrap` crate | Tauri commands + bootstrap UI | [01-bootstrap.md](./01-bootstrap.md) |
| 1 | **Setup wizard** | scan, detect ACPs, config endpoints | wizard flow UI | [02-setup.md](./02-setup.md) |
| 2 | **Observatory** | metrics, recommendations, sessions | dashboard UI | [03-observatory.md](./03-observatory.md) |
| 3 | **Sessions** | session list, events, tool replay | session browser UI | [04-sessions.md](./04-sessions.md) |
| 4 | **Learnings** | memories CRUD, patterns, corrections | learnings UI | [05-learnings.md](./05-learnings.md) |
| 5 | **Libraries** | library list, detail, usage stats | libraries UI | [06-libraries.md](./06-libraries.md) |
| 6 | **Instruments** | MCP tools, effectiveness metrics | playground UI | [07-instruments.md](./07-instruments.md) |
| 7 | **Projects** | project CRUD, graph, analysis | project detail UI | [08-projects.md](./08-projects.md) |
| 8 | **Settings** | config, extensions, export/import | settings UI | [09-settings.md](./09-settings.md) |

Each screen gets:
- A design doc in this directory
- A GitHub issue in `sensei-hq/daemon` (API + crate work)
- A GitHub issue in `sensei-hq/app` (UI + Tauri work)
- Issues reference each other so parallel sessions stay in sync

## API conventions

All daemon endpoints follow:

- **Base URL:** `http://localhost:9823`
- **Content-Type:** `application/json`
- **Errors:** `{ "error": "message" }` with appropriate HTTP status
- **Lists:** `{ "data": [...], "total": n }` with optional `?limit=` and `?offset=`
- **Filters:** query params for simple filters, POST body for complex queries
- **SSE:** streaming endpoints use `text/event-stream` (indexing progress, model pull)

## Workspace refactor (prerequisite)

Before screen work begins, rename existing crate folders:

| Current | New | Package name (unchanged) |
|---------|-----|--------------------------|
| `crates/sensei-cli/` | `crates/cli/` | sensei-cli |
| `crates/sensei-mcp/` | `crates/mcp/` | sensei-mcp |
| — | `crates/bootstrap/` | sensei-bootstrap (new) |

Update `Cargo.toml` workspace members accordingly.
