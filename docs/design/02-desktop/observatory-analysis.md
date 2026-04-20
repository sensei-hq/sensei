---
name: Observatory System Analysis
status: analysis
origin: docs/ideas/24-desktop-observatory.md
date: 2026-04-20
description: Consolidates all observatory ideas into a coherent system design — scope model, source model, state model, extensibility, and known edge cases
---

# Observatory System Analysis

## 1. Core Insight

The sensei desktop is a **development session observatory** — not a code browser, not an IDE, not a project manager. It answers one question at three levels: **"How am I doing?"**

- Global: across all my work
- Solution: across the repos that form one product
- Project: in this specific repo

Every insight shown must have an **action recipe**. Every metric must connect to **quality (FTR), time (turns), or cost (tokens)**. If it doesn't serve one of these, it doesn't belong.

## 2. Scope Model — Global > Solution > Project

### Three levels, clear boundaries

| Scope | What it represents | Data owned | Example |
|-------|--------------------|-----------|---------|
| **Global** | The developer's entire workspace | Skills, plugins, MCP tools, libraries, quota, ACP config | "I use sensei across 5 projects" |
| **Solution** | A product / system (multi-source) | Aggregated metrics, cross-repo graph, solution profiles, source membership | "Acme Platform = api + frontend + docs" |
| **Project** | A single source (usually a repo) | Sessions, code graph, complexity, profiles, indexing | "acme-api" |

### Active vs Inactive

Most developers work on 2-3 solutions or projects at a time. The rest are background.

| State | Meaning | UI treatment |
|-------|---------|-------------|
| **Active** | Currently being worked on (sessions in last 7 days) | Shown in sidebar, metrics computed, watcher running |
| **Recent** | Worked on recently (sessions in last 30 days) | Shown in sidebar collapsed, watcher running |
| **Inactive** | Not touched recently | Hidden from sidebar, no watcher, data preserved |
| **Archived** | Explicitly shelved by user | Hidden everywhere, no watcher, can be restored |

The sidebar shows active + recent (configurable max). Global overview shows sparklines only for active items.

### Session Identity — CWD is the key

ACPs (Claude Code, Codex, etc.) use **CWD** to identify which project they're working in. A session is always tied to one project — the folder where the ACP was launched.

```
CWD = /Users/dev/acme-api  →  session belongs to project "acme-api"
CWD = /Users/dev/acme-frontend  →  session belongs to project "acme-frontend"
```

**Implication:** Sessions enter the system at the project level, never at the solution level. Solution metrics are always aggregated UP from project sessions. The daemon maps CWD → project → solution.

This means:
- Session list for a solution = union of sessions from all its projects
- FTR for a solution = weighted average of project FTRs
- A session can never belong to two projects (single CWD)
- When the user opens Claude Code in `acme-api/`, the session-start hook registers against project "acme-api", and solution membership is looked up

### Profile Cascade

Profiles (mindsets, personas, rules) cascade downward:

```
Global mindsets (shipped with sensei plugin)
  └─ Solution profiles (shared across all sources in the solution)
       └─ Project profiles (source-specific .sensei/ overrides)
```

A project inherits its solution's profiles, which inherit global mindsets. Overrides add or replace, never remove (to prevent accidentally disabling safety mindsets).

## 3. Source Model — What can be indexed

Today: git repos only. Future: any source that produces nodes and edges for the graph.

### Unified Source Interface

Every source — regardless of type — provides:

```
Source
  ├── metadata: name, path/url, type, status, last_indexed
  ├── nodes: symbols, documents, types, functions
  ├── edges: calls, imports, covers, traces_to
  └── refresh(): re-index this source
```

### Source Types

| Type | Today | What it provides | How discovered |
|------|-------|-----------------|----------------|
| **Git repo** | ✅ | Code symbols, edges, docs, tests | `scan_root` finds `.git/` dirs |
| **Non-git folder** | planned | Same as git repo, minus commit history | `scan_root` with `include_folders` flag |
| **Confluence** | future | Requirement docs, design docs, runbooks | Manual connector config |
| **Jira** | future | Issues, epics, acceptance criteria | Manual connector config |
| **Wiki/Notion** | future | Product specs, architecture decisions | Manual connector config |
| **Figma** | future | Design files, component specs | Manual connector config |

