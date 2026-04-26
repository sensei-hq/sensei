# Mockup Design Summary

> Condensed from design discussions. For the interactive prototypes, see `lib/`.
> For user journeys with flow diagrams, see `../journeys/`.

---

## Design System

| Token | Value | Usage |
|-------|-------|-------|
| Paper | `oklch(0.97 0.008 85)` | Washi-paper background |
| Sumi (ink) | near-black | Foreground text |
| Shu (vermillion) | `oklch(0.58 0.15 35)` | Primary accent, signal |
| Jade | `oklch(0.65 0.08 150)` | Positive / calm states |
| Amber | warm yellow | Warning / attention |

**Typography**: Fraunces (display/numerals), Inter (UI), JetBrains Mono (code).

**Voice**: Terse, koan-like. *"The AI does not know your auth."* *"Three corrections. One teacher."*

**Direction chosen**: Merge of Ma + Enso. Collapsible sidebar (wide with names ↔ icon-only). Ma's trendline (not Enso's arc ring). Enso's two-column layout. Shoji rejected as too crowded.

**Fictional product**: Lumen (design tool company) with 3 solutions: lumen-studio, lumen-cloud, brand-kit.

**Mockup reference**: `lib/tokens.css`, `lib/primitives.jsx`

---

## Screens at a Glance

| Screen | Mockup file(s) | Journey |
|--------|---------------|---------|
| Bootstrap | `lib/bootstrap.jsx` | J1 |
| Setup wizard (9 steps) | `lib/setup-wizard.jsx`, `lib/setup-data.js`, `lib/wiz-inference.jsx`, `lib/wiz-assignments.jsx` | J2 |
| Observatory daily | `lib/observatory.jsx` | J3 |
| Sessions (digest + timeline) | `lib/sessions.jsx` | J3, J5 |
| Learnings (triage + anatomy + brief) | `lib/learnings.jsx`, `lib/learnings-v2.jsx`, `lib/learnings-data.js` | J9 |
| Project pages (3 layout variants) | `lib/project-pages.jsx`, `lib/project-shared.jsx`, `lib/project-data.js` | J5 |
| Libraries (2 variants) | `lib/libraries.jsx` | J5 |
| Instruments / MCP (playground + replay + insights) | `lib/instruments.jsx`, `lib/instruments-simple.jsx`, `lib/instruments-data.js`, `lib/mcp-signals-data.js`, `lib/mcp-replay-insights.jsx` | J5 |
| Navigation (3 variants) | `lib/navigation.jsx` | J3 |

---

## 1. Bootstrap

**Purpose**: Runs on every app launch. Verifies prerequisites before the wizard or observatory loads.

**Gates** (sequential):

| # | Gate | Check | Remedy |
|---|------|-------|--------|
| 1 | Homebrew | `which brew` | Link to brew.sh |
| 2 | PostgreSQL | `brew list postgresql@16` | `brew install` |
| 3 | Ollama | `brew list ollama` | `brew install` |
| 4 | Sensei components | `sensei --version` | `brew install` (CLI, MCP bridge, daemon) |
| 5 | Database | `sensei db:create` | Manual psql + DATABASE_URL fallback |
| 6 | Daemon | `sensei daemon:start` | Starts once DB is reachable |

**Behavior**: First blocked gate expands its remedy panel. All-green auto-advances (900ms delay) to empty state (first run) or observatory (returning user).

**Preset scenarios**: all-checking, missing-homebrew, missing-prereqs, missing-db, daemon-starting, all-green.

**Mockup reference**: `lib/bootstrap.jsx`

---

## 2. Setup Wizard

**Layout**: Left rail stepper (completed steps show as collapsed chips, current expanded) + main content area + bottom bar with progress ticks.

### Step 1: Welcome

"A teacher does not write the code." Three pillars: Observe, Teach, Local. ~4 min, nothing leaves your machine.

### Step 2: Components

Auto-detect and install CLI, MCP bridge, daemon. No user input. Animated phase transitions: detecting → installing → starting → ready. Three variant starting states (fresh / partial / all-present).

### Step 3: Assistants (ACPs)

Detect installed AI coding tools (Claude Code, Cursor, Zed, Continue). Toggle checkboxes to register sensei's plugins/skills/commands/logging. Only found tools are selectable.

### Step 4: Folders

User adds root paths (e.g. `~/code/lumen`). Recursive scan. Add/remove/browse. Minimum 1 required to continue.

### Step 5: Scan

Live SSE-style view. **Left**: solution cards materialize as repos are discovered, with progress bars per repo. **Right**: scrolling SSE event log. Stats strip: roots / discovered / queued / processed. Completion banner when done.

