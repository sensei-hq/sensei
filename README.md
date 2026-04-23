# sensei  先生

A teacher for AI coding agents. Sensei watches your sessions, notices what works and what doesn't, and teaches — surfacing patterns, preventing repeated mistakes, and making every session more effective than the last.

## Core Idea

AI agents make the same mistakes across sessions — missing house rules, reinventing patterns that already exist, ignoring context that was corrected last week. The problem isn't token efficiency. It's **effectiveness**: getting it right the first time, every time.

Sensei solves this by observing, not coding:

- **Watches** — a local daemon monitors AI sessions: prompts, tool calls, corrections, outcomes. Nothing leaves your machine.
- **Notices** — after a few sessions, sensei identifies recurring patterns: which corrections repeat, which rules get missed, which modules cause rework.
- **Teaches** — concrete, evidence-based guidance: "The AI doesn't know your auth — 3 sessions corrected refresh-token handling this week. Here's a persona to fix it."

### What sensei measures

**First-Try Rate (FTR)** — the percentage of sessions that succeed without corrections. This is the primary metric. Everything sensei does aims to increase FTR: better context, better rules, better patterns.

Supporting signals: rework count per file, correction frequency per module, pattern adherence, god-node complexity.

### What sensei provides

- **Recommendations** — learnings derived from observing the dialog between user and assistant across multiple sessions. Ranked by urgency and projected FTR impact. Each recommendation includes evidence (which sessions, which corrections) and a ready-to-send prompt for Claude Code, Cursor, or Codex.
- **Pattern detection** — identifies recurring structural patterns in your code (suggested → gap → rule lifecycle). Cross-links anti-patterns to constructive fixes. The goal: avoid having to repeat the same instructions.
- **Code graph** — call graph with 3 visualization modes (force-layout, matrix, clusters) and 5 overlays (rework heat, duplicate clusters, patterns, god-nodes/hotspots, stale/drift).
- **Library intelligence** — indexes documentation for libraries without their own MCP server. Exposes search, usage analysis, drift detection, and rule suggestion tools.
- **Session continuity** — project memory persists across sessions: decisions, patterns, open items. Context loads in ~300 tokens regardless of project age.

### Benchmarking

Token tracking is available but secondary — useful for benchmarking, not as a goal. Benchmarks compare the effectiveness of instructions or task sets on a corpus: FTR improvement, correction reduction, and token usage as a supporting signal.

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Desktop Observatory (Tauri + Svelte)            │
│  daily view · projects · sessions · coaching     │
└───────────────────┬─────────────────────────────┘
                    │ HTTP / SSE
┌───────────────────▼─────────────────────────────┐
│  Daemon (senseid)                  localhost:9823 │
│  watcher · indexer · task queue · API            │
│  ┌──────────────────────────────────────────┐    │
│  │  PostgreSQL (pgvector + HNSW)            │    │
│  │  folders · symbols · chunks · embeddings │    │
│  └──────────────────────────────────────────┘    │
└───────────────────┬─────────────────────────────┘
                    │ stdio / MCP
┌───────────────────▼─────────────────────────────┐
│  MCP Server (sensei-mcp)                         │
│  search · callers · patterns · sessions · libs   │
└───────────────────┬─────────────────────────────┘
                    │
        ┌───────────┴───────────┐
        ▼                       ▼
  Claude Code              Cursor / Codex / Aider
```

### Stack

- **Daemon + MCP + CLI**: Rust (crates/senseid, crates/sensei-mcp, crates/sensei-cli)
- **Desktop**: TypeScript + Svelte (apps/desktop)
- **Database**: PostgreSQL with pgvector (HNSW indexes for semantic search)
- **Package manager**: Bun

## MCP Tools

| Category | Tools |
|----------|-------|
| Search | `search`, `get_callers`, `get_callees`, `get_duplicates` |
| Patterns | `get_patterns`, `get_pattern_for`, `match_pattern`, `get_project_conventions` |
| Context | `get_session_context`, `get_project_summary`, `get_communities` |
| Sessions | `create_session`, `update_session`, `update_phase`, `log_event`, `get_workflow_state` |
| Libraries | `search_lib_docs`, `get_lib_docs`, `add_library` |

## CLI

```bash
sensei init --scope user|project    # set up sensei for a user or project
sensei serve                        # start the daemon
sensei status                       # index age, symbol count, FTR summary
sensei doctor                       # find and fix config issues
```

## Desktop — The Observatory

The desktop app is a **session observatory**: understand, optimize, and act on your AI-powered workflow.

### Daily view
A single focal insight — the most important thing right now. FTR sparkline, adopted teachings, recent sessions.

### Projects
Per-project dashboard with FTR trends, recommendations, code graph (3 lens modes: force-layout, matrix, clusters), pattern catalog, session history, and project settings.

### Coaching
Evidence-based recommendations ranked by urgency and projected FTR impact. Each recommendation has a "send to ACP" action that launches a prompt directly into Claude Code, Cursor, or Codex.

### Libraries
Browse detected and imported libraries. See usage (top symbols, call sites), attached rules, and try MCP tools in the playground.

## Design Principles

1. **No insight without action** — every metric links to a concrete next step
2. **Effectiveness over efficiency** — FTR matters more than token count
3. **Suggest from data, not templates** — "60% of corrections were about UX" suggests a persona; don't offer a generic catalog
4. **Show impact, not inventory** — rank by FTR delta, not alphabetically
5. **A teacher does not write the code** — sensei observes, notices, teaches; never modifies code
6. **Local-first** — nothing leaves your machine

## Docs

- [Design](docs/design/) — architecture, schemas, algorithms
  - [01-daemon](docs/design/01-daemon/) — core engine, intelligence, analytics
  - [02-desktop](docs/design/02-desktop/) — observatory UX design
  - [03-marketplace](docs/design/03-marketplace/) — skills, commands, plugins
- [Ideas](docs/ideas/) — feature explorations and proposals
- [Decisions](docs/design/decisions/) — ADRs
