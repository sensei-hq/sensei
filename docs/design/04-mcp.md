# MCP -- Model Context Protocol Server

## Overview

The MCP server is a Rust binary (`crates/mcp/`) that exposes daemon capabilities as MCP tools over stdio transport. The AI coordinator (Claude Code, Cursor, etc.) never calls the daemon directly -- it only sees MCP tools. See [ideas/04-project](../ideas/04-project.md) for the project intelligence vision this server enables.

The binary speaks the standard MCP protocol. Tool calls arrive as JSON-RPC over stdin; responses go to stdout. Internally, each tool translates to an HTTP request to the running daemon (`senseid`).

## Architecture

```
AI Coordinator (Claude Code / Cursor / ...)
    |
    | stdio (JSON-RPC)
    v
sensei-mcp binary
    |
    | HTTP (localhost)
    v
senseid daemon
    |
    | PostgreSQL / gateway / indexing pipeline
    v
Data layer
```

Three layers inside the binary:

1. **MCP protocol handler** -- parses JSON-RPC, dispatches to tool functions, serializes responses.
2. **Tool registry** -- maps tool names to handler functions with parameter schemas.
3. **HTTP client** -- sends requests to the daemon API and translates responses back to MCP format.

## Tool contracts

### Phase 1 -- Implemented

Code intelligence tools that provide the AI with project understanding.

| Tool | Parameters | Returns |
|------|-----------|---------|
| `search` | `query: string`, `kind?: string`, `limit?: number` | Array of matching symbols with file path, name, kind, and signature |
| `get_session_context` | (none) | Markdown snapshot: repo name, stack, file/symbol counts, recent events (~300 tokens) |
| `get_callers` | `symbol: string` | List of symbols that call the target, with file paths |
| `get_callees` | `symbol: string` | List of symbols called by the target, with file paths |
| `get_duplicates` | `file?: string` | Pairs of similar code with file paths, similarity score, and line counts |
| `get_communities` | (none) | Detected module clusters with member symbols and cohesion scores |
| `get_project_summary` | (none) | High-level project overview: language breakdown, top-level modules, dependency graph |
| `get_project_conventions` | (none) | Detected conventions with evidence count and example files |
| `embed` | `text: string` | Vector embedding via the gateway embedding chain |
| `consensus` | `prompt: string`, `config?: object` | MOE consensus result via the gateway consensus engine |
| `infer` | `prompt: string`, `capability?: string`, `model?: string` | Inference result via the gateway |

### Phase 2 -- Workflow

Workflow intelligence tools for phase management, event logging, and pattern queries.

| Tool | Parameters | Returns |
|------|-----------|---------|
| `update_phase` | `phase: string`, `task?: string`, `issue?: number` | `{ ok, state: WorkflowState }` |
| `get_workflow_state` | (none) | `{ phase, task, issue, last_checkpoint, rules_hash }` |
| `log_event` | `type: string`, `data: object` | `{ ok, id }` -- records a typed event (one of 16 event types) |
| `get_patterns` | `type?: string` | Array of detected patterns with name, type, instance count, files, interface name |
| `match_pattern` | `description: string` | Array of matching patterns with confidence, reference file, invariants |
| `get_pattern_for` | `symbol: string` | Pattern name, type, and role of the symbol in the pattern -- or null |

### Error handling

All tools follow a consistent pattern:

| Condition | Behavior |
|-----------|--------|
| No repos indexed | Descriptive message: "No repos indexed. Run `sensei init` to get started." |
| Invalid input | Validation message naming the invalid field |
| No results | Empty array or empty content -- never silent failure |
| Daemon unreachable | Error message with connection details and suggested fix |

The distinction between "no matches" and "not indexed" is always surfaced explicitly.

## REPO_PATH resolution

Every tool that operates on a project resolves the target via a 3-step fallback:

1. **Environment variable** -- `REPO_PATH` set at MCP registration time (most common). Looked up in the daemon's project registry by path.
2. **Working directory** -- the current working directory of the process that launched the MCP server.
3. **Known projects** -- first project in the daemon's registry (fallback for single-project setups).

This resolution is transparent to the AI coordinator. It does not need to pass a project identifier on every call.

## Multi-coordinator

The same `sensei-mcp` binary serves any AI coding assistant that supports MCP. An adapter pattern handles the differences between coordinators.

### Adapter pattern

Each coordinator adapter knows:
- **Where to register** -- the MCP config file location.
- **How to capture events** -- hooks, OTLP, polling, or extension API.
- **Where to install skills** -- coordinator-specific skill file locations.
- **How to bootstrap sessions** -- session start mechanism.

### Claude Code adapter

- MCP registration: `~/.claude/mcp.json` or `<repo>/.mcp.json`.
- Event capture: hooks (`PreToolUse`, `PostToolUse`) + OTLP.
- Skills: `~/.claude/skills/<repo>-*.md` (Markdown with YAML frontmatter).
- Session start: `SessionStart` hook injects `get_session_context` instruction.

### Cursor adapter

- MCP registration: Cursor-specific config location.
- Event capture: format TBD.
- Skills: `.cursorrules` equivalent.
- Session start: file-based instruction injection.

### Generic adapter

- MCP registration: varies.
- Event capture: git diff polling fallback (less granular, but universal).
- Skills: `<repo>/.sensei/context.md` (plain markdown summary).
- Session start: file-based instruction.

### Design principle

The MCP server itself is coordinator-agnostic -- it speaks standard MCP protocol. What changes per coordinator is **where it is registered** and **how the coordinator discovers it**. Adding a new coordinator requires implementing a new adapter (registration + event capture + skill delivery) with no changes to the MCP server or its tools.