### Step 6: Projects

A project = one or more repos, auto-grouped by folder proximity and naming. User can: rename, split multi-repo into singles, merge projects, move repos between projects. Assign roles per repo (backend / frontend / library / docs / infra). Confirm checkbox per project.

### Step 7: Libraries

Libraries WITHOUT their own MCP — sensei wraps them (indexes docs/code, exposes its own tools). Auto-detected from Cargo.toml / package.json. User can add custom ones (name + docs URL + lang). Distinct from MCP registry.

### Step 8: Inference

**Providers & Models** (`lib/wiz-inference.jsx`). Detects hardware (RAM, GPU, chip). Lists providers: Ollama (local), Anthropic, OpenAI, Google. Auto-detects env vars (ANTHROPIC_API_KEY, etc.). Ollama models pull with progress bars; cloud models enable via API key. Recommended models get a subtle badge.

**Assignments** (`lib/wiz-assignments.jsx`). Five reasoning roles: inference, consolidation, embedding, voice, fallback. User builds ordered priority list per role. First entry is primary; rest are fallbacks.

### Step 9: MCP Registry

Services/tools with their own MCP (Postgres, Redis, Stripe, GitHub, Sentry, Playwright, Figma). Recommended based on detected stack. Shows: publisher, tool count, verified badge, stack trigger.

### Step 10: Enter

"The observatory is ready." Summary stats. Koan: *"the first session is always the teacher."*

**Mockup reference**: `lib/setup-wizard.jsx`, `lib/setup-data.js`, `lib/wiz-inference.jsx`, `lib/wiz-assignments.jsx`

---

## 3. Observatory (Daily View)

The daily home screen. Two maturity states: **early** (listening, forming signals) and **mature** (confident, actionable teachings).

**Layout**: Collapsible left sidebar + main content.

### Sidebar

- Observatory nav: Today, Sessions, Patterns, Libraries, Instruments, Teachings, Settings
- Active projects (with FTR numbers, warning indicators)
- Recent/dormant projects (collapsed)
- Daemon heartbeat status

### Main: Today

- **Greeting strip**: date + personalized hello
- **FTR hero**: arc gauge + 14-day sparkline trendline + delta vs prior period
- **Hero koan**: the single most important teaching. Large kanji, terse title, explanation, projected impact, action button, evidence trail. Mature vs early state.
- **Insights** (max 3): pattern recurring, teaching adopted, drift detected. Tone-coded (warn / good / mute).
- **Adopted teachings**: "System has learned" — rules sensei has applied. Empty state when early.
- **Recent sessions**: compact table (FTR dot, project, title, corrections, duration, time).
- **First-entry toast**: "The observatory is open" — once after setup.

**Mockup reference**: `lib/observatory.jsx`

---

## 4. Sessions

Cross-project session browser with retrospective analysis.

**Two layouts**: **Digest** (retro cards on top + filterable list) and **Timeline** (calendar heatmap + grouped feed).

**Retro sections**:
- Going well: high FTR streaks, adopted patterns, shorter sessions
- Not going well: recurring corrections, low FTR projects, abandoned sessions
- Insights: cross-project signals ("Rust + auth is your hardest combo", "mornings are first-try-right")

**Filters**: project, language, outcome (shipped / abandoned / corrected), stack tags.

**Mockup reference**: `lib/sessions.jsx`

---

## 5. Learnings

Consolidated patterns + memories + recommendations across all projects.

**Original layout** (`lib/learnings.jsx`): hero stats, recommendations inbox, 6 tabs (all / memories / patterns / corrections / lifecycle / archive), scope + project filters, sort (priority / strength / recency), memory drawer.

**Simplified alternatives** (`lib/learnings-v2.jsx`):
- **Triage**: three columns (Now / Soon / Settled) by immediacy
- **Anatomy**: one memory at a time — What / Why / How / Where
- **Brief**: single scrollable brief with chart + grouped lists

**Memory anatomy**: what, because, scope (level + project + modules + stack), strength (1-5), state (active / reinforced / challenged / battle-tested / archived), references (good/bad examples, pattern, evidence sessions).

**Recommendations**: promote-pattern, create-agent, write-skill, archive-memory. Each with impact rating and dismissal.

**Mockup reference**: `lib/learnings.jsx`, `lib/learnings-v2.jsx`, `lib/learnings-data.js`

---

## 6. Project Page

Per-project detail view.

**Three layout variants**: Top tabs (classic), Left rail (more content), Long scroll (zen, right-side "on this page" anchors).

### Overview

