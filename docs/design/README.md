# Sensei — Architecture & Design

## How to read this folder

`docs/design/` describes **how** Sensei is built. For **what** it does and why, see [`docs/ideas/`](../ideas/). Each design doc references the relevant ideas doc so you can trace the "how" back to the "what". User-facing behavior is never re-explained here — if a design doc needs to reference a feature, it links to the ideas doc and moves on.

---

## Architecture diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  USER SURFACES                                                                  │
│                                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────────────┐   │
│  │ Desktop App  │    │ CLI          │    │ AI Assistants                    │   │
│  │ (Tauri +     │    │ (sensei)     │    │ (Claude Code, Cursor, Zed, ...) │   │
│  │  SvelteKit)  │    │              │    │                                  │   │
│  └──────┬───────┘    └──────┬───────┘    └────────────┬─────────────────────┘   │
│         │                   │                         │                         │
└─────────┼───────────────────┼─────────────────────────┼─────────────────────────┘
          │ HTTP              │ HTTP                     │ stdio
          │                   │                         ▼
          │                   │              ┌──────────────────────┐
          │                   │              │ MCP (sensei-mcp)     │
          │                   │              │ Rust binary, stdio   │
          │                   │              │ Translates MCP tools │
          │                   │              │ → daemon HTTP calls  │
          │                   │              └──────────┬───────────┘
          │                   │                         │ HTTP
          ▼                   ▼                         ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  DAEMON (senseid) — Rust binary, background service :7744 / :7745              │
