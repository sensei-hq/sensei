# Documentation Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the sprawling docs/ tree with two clean layers — ideas/ (user perspective) and design/ (engineering perspective) — archiving everything else.

**Architecture:** Flat file structure. ideas/ has 6 module docs + README. design/ has 10 component docs + README. Old docs move to archive/. No code changes — documentation only.

**Tech Stack:** Markdown, git

**Spec:** `docs/superpowers/specs/2026-05-12-docs-restructure-design.md`

**Source material locations:**
- Old ideas: `docs/ideas/` (31 files + gateway/ subfolder with 14 files)
- Old design: `docs/design/` (30+ files across 10 subdirectories)
- Journeys: `docs/journeys/` (13 files)
- Features: `docs/features/` (6 files)
- Blueprints: `docs/blueprints/` (4 files)
- Mockups: `docs/mockups/summary.md`, `docs/mockups/transcript.md`
- Reference: `docs/reference/` (3 files)
- Analysis: `docs/analysis/` (4 files)
- Backlog: `docs/backlog.md`

**Banned terms in new docs:** Supabase, SQLite, Kuzu, better-sqlite3, .llmspec.yaml, TypeScript/Bun daemon, apps/static, apps/dashboard

---

### Task 1: Archive old docs and clean up

Move existing docs to archive, delete disposable directories.

**Files:**
- Create: `docs/archive/` (directory)
- Move: `docs/ideas/` → `docs/archive/ideas/`
- Move: `docs/design/` → `docs/archive/design/`
- Move: `docs/journeys/` → `docs/archive/journeys/`
- Move: `docs/features/` → `docs/archive/features/`
- Move: `docs/blueprints/` → `docs/archive/blueprints/`
- Move: `docs/reference/` → `docs/archive/reference/`
- Move: `docs/analysis/` → `docs/archive/analysis/`
- Move: `docs/superpowers/` → `docs/archive/superpowers/`
- Delete: `docs/experiments/`, `docs/sessions/`, `docs/plans/`, `docs/llms/`, `docs/feature-requests/`

- [ ] **Step 1: Create archive directory and move docs**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei/docs
mkdir -p archive
mv ideas archive/ideas
mv design archive/design
mv journeys archive/journeys
mv features archive/features
mv blueprints archive/blueprints
mv reference archive/reference
mv analysis archive/analysis
mv superpowers archive/superpowers
```

- [ ] **Step 2: Delete disposable directories**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei/docs
rm -rf experiments sessions plans llms feature-requests
```

- [ ] **Step 3: Create empty new directories**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei/docs
mkdir -p ideas design
```

- [ ] **Step 4: Verify structure**

```bash
ls /Users/Jerry/Developer/sensei-hq/sensei/docs/
```

Expected: `archive/  backlog.md  design/  ideas/  mockups/  README.md  task.md`

- [ ] **Step 5: Commit**

```bash
git add -A docs/
git commit -m "docs: archive old docs, create new ideas/ and design/ directories"
```

---

### Task 2: Write ideas/README.md — Product Overview

**Files:**
- Create: `docs/ideas/README.md`
- Read (source material):
  - `docs/archive/ideas/README.md`
  - `docs/archive/journeys/README.md`
  - `docs/mockups/summary.md`

- [ ] **Step 1: Read source material**

Read the three source files listed above to understand the current product overview, journey map, and mockup coverage.

- [ ] **Step 2: Write ideas/README.md**

Write the product overview document with these sections:
- **What is Sensei** — one paragraph: developer tool that observes AI-assisted coding sessions, learns patterns, coaches toward better outcomes
- **Core modules** — brief description with cross-links to 01-06:
  - Bootstrap (01) — get running: install dependencies, verify health, handle upgrades
  - Setup (02) — configure your workspace: 10-step wizard from welcome to done
  - Observatory (03) — observe, measure, learn: daily dashboard, sessions, coaching, memory
  - Project (04) — understand and work with your code: intelligence, patterns, search, traceability
  - Gateway (05) — inference routing: model config, budget, providers, consensus
  - Logging (06) — diagnostics: log viewer, debug mode, issue submission
- **How Sensei works** — the invisible layer: hooks capture events from AI assistants (Claude Code, Cursor, etc.), daemon indexes code and computes analytics, desktop shows insights. User never interacts with sensei during coding — it watches and teaches.
- **Key concepts** — define each: FTR (First-Time-Right), teachings (recommendations adopted from coaching), corrections (when user overrides AI), sessions (one AI coding session), patterns (detected code patterns), context delivery (token-budgeted code served to AI)
- **Status** table:

| Module | Status | Notes |
|--------|--------|-------|
| Bootstrap | Buggy | 6 gates implemented, needs stabilization |
| Setup | Buggy/Partial | Welcome–Roots done, Scan–Libraries partial |
| Setup: Instruments–Assignments | Not started | Needs gateway |
| Observatory | Not started | |
| Project | Partial | Indexing pipeline in progress |
| Gateway | Not started | Design complete |
| Logging | Not started | Design in progress |

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/ideas/README.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/ideas/README.md
git commit -m "docs: add ideas/README.md — product overview and status"
```

---

### Task 3: Write ideas/01-bootstrap.md

**Files:**
- Create: `docs/ideas/01-bootstrap.md`
- Read (source material):
  - `docs/archive/ideas/26-bootstrap-and-dependencies.md`
  - `docs/archive/journeys/01-install-bootstrap.md`
  - `docs/archive/design/bootstrap.md`
  - `docs/archive/design/api/01-bootstrap.md`
  - `docs/mockups/summary.md` (bootstrap artboard section)
  - `docs/backlog.md` (bootstrap section)

- [ ] **Step 1: Read source material**

Read all six source files. Extract: the 6-gate sequence, component states, UI mockup descriptions, upgrade flow, health check behavior, known issues.

- [ ] **Step 2: Write ideas/01-bootstrap.md**

Write user-perspective bootstrap document with these sections:

- **Title + intro** — `# Bootstrap` — First thing the user sees. Sensei checks and installs everything needed to run.
- **The 6 gates** — describe each from user perspective:
  1. Homebrew — package manager (macOS). Installs if missing, verifies if present.
  2. PostgreSQL — database. Installs via Homebrew, creates sensei database, verifies connection.
  3. Ollama — local inference engine. Installs, pulls required models (embedding, classification).
  4. Sensei components — daemon, MCP server, CLI. Installs/upgrades via Homebrew tap.
  5. Database — runs migrations, verifies schema version. Uses `dbd` for reset/apply.
  6. Daemon — starts senseid process, verifies health endpoint responds.
