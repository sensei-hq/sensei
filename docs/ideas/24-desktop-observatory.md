---
name: Desktop Observatory
status: idea
origin: conversation
date: 2026-04-19
description: Redesign the sensei desktop app as a session observatory — analytics, tools, profiles, code intelligence, and community
---

# Desktop Observatory

## Problem

The current desktop app is a code intelligence dashboard — graphs, communities, indexing controls. It answers "what does my code look like?" but not the questions the AI Driven Developer actually asks:

- How efficient are my sessions?
- What is Claude doing and why?
- How fast am I burning through my quota?
- Is this tool/library actually helping?
- Something went wrong — how do I investigate?
- I see problems — how do I act on them?

## Vision

The desktop is a **session observatory** — the place where AI-assisted developers understand, optimize, and debug their AI-powered workflow. Every screen answers a question. Every insight links to an action.

## Scope Model

Every piece of data in the observatory has a scope:

**Project-scoped** — belongs to a specific repo/solution:
- Sessions and session metrics (FTR, turns, rework, token usage)
- Profiles: mindsets, personas, rules (each project has its own `.sensei/`)
- Code intelligence: graph, complexity hotspots, duplicates, dead code, doc drift
- Project-specific skills (opt-in per project)
- Indexing status

**Global** — shared across all projects:
- Skills catalog (installed once, available everywhere)
- Plugins and MCP configuration
- Library docs (indexed once, queried from any project)
- Tool catalog (MCP tools are daemon-wide)
- Benchmarks (cross-project comparison)
- ACP configuration
- Quota and global cost tracking

A user with 5 projects needs "how is project X doing?" — not "how is everything averaged together?"

## Information Architecture

### Navigation

```
Sidebar:
  ┌─ GLOBAL ──────────────┐
  │ Overview (cross-project│
  │   dashboard)           │
  │ Libraries              │
  │ Tools                  │
  │ Skills & Plugins       │
  │ Benchmarks             │
  └────────────────────────┘
  ┌─ PROJECTS ────────────┐
  │ sensei-dev        ★   │  ← active project
  │ acme-api              │
  │ acme-frontend         │
  └────────────────────────┘
  Settings

Clicking a project opens project-scoped pages:
  Project / Dashboard  (FTR, sessions, rework for THIS project)
  Project / Sessions   (sessions for THIS project)
  Project / Profiles   (mindsets, personas, rules for THIS project)
  Project / Code       (graph, complexity, duplicates, doc drift)
  Project / Indexer    (indexing status for THIS project)

Header:
  [active project selector]  daemon status  token/quota burn rate
```

### Key UX decisions

1. **Project selector in header** — always visible, switches all project-scoped pages
2. **Global pages have no project context** — Libraries, Tools, Skills are daemon-wide
3. **Overview page** is global — shows cross-project summary (total sessions, aggregate FTR, per-project sparklines)
4. **Project dashboard** is the landing page when a project is selected — answers "how is THIS project doing?"
5. **Profiles page is project-scoped** — each project has different mindsets/personas/rules. The lever impact table shows impact for THIS project's sessions.

## Pages

### GLOBAL PAGES

### 1. Overview — "How am I doing across all projects?"

The global landing page. Cross-project summary.

**Per-project sparklines:**
- Row per project: name, FTR trend, session count this week, rework rate
- Click a project → opens project dashboard

**Aggregate metrics:**
- Total sessions across all projects
- Global FTR (weighted by sessions)
- Quota gauge with burn rate and projection

**Quick actions:**
- "Start session" (picks active project)
- "View backlog"

### 2. Libraries — "What docs does Claude have?" (GLOBAL)

See section 4 below — unchanged, libraries are global.

### 3. Tools — "What can sensei do?" (GLOBAL)

See section 5 below — unchanged, MCP tools are daemon-wide.

### 4. Skills & Plugins — "What's installed?" (GLOBAL)

- Installed skills with enable/disable toggles
- Installed plugins (MCP servers)
- ACP configuration

### 5. Benchmarks — "Is sensei helping?" (GLOBAL)

Cross-project benchmark comparison. See section 7 below.

---

### PROJECT-SCOPED PAGES

These pages show data for the **active project** (selected in header or sidebar).

### 6. Project Dashboard — "How is THIS project doing?"

The landing page when a project is selected. Answers at a glance for this project only.

**Metrics cards:**
- FTR for THIS project
- Session count for THIS project
- Rework rate for THIS project
- Token usage + cost for THIS project

**Tool adherence bar:**
- % MCP tools vs fallback in this project's sessions
- Goal: 90%+ MCP usage

**Recent sessions:**
- Last 5-10 sessions for THIS project
- Click to drill into any session

**Active context:**
- Current task/issue from workflow state
- Quick actions

### 7. Project Sessions — "What happened in THIS project?"

**List view:**
Sortable table: date, task/issue, project, outcome, FTR, turns, corrections, tokens, cost, duration

Filterable by: project, solution, outcome, date range

**Session detail (drill-in):**

Event timeline — every turn, tool call, phase transition, correction:
```
Turn 1: new_request
Turn 2: tool_used → search("compute_metrics") → 3 results
Turn 3: tool_used → get_callers("insert_event") → 5 callers
Turn 4: continuation
Turn 5: tool_used → grep (fallback!) ⚠️
Turn 6: revision_requested — "that's wrong, use MCP"
Turn 7: tool_used → search("insert_event") → corrected ✓
```

Profiles applied — which mindsets/personas fired, which didn't, adherence score

Rules checked — which rules were followed/violated

