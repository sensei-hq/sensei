# Documentation Restructure — Design Spec

> Replace the sprawling docs/ tree with two clean layers: **ideas/** (what Sensei does, user perspective) and **design/** (how it's built, engineering perspective). Archive everything else.

## Goals

1. Single authoritative source for "what is this product" (ideas/) and "how is it built" (design/)
2. Remove all outdated references to Supabase, SQLite, Kuzu, and previous iterations
3. Synthesize content from ideas, journeys, features, mockups, and blueprints into consolidated docs
4. Identify gaps in the overall product vision
5. Clean, scannable structure — flat files, numbered for ordering

## Final Structure

```
docs/
  ideas/                         ← "What Sensei does" (user perspective)
    README.md                    ← Product overview + status table
    01-bootstrap.md
    02-setup.md
    03-observatory.md
    04-project.md
    05-gateway.md
    06-logging.md
  design/                        ← "How it's built" (engineering, references ideas/)
    README.md                    ← Architecture diagram + layers + principles + component map
    01-app.md
    02-daemon.md
    03-gateway.md
    04-mcp.md
    05-cli.md
    06-marketplace.md
    07-logging.md
    08-website.md
    09-homebrew.md
    10-build-and-release.md
  mockups/                       ← Retained as visual reference
  archive/                       ← Old docs moved here for review/removal
  backlog.md                     ← Stays (active work tracker)
  README.md                      ← Updated to reflect new structure
```

## Relationship Between ideas/ and design/

- **ideas/** = "what the user sees and does" (product perspective)
- **design/** = "how it's built" (engineering perspective, references ideas/ for the "what")
- Clean separation, no duplication. A developer reads both.
- Design docs never re-explain user-facing behavior — they link to the relevant ideas/ doc.

## Content Plan: ideas/

### `ideas/README.md` — Product Overview

- **What is Sensei** — Developer tool that observes AI-assisted coding sessions, learns patterns, and coaches developers toward better outcomes.
- **Core modules** — Brief description of each with cross-links:
  - Bootstrap — get running
  - Setup — configure your workspace
  - Observatory — observe, measure, learn
  - Project — understand and work with your code
  - Gateway — inference routing, model management
  - Logging — diagnostics, debug, issue reporting
- **How Sensei works** — The invisible layer: hooks capture events from AI assistants (Claude Code, Cursor, etc.), daemon indexes code and computes analytics, desktop shows insights. User never interacts with sensei during coding — it watches and teaches.
- **Key concepts** — FTR (First-Time-Right), teachings, corrections, sessions, patterns, context delivery
- **Status table** — Per-module build status:

| Module | Status | Notes |
|--------|--------|-------|
| Bootstrap | Buggy | 6 gates implemented, needs stabilization |
| Setup | Buggy/Partial | Welcome–Roots done, Scan–Libraries partial |
| Setup: Instruments–Assignments | Not started | Needs gateway |
| Observatory | Not started | |
| Project | Partial | Indexing pipeline in progress |
| Gateway | Not started | Design complete |
| Logging | Not started | Design in progress |

### `ideas/01-bootstrap.md`

Synthesizes from: ideas/26, journeys/01, design/bootstrap.md, mockups (bootstrap artboard), backlog

- **What the user sees** — App launches → dependency screen with 6 gates (Homebrew, PostgreSQL, Ollama, Sensei components, Database, Daemon). Each shows status, auto-installs if missing, progress feedback.
- **Repeat launches** — Health check in <2s. Only shows bootstrap screen if something is wrong.
- **Upgrades** — Detects version changes, re-runs relevant gates. Reconfigure assistants after upgrade.
- **What can go wrong** — Gate failures, partial installs, recovery. Diagnostic logging and debug mode.
- **Screens** — Reference mockup artboard. Describe each gate's UI state (checking → installing → ready → error).

### `ideas/02-setup.md`

Synthesizes from: ideas/03, ideas/27, journeys/02, design/configure.md, mockups (setup artboard), features/01-02

- **When it runs** — First launch after bootstrap completes. Can be re-entered from settings.
- **Steps** (10-step wizard, in order):
  1. Welcome — what sensei is, what to expect
  2. Preferences — coding style, communication preferences
  3. Scan Roots — pick folders to watch
  4. Scan — live SSE progress (projects discovered, files indexed, activity log)
  5. Projects — group/rename discovered projects, set solution groupings
  6. Libraries — identify shared/third-party libraries, configure docs sources
  7. Instruments — MCP tool registry, which tools sensei can use per assistant
  8. Inference — configure local vs external models, hardware-aware recommendations, budget limits, task-to-model mapping
  9. Assignments — map assistants to roles, which assistant handles which workflow phase, default behaviors per project
  10. Done — summary, enter observatory
- **What the user learns** — Sensei now knows their workspace. They understand projects, libraries, instruments, and how inference is configured.

### `ideas/03-observatory.md`

Synthesizes from: ideas/07, ideas/10, ideas/24, ideas/25, ideas/29, ideas/30, journeys/03, journeys/06, journeys/09, mockups (observatory artboard), features/05

- **Daily view** — What the user opens each day. Hero insight (koan), FTR trend, recent sessions, adopted teachings.
- **Early mode vs Mature mode** — <5 sessions: listening, building baseline, minimal insights. 5+ sessions: coaching, recommendations, trend analysis.
- **Sessions** — Session list, drill into timeline (events, tool calls, corrections, phase transitions). Replay what happened.
- **Teachings & Coaching** — Recommendations ranked by impact. Evidence trail. Action drawer with pre-built prompts. 7-day impact measurement with verdict (Positive/Neutral/Negative).
- **Memory** — Corrections → teachings → consolidated knowledge. Memory detail view with evidence. Context assembly for session startup.
- **Metrics** — FTR trend, correction rate per module, tool usage, rework patterns. Drill into any metric.
- **Settings/Config** — Accessible from observatory sidebar. Re-run setup steps, adjust preferences.

### `ideas/04-project.md`

Synthesizes from: ideas/08, ideas/09, ideas/13-18, ideas/21-23, ideas/31, journeys/04-05, journeys/07, features/01-04

- **Project dashboard** — Per-project view. Code graph, patterns, libraries, sessions, traceability.
- **Code intelligence** — Symbol graph (3 lens modes, 5 overlays), complexity heatmaps, call chains. Powered by indexing pipeline + local inference.
- **Patterns** — Auto-detected from code + industry catalogs. Enforcement during build (AI finds patterns before coding) and review (catches violations). Pattern lifecycle: detect → surface → enforce → grow.
- **Search & Context** — Hybrid search (full-text + semantic + structural). Context delivery at resolution levels (L0 signature → L3 full source). Token-budgeted, task-relevant.
- **Libraries** — Third-party and internal library intelligence. Docs, APIs, usage patterns. AI uses current docs instead of hallucinating.
- **Traceability** — Docs linked to code. Drift detection when code changes but docs don't. Coverage matrix.
- **Testability** — Testability scoring. Decomposition guidance (pure logic vs side effects). TDD enforcement: tests before implementation.
- **Working with assistants** — The invisible session: workflow phases (brainstorm → build → validate), context injection (~300 tokens), correction detection, post-session analytics. User doesn't see sensei during coding — it works through the assistant.
- **Extensions** — Skills browser, agent editor, persona editor. Customize how sensei behaves per project.

### `ideas/05-gateway.md`

Synthesizes from: ideas/20, ideas/28, ideas/gateway/*, journeys/gateway/*

- **What the user sees** — Not a standalone screen, but a capability that surfaces everywhere: model selection in setup, cost tracking in observatory, search quality in project.
- **Configuring inference** — Local models (Ollama), external providers (Anthropic, OpenAI, Google). Per-task routing: embedding stays local, chat goes external, classification uses cheapest.
- **Budget management** — Daily/monthly limits. Spend tracking visible in observatory. Local-first degradation when budget exhausted.
- **Provider health** — Circuit breaker status. Fallback chains. What happens when a provider goes down.
- **Consensus** — MOE panel for high-stakes decisions. Multiple models weigh in, results synthesized.
- **Voice** — STT/TTS capabilities. Whisper for transcription, streaming audio.

### `ideas/06-logging.md`

Synthesizes from: ideas/29 (telemetry), backlog (diagnostic logging section)

- **Bootstrap diagnostics** — Structured trace logging during install/startup. What went wrong and where.
- **Log viewer** — Filterable log screen. Levels, components, time ranges. Tail mode for live watching.
- **Debug mode** — Toggle from settings. Increases verbosity. Shows daemon internals, API calls, indexing progress.
- **Issue submission** — Package logs + system info into a GitHub issue template. One-click from the app.
- **Session traces** — Per-session event log. What hooks fired, what MCP calls happened, what the daemon processed.

## Content Plan: design/

### `design/README.md` — Architecture Overview

#### Architecture Diagram

New SVG showing current component topology:

```
[Desktop App (Tauri + SvelteKit)]
    ├── sidecar → [Daemon (senseid)]
    └── UI calls → [Daemon HTTP API]

[AI Assistants: Claude Code, Cursor, ...]
    └── hooks → [MCP (sensei-mcp)] → [Daemon HTTP API]

[CLI (sensei)]
    └── → [MCP | Daemon HTTP API]

[Daemon (senseid)]
    ├── → [PostgreSQL (pgvector + AGE)]
    ├── → [Gateway (inference routing)]
    │       ├── → [Ollama (local)]
    │       └── → [API providers (external)]
    ├── → [Marketplace (GitHub)]
    └── → [Homebrew (distribution)]
```

Note: Old D3 architecture diagram was in the sensei-site repo (formerly apps/static). It references Kuzu/SQLite and needs replacement.

#### Layered Architecture

- Desktop App → MCP → Daemon → PostgreSQL
- Gateway as cross-cutting inference layer
- Marketplace as the plugin surface

#### Tech Stack

- Rust: daemon (senseid), MCP (sensei-mcp), CLI (sensei), gateway
- Tauri + SvelteKit: desktop app
- PostgreSQL + pgvector + Apache AGE: all persistent state
- Ollama: local inference (embedding, classification, summarization)
- Claude Code plugin system: marketplace (commands, skills, hooks)

#### Component Map

Brief description of each component with cross-refs to ideas/ for the "what":

| Component | Binary/Package | Port | Purpose | Ideas Ref |
|-----------|---------------|------|---------|-----------|
| App | sensei (Tauri) | — | Desktop observatory | ideas/01-06 |
| Daemon | senseid | 7744/7745 | Core engine, API, indexing, analytics | ideas/01-04 |
| Gateway | sensei-gateway (crate) | — | LLM routing, inference | ideas/05 |
| MCP | sensei-mcp | stdio | AI assistant interface | ideas/04 |
| CLI | sensei | — | Manual operations, benchmarking | ideas/02 |
| Marketplace | — | — | Commands, skills, hooks, plugins | ideas/03-04 |
| Website | sensei-site | — | Marketing, docs | — |
| Homebrew | sensei tap | — | Distribution | — |

#### Data Flow

- Hooks → daemon events → analytics → observatory
- Indexing: file system → adapters (tree-sitter) → IR → graph → intelligence
- Context: task → budget → resolution selection → delivery to MCP → AI assistant

#### Principles

**Design Principles**
- Single source of truth — PostgreSQL owns all persistent state
- Agent-agnostic — MCP interface works with any AI assistant
- Local-first — Prefer local inference, degrade gracefully when unavailable
- Zero-config start — Works with just Ollama, no API keys needed
- Invisible integration — User doesn't interact with sensei during coding sessions
- Token efficiency — Serve context at the right resolution level (L0-L3)

**Rules**
- Compile-time mode separation — dev/release via Cargo feature flag, not env vars
- No direct daemon calls from AI — always through MCP
- No code in marketplace — markdown commands/skills only
- Desktop observes, never blocks — observatory is read-only, never interrupts workflows
- DRY — reuse over duplication, modular and testable
- No silent workarounds — fix root causes

**Patterns**
- Adapter IR — Common intermediate representation across all language parsers
- Sidecar lifecycle — Tauri manages daemon process
- SSE for progress — Long-running operations stream events to UI
- Hook-based capture — Events flow from AI assistants via hook system
- Chain-based routing — Gateway uses fallback chains per capability
- Resolution levels — L0 (signature) → L1 (IO pattern) → L2 (logic flow) → L3 (full source)

**Non-Functional Requirements**
- Bootstrap health check < 2s on repeat launches
- Context delivery within token budget per request
- Graceful degradation when Ollama or external providers unavailable
- All state survives app restart (PostgreSQL-backed)
- Debug/release fully isolated (ports, databases, directories)
- No secrets in marketplace artifacts

### `design/01-app.md`

Covers: Tauri desktop, SvelteKit frontend, sidecar lifecycle, state management

- **Architecture** — Tauri shell wrapping SvelteKit. Sidecar manages daemon lifecycle.
- **Bootstrap flow** — App launch → sidecar starts → dependency checks → daemon startup. SSE for progress.
- **Setup wizard** — Step-by-step data flow. Each step's API calls, state transitions, SSE events.
- **Observatory screens** — Component structure, data fetching, reactive state.
- **Project views** — Graph visualization, pattern display, session replay.
- **State management** — Svelte 5 runes, stores, how app state maps to daemon API responses.
- **Styling** — Rokkit design system, token palette (Paper, Sumi, Shu, Jade, Amber, Matcha), type scale, spacing system.
- References: `ideas/01-bootstrap`, `ideas/02-setup`, `ideas/03-observatory`, `ideas/04-project`

### `design/02-daemon.md`

Covers: senseid binary, API, indexing, task queue, intelligence, analytics, events

- **Architecture** — Rust binary. HTTP API on port 7744 (release) / 7745 (dev). PostgreSQL for all state.
- **API surface** — Endpoint inventory organized by screen (bootstrap, setup, observatory, project). Request/response contracts. SSE endpoints for scan progress.
- **Indexing pipeline** — ScanRoot → ProcessRepo → ProcessFolder → ProcessFile → ResolveEdges → BuildConnections. Adapter IR pattern. Language adapters (tree-sitter AST → IR nodes).
- **Task queue** — Hierarchical queue with LISTEN/NOTIFY. Parallelism, incremental updates, priority.
- **Intelligence layer** — Context compression (L0-L3), pattern store, context manager, metadata model, semantic search (pgvector).
- **Analytics** — Event capture (16 event types), FTR computation, correction classification, session lifecycle, project memory.
- **Traceability** — Doc-to-code linking, drift detection via git diff, traceability matrix.
- **Database schema** — Schema overview (5 schemas: gateway, sensei, inference, activity, history, ~39 tables). Key tables and relationships. Enum reference.
- References: `ideas/03-observatory`, `ideas/04-project`

### `design/03-gateway.md`

Covers: inference routing library, chains, circuit breaker, budget, consensus

- **Architecture** — Standalone Rust crate. Engine → selection → adapters → execution. Port of Strategos concepts.
- **Model selection** — 3-tier resolution (reasoning, cost, fallback). Chain walking, candidate validation.
- **Chains** — Inference, embedding, chat, consolidation, voice. Each chain's fallback behavior (local → external).
- **Adapters** — Provider abstraction. Ollama adapter, API adapters (Anthropic, OpenAI, Google). Noop adapter for graceful degradation.
- **Circuit breaker** — Per-endpoint failure tracking, state machine, auto-recovery.
- **Budget** — Cost estimation, daily/monthly limits, spend tracking, local-first degradation.
- **Consensus** — MOE panel. Multiple models, synthesized results.
- **Data model** — `inference_calls` and `execution_traces` tables.
- **Integration** — Daemon lifecycle, health endpoints, MCP tool exposure.
- References: `ideas/05-gateway`

### `design/04-mcp.md`

Covers: sensei-mcp binary, tool contracts, transport

- **Architecture** — Rust binary, stdio transport. Translates MCP tool calls → daemon HTTP. AI never calls daemon directly.
- **Tool contracts** — Phase 1 (code intelligence): search, context, compression, session_context. Phase 2 (workflow): state, events, metrics, patterns. Full tool signatures and responses.
- **REPO_PATH resolution** — How MCP determines which project context to use.
- **Multi-coordinator** — How the same MCP server works with Claude Code, Cursor, others. Adapter pattern for event capture differences.
- References: `ideas/04-project`

### `design/05-cli.md`

Covers: sensei CLI commands, profiles, hooks

- **Architecture** — `sensei` binary. Commands: init, add, upgrade, status, guidelines, cache, hooks.
- **Profile system** — Global + project-level layering.
- **Hook integration** — Pre-commit drift detection. How CLI plugs into git workflows.
- **Interactive UX** — Prompt patterns for terminal interaction.

### `design/06-marketplace.md`

Covers: commands, skills, hooks, plugins, auto-triggering

- **Architecture** — Claude Code plugin layer. Markdown-based commands and skills.
- **Commands** — 13 commands: 7 phase-based (brainstorm → build), 2 cross-cutting (review, validate), 4 refocus (rules, patterns, refocus, tools).
- **Hooks** — Pre-compact, user-prompt, session-start. Event capture integration with daemon.
- **Skills** — Format, frontmatter, triggering. How skills deliver context without overwhelming token budgets.
- **Auto-triggering** — How marketplace behaviors activate based on workflow state and session context.
- References: `ideas/03-observatory`, `ideas/04-project`

### `design/07-logging.md`

Covers: structured tracing, log viewer, debug mode, diagnostics

- **Architecture** — Structured trace logging across all Rust binaries (senseid, sensei-mcp, sensei). Log levels, component tags.
- **Bootstrap diagnostics** — Trace capture during install/startup. Error categorization and recovery hints.
- **Log viewer** — API endpoints for log retrieval. Filtering by level/component/time. Tail mode for streaming.
- **Debug mode** — Compile-time vs runtime verbosity. What gets exposed in each mode.
- **Issue submission** — Log packaging, system info collection, GitHub template generation.
- References: `ideas/06-logging`

### `design/08-website.md`

Covers: marketing/docs site

- **Architecture** — SvelteKit static site (separate sensei-site repo).
- **Content** — Landing page, docs, FAQ, privacy, changelog.
- **Architecture diagram** — Site hosts the interactive architecture visualization (needs update from old D3 version that referenced Kuzu/SQLite).
- **Relationship to docs/** — Website docs reference ideas/ for product narrative.

### `design/09-homebrew.md`

Covers: formula, tap, cask, distribution

- **Tap structure** — Formula for daemon + CLI, cask for desktop app.
- **Dependencies** — What Homebrew manages (install) vs what bootstrap manages (verify + configure).
- **Versioning** — How VERSION file drives formula updates.
- **Release flow** — Tag → build → update formula → publish.

### `design/10-build-and-release.md`

Covers: Cargo features, debug/release, versioning, CI

- **Debug vs Release** — Compile-time `dev` feature. Port (7745/7744), DB name (sensei_dev/sensei), directory (~/.sensei-dev/~/.sensei) separation.
- **Version management** — VERSION file as source of truth. Bump propagation across all crates and packages.
- **Build targets** — Daemon, MCP, CLI, app, website. Build order and dependencies.
- **Release process** — Tagging, homebrew formula update, distribution channels.
- **Makefile** — Key targets: setup-hooks, install-dev, app-dev, test, lint.

## Archive Plan

Move to `docs/archive/`:
- `docs/ideas/` (entire current directory — replaced by new ideas/)
- `docs/journeys/` (absorbed into new ideas/)
- `docs/features/` (absorbed into new ideas/)
- `docs/blueprints/` (absorbed into new design/)
- `docs/design/` (entire current directory — replaced by new design/)
- `docs/analysis/`
- `docs/reference/`

Delete outright:
- `docs/experiments/`
- `docs/sessions/`
- `docs/plans/`
- `docs/llms/`
- `docs/feature-requests/`
- `docs/superpowers/` (conversation-specific specs — move to archive/ first, delete after confirming nothing is referenced)

Retain:
- `docs/mockups/` (visual reference)
- `docs/backlog.md` (active work)
- `docs/README.md` (updated)

## Outdated References to Remove

All new documents must have zero references to:
- **Supabase** — replaced by self-hosted PostgreSQL
- **SQLite** — replaced by PostgreSQL (per ADR-005)
- **Kuzu** — replaced by PostgreSQL + Apache AGE (per ADR-005)
- **better-sqlite3** — removed
- **.llmspec.yaml** — superseded by metadata model
- **TypeScript/Bun daemon** — rewritten in Rust
- **apps/static** or **apps/dashboard** — old site structure

## Gap Identification

During the writing of each document, explicitly note gaps where:
- User-facing behavior is described but no engineering design exists
- Engineering design exists but user-facing behavior is unclear
- A feature is referenced across multiple modules but has no single owner
- Status is "not started" but blocks other modules

Gaps will be collected into a section at the bottom of `design/README.md` and/or flagged inline in each document.

## Implementation Order

1. Create `docs/archive/` and move old docs there
2. Write `ideas/README.md` + all 6 idea docs
3. Write `design/README.md` + all 10 design docs
4. Update `docs/README.md` to reflect new structure
5. Update `docs/backlog.md` if needed
6. Create architecture SVG diagram
7. Final review for outdated references
8. Commit and push