- **What the user sees** — screen layout: list of gates, each with status indicator (checking → installing → ready → error). Progress bar or spinner per gate. Activity log at bottom showing what's happening.
- **Repeat launches** — health check runs on every launch, completes in <2s when everything is healthy. Bootstrap screen only appears if a gate fails.
- **Upgrades** — when VERSION changes: re-check Sensei components gate, re-run database migrations, restart daemon. User sees a brief upgrade flow, not the full wizard.
- **Error recovery** — what happens when a gate fails: shows error message, retry button, diagnostic log access. User can skip non-critical gates (Ollama) and continue with degraded functionality.
- **Reference** — link to mockups for visual reference. Note: see `design/01-app.md` for implementation details.

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/ideas/01-bootstrap.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/ideas/01-bootstrap.md
git commit -m "docs: add ideas/01-bootstrap.md — user-facing bootstrap experience"
```

---

### Task 4: Write ideas/02-setup.md

**Files:**
- Create: `docs/ideas/02-setup.md`
- Read (source material):
  - `docs/archive/ideas/03-configuration.md`
  - `docs/archive/ideas/27-developer-preferences.md`
  - `docs/archive/journeys/02-setup-discovery.md`
  - `docs/archive/design/configure.md`
  - `docs/archive/design/api/02-scan-events.md`
  - `docs/archive/design/api/03-scan-event-flow.md`
  - `docs/archive/features/01-workflow-commands.md`
  - `docs/archive/features/02-rules-context.md`
  - `docs/mockups/summary.md` (setup wizard artboard)
  - `docs/backlog.md` (setup wizard section)

- [ ] **Step 1: Read source material**

Read all source files. Extract: the 10-step wizard flow, per-step data model and user interactions, scan SSE event format, configuration model, mockup screen descriptions.

- [ ] **Step 2: Write ideas/02-setup.md**

Write user-perspective setup document with these sections:

- **Title + intro** — `# Setup` — First-run wizard after bootstrap. Configures Sensei for your workspace. Can be re-entered from settings anytime.
- **Overview** — 10 steps in sequence. User can go back to any previous step. Progress indicator shows current position.
- **Step 1: Welcome** — What sensei is, what to expect. Brief product intro. "Get started" button.
- **Step 2: Preferences** — Coding style preferences (naming conventions, indentation, file organization). Communication style (terse/verbose, explanation depth). These inform how sensei coaches.
- **Step 3: Scan Roots** — Pick folders to watch. File browser to add/remove root directories. Sensei will scan these for projects. Default: home directory developer folders.
- **Step 4: Scan** — Live progress as sensei discovers projects. SSE-driven updates: projects found (cards appearing), files processed (progress bar), activity log (scrolling event feed). User watches but doesn't need to interact. Can take minutes for large codebases.
- **Step 5: Projects** — Review discovered projects. Group related repos into solutions. Rename, merge, or exclude projects. Set project-level preferences.
- **Step 6: Libraries** — Identify shared and third-party libraries. Configure documentation sources for each. Sensei uses these to serve accurate library context instead of hallucinating.
- **Step 7: Instruments** — MCP tool registry. Which MCP tools sensei can use, per assistant. Register additional tool servers. Configure tool permissions.
- **Step 8: Inference** — Configure local vs external models. Hardware-aware recommendations (GPU, RAM). Budget limits (daily/monthly spend caps). Task-to-model mapping: which model handles embedding, classification, chat, summarization.
- **Step 9: Assignments** — Map assistants to roles. Which AI assistant handles which workflow phase (analysis, coding, review). Default behaviors per project. Role-based context delivery.
- **Step 10: Done** — Summary of what was configured. Project count, library count, model count. "Enter Observatory" button.
- **Re-entering setup** — Any step can be revisited from observatory settings. Changes take effect immediately.
- **Reference** — link to mockups. Note: see `design/01-app.md` for wizard state architecture and data flow.

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/ideas/02-setup.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/ideas/02-setup.md
git commit -m "docs: add ideas/02-setup.md — 10-step setup wizard"
```

---

### Task 5: Write ideas/03-observatory.md

**Files:**
- Create: `docs/ideas/03-observatory.md`
- Read (source material):
  - `docs/archive/ideas/07-metrics-analytics.md`
  - `docs/archive/ideas/10-visualization.md`
  - `docs/archive/ideas/24-desktop-observatory.md`
  - `docs/archive/ideas/25-playground-and-insights.md`
  - `docs/archive/ideas/29-telemetry.md`
  - `docs/archive/ideas/30-contextual-memory.md`
  - `docs/archive/journeys/03-observe-orient.md`
  - `docs/archive/journeys/06-measure-improve.md`
  - `docs/archive/journeys/09-memory-and-learning.md`
  - `docs/archive/features/05-quality-coaching.md`
  - `docs/mockups/summary.md` (observatory artboard)

- [ ] **Step 1: Read source material**

Read all source files. Extract: observatory screen layout, early vs mature mode, session timeline, coaching workflow, memory lifecycle, metrics definitions, mockup descriptions.

- [ ] **Step 2: Write ideas/03-observatory.md**

Write user-perspective observatory document with these sections:

- **Title + intro** — `# Observatory` — Your daily view into AI-assisted development. Sensei watches your sessions and surfaces what matters.
- **Daily view** — what the user opens each day: hero insight (a "koan" — one sentence teaching), FTR trend chart, recent sessions list, adopted teachings summary.
- **Early mode** (<5 sessions) — Sensei is listening. Building baseline. Minimal insights shown. "Learning your patterns" indicator. User sees sessions accumulating but no coaching yet.
- **Mature mode** (5+ sessions) — Sensei is teaching. Full coaching active. Recommendations appear. Trend analysis meaningful. Impact measurement begins.
- **Sessions** — session list showing: date, duration, project, FTR score, correction count. Drill into any session to see timeline: events (tool calls, file edits, corrections), phase transitions, what sensei observed. Replay what happened chronologically.
- **Teachings & Coaching** — recommendations ranked by estimated impact (high/medium/low) and urgency. Each has:
  - Evidence trail — which sessions and corrections led to this recommendation
  - Action drawer — pre-built prompts the user can copy into their AI assistant
  - Impact measurement — 7-day window after adoption. Verdict: Positive (FTR improved), Neutral (no change), Negative (FTR worsened). Negative triggers MOE reasoning trace explaining why.
- **Memory** — the knowledge sensei builds over time:
  - Corrections → individual observations from sessions
  - Teachings → consolidated patterns from multiple corrections
  - Consolidated knowledge — merged, deduplicated, strengthened with evidence
  - Memory detail view: evidence trail, strength indicator, related sessions
  - Context assembly: what sensei sends to the AI assistant at session start (~300 tokens)
  - Context pack tool: mid-session rotation of relevant memories
- **Metrics** — quantitative views:
  - FTR trend (first-time-right rate over time)
  - Correction rate per module (which areas of code cause most rework)
  - Tool usage (which MCP tools are used, how effectively)
  - Rework patterns (common correction types, recurring issues)
  - Drill into any metric for per-session breakdown
- **Settings** — accessible from sidebar:
  - Re-run any setup wizard step
  - Adjust preferences (coding style, communication)
  - Configure notifications
  - Debug mode toggle
- **Reference** — link to mockups. Note: see `design/01-app.md` for component architecture, `design/02-daemon.md` for analytics engine.

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/ideas/03-observatory.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/ideas/03-observatory.md
git commit -m "docs: add ideas/03-observatory.md — daily dashboard, coaching, memory"
```

---

### Task 6: Write ideas/04-project.md

**Files:**
- Create: `docs/ideas/04-project.md`
- Read (source material):
  - `docs/archive/ideas/08-codebase-intelligence.md`
  - `docs/archive/ideas/09-library-intelligence.md`
  - `docs/archive/ideas/13-doc-traceability.md`
  - `docs/archive/ideas/14-context-delivery.md`
  - `docs/archive/ideas/15-pattern-store.md`
  - `docs/archive/ideas/16-workspace-system-intelligence.md`
  - `docs/archive/ideas/17-pattern-knowledge.md`
  - `docs/archive/ideas/18-testability-tdd.md`
  - `docs/archive/ideas/21-custom-agents.md`
  - `docs/archive/ideas/22-adapter-ir.md`
  - `docs/archive/ideas/23-personas-mindsets.md`
  - `docs/archive/ideas/31-semantic-search-layer.md`
  - `docs/archive/journeys/04-work-with-assistants.md`
  - `docs/archive/journeys/05-understand-codebase.md`
  - `docs/archive/journeys/07-extend-customize.md`
  - `docs/archive/features/01-workflow-commands.md` through `04-tdd-testability.md`

- [ ] **Step 1: Read source material**

Read all source files. Extract: project dashboard layout, code graph visualization, pattern lifecycle, search capabilities, context delivery levels, library intelligence, traceability, testability scoring, assistant interaction model, extensions system.

- [ ] **Step 2: Write ideas/04-project.md**

Write user-perspective project document with these sections:

- **Title + intro** — `# Project` — Deep view into a single project. Understand your code, its patterns, and how AI assistants work with it.
- **Project dashboard** — per-project entry point. Tabs: Overview, Code Graph, Patterns, Libraries, Sessions, Traceability, Extensions.
- **Code intelligence** — symbol graph visualization:
  - 3 lens modes: dependency (import graph), call (function calls), inheritance (type hierarchy)
  - 5 overlays: complexity heatmap, test coverage, change frequency, age, ownership
  - Interactive: click nodes to see details, zoom to module/file/function level
  - Call chain exploration: trace callers/callees through the graph
  - Powered by indexing pipeline (tree-sitter AST → Adapter IR → PostgreSQL graph)