│                                                                                 │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────────┐  │
│  │ Indexing Pipeline │  │ Intelligence     │  │ Analytics Engine             │  │
│  │ Language adapters │  │ Context delivery │  │ Event capture, FTR scoring,  │  │
│  │ (tree-sitter,    │  │ Resolution levels│  │ session tracking, teachings, │  │
│  │  OXC, sqlparser) │  │ (L0-L3), pattern │  │ corrections, coaching        │  │
│  │ Task queue, graph │  │ detection, memory│  │                              │  │
│  │ builder           │  │                  │  │                              │  │
│  ├──────────────────┤  ├──────────────────┤  ├──────────────────────────────┤  │
│  │ API Surface      │  │ Gateway          │  │                              │  │
│  │ /api/graph/*     │  │ (sensei-gateway) │  │                              │  │
│  │ /api/events/*    │  │ Inference routing│  │                              │  │
│  │ /api/state/*     │  │                  │  │                              │  │
│  │ /api/metrics/*   │  │ ┌────────────┐   │  │                              │  │
│  │ /api/libs/*      │  │ │ Ollama     │   │  │                              │  │
│  │ /health, /stop   │  │ │ (local)    │   │  │                              │  │
│  │                  │  │ ├────────────┤   │  │                              │  │
│  │                  │  │ │ Anthropic  │   │  │                              │  │
│  │                  │  │ │ (API)      │   │  │                              │  │
│  │                  │  │ ├────────────┤   │  │                              │  │
│  │                  │  │ │ OpenAI /   │   │  │                              │  │
│  │                  │  │ │ Google     │   │  │                              │  │
│  │                  │  │ │ (API)      │   │  │                              │  │
│  │                  │  │ └────────────┘   │  │                              │  │
│  └──────────────────┘  └──────────────────┘  └──────────────────────────────┘  │
│                                                                                 │
└─────────────────────────────────────────┬───────────────────────────────────────┘
                                          │
                                          ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│  PostgreSQL (pgvector)                                                          │
│                                                                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌──────────┐ ┌─────────────┐ │
│  │ gateway (7) │ │ sensei (24) │ │inference (9)│ │activity(6│ │ history (2) │ │
│  │ providers,  │ │ projects,   │ │ communities,│ │) events, │ │ past_ext,   │ │
│  │ models,     │ │ nodes,      │ │ patterns,   │ │ sessions,│ │ past_mem    │ │
│  │ routers,    │ │ edges,      │ │ hyperedges, │ │ hooks,   │ │             │ │
│  │ chains,     │ │ folders,    │ │ insights,   │ │ tasks,   │ │             │ │
│  │ assignments │ │ libraries,  │ │ drift,      │ │ snapshots│ │             │ │
│  │             │ │ memories,   │ │ traces,     │ │          │ │             │ │
│  │             │ │ config, ... │ │ recommend.  │ │          │ │             │ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └──────────┘ └─────────────┘ │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘

EXTERNAL
  ┌──────────────────────┐    ┌──────────────────────┐
  │ Marketplace (GitHub) │    │ Homebrew (distrib.)  │
  │ Skills, commands,    │    │ sensei-hq/           │
  │ hooks, plugins       │    │   homebrew-tap        │
  └──────────────────────┘    └──────────────────────┘
```

---

## Layered architecture

Sensei has three tiers, with two cross-cutting concerns:

| Tier | What lives here | Role |
|------|----------------|------|
| **User surfaces** | Desktop app, CLI, AI assistants via MCP | Present information, accept commands, deliver context |
| **Core engine** | Daemon (senseid) — indexing pipeline, intelligence layer, analytics engine, gateway, API surface | Owns all logic: indexing, graph building, pattern detection, event processing, inference routing, context delivery |
| **Storage** | PostgreSQL with pgvector | Single source of truth for all state: relational, vector, and queue data. Graph queries via SQL JOINs on edge tables. |

**Cross-cutting: Gateway** — the inference routing library (`sensei-gateway` crate) lives inside the daemon but serves any component that needs model inference. Routes requests across Ollama (local), Anthropic, OpenAI, and Google based on configurable fallback chains.

**Cross-cutting: Marketplace** — a separate GitHub repo (`sensei-hq/marketplace`, synced as a git subtree) containing skills, commands, hooks, and plugin configs for Claude Code integration. No executable code — markdown and JSON only. Community-contributable.

---

## Tech stack

| Layer | Technology |
|-------|-----------|
| Daemon, MCP server, CLI | **Rust** — Cargo workspace with crates: `senseid`, `mcp`, `cli`, `gateway`, `bootstrap`, `logger`, `sensei-config` |
| Desktop app | **Tauri** (Rust native shell) + **SvelteKit** (Svelte 5 runes) |
| Design system | **Rokkit** — custom component library |
| Database | **PostgreSQL** with **pgvector** (vector similarity). Graph queries via relational JOINs on edge tables. |
| Local inference | **Ollama** — embedding generation, local model inference for indexing and classification |
| Plugin system | **Claude Code plugin format** — skills, commands, hooks, MCP server registrations |
| Distribution | **Homebrew** tap (`sensei-hq/homebrew-tap`), desktop installers (DMG/MSI) |

---

## Component map

| # | Component | Binary / Package | Port | Purpose | Design Doc | Ideas Ref |
|---|-----------|-----------------|------|---------|------------|-----------|
| 1 | **App** | `app/` (Tauri + SvelteKit) | — | Desktop observatory: dashboards, setup wizard, configuration, session viewer | `design/app/` | [ideas/02](../ideas/02-setup.md), [ideas/03](../ideas/03-observatory.md) |
| 2 | **Daemon** | `senseid` (`crates/senseid`) | :7744 (release) / :7745 (dev) | Core engine: indexer, graph store, events, analytics, HTTP API | `design/daemon/` | [ideas/04](../ideas/04-project.md) |
| 3 | **Gateway** | `sensei-gateway` (`crates/gateway`) | — (library) | Inference routing: provider fallback chains, model assignments, budget tracking | `design/gateway/` | [ideas/05](../ideas/05-gateway.md) |
| 4 | **MCP** | `sensei-mcp` (`crates/mcp`) | — (stdio) | MCP server: translates MCP tool calls to daemon HTTP; the AI assistant's interface | `design/mcp/` | [ideas/04](../ideas/04-project.md) |
| 5 | **CLI** | `sensei` (`crates/cli`) | — | Command-line interface for manual operations, diagnostics, database management | `design/cli/` | [ideas/01](../ideas/01-bootstrap.md) |
| 6 | **Marketplace** | `marketplace/` (git subtree) | — | Claude Code plugins: skills, commands, hooks, plugin configs, catalog | `design/marketplace/` | — |
| 7 | **Logging** | `sensei-logger` (`crates/logger`) | — (library) | Structured logging crate shared across all binaries | `design/logging/` | [ideas/06](../ideas/06-logging.md) |
| 8 | **Website** | `website/` (SvelteKit) | — | Marketing site: philosophy, downloads, benchmarks | — | — |
| 9 | **Homebrew** | `homebrew/` (git subtree) | — | Homebrew tap formula for CLI + daemon distribution | — | [ideas/01](../ideas/01-bootstrap.md) |
| 10 | **Build** | `Makefile`, `Cargo.toml` workspace | — | Build orchestration: `make crates-dev`, `make bump`, `make test` | — | — |

---

## Data flows

### 1. Event capture

AI assistant activity flows through hooks into the daemon, where it is processed into analytics that surface in the observatory.

```
AI assistant          Marketplace hooks         MCP / daemon            Analytics            Observatory
(Claude Code,    -->  (pre-tool, post-tool, --> (senseid)          --> engine           --> (desktop app)
 Cursor, ...)         user-prompt, session-     event store,           FTR scoring,         daily dashboard,
                      start, pre-compact)       session tracking       teachings,           session detail,
                                                                       corrections          coaching
```

### 2. Indexing

Source files are parsed into an intermediate representation, then written to the PostgreSQL graph where they become queryable by the intelligence layer.

```
File system       Language adapters       Adapter IR             PostgreSQL graph       Intelligence
(project     -->  (tree-sitter, OXC, --> (nodes, edges,    --> (sensei.nodes,     --> (context delivery,
 source files)     sqlparser, markdown)   patterns, metadata)    sensei.edges,         pattern detection,
                                                                 inference.*)          search, memory)
```

### 3. Context delivery

When an AI assistant needs context for a task, the intelligence layer selects and ranks information within a token budget, then delivers it through MCP.

```
Task / query      Token budget          Resolution selection     MCP                   AI assistant
(from AI     -->  (configured      -->  L0: signature only  --> (sensei-mcp     -->  (receives ranked,
 assistant)       per-project)          L1: + description       stdio transport)      budgeted context
                                        L2: + relationships                           for the task)
                                        L3: + full body
```

---

## Principles

### Design principles

- **Single source of truth** — PostgreSQL owns all state. No split-brain between files and database. Configuration that must be version-controlled (`.sensei/config.yaml`, marketplace content) is the only exception.
- **Agent-agnostic** — Sensei serves any AI coding assistant through MCP. Claude Code is the reference integration; others connect via the same protocol. No assistant-specific logic in the daemon.
- **Local-first** — All processing runs on the developer's machine. No cloud dependency required. Ollama provides local inference. PostgreSQL runs locally.
- **Zero-config start** — `sensei init` detects the stack, creates config, starts the daemon, and installs hooks. A developer should be productive within minutes.
- **Invisible integration** — Sensei observes sessions without interrupting them. The desktop app is a passive observatory, never a blocker. Hooks capture events silently.
- **Token efficiency** — Context delivery uses resolution levels (L0 through L3) to pack maximum signal into minimum tokens. The intelligence layer ranks, slices, and assembles context within a configurable budget.

### Rules

- **Compile-time mode separation** — Debug and release builds are isolated via Cargo feature flags, not runtime environment detection. Debug uses port :7745 and database `sensei_dev`; release uses :7744 and `sensei`. Directories: `~/.sensei-dev/` vs `~/.sensei/`.
- **No direct daemon calls from AI** — AI assistants interact exclusively through MCP. The MCP server translates tool calls to daemon HTTP requests. This keeps the daemon API internal and the MCP contract stable.
- **No executable code in marketplace** — Marketplace contains markdown (skills, commands), JSON (catalog, plugin configs), and bash (hooks). No compiled binaries, no package dependencies, no build steps.
- **Desktop observes, never blocks** — The desktop app reads from the daemon and database. It never writes to the code graph or modifies session state. It is a consumer, not a producer.
- **DRY** — Before writing any function, type, or constant, search the codebase for an existing implementation. If one exists in a shared crate, use it. Never duplicate logic owned by a shared location.
- **No silent workarounds** — Every deviation from established architecture must be raised and documented before implementation. No inline comments explaining why shared code was avoided.

### Patterns

- **Adapter IR** — Language adapters (tree-sitter, OXC, sqlparser) parse source into a common intermediate representation (nodes, edges, metadata). The indexing pipeline consumes the IR, not raw AST. New languages require only a new adapter.
- **Sidecar lifecycle** — The daemon runs as a background service managed by the desktop app (or CLI). Start, stop, health check, and auto-restart follow a sidecar pattern. The daemon does not self-daemonize.
- **SSE for progress** — Long-running operations (indexing, scanning) stream progress to clients via Server-Sent Events over the daemon HTTP API. Backed by PostgreSQL LISTEN/NOTIFY.
- **Hook-based capture** — Event data flows from AI assistants through marketplace hooks (bash scripts that POST to the daemon). This decouples event capture from any specific assistant's internals.
- **Chain-based routing** — The gateway routes inference requests through configurable fallback chains: try the preferred provider, fall back to the next if unavailable. Chain configuration lives in the database.
- **Resolution levels** — Context delivery selects detail level per symbol: L0 (signature), L1 (+ description), L2 (+ relationships), L3 (+ full body). The intelligence layer picks the level that fits the token budget.

### Non-functional requirements

- **Bootstrap health under 2 seconds** — Prerequisite checks (PostgreSQL, Ollama, daemon) complete within 2s. Long operations (first index, model pull) run asynchronously with SSE progress.
- **Context within token budget** — Every context delivery response fits within the configured token budget. The system degrades resolution (L3 down to L0) rather than exceeding the limit.
- **Graceful degradation** — If Ollama is unavailable, indexing proceeds without model-assisted descriptions (L0 only). If PostgreSQL is down, the daemon reports the error and exits cleanly. No component crashes silently.
- **All state survives restart** — Session state, task queue, events, and graph data persist in PostgreSQL. A daemon restart resumes from where it left off. No in-memory-only state.
- **Debug/release isolation** — Ports: :7744 (release) / :7745 (dev). Databases: `sensei` / `sensei_dev`. Directories: `~/.sensei/` / `~/.sensei-dev/`. A developer can run both modes simultaneously without conflict.
- **No secrets in marketplace** — The marketplace is a public GitHub repo. API keys, tokens, and credentials never appear in skills, commands, hooks, or plugin configs.

---

## Gaps

Gaps identified during design doc writing.

| Gap | Module | Description | Blocks |
|-----|--------|-------------|--------|
| G1 | Observatory | No daemon API endpoints designed for observatory screens (metrics, teachings, memory) | Observatory UI |
| G2 | Setup | Inference + Assignments wizard steps blocked on gateway integration | Setup completion |
| G3 | Memory | Memory consolidation described in ideas/03 but daemon implementation not designed | Observatory coaching |
| G4 | MCP | Multi-coordinator adapter implementations not designed beyond Claude Code | Cursor/Zed support |
| G5 | App | Desktop app distribution mechanism undefined (macOS DMG? Cask only?) | Release process |
| G6 | Website | Architecture diagram needs update from old D3 version (referenced deprecated tech) | Website accuracy |
| G7 | Daemon | PgStore error handling (S3 from issues catalog) — 110 sites erase sqlx::Error to String | Error recovery |
| G8 | Gateway | Voice chain (STT/TTS) described in ideas but no model selection or streaming design | Voice features |
| G9 | Logging | sensei-logger crate exists but log viewer API endpoints not implemented | Log viewer screen |
| G10 | Daemon | Benchmarking framework described but no task corpus or evaluation harness built | Credibility metrics |