### Design for extensibility

The indexing pipeline already works on folder paths, not git internals:

```
scan_root → discover sources → process_source → process_folder → process_file → resolve_edges
```

To add a new source type:
1. Implement a **source adapter** that produces `nodes` and `edges`
2. Register it in the solution as a source
3. The graph and all downstream queries work unchanged

The graph doesn't care where a node came from — a function from acme-api and a requirement from Confluence are both nodes with edges. This is the key extensibility point.

### Non-git folders specifically

Current `find_git_repos` (handlers.rs:65) skips non-git dirs. The change:

- `scan_root` gains `include_unmanaged: bool` (default false, opt-in)
- Non-git folders with code files registered as `status: "unmanaged"`
- Same indexing pipeline — adapters don't care about `.git/`
- Dedup detector cross-references managed + unmanaged → flags copies
- Desktop shows distinct badge: "unmanaged" vs "repo"
- User can add unmanaged folders to solutions with role `reference` or `archive`

## 4. Watcher Model — Edge Cases

### Problem: Nested root folders

User scans `/Users/dev/projects` which contains `acme/` (a git repo). Later they scan `/Users/dev/projects/acme` directly. Now two watchers fire for the same file changes.

### Rules

1. **Dedup on registration** — when scanning a root, check if any existing scanned root is a parent or child. If so, warn and skip.
2. **Watcher per project, not per root** — watchers are keyed by repo_id, not by scan root path. Two scan roots discovering the same repo create one watcher.
3. **Active projects only** — watchers run only for active/recent projects. Inactive projects have no watcher.
4. **Root hierarchy check** — `scanned_roots` table stores root paths. Before adding a new root, verify no existing root is a prefix or suffix.

```
Attempting to scan: /Users/dev/projects/acme
Existing root:      /Users/dev/projects

→ Warning: /Users/dev/projects/acme is inside already-scanned root /Users/dev/projects
  Action: Skip (already covered) or Replace parent with more specific path
```

### Resource management

With 2-3 active projects and 50+ total, watchers can't run for everything:

| Project state | Watcher | Index on change | Polling |
|--------------|---------|-----------------|---------|
| Active | ✅ running | immediate | none |
| Recent | ✅ running | immediate | none |
| Inactive | ❌ stopped | none | re-index on activate |
| Archived | ❌ stopped | none | manual only |

## 5. Data Capture Model — What we measure

### Events (already captured)

| Event | Source | Scope |
|-------|--------|-------|
| `turn` | user-prompt hook | project (via session) |
| `revision_requested` | user-prompt hook | project |
| `tool_used` | pre-tool hook | project |
| `phase_transition` | command logging | project |

### Events (buildable — add to commands)