- **Patterns** — auto-detected code patterns:
  - Detection: naming conventions, structural analysis, industry catalog matching (GoF, patterns.dev)
  - Lifecycle: detect → surface (show to user) → enforce (AI checks before coding) → grow (user confirms/extends)
  - Pattern catalog: per-project list with examples, usage count, locations
  - Enforcement: during build phase AI finds applicable patterns before writing code; during review phase AI catches violations
  - First encounter: when AI sees a pattern for the first time, presents options to user
- **Search & Context delivery** — how sensei serves code to AI assistants:
  - Hybrid search: full-text (PostgreSQL tsvector) + semantic (pgvector embeddings) + structural (graph queries)
  - Resolution levels: L0 (signature only) → L1 (IO pattern) → L2 (logic flow) → L3 (full source)
  - Token budget: sensei picks the right resolution to fit the task's token budget
  - Task-relevant: sensei uses the current task context to prioritize which code to serve
- **Libraries** — third-party and internal library intelligence:
  - Registered during setup, enriched by indexing
  - Docs, APIs, usage patterns per library
  - AI uses current library docs instead of hallucinating outdated APIs
  - Library skill generation: sensei creates skills from library documentation
- **Traceability** — linking docs to code:
  - Traceability matrix: which docs cover which source files
  - Drift detection: when code changes but linked docs don't, sensei flags the drift
  - Coverage: which code has documentation, which doesn't
  - Doc tools: find existing docs, scaffold new ones, reformat without losing content
- **Testability** — code quality scoring:
  - Testability score per function: based on parameter count, side effects, complexity, existing test coverage
  - Decomposition guidance: suggest splitting impure functions into pure logic + side effect wrappers
  - TDD enforcement: during build phase, sensei ensures tests are written before implementation
  - Test-first presentation: tests shown to user for approval before implementation begins
