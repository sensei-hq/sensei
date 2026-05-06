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
| Matcha | greenish | Positive states (secondary) |

**Typography**: Fraunces (display/numerals), Inter (UI), JetBrains Mono (code).

**Voice**: Terse, koan-like. *"The AI does not know your auth."* *"Three corrections. One teacher."*

**Direction chosen**: Merge of Ma + Enso. Collapsible sidebar (wide with names <> icon-only). Ma's trendline (not Enso's arc ring). Enso's two-column layout. Shoji rejected as too crowded.

**Fictional product**: Lumen (design tool company) with 3 solutions: lumen-studio, lumen-cloud, brand-kit.

**Mockup reference**: `lib/tokens.css`, `lib/primitives.jsx`

---

## Three Main Artboards

| # | Artboard | Purpose | Entry |
|---|----------|---------|-------|
| 1 | **Bootstrap** | Ensure all prerequisites are in place | Every app launch |
| 2 | **Setup Wizard** | Guide user through first-time configuration | After bootstrap on first run |
| 3 | **Observatory** | Daily landing — observe, understand, improve | Returning users |

All other screens are embedded within these three or are discarded exploration.

---

## All Screens

| Screen | Mockup file(s) | Artboard | Journey |
|--------|---------------|----------|---------|
| Bootstrap (6 gates) | `bootstrap.jsx` | 1 | J1 |
| Setup wizard (11 stages) | `setup-wizard.jsx`, `setup-data.js` | 2 | J2 |
| Inference providers + models | `wiz-inference.jsx` | 2 | J2 |
| Model role assignments | `wiz-assignments.jsx` | 2 | J2 |
| Observatory daily (early + mature) | `observatory.jsx` | 3 | J3 |
| Sessions (digest + retro) | `sessions.jsx`, `sessions-zen.jsx` | 3 | J3, J5 |
| Learnings (original + triage + anatomy + brief) | `learnings.jsx`, `learnings-v2.jsx`, `learnings-anatomy-v2.jsx`, `learnings-data.js` | 3 | J9 |
| Project pages (3 layout variants) | `project-pages.jsx`, `project-shared.jsx`, `project-data.js` | 3 | J5 |
| Project filter | `project-filter.jsx` | 3 | J5 |
| Code graph (3 lenses) | `project-shared.jsx` | 3 | J5 |
| Patterns + anti-patterns | `project-shared.jsx` | 3 | J5 |
| Recommendations + action drawer | `project-shared.jsx` | 3 | J5, J6 |
| Project settings (2 variants) | `project-shared.jsx` | 3 | J5 |
| Libraries (2 variants) | `libraries.jsx` | 3 | J5 |
| Instruments: Playground | `instruments.jsx`, `instruments-simple.jsx`, `instruments-data.js` | 3 | J5 |
| Instruments: Replay | `mcp-replay-insights.jsx`, `mcp-signals-data.js` | 3 | J5 |
| Instruments: Insights | `mcp-replay-insights.jsx`, `mcp-signals-data.js` | 3 | J5 |
| Navigation (3 variants) | `navigation.jsx` | 3 | J3 |
| Design tokens + primitives | `tokens.css`, `primitives.jsx` | All | All |
| Design canvas (meta) | `design-canvas.jsx` | — | — |

**Data fixtures**: `data.js` (shared sessions/events), `project-data.js`, `setup-data.js`, `instruments-data.js`, `mcp-signals-data.js`, `learnings-data.js`

**Discarded explorations**: `lib/discarded/wiz-inference-ladder.jsx`, `lib/discarded/wiz-assignments-tabs.jsx`

---

## 1. Bootstrap

**Purpose**: Runs on every app launch. Verifies prerequisites before the wizard or observatory loads.

**Gates** (sequential):

| # | Gate | Kanji | Check | Remedy |
|---|------|-------|-------|--------|
| 1 | Homebrew | 一 | `which brew` | Link to brew.sh |
| 2 | PostgreSQL | 二 | `brew list postgresql@16` | `brew install` |
| 3 | Ollama | 三 | `brew list ollama` | `brew install` |
| 4 | Sensei components | 四 | `sensei --version` | `brew install` (CLI, MCP bridge, daemon) |
| 5 | Database | 五 | `sensei db:create` | Manual psql + DATABASE_URL fallback |
| 6 | Daemon | 六 | `sensei daemon:start` | Starts once DB is reachable |

**Sub-components** (gate 4): cli (`sensei --version`), mcp (`sensei mcp --check`), daemon (`sensei daemon --check`).

**Statuses per gate**: `ready`, `checking`, `starting`, `missing`, `error`, `pending`.

**Behavior**: First blocked gate expands its remedy panel. All-green auto-advances (900ms delay) to empty state (first run) or observatory (returning user).

**Preset scenarios**: all-checking, missing-homebrew, missing-prereqs, missing-db, daemon-starting, all-green.

### Upgrade flow (post-gate-6)

After all 6 gates pass, bootstrap checks `sensei.assistants` rows where `configured = true`. If any assistant's `configured_version` is older than the running sensei version:

1. Pull updated skills, commands, agents, hooks from the current sensei release
2. Re-register extensions with each stale assistant (re-push MCP config, skills, hooks)
3. Update `configured_version` and `configured_at` on each reconfigured assistant row
4. If all assistants are current, skip silently (< 2s total bootstrap)

This ensures that `brew upgrade sensei` (or desktop auto-update) propagates new capabilities to every configured assistant without requiring the user to re-run the setup wizard. The upgrade is invisible on healthy systems — same as the health check.

**Mockup reference**: `lib/bootstrap.jsx` (upgrade flow not yet mocked — runs within existing gate sequence)

---

## 2. Setup Wizard

**Layout**: Left rail stepper (completed steps show as collapsed chips, current expanded) + main content area + bottom bar with progress ticks.

### Stage 1: Welcome (礼)

"A teacher does not write the code." Three pillars: Observe, Teach, Local. ~4 min, nothing leaves your machine.

### Stage 2: Preferences (名)

User sets display name (derived from `$HOME`), home directory, sharing preferences:
- `contributeLearnings` — share anonymized insights
- `reviewBeforeShare` — manual approval before sharing
- `shareSchedule` — cadence (e.g. weekly-saturday)
- `downloadCollective` — receive collective insights cadence
- `correctionAggressiveness` — how aggressively to flag corrections
- `digestCadence` — daily/weekly digest frequency
- `nudgeOnRegression` — alert on FTR regression
- `anonymizedTelemetry` — opt-in usage telemetry
- `showWelcome` — show welcome on next launch

### Stage 3: Assistants (連)

Detect installed AI coding tools (Claude Code, Cursor, Zed, Continue). Toggle checkboxes to register sensei's plugins/skills/commands/logging. Only found tools are selectable. Per ACP: `{id, name, version, found, path}`.

### Stage 4: Folders (庵)

User adds root paths (e.g. `~/code/lumen`). Recursive scan. Add/remove/browse. Minimum 1 required to continue.

### Stage 5: Scan (観)

Live SSE-style view. **Left**: solution cards materialize as repos are discovered, with progress bars per repo. **Right**: scrolling SSE event log. Stats strip: roots / discovered / queued / processed. Completion banner when done.

**Discovered data per solution**: `{id, name, kanji, path, autoDetected, confidence, projects[]}`. Per project within a solution: `{id, name, path, files, lang, suggestedRole}`.

**Detected stack**: `{languages[], frameworks[], runtimes[], services[]}`.

### Stage 6: Projects (組)

A project = one or more repos, auto-grouped by folder proximity and naming. User can: rename, split multi-repo into singles, merge projects, move repos between projects. Assign roles per repo (backend / frontend / library / docs / infra). Set project metadata: `{status, client, goal}`.

### Stage 7: Libraries (書)

Libraries WITHOUT their own MCP — sensei wraps them (indexes docs/code, exposes its own tools). Auto-detected from Cargo.toml / package.json. User can add custom ones (`{name, url, lang}`). Distinct from MCP registry. Per library: `{id, name, version, lang, usage, source, docs, why}`.

### Stage 8: Instruments (器)

MCP registry: services/tools with their own MCP (Postgres, Redis, Stripe, GitHub, Sentry, Playwright, Figma). Recommended based on detected stack. Per service: `{id, name, publisher, kind, summary, trigger[], tools, verified, installed, recommended}`.

### Stage 9: Inference (想)

**Providers & Models** (`lib/wiz-inference.jsx`). Detects hardware (`{chip, ram, cores, os}`). Lists providers: Ollama (local), Anthropic, OpenAI, Google. Auto-detects env vars (ANTHROPIC_API_KEY, etc.). Ollama models pull with progress bars; cloud models enable via API key. Recommended models get a subtle badge.

**Per provider**: `{id, name, envVar, configured, models[]}`. Per model: `{id, name, sizeGB, recommended, pulled, note}`.

### Stage 10: Assignments (任)

**Five reasoning roles** (`lib/wiz-assignments.jsx`): inference, consolidation, embedding, voice, fallback. User builds ordered priority list per role. First entry is primary; rest are fallbacks. Drag-reorder within role. See model specs (GPU requirement, latency estimate).

### Stage 11: Done (入)

"The observatory is ready." Summary stats. Koan: *"the first session is always the teacher."*

**Mockup reference**: `lib/setup-wizard.jsx`, `lib/setup-data.js`, `lib/wiz-inference.jsx`, `lib/wiz-assignments.jsx`

---

## 3. Observatory (Daily View)

The daily home screen. Two maturity states: **early** (listening, forming signals) and **mature** (confident, actionable teachings).

**Layout**: Collapsible left sidebar + main content.

### Sidebar

- Observatory nav: Today, Sessions, Insights, Memories, Libraries
- Active projects (with FTR dots, warning indicators)
- Recent/dormant projects (collapsed)
- Daemon heartbeat status
- Mode toggle (early / mature)
- Settings link

### Main: Today