Project header (kanji, name, goal, stack tags). FTR trend. Recommendations panel — each with urgency, koan title, why, impact, evidence, and a **pre-built prompt** for the ACP. "Apply" flow generates a task.

### Graph

Code-graph visualization. Nodes = files, sized by fan-in, colored by overlay. Three lenses: Complexity (god-nodes), Rework (repeat edits), Staleness (days untouched). Edges = call relationships. Duplicate-cluster highlighting.

### Patterns

**Followed**: Adapter, Observer, Factory, Repository. Confidence, places, enforcement status (rule / suggested / gap). Example code.

**Anti-patterns**: duplicated auth guard, god-node router, duplicated retry, monolithic session.ts, dead code, copy-paste error handling. Severity, occurrences, suggested fix cross-linked to a constructive pattern.

### Sessions

Scoped to this project. Same format as observatory sessions table.

### Settings

External links (Jira, Grafana, Linear — auto-discovered from READMEs). Project guidelines/rules. Backlog items. Skill toggles. Exclusion globs. Privacy (log prompts, redact secrets, share toggle).

**Mockup reference**: `lib/project-pages.jsx`, `lib/project-shared.jsx`, `lib/project-data.js`

---

## 7. Libraries

Three groups: **Detected** (from manifests), **Imported** (internal SDKs, llms.txt), **External services** (via MCP).

**Detail panel**: tagline, version, doc status, usage (top symbols + call sites), rules attached, **MCP example interactions** (library.explain, library.find-usage, library.suggest-rule, library.doc-drift).

**Two layouts**: Unified list + side detail (flat), Workspace with tabs (detected / imported / services).

**Mockup reference**: `lib/libraries.jsx`

---

## 8. Instruments (MCP Tools)

Replaced the single "MCP Playground" with a three-tab surface:

| Tab | Kanji | Question it answers |
|-----|-------|-------------------|
| Playground | 具 | What CAN these tools do? |
| Replay | 録 | What DID the assistant do? |
| Insights | 照 | What SHOULD we change? |

**Playground**: MCP scope switcher (sensei + third-party), tool list with kind chips (action / query), interactive form, request/response preview. Simplified variant replaces pill row with collapsible tree in left rail.

**Replay**: Per-session tool-call timeline. Each call shows: tool, args, response snippet, duration, and whether the assistant **used** the response (used / partial / ignored).

**Insights**: Aggregated usage + effectiveness metrics across sessions. Tool call frequency, success rates, usage patterns.

**Mockup reference**: `lib/instruments.jsx`, `lib/instruments-simple.jsx`, `lib/instruments-data.js`, `lib/mcp-signals-data.js`, `lib/mcp-replay-insights.jsx`

---

## 9. Navigation

**Three variants**: Grid (cards, search, status filter), Command-K palette (overlay, searches everything), Browser (left tree + main grid).

**Mockup reference**: `lib/navigation.jsx`

---

## Terminology: Mockup → Database

The mockups use a fictional product (Lumen). The real database uses different names:

| Mockup term | DB entity | DB table |
|-------------|-----------|----------|
| Solution (e.g. "Lumen Studio") | Project | `sensei.projects` |
| Repo (e.g. "lumen-canvas") | Folder (kind=git) | `sensei.folders` |
| Folder root (e.g. "~/code/lumen") | Watch root | `sensei.folders_to_watch` |
| ACP / assistant | Coordinator install | `sensei.coordinator_installs` |
| Coaching koan | Recommendation | `inference.recommendations` |
| MCP server | Service (protocol=mcp) | `sensei.services` |
| Inference provider | Service (kind=inference) | `sensei.services` |

---

## Key Design Decisions

1. **FTR (First-Try-Right)** is the hero metric — `activity.sessions.ftr` boolean (corrections == 0)
2. **Projects** group folders (repos) into products; folders are individual git/subtree entries within projects
3. **Libraries sensei wraps** (no MCP) vs **services** (have their own MCP tools) — clearly separated
4. **Coaching is koan-like** — one hero insight at a time, terse zen phrasing
5. **Two maturity states**: early (listening) and mature (teaching)
6. **Recommendations carry pre-built prompts** — `inference.recommendations.prompt` sent to the ACP
7. **Everything local** — daemon on localhost:9823, nothing leaves the machine
8. **Instruments replace Playground** — three tabs: try / replay / analyze
9. **Memory is structured** — `sensei.memories` with scope, strength, status, impact + evidence/examples
10. **Sessions are cross-project** — retrospective analysis surfaces cross-cutting patterns

---

## DB ↔ UI Gap Analysis

See `../journeys/README.md` for the full gap analysis with action items.