| Event | Source | Scope | Blocked by |
|-------|--------|-------|-----------|
| `mindset_applied` | command prompts call log_event() | project | log_event() MCP tool (#80) |
| `persona_applied` | command prompts call log_event() | project | log_event() MCP tool |
| `rule_checked` | command prompts call log_event() | project | log_event() MCP tool |
| `session_closed` | session-start closes previous | project | workaround via SessionStart hook |

### Events (blocked by ACP)

| Event | What's needed | FR |
|-------|--------------|-----|
| Token counts | ACP exposes tokens per session | [#11008](https://github.com/anthropics/claude-code/issues/11008) |
| Cost per session | Derived from token counts | [#50926](https://github.com/anthropics/claude-code/issues/50926) |
| Quota remaining | ACP exposes usage limits | [#50926](https://github.com/anthropics/claude-code/issues/50926) |
| Tool responses | ACP includes response preview in PostToolUse | workaround: MCP cache |
| Task boundaries | ACP fires PreTask/PostTask hooks | [#50931](https://github.com/anthropics/claude-code/issues/50931) |
| Headless execution | ACP supports batch mode | [#50927](https://github.com/anthropics/claude-code/issues/50927) |

### Capability registry

Each metric is backed by a capability that can be:
- **Real** — primary source available
- **Workaround** — temporary alternative, tagged with `discard_when` linking to upstream FR
- **Unavailable** — not possible yet, UI shows disabled state + tracking link

See `docs/ideas/24b-capability-registry.md` for full spec.

## 6. Desktop Page Map

### Navigation

```
Sidebar
├── GLOBAL
│   ├── Overview          (cross-solution sparklines, quota)
│   ├── Libraries         (indexed docs, freshness, usage)
│   ├── Tools             (MCP catalog, try-it, usage stats)
│   ├── Skills & Plugins  (installed, enable/disable)
│   └── Benchmarks        (cross-solution comparison)
│
├── SOLUTIONS (active + recent)
│   ├── Acme Platform  ★     → click opens Solution Dashboard
│   ├── sensei-dev           → click opens Solution Dashboard
│   └── + Add solution
│
└── Settings

Solution pages (when solution selected):
├── Dashboard       (consolidated metrics from all projects)
├── Sessions        (consolidated view — project sessions grouped/filtered)
├── Architecture    (merged graph, cross-source connections)
├── Profiles        (solution-level mindsets, personas, rules)
└── Sources         (repos + folders + future connectors, roles)
    └── click repo → Project pages

Project pages (when project selected within solution):
├── Dashboard       (metrics for THIS project — sessions originate here)
├── Sessions        (sessions for THIS project — the source of truth)
├── Code            (graph, complexity, dead code, duplicates)
├── Profiles        (project-specific overrides)
└── Indexer         (indexing status, errors)

Note: Sessions always belong to a project (identified by CWD).
Solution-level sessions page is a consolidated view, not separate data.
```

### Route structure

```
/overview                        global
/libraries                       global
/tools                           global
/skills                          global
/benchmarks                      global
/settings                        global

/s/[solutionId]                  solution dashboard
/s/[solutionId]/sessions         solution sessions
/s/[solutionId]/arch             solution architecture
/s/[solutionId]/profiles         solution profiles
/s/[solutionId]/sources          solution source management

/s/[solutionId]/p/[projectId]             project dashboard
/s/[solutionId]/p/[projectId]/sessions    project sessions
/s/[solutionId]/p/[projectId]/code        project code
/s/[solutionId]/p/[projectId]/profiles    project profiles
/s/[solutionId]/p/[projectId]/indexer     project indexer
```

## 7. What to Build — Ordered

### Phase 1: Foundation (daemon)
1. `log_event()` MCP tool — unblocks all profile/rule event tracking
2. Capability registry — config + `is_capable()` API
3. ACP profiles — `marketplace/acp-profiles/claude-code.yaml`
4. Session close on SessionStart — workaround for SessionEnd

### Phase 2: Data pipeline (daemon + hooks)
5. Enrich pre-tool hook — capture tool params in events
6. Profile tracking — commands log mindset/persona/rule events
7. MCP response cache — workaround for tool response capture
8. Token estimation fallback — turns × avg tokens

### Phase 3: Desktop restructure (frontend)
9. Route restructure — `/overview`, `/s/[id]`, `/s/[id]/p/[id]`
10. Solution dashboard — aggregate metrics
11. Project dashboard — single-source metrics
12. Adapt existing page shells (Home→Overview, Sessions→Solution Sessions)

### Phase 4: Extensibility (daemon)
13. Non-git folder scanning — `include_unmanaged` flag
14. Watcher dedup — nested root detection
15. Active/inactive project state — watcher lifecycle
16. Source adapter interface — prepare for future connectors

### Phase 5: Future connectors
17. Confluence adapter
18. Jira adapter
19. Connector config in solution management UI

## 8. Agents — Opt-in Autonomous Specialists

### Commands vs Agents

| | Command | Agent |
|---|---------|-------|
| Format | Markdown procedure | Markdown system prompt + frontmatter |
| Execution | Claude follows steps in main context | Isolated context window, autonomous |
| Context cost | Steps consume main conversation tokens | Isolated — results return as summary only |
| Control | Prescriptive — "do step 1, then step 2" | Autonomous — "you are X, figure it out" |
| Best for | Structured workflows with user checkpoints | Deep analysis, review, investigation |

### Why both?

A command like `/sensei:build` walks through a structured workflow with user approval at each step. An agent like `@sensei-reviewer` can autonomously analyze code quality, read profiles, check patterns, and return a structured report — all without consuming main context tokens.

Agents are especially useful for:
- **Review tasks** — deep analysis that reads many files (context-heavy, benefits from isolation)
- **Investigation** — "trace this dead code", "analyze why this session had 5 corrections"
- **Session analysis** — review events, correlate with profiles, suggest improvements
- **Codebase audit** — check all files against patterns, find violations

### Proposed agents (opt-in, in `marketplace/agents/`)

| Agent | Description | Tools | When to use |
|-------|-------------|-------|-------------|
| `session-analyst` | Analyzes session events, identifies patterns in corrections, suggests profile improvements | Read, Grep, Glob | After a session with low FTR or many corrections |
| `code-investigator` | Traces dead code, complexity hotspots, and duplicate clusters with recommendations | Read, Grep, Glob, Bash | When code page shows actionable findings |
| `profile-tuner` | Reviews project profiles against session data, suggests additions/removals | Read, Grep, Glob | Periodically, or when Profiles page shows unused levers |
| `doc-auditor` | Checks docs against code for drift, missing coverage, stale references | Read, Grep, Glob | After code changes to documented areas |

### Integration with desktop

The "Tell Claude" action buttons on observatory pages could offer a choice:
- **Copy prompt** — pastes into main conversation (current behavior)
- **Run agent** — launches the appropriate agent in isolated context, returns summary

This turns the desktop from "observe and copy" to "observe and delegate."

### Mindset-to-Agent Promotion

Mindsets and agents are not separate things — they're layers of the same concept.

| Layer | Contains | Execution | Example |
|-------|----------|-----------|---------|
| **Mindset** (what + why) | Questions to ask, principles to follow | Passive context — loaded at session start | "Walk the user journey. Does it flow?" |
| **Agent** (what + why + how) | Same questions PLUS procedures, tool usage, report format | Active autonomous execution in isolated context | "Read the route files. Trace from trigger to output. Check each step against persona validates. Report findings." |

The agent **includes** the full mindset content — it doesn't replace it. The mindset questions become the agent's checklist. The agent adds:
- Which files to read
- Which tools to use
- What to check specifically
- How to format the report
- What actions to suggest

```markdown
# .sensei/agents/bat.md
---
name: bat
description: BAT review — verify implementation from user perspective
tools: Read, Glob, Grep, Bash
model: sonnet
---

## Mindset (what + why)

[full content from .sensei/mindsets/bat.md — questions preserved exactly]

1. **Walk the user journey** — Start from the trigger...
2. **Test the happy path end-to-end** — Not unit by unit...
3. **Test the first-time experience** — No config, no state...
...

## Procedure (how)

For each question above:

1. Read `.sensei/rules.md` — understand project rules
2. Read `.sensei/personas/*.md` — load all personas
3. Identify the files changed in this session (git diff)
4. For each persona, walk through the changed code from their perspective
5. For each mindset question, check if it was addressed
6. Report:
   - Questions answered ✓ / missed ✗
   - Persona-specific findings
   - Action recipes for anything missed
```

This means:
- A project starts with mindsets only (passive, low overhead)
- When the user wants deeper verification, they promote a mindset to an agent
- The agent contains the full mindset + the procedural how
- `sensei init` can optionally install agents alongside mindsets
- The Profiles page shows both: "Analyst mindset active, BAT agent available"

### How agents are installed

Agents live in `marketplace/agents/` as `.md` files. The `sensei init` command copies them to `.sensei/agents/` (project-level) or `.claude/agents/` (Claude Code discovers them). They appear as opt-in — user enables them per project.

## 9. Open Questions

1. **Solution profiles storage** — where do solution-level profiles live? A `.sensei/` in a shared location? Or in the daemon DB only?
2. **Cross-solution dedup** — should dedup run across solutions? ("This backup in Solution B is a copy of repo in Solution A")
3. **Connector auth** — future connectors need credentials (Confluence API key, Jira token). Where stored? Daemon config? OS keychain?
4. **Offline connectors** — if Confluence is down, the solution's graph has gaps. How to handle stale connector data?