- **Working with assistants** — the invisible session (user doesn't see this directly, but should understand it):
  - Workflow phases: brainstorm → analyze → blueprint → plan → build → validate
  - Context injection: ~300 tokens of session context delivered at session start
  - Correction detection: sensei classifies each user turn as correction/continuation/clarification
  - Post-session analytics: FTR computation, module correction rates, tool usage, recommendations
  - This happens inside the AI assistant (Claude Code, Cursor) — sensei works through hooks and MCP, not through its own UI
- **Extensions** — customize sensei per project:
  - Skills browser: discover and install skills from marketplace
  - Agent editor: create custom autonomous agents with mindsets (why+what) and procedures (how)
  - Persona editor: define end-user, admin, API consumer personas for context-aware assistance
  - Inference settings: per-project model configuration
  - Benchmark runner: evaluate sensei's effectiveness on your codebase
- **Reference** — link to mockups. Note: see `design/02-daemon.md` for indexing pipeline, `design/04-mcp.md` for tool contracts, `design/06-marketplace.md` for extensions.

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/ideas/04-project.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/ideas/04-project.md
git commit -m "docs: add ideas/04-project.md — code intelligence, patterns, context delivery"
```

---

### Task 7: Write ideas/05-gateway.md and ideas/06-logging.md

**Files:**
- Create: `docs/ideas/05-gateway.md`
- Create: `docs/ideas/06-logging.md`
- Read (source material for gateway):
  - `docs/archive/ideas/20-local-inference.md`
  - `docs/archive/ideas/28-inference-gateway.md`
  - `docs/archive/ideas/gateway/` (all 14 files)
  - `docs/archive/journeys/gateway/01-first-run.md`
- Read (source material for logging):
  - `docs/archive/ideas/29-telemetry.md`
  - `docs/backlog.md` (diagnostic logging section)

- [ ] **Step 1: Read gateway source material**

Read the gateway idea files and journey. Extract: user-facing surfaces (setup, observatory, project), model configuration, budget management, provider health, consensus, voice, zero-config first run.

- [ ] **Step 2: Write ideas/05-gateway.md**

Write user-perspective gateway document with these sections:

- **Title + intro** — `# Gateway` — Inference routing. Sensei manages which AI models handle which tasks, optimizing for quality, cost, and availability.
- **What the user sees** — Not a standalone screen. Gateway surfaces through:
  - Setup step 8 (Inference): model configuration
  - Observatory: cost tracking, provider status
  - Project: search quality (powered by embeddings), context delivery quality
- **Zero-config start** — when Ollama is running locally, sensei auto-detects it, creates default chains (embedding, inference, chat), and starts working. No configuration needed. User sees nothing about the gateway — it's invisible.
- **Configuring inference** — for users who want control:
  - Local models: Ollama with hardware-aware recommendations (which models fit your GPU/RAM)
  - External providers: Anthropic, OpenAI, Google. Add API keys, set preferences.
  - Per-task routing: embedding (always local), classification (cheapest available), chat (quality-optimized), summarization (local preferred), consolidation (multi-model)
- **Budget management** — cost controls:
  - Daily and monthly spend limits
  - Spend tracking visible in observatory metrics
  - Local-first degradation: when budget exhausted, fall back to local models only
  - Cost estimation per request (shown in observatory session detail)
- **Provider health** — reliability:
  - Circuit breaker per endpoint: tracks failures, auto-disables unhealthy providers, auto-recovers
  - Fallback chains: if primary provider fails, automatically tries next in chain
  - What the user sees: provider status in settings (healthy/degraded/down), notification when degrading
- **Consensus** — high-stakes decisions:
  - MOE (Mixture of Experts) panel: multiple models analyze the same question
  - Results synthesized with confidence scoring
  - Used for: coaching recommendations, impact verdicts, correction classification
- **Voice** — audio capabilities:
  - Speech-to-text: Whisper model via Ollama for transcription
  - Text-to-speech: streaming audio generation
  - Use cases: voice notes, accessibility, audio code review
- **Reference** — see `design/03-gateway.md` for chain architecture, adapter system, and budget implementation.

- [ ] **Step 3: Read logging source material**

Read telemetry idea and backlog diagnostic section. Extract: log viewer concept, debug mode, bootstrap diagnostics, issue submission, session traces.

- [ ] **Step 4: Write ideas/06-logging.md**

Write user-perspective logging document with these sections:

- **Title + intro** — `# Logging & Diagnostics` — When things go wrong, sensei helps you understand why and report issues easily.
- **Bootstrap diagnostics** — structured trace logging during install and startup:
  - Each gate logs its check/install/verify steps
  - Errors include context: what was expected, what happened, what to try
  - Accessible from bootstrap error screen
- **Log viewer** — dedicated screen for examining logs:
  - Filter by: level (error, warn, info, debug, trace), component (daemon, mcp, bootstrap, gateway), time range
  - Tail mode: live-streaming new log entries as they arrive
  - Search within logs
  - Export log selection
- **Debug mode** — toggle from observatory settings:
  - Increases logging verbosity across all components
  - Shows: daemon API calls, indexing progress details, MCP tool invocations, gateway routing decisions
  - Useful for understanding why sensei made a particular recommendation or why indexing is slow
- **Issue submission** — one-click bug reporting:
  - Packages: recent logs, system info (OS, hardware, component versions), reproduction context
  - Generates GitHub issue template pre-filled with diagnostics
  - User adds description and submits
- **Session traces** — per-session diagnostic view:
  - Event log: which hooks fired, which MCP tools were called, what the daemon processed
  - Timing: how long each operation took
  - Useful for debugging slow sessions or unexpected behavior
- **Reference** — see `design/07-logging.md` for structured tracing architecture and log viewer API.

- [ ] **Step 5: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/ideas/05-gateway.md docs/ideas/06-logging.md
```

Expected: no matches

- [ ] **Step 6: Commit**

```bash
git add docs/ideas/05-gateway.md docs/ideas/06-logging.md
git commit -m "docs: add ideas/05-gateway.md and ideas/06-logging.md"
```

---

### Task 8: Write design/README.md — Architecture Overview

**Files:**
- Create: `docs/design/README.md`
- Read (source material):
  - `docs/archive/design/README.md`
  - `docs/archive/blueprints/02-system-architecture.md`
  - `docs/archive/design/platform/architecture-overview.md`
  - `docs/archive/design/daemon/README.md`
  - `docs/archive/design/decisions/adr-005-postgres-migration.md`

- [ ] **Step 1: Read source material**

Read the existing architecture docs. Extract: component inventory, layered architecture, data flows, integration points, tech decisions.

- [ ] **Step 2: Write design/README.md**

Write the architecture overview with these sections:

- **Title** — `# Sensei — Architecture & Design`
- **How to read these docs** — design/ describes how sensei is built. For what it does from a user perspective, see `ideas/`. Each design doc references the relevant ideas doc. No user-facing behavior is re-explained here.
- **Architecture diagram** — ASCII representation of component topology (to be replaced with SVG later):

```
┌─────────────────────────────────────────────────┐
│                  User Surfaces                   │
├──────────────┬──────────────┬───────────────────┤
│ Desktop App  │     CLI      │   AI Assistants   │
│ (Tauri+Svelte)│  (sensei)   │ (Claude, Cursor…) │
└──────┬───────┴──────┬───────┴────────┬──────────┘
       │              │                │
       │ HTTP         │ HTTP           │ stdio
       │              │                │
       │              │         ┌──────┴──────┐
       │              │         │     MCP     │
       │              │         │ (sensei-mcp)│
       │              │         └──────┬──────┘
       │              │                │ HTTP
       ▼              ▼                ▼
┌─────────────────────────────────────────────────┐
│                 Daemon (senseid)                  │
│  ┌──────────┬───────────┬──────────┬──────────┐ │
│  │ Indexing  │Intelligence│Analytics │   API    │ │
│  │ Pipeline  │  Layer    │  Engine  │ Surface  │ │
│  └──────────┴───────────┴──────────┴──────────┘ │
│  ┌──────────────────────────────────────────────┐│
│  │            Gateway (inference routing)        ││
│  │  ┌────────┐  ┌──────────┐  ┌──────────────┐ ││
│  │  │ Ollama │  │ Anthropic│  │ OpenAI/Google│ ││
│  │  │(local) │  │  (API)   │  │    (API)     │ ││
│  │  └────────┘  └──────────┘  └──────────────┘ ││
│  └──────────────────────────────────────────────┘│
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│         PostgreSQL (pgvector + Apache AGE)        │
│  ┌────────┬────────┬──────────┬────────┬───────┐│
│  │gateway │ sensei │inference │activity│history ││
│  │  (6)   │  (18)  │  (10)   │  (4)   │  (1)  ││
│  └────────┴────────┴──────────┴────────┴───────┘│
└─────────────────────────────────────────────────┘

External:  [Marketplace (GitHub)]  [Homebrew (distribution)]
```

- **Layered architecture** — three tiers:
  1. User surfaces: Desktop app, CLI, AI assistants (via MCP)
  2. Core engine: Daemon with indexing, intelligence, analytics subsystems
  3. Storage: PostgreSQL as single source of truth for all persistent state
  - Cross-cutting: Gateway (inference routing), Marketplace (plugin surface)

- **Tech stack**:
  - Rust: senseid (daemon), sensei-mcp (MCP server), sensei (CLI), sensei-gateway (crate)
  - Tauri + SvelteKit: desktop app (Svelte 5 with runes)
  - PostgreSQL + pgvector + Apache AGE: all persistent state, vector search, graph queries
  - Ollama: local inference (embedding, classification, summarization)
  - Claude Code plugin system: marketplace (commands, skills, hooks)
  - Rokkit: design system (tokens, skins, utilities)

- **Component map** table:

| # | Component | Binary/Package | Port | Purpose | Design Doc | Ideas Ref |
|---|-----------|---------------|------|---------|------------|-----------|
| 1 | App | sensei (Tauri) | — | Desktop observatory | `01-app.md` | `ideas/01-06` |
| 2 | Daemon | senseid | 7744/7745 | Core engine | `02-daemon.md` | `ideas/01-04` |
| 3 | Gateway | sensei-gateway | — | LLM routing | `03-gateway.md` | `ideas/05` |
| 4 | MCP | sensei-mcp | stdio | AI interface | `04-mcp.md` | `ideas/04` |
| 5 | CLI | sensei | — | Manual ops | `05-cli.md` | `ideas/02` |
| 6 | Marketplace | — | — | Plugins | `06-marketplace.md` | `ideas/03-04` |
| 7 | Logging | — | — | Diagnostics | `07-logging.md` | `ideas/06` |
| 8 | Website | sensei-site | — | Marketing | `08-website.md` | — |
| 9 | Homebrew | tap | — | Distribution | `09-homebrew.md` | — |
| 10 | Build | — | — | CI/Release | `10-build-and-release.md` | — |

- **Data flows** — three primary flows:
  1. Event capture: AI assistant → hooks → MCP → daemon events → analytics → observatory
  2. Indexing: file system → tree-sitter → Adapter IR → PostgreSQL graph → intelligence layer
  3. Context delivery: task → token budget → resolution selection (L0-L3) → MCP → AI assistant

- **Principles** section with four subsections:

  **Design Principles**
  - Single source of truth — PostgreSQL owns all persistent state
  - Agent-agnostic — MCP interface works with any AI assistant
  - Local-first — prefer local inference, degrade gracefully when unavailable
  - Zero-config start — works with just Ollama, no API keys needed
  - Invisible integration — user doesn't interact with sensei during coding sessions
  - Token efficiency — serve context at the right resolution level (L0-L3)

  **Rules**
  - Compile-time mode separation — dev/release via Cargo feature flag, not env vars
  - No direct daemon calls from AI — always through MCP
  - No code in marketplace — markdown commands/skills only
  - Desktop observes, never blocks — observatory is read-only, never interrupts workflows
  - DRY — reuse over duplication, modular and testable
  - No silent workarounds — fix root causes, document deviations in ADRs

  **Patterns**
  - Adapter IR — common intermediate representation across all language parsers
  - Sidecar lifecycle — Tauri manages daemon process start/stop/health
  - SSE for progress — long-running operations stream events to UI
  - Hook-based capture — events flow from AI assistants via hook system
  - Chain-based routing — gateway uses fallback chains per capability type
  - Resolution levels — L0 (signature) → L1 (IO pattern) → L2 (logic flow) → L3 (full source)

  **Non-Functional Requirements**
  - Bootstrap health check < 2s on repeat launches
  - Context delivery within token budget per request
  - Graceful degradation when Ollama or external providers unavailable
  - All state survives app restart (PostgreSQL-backed)
  - Debug/release fully isolated (ports 7744/7745, databases sensei/sensei_dev, directories ~/.sensei/~/.sensei-dev)
  - No secrets in marketplace artifacts

- **Gaps** section — placeholder to be populated during design doc writing. Format:

| Gap | Module | Description | Blocks |
|-----|--------|-------------|--------|
| (populated during Tasks 9-18) | | | |

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/design/README.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/design/README.md
git commit -m "docs: add design/README.md — architecture overview, principles, component map"
```

---

### Task 9: Write design/01-app.md

**Files:**
- Create: `docs/design/01-app.md`
- Read (source material):
  - `docs/archive/design/desktop/README.md`
  - `docs/archive/design/bootstrap.md`
  - `docs/archive/design/configure.md`
  - `docs/archive/design/api/00-api-surface-overview.md`
  - `docs/archive/design/api/01-bootstrap.md`
  - `docs/archive/design/api/02-scan-events.md`
  - `docs/archive/design/api/03-scan-event-flow.md`
  - `docs/archive/superpowers/specs/2026-04-30-wizard-state-architecture-design.md`
  - `docs/archive/superpowers/specs/2026-05-07-bootstrap-engine-design.md`
  - `docs/mockups/summary.md`

- [ ] **Step 1: Read source material**

Read all source files. Extract: Tauri architecture, sidecar pattern, bootstrap flow, wizard state machine, SSE event contracts, screen-by-screen API mapping, Rokkit design system, Svelte 5 state patterns.

- [ ] **Step 2: Write design/01-app.md**

Write engineering-perspective app document. Sections:

- **Title** — `# App — Desktop Observatory`
- **Overview** — Tauri 2.x shell wrapping SvelteKit frontend. Not a code editor or project manager. Observes and displays — never blocks or interrupts. See `ideas/01-bootstrap` through `ideas/04-project` for what the user experiences.
- **Architecture** — Tauri binary with Rust backend + SvelteKit webview frontend. Sidecar manages daemon lifecycle (start, health check, restart). All data comes from daemon HTTP API — app has no direct database access.
- **Sidecar lifecycle** — how Tauri manages senseid:
  - App launch → sidecar starts daemon binary
  - Health endpoint polling until ready
  - Restart on crash detection
  - Graceful shutdown on app close
  - Platform considerations (macOS launchd vs direct process)
- **Bootstrap flow** — engineering details:
  - 6-gate sequence, each gate is a checker + fixer pair
  - ComponentStatus types: Checking, Installing, Ready, Error, Skipped
  - Parallel checks where dependencies allow (homebrew is base, others depend)
  - API: `GET /health` for quick check, `GET /bootstrap/status` for full gate status
  - SSE: progress events per gate during installation
- **Setup wizard** — state architecture:
  - Linear step flow with back navigation
  - Per-step state: data model, validation, API calls
  - SSE for scan progress (StateEvent format: entity, status, metadata)
  - Scan events: project cards appearing, file progress bars, activity log
  - API endpoints per step (refer to `02-daemon.md` for full contracts)
- **Observatory screens** — component structure:
  - Dashboard: hero koan, FTR trend chart, session list, teaching summary
  - Session detail: timeline component with event markers
  - Coaching: recommendation cards with action drawer
  - Memory: memory list with detail panel
  - Metrics: chart components with drill-down
  - Project views: graph visualization (D3 or similar), pattern catalog, library list
- **State management** — Svelte 5 patterns:
  - Runes ($state, $derived, $effect) for reactive state
  - Store pattern for shared state across routes
  - API response → store update → reactive UI
  - Optimistic updates where appropriate
- **Styling** — Rokkit design system:
  - Token palette: Paper (background), Sumi (text), Shu vermillion (primary/accent), Jade (success), Amber (warning), Matcha (info)
  - Typography: Fraunces (display), Inter (body), JetBrains Mono (code)
  - Spacing system, border radius, elevation
  - CSS custom properties via `-z` prefix
  - Dark mode via skin system
- **Routing** — SvelteKit route structure:
  - `(config)/setup/` — bootstrap and wizard routes
  - `(app)/` — observatory and project routes
  - Layout groups for shared navigation/chrome

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/design/01-app.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/design/01-app.md
git commit -m "docs: add design/01-app.md — Tauri desktop architecture"
```

---

### Task 10: Write design/02-daemon.md

**Files:**
- Create: `docs/design/02-daemon.md`
- Read (source material):
  - `docs/archive/design/daemon/README.md`
  - `docs/archive/design/daemon/architecture.md`
  - `docs/archive/design/daemon/database-schema.md`
  - `docs/archive/design/daemon/task-queue.md`
  - `docs/archive/design/daemon/enhancements.md`
  - `docs/archive/design/daemon/debug-vs-release.md`
  - `docs/archive/design/daemon/intelligence/` (all files)
  - `docs/archive/design/daemon/traceability/` (all files)
  - `docs/archive/design/daemon/analytics/` (all files)
  - `docs/archive/design/daemon/workers/folder-model.md`
  - `docs/archive/blueprints/01-workflow-engine.md`
  - `docs/archive/blueprints/03-adapter-ir.md`
  - `docs/archive/journeys/08-system-pipelines/` (all files)

- [ ] **Step 1: Read source material**

Read all source files. Skip any that are marked SUPERSEDED but note what they were superseded by. Extract: current Rust architecture, API endpoints, indexing pipeline, task queue, intelligence subsystems, analytics, traceability, database schema, workflow engine.

**Important:** The old architecture.md references Supabase and TypeScript. The old indexing-architecture.md is SUPERSEDED. The old graph-intelligence.md references Kuzu. Use only current information (Rust, PostgreSQL, Apache AGE).

- [ ] **Step 2: Write design/02-daemon.md**

Write engineering-perspective daemon document. Sections:

- **Title** — `# Daemon — senseid`
- **Overview** — Rust HTTP server. Core engine that owns all data and intelligence. See `ideas/03-observatory` and `ideas/04-project` for what it powers. Port 7744 (release), 7745 (dev).
- **Architecture** — crate structure:
  - `crates/senseid/` — main binary, HTTP routes, server setup
  - Depends on: sensei-bootstrap, sensei-gateway, shared types
  - Axum-based HTTP with tower middleware
  - PostgreSQL connection pool (deadpool-postgres or similar)
- **API surface** — endpoint inventory organized by user surface:
  - Bootstrap: `GET /health`, `GET /bootstrap/status`, `POST /bootstrap/fix/:gate`
  - Setup: `GET/POST /roots`, `GET /scan/status`, `SSE /scan/events`, `GET/POST /projects`, `GET/POST /libraries`, `GET/POST /instruments`
  - Observatory: `GET /sessions`, `GET /sessions/:id`, `GET /metrics/ftr`, `GET /teachings`, `GET /memory`
  - Project: `GET /projects/:id/graph`, `GET /projects/:id/patterns`, `GET /search`, `GET /context`
  - Include request/response contract summaries for key endpoints
- **Indexing pipeline** — how code gets into the graph:
  - Task hierarchy: ScanRoot → ProcessRepo → ProcessFolder → ProcessFile → ResolveEdges → BuildConnections
  - Adapter IR: tree-sitter AST → IRModule/IRClass/IRDoc nodes with base fields (kind, name, path, line range, language)
  - Language adapters: Rust, TypeScript, JavaScript, Svelte, Python, Markdown (each produces IR)
  - Post-processing: community detection, duplicate detection, testability scoring
  - Optional local inference pass: embeddings (pgvector), docstring generation, classification
- **Task queue** — job processing:
  - PostgreSQL-backed queue with LISTEN/NOTIFY for wake-up
  - Hierarchical: parent tasks spawn child tasks
  - Parallelism: multiple workers, configurable concurrency
  - Incremental: file watcher triggers re-index of changed files only
  - Priority: user-initiated scans > background re-index > enhancement passes
- **Intelligence layer** — context and analysis:
  - Context compression: L0 (signature) → L1 (IO pattern: params + return) → L2 (logic flow notation) → L3 (full source with docstrings stripped)
  - Context manager: recommend_next() maps task type to resolution level and scope, budget-aware
  - Pattern store: detect patterns during indexing, persist, match on query, dedup on update
  - Metadata model: orientation (project info), symbol graph (compressed), edges (call, import), fingerprints (change detection)
  - Semantic search: pgvector embeddings for similarity queries, combined with full-text tsvector
- **Analytics engine** — session intelligence:
  - 16 event types captured from hooks (SessionStart, ToolUse, TurnClassification, PhaseTransition, etc.)
  - FTR computation: ratio of sessions with zero corrections to total sessions
  - Correction classification: heuristic + local model classifies turns as correction/continuation/clarification
  - Project memory: cross-session knowledge distilled at session end, bounded token budget
  - Recommendation generation: based on correction patterns, FTR trends, module hotspots
- **Traceability** — doc-code linking:
  - Traceability matrix: maps doc paths to source file paths, stored in PostgreSQL
  - Drift detection: git diff + matrix → flag docs whose linked code changed
  - Doc tools: find_doc, scaffold, doctor (reformat)
- **Database schema** — overview:
  - 5 schemas: `gateway` (6 tables), `sensei` (18 tables), `inference` (10 tables), `activity` (4 tables), `history` (1 table)
  - Key tables: `folders`, `projects`, `symbols`, `call_edges`, `imports`, `patterns`, `events`, `sessions`, `memories`, `teachings`
  - Staging tables for seeded data (timestamp-guarded to not overwrite production)
  - Enum types: folder_kind, symbol_kind, edge_type, event_type, scan_status, etc.
  - DDL lives in `database/` directory, managed by `dbd`

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/design/02-daemon.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/design/02-daemon.md
git commit -m "docs: add design/02-daemon.md — daemon architecture, API, indexing, analytics"
```

---

### Task 11: Write design/03-gateway.md

**Files:**
- Create: `docs/design/03-gateway.md`
- Read (source material):
  - `docs/archive/design/gateway/` (all 4 files)
  - `docs/archive/ideas/gateway/` (all 14 files)
  - `docs/archive/ideas/28-inference-gateway.md`

- [ ] **Step 1: Read source material**

Read all gateway design and idea files. Extract: crate architecture, engine flow, type system, chain definitions, adapter trait, circuit breaker state machine, budget management, consensus panel, data model, daemon integration.

- [ ] **Step 2: Write design/03-gateway.md**

Write engineering-perspective gateway document. Sections:

- **Title** — `# Gateway — Inference Routing`
- **Overview** — standalone Rust crate (`crates/gateway/`). LLM routing engine. See `ideas/05-gateway` for user perspective. Port of Strategos concepts to Rust.
- **Architecture** — module layout:
  - `engine` — execution engine, fallback chain walking, hook lifecycle
  - `selection` — 3-tier model resolution, chain walking, candidate validation
  - `adapters` — provider trait, Ollama adapter, API adapters, noop adapter
  - `circuit_breaker` — per-endpoint failure tracking, state machine
  - `budget` — cost estimation, limits, tracking
  - `consensus` — MOE panel coordination
  - `config` — builder pattern, validation, DB-backed, hot reload
  - `types` — Capability enum, error types, request/response types
- **Type system** — key types:
  - Capability enum: Chat, Embed, Classify, Summarize, Consolidate, VoiceStt, VoiceTts
  - Error enum with thiserror derivation
  - Request/response types per capability
  - Compile-time exhaustive match on capabilities
- **Chains** — one per capability type:
  - Inference: general reasoning, classification, summarization. Local → external fallback.
  - Embedding: vector generation for semantic search. Local-only (Ollama), no external fallback.
  - Chat: conversational interaction, tool use, multi-turn. Quality-optimized routing (external preferred).
  - Consolidation: merge, deduplicate, synthesize from multiple sources.
  - Voice: STT (Whisper via Ollama), TTS. Streaming audio support.
- **Model selection** — 3-tier resolution:
  1. Exact match (user specified model)
  2. Capability match (find models that support the capability)
  3. Fallback chain walk (try each candidate in order)
  - Candidate validation: check model supports capability, provider is healthy, budget allows
- **Adapters** — provider abstraction:
  - Adapter trait: send(), capabilities(), health_check()
  - OllamaAdapter: local models, HTTP to Ollama API
  - API adapters: Anthropic, OpenAI, Google. HTTP with auth.
  - NoopAdapter: graceful degradation when no provider available. Returns structured "unavailable" response.
  - Registry: adapter registration, lookup by provider name
- **Circuit breaker** — reliability:
  - Per-endpoint state machine: Closed → Open → HalfOpen → Closed
  - Failure threshold, recovery timeout, success threshold for HalfOpen
  - Auto-recovery: periodic health probes in Open state
- **Budget** — cost management:
  - Cost estimation: per-token pricing by model, estimated before request
  - Daily/monthly limits: configurable per project or global
  - Spend tracking: stored in `inference_calls` table
  - Degradation: when budget exhausted, route only to local (free) models
- **Consensus** — MOE panel:
  - Multiple models process same input
  - Weighted synthesis based on model capability and confidence
  - Used for high-stakes decisions (coaching verdicts, correction classification)
- **Data model** — PostgreSQL tables:
  - `gateway.inference_calls`: request log with cost, model, capability, latency
  - `gateway.execution_traces`: debug/analytics trace per call, attempt history
  - Indexes on project, session, created_at, model, capability
- **Integration** — how gateway fits in daemon:
  - Daemon owns gateway instance lifecycle
  - Health endpoints exposed through daemon API
  - MCP tools for gateway status and model listing
  - Configuration loaded from PostgreSQL, hot-reloadable

- [ ] **Step 3: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/design/03-gateway.md
```

Expected: no matches

- [ ] **Step 4: Commit**

```bash
git add docs/design/03-gateway.md
git commit -m "docs: add design/03-gateway.md — inference routing architecture"
```

---

### Task 12: Write design/04-mcp.md and design/05-cli.md

**Files:**
- Create: `docs/design/04-mcp.md`
- Create: `docs/design/05-cli.md`
- Read (source material):
  - `docs/archive/design/mcp/` (all files)
  - `docs/archive/design/cli/` (all files)
  - `docs/archive/design/platform/` (coordinator-adapters.md)

- [ ] **Step 1: Read source material**

Read MCP and CLI design docs. Extract: MCP binary architecture, tool contracts with signatures, REPO_PATH resolution, multi-coordinator support, CLI commands, profile system, hook integration.

- [ ] **Step 2: Write design/04-mcp.md**

Sections:
- **Title** — `# MCP — Model Context Protocol Server`
- **Overview** — Rust binary (`crates/mcp/`). stdio transport. Translates MCP tool calls to daemon HTTP requests. AI assistants never call daemon directly — MCP is the interface. See `ideas/04-project` for what it enables.
- **Architecture** — MCP protocol handler, tool registry, HTTP client to daemon, REPO_PATH resolution.
- **Tool contracts** — organized by phase:
  - Phase 1 (code intelligence, implemented): `search`, `get_session_context`, `get_callers`, `get_callees`, `get_duplicates`, `get_communities`, `get_project_summary`, `get_project_conventions`, `embed`, `consensus`, `infer`
  - Phase 2 (workflow): `update_phase`, `get_workflow_state`, `log_event`, `get_patterns`, `match_pattern`, `get_pattern_for`
  - For each tool: name, parameters, return type, what daemon endpoint it calls
- **REPO_PATH resolution** — how MCP determines project context: check environment, check working directory, resolve against known projects in daemon
- **Multi-coordinator support** — adapter pattern:
  - Claude Code: hooks for event capture, ~/.claude/mcp.json for config
  - Cursor: different config location, different event capture
  - Others: generic adapter with manual configuration
  - Same MCP binary serves all — event capture differences handled by daemon

- [ ] **Step 3: Write design/05-cli.md**

Sections:
- **Title** — `# CLI — sensei`
- **Overview** — Rust binary (`crates/cli/`). For manual operations and benchmarking. Optional — most users interact through the desktop app. See `ideas/02-setup` for how it relates to setup.
- **Commands** — `init` (initialize project), `add` (add library/root), `upgrade` (update components), `status` (show project status), `guidelines` (display rules), `cache` (manage cached data), `hooks` (configure git hooks)
- **Profile system** — two layers:
  - Global: `~/.sensei/config.yaml` — user-wide preferences
  - Project: `.sensei/config.yaml` — project-specific overrides
  - Merge strategy: project overrides global, keys not in project fall through to global
- **Hook integration** — pre-commit hook for drift detection:
  - `sensei hooks install` adds git pre-commit hook
  - Hook checks traceability matrix, flags if committed code has linked docs that weren't updated
- **Interactive UX** — terminal prompt patterns for user interaction

- [ ] **Step 4: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/design/04-mcp.md docs/design/05-cli.md
```

Expected: no matches

- [ ] **Step 5: Commit**

```bash
git add docs/design/04-mcp.md docs/design/05-cli.md
git commit -m "docs: add design/04-mcp.md and design/05-cli.md"
```

---

### Task 13: Write design/06-marketplace.md and design/07-logging.md

**Files:**
- Create: `docs/design/06-marketplace.md`
- Create: `docs/design/07-logging.md`
- Read (source material):
  - `docs/archive/design/marketplace/` (all files)
  - `docs/archive/ideas/hooks.md`
  - `docs/archive/superpowers/specs/2026-05-01-bootstrap-logging-design.md`
  - `docs/backlog.md` (diagnostic logging section)

- [ ] **Step 1: Read source material**

Read marketplace design files and logging specs. Extract: command format, hook types, skill structure, auto-triggering, logging architecture, log levels, bootstrap diagnostics, debug mode.

- [ ] **Step 2: Write design/06-marketplace.md**

Sections:
- **Title** — `# Marketplace — Commands, Skills, Hooks, Plugins`
- **Overview** — Claude Code plugin layer. All user-facing interaction with AI assistants flows through marketplace artifacts. See `ideas/03-observatory` and `ideas/04-project` for what it enables. Lives in `marketplace/` directory (git subtree → sensei-hq/marketplace).
- **Architecture** — marketplace is a Claude Code plugin with:
  - Commands (markdown files invoked by user via /command)
  - Skills (markdown files auto-triggered by agent)
  - Hooks (event handlers: PreToolUse, PostToolUse, SessionStart, PreCompact, UserPromptSubmit)
  - Agents (subagent definitions for delegation)
- **Commands** — 13 commands organized by purpose:
  - Phase commands (7): brainstorm, idea, analyze, blueprint, experiment, plan, build
  - Cross-cutting (2): review, validate
  - Refocus (4): rules, patterns, refocus, tools
  - Each command: name, trigger, what it does, how it interacts with daemon (via MCP)
- **Hooks** — event-driven behaviors:
  - session-start: initialize session in daemon, deliver context
  - user-prompt: classify intent, update workflow state
  - pre-compact: preserve critical context before compaction
  - Event types and payload formats
  - Integration: hooks call MCP tools which call daemon API
- **Skills** — format and triggering:
  - Directory structure: `skills/<name>/` with SKILL.md
  - Frontmatter: name, description ("Use when..."), type
  - Content: overview, when-to-use, core content, quick reference
  - Token efficiency: skills are designed to fit in context without overwhelming
  - Auto-triggering: agent reads skill descriptions, invokes when relevant
- **Auto-triggering** — how behaviors activate:
  - Workflow state drives which commands/skills are relevant
  - Session context (current phase, recent corrections) influences suggestions
  - No unsolicited interruptions — marketplace responds to user or agent actions

- [ ] **Step 3: Write design/07-logging.md**

Sections:
- **Title** — `# Logging — Structured Tracing & Diagnostics`
- **Overview** — structured logging across all Rust binaries. See `ideas/06-logging` for user perspective.
- **Architecture** — tracing crate with structured spans and events:
  - All binaries (senseid, sensei-mcp, sensei) use same logging infrastructure
  - Levels: error, warn, info, debug, trace
  - Component tags: bootstrap, daemon, mcp, gateway, indexing, analytics
  - Output: structured JSON to file, human-readable to stderr
- **Bootstrap diagnostics** — trace capture during install/startup:
  - Each gate logs: check start, check result, fix start, fix progress, fix result
  - Error events include: expected state, actual state, suggested action
  - Log file location: `~/.sensei/logs/bootstrap.log` (release), `~/.sensei-dev/logs/bootstrap.log` (dev)
- **Log viewer API** — daemon endpoints for log access:
  - `GET /logs` — query logs with filters (level, component, time range, limit)
  - `GET /logs/stream` — SSE endpoint for tail mode
  - Response format: timestamp, level, component, message, structured fields
- **Debug mode** — increased verbosity:
  - Toggle via daemon API: `POST /config/debug`
  - When active: trace-level logging, API request/response bodies, indexing per-file details, gateway routing decisions
  - Persists across restart (stored in config)
- **Issue submission** — diagnostic packaging:
  - `POST /diagnostics/report` — collects: recent logs, system info (OS, hardware, versions), config (sanitized), error context
  - Returns: GitHub issue template markdown
  - App provides one-click "Report Issue" button that calls this endpoint and opens GitHub

- [ ] **Step 4: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/design/06-marketplace.md docs/design/07-logging.md
```

Expected: no matches

- [ ] **Step 5: Commit**

```bash
git add docs/design/06-marketplace.md docs/design/07-logging.md
git commit -m "docs: add design/06-marketplace.md and design/07-logging.md"
```

---

### Task 14: Write design/08-website.md, design/09-homebrew.md, design/10-build-and-release.md

**Files:**
- Create: `docs/design/08-website.md`
- Create: `docs/design/09-homebrew.md`
- Create: `docs/design/10-build-and-release.md`
- Read (source material):
  - `docs/archive/design/daemon/debug-vs-release.md`
  - `/Users/Jerry/Developer/sensei-hq/sensei/Makefile`
  - `/Users/Jerry/Developer/sensei-hq/sensei/VERSION`
  - `/Users/Jerry/Developer/sensei-hq/sensei/homebrew/` (check contents)
  - `/Users/Jerry/Developer/sensei-hq/sensei/website/` (check structure)

- [ ] **Step 1: Read source material**

Read debug-vs-release doc, Makefile, VERSION, homebrew directory structure, website directory structure. Extract: build modes, version management, homebrew formula, website architecture.

- [ ] **Step 2: Write design/08-website.md**

Sections:
- **Title** — `# Website — sensei-site`
- **Overview** — SvelteKit static site. Marketing, documentation, changelog. Lives in `website/` directory.
- **Architecture** — SvelteKit with static adapter. Rokkit design system. Routes: landing, docs, FAQ, privacy, changelog.
- **Architecture visualization** — site hosts an interactive component diagram (currently needs update — old version referenced Kuzu/SQLite). Should be updated to match the architecture diagram in `design/README.md`.
- **Relationship to docs/** — website docs pages reference `ideas/` for product narrative. Changelog generated from git tags and release notes.
- **Deployment** — static build, GitHub Pages or similar.

- [ ] **Step 3: Write design/09-homebrew.md**

Sections:
- **Title** — `# Homebrew — Distribution`
- **Overview** — Homebrew tap for macOS distribution. Lives in `homebrew/` directory (git subtree → sensei-hq/homebrew-tap).
- **Tap structure** — formula for daemon + CLI binaries, cask for desktop app.
- **What Homebrew manages** — installation of sensei binaries. Bootstrap verifies what Homebrew installed and handles configuration/startup.
- **Versioning** — VERSION file at repo root drives formula version. `make bump v=X.Y.Z` propagates version, updates formula, tags, pushes.
- **Subtree sync** — `make tap-push` syncs homebrew/ to sensei-hq/homebrew-tap.

- [ ] **Step 4: Write design/10-build-and-release.md**

Sections:
- **Title** — `# Build & Release`
- **Overview** — build system, version management, debug/release separation, CI pipeline.
- **Debug vs Release** — compile-time `dev` Cargo feature:
  - Dev: port 7745, database `sensei_dev`, directory `~/.sensei-dev/`, binary names unchanged
  - Release: port 7744, database `sensei`, directory `~/.sensei/`
  - Feature propagation: root Cargo.toml `dev` feature enables `dev` in all dependent crates
  - No runtime env var detection — mode is baked in at compile time
- **Version management** — `VERSION` file is single source of truth:
  - `make bump v=X.Y.Z` updates: VERSION, all Cargo.toml files, package.json files, homebrew formula
  - Creates git tag, commits, pushes, syncs subtrees
- **Build targets** — order matters:
  1. Rust crates: `make crates-dev` or `make crates-release` (senseid, sensei-mcp, sensei-cli)
  2. Desktop app: `make app-dev` (Tauri dev with Vite HMR)
  3. Website: standard SvelteKit build
  4. Homebrew: formula update after release build
- **Key Makefile targets**:
  - `make setup-hooks` — install git hooks
  - `make install-dev` — build dev crates + install to ~/.local/bin
  - `make install-release` — build release crates + install
  - `make app-dev` — Tauri dev server
  - `make test` — run all tests (requires DB)
  - `make test-fast` — run tests without DB (pre-commit)
  - `make bump v=X.Y.Z` — version bump + tag + push + subtree sync
  - `make tap-push` / `make marketplace-push` — subtree sync
- **Release process**:
  1. Ensure all tests pass on develop
  2. `make bump v=X.Y.Z` on develop
  3. Merge develop → main
  4. Homebrew formula auto-updated by bump
  5. Desktop app built and distributed (mechanism TBD)

- [ ] **Step 5: Verify no banned terms**

```bash
grep -i -E 'supabase|sqlite|kuzu|better-sqlite3|llmspec|bun daemon|apps/static|apps/dashboard' docs/design/08-website.md docs/design/09-homebrew.md docs/design/10-build-and-release.md
```

Expected: no matches

- [ ] **Step 6: Commit**

```bash
git add docs/design/08-website.md docs/design/09-homebrew.md docs/design/10-build-and-release.md
git commit -m "docs: add design/08-website, 09-homebrew, 10-build-and-release"
```

---

### Task 15: Update docs/README.md and populate gaps

**Files:**
- Modify: `docs/README.md`
- Modify: `docs/design/README.md` (gaps section)

- [ ] **Step 1: Read current docs/README.md**

```bash
cat /Users/Jerry/Developer/sensei-hq/sensei/docs/README.md
```

- [ ] **Step 2: Rewrite docs/README.md**

Replace with new structure overview:

```markdown
# Sensei Documentation

## Structure

| Directory | Purpose |
|-----------|---------|
| `ideas/` | What Sensei does — user perspective. Start here. |
| `design/` | How it's built — engineering perspective. References ideas/. |
| `mockups/` | Visual reference — HTML mockups and design system. |
| `archive/` | Old documentation — retained for historical context. |
| `backlog.md` | Active implementation backlog. |

## Reading order

1. `ideas/README.md` — product overview, module map, status
2. `ideas/01-bootstrap.md` through `ideas/06-logging.md` — what the user experiences
3. `design/README.md` — architecture, principles, component map
4. `design/01-app.md` through `design/10-build-and-release.md` — how each component is built

## Monorepo structure

| Directory | Language | Purpose |
|-----------|----------|---------|
| `app/` | SvelteKit + Tauri | Desktop observatory |
| `crates/` | Rust | All Rust crates (senseid, cli, mcp, bootstrap, gateway) |
| `website/` | SvelteKit | Marketing site |
| `database/` | SQL | DDL definitions |
| `homebrew/` | Ruby | Homebrew tap (subtree) |
| `marketplace/` | Markdown | Skills & plugins (subtree) |
| `docs/` | Markdown | This documentation |
```

- [ ] **Step 3: Populate gaps in design/README.md**

Review all written docs. Add identified gaps to the gaps table in design/README.md. Common gaps to check:
- Observatory screens: user-facing behavior defined in ideas/03 but no daemon API endpoints designed yet
- Inference/Assignments wizard steps: user-facing behavior defined in ideas/02 but blocked on gateway
- Memory consolidation: described in ideas/03 but daemon implementation unclear
- Multi-coordinator event capture: described in design/04-mcp but adapter implementations not designed
- Desktop app distribution: mentioned in design/10 but mechanism is TBD
- Website architecture diagram: needs update from old D3 version

- [ ] **Step 4: Commit**

```bash
git add docs/README.md docs/design/README.md
git commit -m "docs: update README.md, populate identified gaps"
```

---

### Task 16: Final verification and cleanup

- [ ] **Step 1: Verify complete structure**

```bash
find /Users/Jerry/Developer/sensei-hq/sensei/docs/ -maxdepth 2 -name "*.md" | sort
```

Expected: ideas/ (7 files), design/ (11 files), mockups/ (retained), archive/ (everything old), backlog.md, README.md, task.md

- [ ] **Step 2: Full banned-term scan across all new docs**

```bash
grep -r -i -E 'supabase|sqlite|kuzu|better-sqlite3|\.llmspec|bun daemon|apps/static|apps/dashboard' /Users/Jerry/Developer/sensei-hq/sensei/docs/ideas/ /Users/Jerry/Developer/sensei-hq/sensei/docs/design/
```

Expected: no matches. If any found, fix them.

- [ ] **Step 3: Verify cross-references**

Check that all cross-references between ideas/ and design/ docs point to files that exist:

```bash
grep -r -oE '(ideas|design)/[0-9a-z-]+\.md' /Users/Jerry/Developer/sensei-hq/sensei/docs/ideas/ /Users/Jerry/Developer/sensei-hq/sensei/docs/design/ | sort -u
```

Verify each referenced file exists.

- [ ] **Step 4: Final commit**

```bash
git add -A docs/
git commit -m "docs: complete documentation restructure — ideas/ (user) + design/ (engineering)"
```

- [ ] **Step 5: Push**

```bash
git push origin develop
```