- **Greeting strip**: date + personalized hello (morning/afternoon/evening variants)
- **FTR hero**: value + delta vs prior period + 14-day sparkline trend
- **Hero koan**: the single most important teaching. Large kanji, terse title, explanation, projected impact, action button, evidence trail. Mature vs early state.
- **Insights** (max 3): pattern recurring, teaching adopted, drift detected. Each: `{kanji, label, text, tag, tone}`. Tone-coded (warn / good / mute).
- **Adopted teachings**: "System has learned" — rules sensei has applied. Each: `{when, what, scope, source}`. Empty state when early.
- **Recent sessions**: compact table. Each: `{id, project, title, time, duration, ftr, corrections}`.
- **First-entry toast**: "The observatory is open" — once after setup.

**FTR data model**: `{value, delta, prev, trend14[]}`.

**Mockup reference**: `lib/observatory.jsx`

---

## 4. Sessions

Cross-project session browser with retrospective analysis.

**Two layouts**: **Digest** (retro cards on top + filterable list) and **Zen** (`sessions-zen.jsx`, more focused).

**Session data**: `{id, title, project, when, time, duration, turns, tokens, ftr, corrections, module, outcome, summary, agent}`.
- Outcomes: `first-try`, `shipped`, `corrected`, `abandoned`
- Agents: `claude-code`, `cursor`

**Retro sections**:
- Going well: `{kanji, title, body, evidence[], delta}` — high FTR streaks, adopted patterns, shorter sessions
- Not going well: `{kanji, title, body, evidence[], delta, action?}` — recurring corrections, low FTR projects, abandoned sessions
- Insights: `{kanji, title, body, evidence[], tone}` — cross-project signals