Clicking on:
- A tool call → shows what it returned (input params + response)
- A profile → shows its questions and whether they were answered
- A warning → explains the issue and suggests a fix

### 8. Project Code — "What does THIS project's code look like?"

**Overview:**
- Symbol count (functions, types), edge count
- Stack and indexed status

**Actionable insights (each with "Tell Claude" button):**
- Complexity hotspots — functions with high cyclomatic complexity → "Investigate and decompose"
- Dead code candidates — exported but never called → "Check if dead or used via HTTP"
- Duplicates — identical functions across files → "Extract to shared util"
- Doc drift — docs referencing changed code → "Review and update doc"

**Graph visualization:**
- Interactive code structure graph (existing GraphCanvas)
- Communities and architecture clusters

### 9. Project Profiles — "What's helping THIS project?"

Lever impact table ranked by FTR/token impact — but scoped to THIS project's sessions only.

Each project can have:
- Custom mindsets (beyond the global defaults)
- Custom personas specific to this project's users
- Custom rules in this project's `.sensei/rules.md`
- Project-specific skills (opted in per project)

Suggestions are derived from THIS project's correction patterns.

### 10. Project Indexer — "Is THIS project indexed?"

Indexing status, progress, errors, re-index controls for this project only.

---

### Scope Reference

| Page | Scope | What it shows |
|------|-------|---------------|
| Overview | global | Cross-project sparklines, aggregate FTR, quota |
| Libraries | global | Indexed docs shared across all projects |
| Tools | global | MCP tool catalog, usage stats, "Try it" |
| Skills & Plugins | global | Installed skills, plugins, ACP config |
| Benchmarks | global | Cross-project comparison runs |
| Settings | global | Daemon, display, workspace |
| Project Dashboard | project | FTR, sessions, rework for THIS project |
| Project Sessions | project | Session list + detail for THIS project |
| Project Code | project | Graph, complexity, duplicates, doc drift |
| Project Profiles | project | Mindsets, personas, rules + impact for THIS project |
| Project Indexer | project | Indexing status for THIS project |

## Design Principles

### 1. No insight without action

Every piece of information shown must have a clear "so what?" and a clear "do this next." If the user can't act on it, don't show it. A number on screen without a recipe to improve it is decoration, not intelligence.

**Test:** For every element on every page, ask: "What does the user DO with this?" If the answer is "look at it," remove it or attach an action.

Examples:
- Bad: "FTR: 83%" — a number
- Good: "FTR: 83% ↓ — 3 corrections this week were about missing user perspective → [Create end-user persona] [View corrections]"
- Bad: "Security Reviewer mindset: applied 0 times" — a stat
- Good: "Security Reviewer: never triggered. [Remove] or [Lower trigger threshold]"

### 2. Progressive disclosure, not information dump

Show the minimum needed to decide. Details on demand. The home page has 4 numbers and a list. Clicking a number reveals the story. Clicking the story reveals the events. Never show all three layers at once.

**Hierarchy:** Metric → Insight → Detail → Action
- Metric: "Rework rate: 18%"
- Insight: "3 of 5 corrections were about missed rules"
- Detail: "Session #12, turn 5: Claude used grep instead of MCP search"
- Action: "[Add rule: prefer MCP tools] [View session]"

A cluttered UI means the hierarchy collapsed — detail is showing where metric should be.

### 3. Three outcomes drive everything

Quality (FTR), Time (turns), Cost (tokens). Every page connects back to at least one. If a feature doesn't move these numbers, question whether it belongs.

### 4. Show impact, not inventory

Don't list mindsets. Show which ones improved FTR and which cost tokens without helping. The user tunes levers by impact, not by reading descriptions.

### 5. Suggest from data, not templates

"60% of corrections were about UX" → suggests a persona. Don't offer a generic catalog of personas to browse. Surface what THIS user's sessions need.

### 6. Token/cost awareness everywhere

Header shows burn rate. Sessions show cost. Dashboard shows quota remaining. The user should never be surprised by their usage.

### 7. Simulate before committing

Test a tool call. Preview a library index. Run a benchmark on a subset. Reduce the cost of trying something new.

## Personas served

| Persona | Primary pages | Key question |
|---------|--------------|--------------|
| AI Driven Developer | Overview, Project Dashboard, Project Sessions, Project Profiles | "How efficient am I on THIS project and how do I improve?" |
| Plugin Developer | Tools, Skills & Plugins, Libraries | "Is my plugin/skill working and helping?" |
| API Consumer | Tools | "What tools exist and how do I use them?" |
| Evaluator (new) | Benchmarks, Overview | "Should I adopt sensei for my team?" |

## What gets removed from current app

| Current page | Disposition |
|-------------|-------------|
| `/s/[id]/arch` | Merged into Project detail |
| `/s/[id]/trace` | Removed — placeholder, no real data |
| `/s/[id]/skills` | Merged into Profiles |
| `/acp` | Merged into Settings |
| `/catalog` | Replaced by Tools page |
| `/graph` redirect | Removed — graph lives inside Project detail |
| Setup wizard | Keep but simplify |

## Implementation approach

1. Extract shared utilities first (ftrClass, getStatus, STATUS_CLS, catalogs)
2. Build new page shells with the revised nav
3. Migrate data from existing pages into new structure
4. Add new functionality incrementally (tool simulation, benchmark runner, community features)

## Open questions

- Should "Tell Claude" generate a clipboard prompt or open Claude Code directly?
- How do we track mindset/persona application? (Needs daemon-side event logging)
- Benchmark comparison requires running sessions — is this automated or manual?
- Community features need a backend — hosted service or GitHub-based?