**Session totals**: count (7d), ftr (%), corrections (#), projects (#).

**Checkpoints**: `{id, when, kanji, title, body, affects[]}`.

**Filters**: project, language (Rust/TypeScript/Python), outcome (shipped/corrected/abandoned).

**Session events** (timeline drill-in): `{kind, t, text}`. Kinds: start, context, edit, test, correction, end.

**Mockup reference**: `lib/sessions.jsx`, `lib/sessions-zen.jsx`

---

## 5. Learnings

Consolidated patterns + memories + recommendations across all projects.

**Original layout** (`lib/learnings.jsx`): hero stats, recommendations inbox, 6 tabs (all / memories / patterns / corrections / lifecycle / archive), scope + project filters, sort (priority / strength / recency), memory drawer.

**Simplified alternatives** (`lib/learnings-v2.jsx`, `lib/learnings-anatomy-v2.jsx`):
- **Triage**: three columns (Now / Soon / Settled) by immediacy
- **Anatomy**: one memory at a time — What / Why / How / Where
- **Brief**: single scrollable brief with chart + grouped lists

**Memory data**: `{id, what, because, scope, strength, state, references}`.
- Scope: `{level, project, modules[], stack[]}`
- State: `active`, `reinforced`, `challenged`, `battle-tested`, `archived`
- Strength: 1-5
- References: `{good_example, bad_example, pattern, evidence, related, doc}`

**Recommendations**: `{id, kind, title, reasoning, action, impact, targetKind, targetName}`.
- Kinds: `promote-pattern`, `create-agent`, `write-skill`, `archive-memory`, `enrich-memory`, `cross-project`
- Impact: `high`, `medium`, `low`

**Counts**: memories, patterns, recs, ftrFromMemory.

**Mockup reference**: `lib/learnings.jsx`, `lib/learnings-v2.jsx`, `lib/learnings-anatomy-v2.jsx`, `lib/learnings-data.js`

---

## 6. Project Page

Per-project detail view.

**Three layout variants**: Top tabs (classic), Left rail (more content), Long scroll (zen, right-side "on this page" anchors).

**Project data**: `{id, kanji, name, client, goal, icon, stack, repos[], ftr, ftrPrev, sessions7d, preferredAcp, ftr14[]}`.
- Icon: `{kind, value, bg, fg}`
- Stack: `{languages[], frameworks[], runtimes[], services[]}`
- Repos: `{id, path, stars, size, lang}`

### Overview

Project header (kanji, name, goal, stack tags). FTR trend. Recommendations panel — each with urgency, koan title, why, impact, evidence, and a **pre-built prompt** for the ACP. "Apply" flow generates a task.

### Graph

Code-graph visualization. Nodes: `{id, repo, fan, rework, stale, pattern, group, hot, x, y, size}`. Edges: `[from, to]` pairs. Duplicates: `{id, title, confidence, files[], sketch}`.

Three lenses: Complexity (god-nodes by fan-in), Rework (repeat edits), Staleness (days untouched).

### Patterns

**Followed**: `{id, kanji, name, family, places, recent, confidence, status, summary, example, file, enforcement}`. Status: `rule`, `suggested`, `gap`.

**Anti-patterns**: `{id, kanji, name, type, severity, occurrences, places[], summary, example, suggest}`. Types: duplication, god-node, monolith, dead-code. Suggest cross-links to constructive pattern.

### Sessions

Scoped to this project. Same format as observatory sessions table.

### Settings

- **Links**: `{id, kind, label, url}` — external integrations (docs, dashboard, issues, runbook)
- **Guidelines**: `{id, rule}` — project rules
- **Backlog**: `{id, task, added}` — tasks
- **Skills**: `{id, name, on}` — per-project skill toggles
- **Exclusion globs** — files/folders to ignore
- **Privacy**: `{logPrompts, logFileContents, redactSecrets, shareWithCloud}`

**Mockup reference**: `lib/project-pages.jsx`, `lib/project-shared.jsx`, `lib/project-data.js`, `lib/project-filter.jsx`

---

## 7. Libraries

Three groups: **Detected** (from manifests), **Imported** (internal SDKs, llms.txt), **External services** (via MCP).

**Library data**: `{name, tagline, version, lang, source, summary, docs, usage, rules[], mcpExamples[]}`.
- Docs status: `indexed`, `partial`, `schema`, `none`
- Usage: `{topSymbols[], places[]}`
- MCP examples: `{tool, intent, request, response}`

**Filters**: kind (all/code/service), language (all/rust/ts/docs/mcp), search by name.

**Two layouts**: Unified list + side detail (flat), Workspace with tabs (detected / imported / services).

**Mockup reference**: `lib/libraries.jsx`

---

## 8. Instruments (MCP Tools)

Three-tab surface with shared chrome (`InstrumentsShell`):

| Tab | Kanji | Question it answers |
|-----|-------|-------------------|
| Playground | 具 | What CAN these tools do? |
| Replay | 録 | What DID the assistant do? |
| Insights | 照 | What SHOULD we change? |

**MCP data**: `{id, name, kanji, tagline, toolCount, actionCount, queryCount}`. Per tool: `{id, mcp, name, kind, summary}`. Kind: `action` (mutates state), `query` (reads info).

**Playground**: MCP scope switcher (sensei + third-party), tool list with kind chips (action / query), interactive form, request/response preview. Simplified variant replaces pill row with collapsible tree in left rail.

**Replay**: Per-session tool-call timeline. Each call shows: tool, args, response snippet, duration, and whether the assistant **used** the response (used / partial / ignored).

**Insights**: Aggregated usage + effectiveness metrics across sessions. Tool call frequency, success rates, usage patterns.

**Mockup reference**: `lib/instruments.jsx`, `lib/instruments-simple.jsx`, `lib/instruments-data.js`, `lib/mcp-signals-data.js`, `lib/mcp-replay-insights.jsx`

---

## 9. Navigation

**Three variants**: Grid (cards + search + status filter), Command-K palette (overlay, searches everything), Browser (left tree + main grid).

**Project card data**: `{id, kanji, name, client, status, ftr, warn, sessions7d, repos, libs, lastSession}`. Statuses: `active`, `recent` (dormant), `archived`.

**Command-K palette** searches: projects (with FTR readout), libraries (with version + usage), recent sessions, commands (import project, import library, run scan).

**Mockup reference**: `lib/navigation.jsx`

---

## Terminology: Mockup -> Database

The mockups use a fictional product (Lumen). The real database uses different names:

| Mockup term | DB entity | DB table |
|-------------|-----------|----------|
| Solution (e.g. "Lumen Studio") | Project | `sensei.projects` |
| Repo (e.g. "lumen-canvas") | Folder (kind=git) | `sensei.folders` |
| Folder root (e.g. "~/code/lumen") | Watch root | `sensei.folders_to_watch` |
| ACP / assistant | Assistant | `sensei.assistants` (family, configured, configured_version) |
| Coaching koan | Recommendation | `inference.recommendations` |
| Teaching adopted | Memory (status=reinforced+) | `sensei.memories` |
| MCP server | Service (protocol=mcp) | `sensei.services` |
| Inference provider | Service (kind=inference) | `sensei.services` |
| Inference role assignment | Inference assignment | `gateway.inference_assignments` |

---

## Kanji Mapping

| Kanji | Meaning | Used in |
|-------|---------|---------|
| 先生 | sensei/teacher | Brand |
| 観 | observatory | Daily view, scan step |
| 家 | home/today | Sidebar nav |
| 刻 | session/moment | Sessions |
| 學 | learnings | Learnings page |
| 紋 | patterns/crest | Patterns section |
| 場 | projects/place | Projects |
| 具 | instruments/tools | Playground tab |
| 録 | replay/record | Replay tab |
| 健 | health | Health tab |
| 書 | libraries | Libraries |
| 設 | settings | Settings |
| 省 | retrospective | Retro cards |
| 薦 | recommendations | Recommendation cards |
| 礼 | welcome | Wizard step 1 |
| 名 | name/preferences | Wizard step 2 |
| 連 | assistants/connect | Wizard step 3 |
| 庵 | folders/hermitage | Wizard step 4 |
| 組 | projects/group | Wizard step 6 |
| 器 | instruments/MCP | Wizard step 8 |
| 想 | inference/thought | Wizard step 9 |
| 任 | assignments/duty | Wizard step 10 |
| 入 | enter/done | Wizard step 11 |

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
11. **Wizard has Preferences step** — sharing, privacy, digest cadence set up-front before any scanning

---

## Gap Analysis: Mockups <> Ideas <> Journeys <> Database

### Legend

- **Covered** = mockup exists, DB supports it, journey describes it
- **Mockup gap** = journey/idea describes a screen not yet mocked
- **DB gap** = mockup/journey references data the schema doesn't store
- **Journey gap** = mockup/idea exists but no journey narrative
- **Idea only** = idea documented but neither mocked nor in journey UI

---

### A. DB supports the mockup (no gaps)

| UI element | DB table(s) | Notes |
|-----------|-------------|-------|
| Projects (mockup "solutions") | `sensei.projects` | name, client, maturity, goal, icon, stack, links, guidelines, preferred_acp, tags |
| Folders (mockup "repos") | `sensei.folders` | kind, role (typed enum), project_id FK, props, remote_urls |
| Watch roots (wizard step 4) | `sensei.folders_to_watch` | path, name, note, status, excluded |
| Sessions + FTR | `activity.sessions` | folder_id, project_id, acp_id, outcome, ftr boolean, turns, corrections, tokens, duration_ms |
| Task sessions | `activity.task_sessions` | ftr_score numeric, ftr_signals JSON, task_type |
| Events (session timeline) | `activity.events` | event_type enum, data JSONB (tool_call, correction, phase_transition, etc.) |
| Session snapshots | `activity.snapshots` | progress_summary, next_step_hint, completed_steps, in_flight_files |
| Code graph | `sensei.nodes`, `sensei.edges` | kind, degree, community_id, embedding; edge kinds + confidence |
| Communities | `inference.communities` | folder_id, community_id, god_node_ids, node_count |
| Patterns + anti-patterns | `inference.detected_patterns` | lifecycle, is_anti_pattern, severity, confidence, instances, evidence, fix_pattern_id |
| Recommendations | `inference.recommendations` | urgency, title, why, impact, evidence, action_type, prompt, baseline_ftr, current_ftr, verdict |
| Reasoning traces (MOE panel) | `inference.reasoning_traces` | models_used, exchanges, consensus, action_proposed |
| Memories | `sensei.memories` | scope, type, strength, status, reinforced_count, violated_count, last_relevant_at |
| Memory evidence + examples + links | `sensei.memory_*` tables | Good/bad examples, evidence sessions, memory-to-memory relationships |
| Memory audit trail | `history.past_memories` | Auto-populated by trigger |
| Libraries | `sensei.libraries`, `library_pages`, `referenced_libraries` | Per-folder library usage, doc pages, version tracking |
| Services / MCP registry | `sensei.services` | kind, protocol (mcp/ollama/anthropic/openai), trigger_stacks, installed, verified |
| Extensions (skills/agents/hooks) | `sensei.extensions` | kind, scope, source, content, enabled; audit trail in `history.past_extensions` |
| Extension audit trail | `history.past_extensions` | Auto-populated by trigger |
| Inference role assignments | `gateway.inference_assignments` | Maps roles to fallback chains |
| Inference models + routing | `gateway.*` tables | providers, models, routers, fallback_chains, fallback_chain_models |
| Drift detection | `inference.drift_items` | doc_node, code_node, status, signatures |
| Hyperedges | `inference.hyperedges`, `hyperedge_members` | Multi-node relationships (DB ready, no mockup yet) |
| Scan state | `sensei.scan_state` | folder_id, file_path, mtime, content_hash |
| Index errors | `sensei.index_errors` | folder_id, file_path, error, adapter, phase |
| Config (key-value) | `sensei.config` | Stores user preferences from wizard |
| Workflow state | `sensei.workflow_state` | Per-project phase tracking |
| Benchmarks | `sensei.benchmark_runs`, `benchmark_reports` | A/B runs with cost tracking |
| Collective insights | `inference.insights`, `insight_batches` | Anonymized sharing batches |
| Tags | `sensei.tags` | Controlled vocabulary |
| External links | `sensei.projects.links` | JSONB array on projects |
| Project guidelines | `sensei.projects.guidelines` | JSONB array on projects |

---

### B. Resolved DB gaps

All previously identified DB gaps have been addressed:

| Gap | Resolution | DDL file(s) |
|-----|-----------|-------------|
| **Assistants (ACPs)** | New `sensei.assistants` table with `family` enum for grouping, `configured`/`configured_at`/`configured_version` for upgrade tracking | `enum/sensei/assistant_family.ddl`, `table/sensei/assistants.ddl` |
| **Project backlog** | Added `projects.backlog` JSONB column (`[{id, task, added}]`) | `table/sensei/projects.ddl` |
| **Project privacy** | Added `projects.privacy` JSONB column (`{logPrompts, logFileContents, redactSecrets, shareWithCloud}`) | `table/sensei/projects.ddl` |
| **Project exclusion globs** | Added `projects.excluded_globs` JSONB column (`["dist/**", "*.generated.*"]`) | `table/sensei/projects.ddl` |
| **Session summary** | Added `sessions.summary` text column, populated at checkpoint time | `table/activity/sessions.ddl` |
| **User preferences** | Stored in `sensei.config` key-value table (no change needed) | `table/sensei/config.ddl` |
| **Upgrade flow** | `assistants.configured_version` compared against running version at bootstrap; stale assistants get extensions re-pushed | `table/sensei/assistants.ddl` |

---

### C. Mockup gaps — per-screen breakdown

Each gap is a screen or section described in journeys/ideas but not yet mocked. Grouped by where it fits in the three artboards, with what it needs to show and what DB support exists.

---

#### Bootstrap (artboard 1)

**Upgrade reconfiguration indicator**
- After gates pass, bootstrap checks `sensei.assistants` for rows where `configured_version < running_version`
- Needs to show which assistants are being reconfigured, what's being pushed (skills count, hooks count, commands count), and completion status
- DB support: `sensei.assistants` has `configured`, `configured_at`, `configured_version`; `sensei.extensions` has all extension definitions with `revision` for diffing
- Why: without this, users who upgrade sensei have no visibility into whether their assistants received the new capabilities

---

#### Observatory daily (artboard 3) — new sections within existing sidebar nav

**Doc traceability view** (sidebar: project drill-in or dedicated nav item)
- Show per-project documentation health: how many doc-to-code links exist, how many are current vs drifted vs broken
- List doc files with their link count and health status (percentage current)
- Drill into a doc file to see each reference and whether the target symbol still matches
- For drifted/broken references, surface a "fix drift" action that sends a pre-built prompt to the assistant
- DB support: `inference.drift_items` stores doc_node_id, code_node_id, status (current/drifted/broken), expected_signature, actual_signature. `sensei.nodes` has the doc and code nodes
- Why: drift is invisible until someone reads stale docs and writes wrong code. This is the only place drift becomes visible before it causes a session correction

**Change impact report** (accessible from: recommendation cards, adopted teachings, observatory insights)
- Show the before/after comparison for an accepted recommendation: FTR percentage, average corrections per session, tool usage delta, average session duration
- Show the measurement window (when accepted, how many sessions measured, days elapsed)
- Show a verdict (positive / neutral / negative) with a one-paragraph reasoning summary from the MOE panel
- Allow the user to revise the rule, revert the change, or keep monitoring
- DB support: `inference.recommendations` has `baseline_ftr`, `current_ftr`, `verdict`, `acted_at`, `measured_at`, `props` (for corrections_avg, tool_usage_delta). `inference.reasoning_traces` has the full MOE exchange (models_used, exchanges, consensus)
- Why: without measurement, recommendations are faith-based. This closes the loop — users see whether sensei's advice actually improved their FTR, which builds trust or triggers correction

**Negative impact alert** (banner/overlay, triggered when verdict = negative)
- Surface when a recommendation's measured impact is negative (FTR dropped after applying the change)
- Show the FTR and corrections deltas (before vs after) with directional indicators
- Show the MOE reasoning trace — what each model concluded about why the change hurt
- Offer actions: revise the rule (opens action drawer with revision prompt), revert the change, keep monitoring, dismiss
- DB support: same as change impact report — `recommendations.verdict = 'negative'`, `reasoning_traces` for the analysis
- Why: regression safety net. If sensei gives bad advice, the user needs to know immediately and have a clear path to undo it

**Memory consolidation review** (within Learnings/Memories section)
- Show memories that the system has identified as candidates for merging (overlapping scope, similar content, complementary evidence)
- For each consolidation proposal: list the source memories being merged, show the proposed merged memory (combined title, combined rules, merged "because" text, combined evidence), show the resulting strength (highest of sources)
- Allow the user to accept (archives originals, creates merged) or keep separate (dismiss suggestion)
- DB support: `sensei.memory_links` stores parent_id/child_id relationships for consolidation. `sensei.memories` has all memory fields. `history.past_memories` preserves the audit trail when originals are archived
- Why: without consolidation, memories accumulate and overlap. A project that has run for months will have dozens of memories saying slightly different things about the same topic. Consolidation keeps the memory set focused and reduces context token cost

**Collective intelligence settings** (within Settings or Preferences)
- Toggle sharing mode: auto-share anonymized insights, review before sharing, or off
- Show sharing history: what was shared, when, how many insights per batch
- Show contribution summary: "contributed X insights helping Y users"
- DB support: `inference.insights` links source records to shared batches via `insight_batches`. Categories: pattern, model, skill, correction, ftr, anti_pattern, tool, stack
- Why: sharing is promised in the wizard (step 2 preferences) but there's no place to review what was actually shared or change the setting after setup

---

#### Project pages (artboard 3) — new tabs or sections within existing project view

**Architecture conformance** (new section within project Graph tab, or standalone tab)
- Cross-repo edge analysis for multi-repo projects: which repos call which, expected vs unexpected dependencies
- Surface boundary violations (e.g. frontend directly calling database), circular dependencies, isolated repos with no edges
- Show a service map: which repos use which external services (Postgres, Stripe, etc.)
- DB support: `sensei.edges` across folders within the same project. Edge kind `calls`/`imports`/`depends_on` with cross-folder source_id/target_id. `sensei.services` for external service mapping via `referenced_libraries` and `services.trigger_stacks`
- Why: single-repo code graphs are already mocked. Multi-repo projects (the "solution" concept) have no way to see how repos connect, which is where the most damaging architectural drift happens

**Testability overlay** (new lens in existing code graph, alongside Complexity/Rework/Staleness)
- Per-function testability score (0-100) based on: cyclomatic complexity, side effects, dependency count, test existence
- Color-code nodes by testability score on the existing graph visualization
- Flag files below threshold (e.g. < 40) as "hard to test"
- DB support: scores stored in `nodes.props` as computed by the workspace intelligence pipeline. Components: complexity from AST, side_effects from call analysis, dependency_count from edges, test existence from file naming conventions
- Why: the code graph already shows complexity, rework, and staleness. Testability is the fourth lens that directly connects to FTR — hard-to-test code generates more corrections

**Pattern catalog (industry)** (new section within project Patterns tab)
- Browse patterns from external catalogs (GoF structural/behavioral/creational, resilience, data-access) alongside detected project patterns
- For each catalog pattern: name, family, description, detection status ("detected in N places" or "not present — recommended"), FTR correlation if detected
- Allow importing a pattern as a project rule or marking it as a gap to track
- DB support: `inference.detected_patterns` has lifecycle (suggested/gap/rule), family, is_anti_pattern. Pattern names matched against external catalogs at indexing time. FTR correlation computed by joining patterns with sessions
- Why: current patterns tab shows what's detected in the codebase. Catalog adds what *could* be there — industry best practices the project hasn't adopted yet

---

#### Settings (artboard 3) — new sections within existing settings area

**Extensions browser**
- List all installed extensions filtered by kind (skill, command, agent, hook, plugin) and scope (global, project-specific)
- Show per extension: name, kind, source (builtin/marketplace/local), enabled toggle, version, description
- Allow creating new extensions, importing from .md files, and toggling per-project
- DB support: `sensei.extensions` has kind, scope, source, enabled, content, props, revision. `history.past_extensions` provides full edit history
- Why: extensions are the customization surface for sensei. Without a browser, users can only manage them via file system or CLI. The wizard creates initial extensions; this is where users maintain them

**Skill / agent / persona editors** (drill-in from extensions browser)
- Skill editor: frontmatter fields (name, description, trigger patterns) + markdown body, with live preview of assembled `get_session_context()` output when the skill is active
- Agent editor: name, trigger type (manual/scheduled/event-driven), tool access checklist, autonomy level (autonomous/approval_required/manual), template selector, test panel that replays against historical sessions
- Persona editor: trigger patterns (cwd globs, file type filters), rules list, context files, evidence trail (sessions/corrections that inspired this persona), preview panel
- DB support: all stored in `sensei.extensions` with kind-specific `props` (triggers, inputs, tool_access for agents; trigger patterns for personas). `activity.sessions` for test replay data
- Why: power-user feature for customizing how sensei behaves in sessions. Low traffic but high impact for users who need it

**Multi-ACP configuration** (within Settings > Assistants)
- List detected assistants grouped by family, with registration status and version
- Per assistant: what's registered (MCP server, plugins, skills, hooks, logging), transport config (stdio/http), connection test result
- Allow re-registering if config changed, or deregistering
- DB support: `sensei.assistants` has family, name, version, path, transport, configured, configured_at, configured_version, props
- Why: wizard step 3 handles initial registration, but users add/remove/update assistants over time. This is also where the upgrade flow surfaces if automatic reconfiguration encountered issues

**Benchmark runner**
- Define a task corpus (list of task descriptions drawn from project history or manually written)
- Configure A/B variants: baseline (no change) vs experimental (with a specific skill, rule, or persona)
- Run benchmark and see results: FTR, average corrections, token usage, duration — per variant with deltas
- Show a conclusion summary and allow promoting the change to permanent or discarding
- DB support: `sensei.benchmark_runs` has folder_id, task_description, sensei_enabled (A/B flag), cost tracking, token tracking. `sensei.benchmark_reports` has run_name, strategy, score, payload
- Why: credibility feature — users want evidence that sensei's recommendations actually help before committing to them. Also used for comparing configurations (e.g. "is this new skill better than the old one?")

**Capability registry / degraded features** (within Settings > Assistants)
- Per assistant, show which sensei features are fully supported, which use workarounds, and which are unavailable due to ACP limitations
- Link to upstream feature requests for unavailable capabilities (e.g. token counting, tool response capture, quota introspection)
- DB support: `sensei.assistants.props` can store a capabilities map. Feature request references stored as links. Workaround status tracked per capability per assistant family
- Why: different assistants support different features (Claude Code has hooks, Cursor doesn't; some expose token counts, others don't). Users need to understand why certain features are missing for their chosen ACP

---

#### In-session (no desktop screen — ACP context)

**Session recovery prompt**
- When a session starts in a project/folder that has an interrupted prior session, offer to restore from the last snapshot
- Show: what was being worked on, which files were in flight, pending steps, last tool results
- Allow continuing from snapshot or starting fresh
- DB support: `activity.snapshots` has session_id, progress_summary, next_step_hint, completed_steps, in_flight_files, worktree_refs. Linked to `activity.sessions` for the interrupted session
- Why: crash recovery prevents lost context. Without this, every ACP crash or token limit hit forces a cold start

**Context pack tool**
- Mid-session tool for when context is stale (session has been running long, assistant is forgetting rules)
- Show current progress snapshot, which memories would be reloaded (grouped by scope), what accumulated context noise would be cleared
- Execute rotation: save snapshot, clear stale context, reload memories and active files
- DB support: `activity.snapshots` for saving state. `sensei.memories` for context assembly. `activity.events` for tracking what's been loaded
- Why: long sessions accumulate stale file reads and lose track of rules. This is the "refresh" button that prevents quality degradation without restarting the session

---

### D. Idea coverage

| Idea | Title | Mocked? | Journey | Gap |
|------|-------|---------|---------|-----|
| 01 | Workflow System | — | J4 | No gap — invisible, runs inside ACP via slash commands |
| 02 | Commands | — | J4 | No gap — ACP-level, no desktop UI needed |
| 03 | Configuration | Yes | J1, J7 | Covered |
| 04 | Cross-Cutting | — | J4 | No gap — backend quality gates enforced in sessions |
| 05 | Decisions | — | — | No gap — reference doc, no UI needed |
| 06 | Docs Disposition | — | — | No gap — file reorganization plan, no UI needed |
| 07 | Metrics & Analytics | Mostly | J3, J6 | **Gap**: change impact report, negative impact alert (see C) |
| 08 | Codebase Intelligence | Yes | J5 | Covered |
| 09 | Library Intelligence | Yes | J2, J5 | Covered |
| 10 | Visualization | Yes | J3, J5 | Covered |
| 11 | Session Continuity | — | J4, J9 | **Gap**: recovery prompt, context pack tool (see C) |
| 12 | Multi-Coordinator | Partial | J2, J7 | **Gap**: multi-ACP config in Settings (see C) |
| 13 | Doc Traceability | — | J5 | **Gap**: traceability view (see C). DB ready |
| 14 | Context Delivery | — | J4 | No gap — backend resolution levels (L0-L3) |
| 15 | Pattern Store | Yes | J5 | Covered |
| 16 | Workspace & System | Partial | J2 | **Gap**: architecture conformance (see C) |
| 17 | Pattern Knowledge | — | J5 | **Gap**: industry pattern catalog (see C) |
| 18 | Testability & TDD | — | Sys 1,3 | **Gap**: testability overlay in code graph (see C) |
| 19 | Benchmarking | — | J7 | **Gap**: benchmark runner (see C). DB has tables |
| 20 | Local Inference | Yes | J2, J7 | Covered |
| 21 | Custom Agents | — | J7 | **Gap**: agent editor (see C) |
| 22 | Adapter IR | — | Sys 1 | No gap — backend only |
| 23 | Personas & Mindsets | — | J4, J7 | **Gap**: persona editor (see C) |
| 24 | Desktop Observatory | Yes | J2, J3 | Covered |
| 24a | Data Audit | — | — | No gap — reference analysis doc |
| 24b | Capability Registry | — | J7 | **Gap**: degraded feature indicators in Settings (see C) |
| 25 | Playground & Insights | Yes | J5, J6 | **Gap**: change impact report only |
| 26 | Bootstrap | Yes | J1 | **Gap**: upgrade reconfiguration indicator (see C) |
| 27 | Developer Preferences | Yes | J4, J7, J9 | Covered — Insights/Memories section shows learned preferences with strength and evidence |
| 28 | Inference Gateway | Yes | J2, J7 | Covered for config. MOE panel is backend |
| 29 | Collective Intelligence | — | J9 | **Gap**: sharing settings and history (see C) |
| 30 | Contextual Memory | Mostly | J9 | **Gap**: consolidation review (see C) |
| 31 | Semantic Search | — | — | No gap — transparent MCP tool, no desktop UI |

---

### E. Journey <> DB alignment notes

| Journey reference | DB reality | Status |
|-------------------|-----------|--------|
| J4 mentions `tool_calls` as separate table | `activity.events` with `event_type = 'tool_call'` | No issue — events table stores these |
| J4 mentions `sessions.metadata` JSONB | `sessions.props` JSONB | No issue — naming difference only |
| J5 mentions `symbol_map.l2` for logic flow summaries | Not a dedicated table | Use `nodes.description` or `nodes.props.l2_summary` — no DDL change needed |
| J5 mentions `ftr_correlation` on detected_patterns | Not a column | Compute at query time (join patterns with sessions) — no DDL change needed |
| System 3 mentions `folders.props.doc_health_score` | `folders.props` is JSONB | Convention, not schema gap — document expected JSONB keys |
| J9 mentions memories with `what` + `because` fields | `memories` has `title` + `content` + `impact` | Naming mismatch only: title=what, content=because+how, impact=consequence |
| J7 mentions `coordinator_installs` | Now `sensei.assistants` | Resolved — see section B |
