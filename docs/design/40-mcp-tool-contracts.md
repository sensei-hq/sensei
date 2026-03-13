---
id: mcp-tool-contracts
type: design
supersedes: 03-mcp-server.md
implements:
  - feature: codebase-intelligence
    items: [multi-modal-search, symbol-graph]
  - feature: session-continuity
    items: [session-resume, session-checkpoint]
---

# MCP Tool Contracts

This document defines the contracts for all MCP tools exposed by the sensei server. It supersedes `03-mcp-server.md`.

**Key architectural change**: all persistent state lives in Supabase PostgreSQL. There is no `.index/` directory and no `.llmspec.yaml` queries. Tools read from `sensei.*` tables, except L3 which reads live files from disk.

See `20-pipeline-adapter.md` for pipeline details. See `06-compression.md` for L0–L3 resolution levels.

---

## Server

- **Entry point**: `packages/server/src/mcp-server.ts`
- **Transport**: stdio
- **Server name**: `"sensei"`
- **Schema**: Supabase PostgreSQL — tables `sensei.repos`, `sensei.symbols`, `sensei.call_edges`, `sensei.imports`, `sensei.scan_state`, `sensei.events`

---

## REPO_PATH Resolution

Every tool that accepts an optional `repo_id` resolves the target repo as follows:

1. `repo_id` parameter if provided.
2. `REPO_PATH` environment variable (set at MCP registration time) — looked up in `sensei.repos` by `path`.
3. First row in `sensei.repos` (fallback for single-repo setups).

---

## Phase 1 Tools (Implemented)

### `get_session_context`

Returns a compact project snapshot for starting or resuming a session.

**Input**: `{}` — no parameters.

**Output**: Markdown string (~300 tokens).

```
# {repo_name}

{description}

**Stack**: {stack}
**Indexed files**: {file_count}
**Symbols**: {symbol_count}

## Recent Events
- {event_1}
- {event_2}
...
```

**Reads from**: `sensei.repos`, `sensei.symbols` (counts), `sensei.events` (recent).

**Error**: If no repos are indexed:
```
No repos indexed. Run `sensei init` to get started.
```

---

### `search`

Find symbols by name or content.

**Input**:

| Field | Type | Required | Description |
|---|---|---|---|
| `query` | string | yes | Search term |
| `repo_id` | string | no | Restrict to a specific repo (see REPO_PATH resolution) |
| `kind` | string | no | Filter by symbol kind (`function`, `class`, `interface`, etc.) |
| `limit` | number | no | Max results (default: 20) |

**Output**: JSON array of matching symbols.

```json
[
  {
    "file_path": "packages/collector/src/install.ts",
    "name": "installCollector",
    "kind": "function",
    "signature": "installCollector(opts: InstallOptions): Promise<void>"
  }
]
```

**Phase 1 implementation**: substring text search using `ilike` on `name` and `signature`.

**Phase 2**: BM25 full-text search via `pg_trgm` / `to_tsvector`.

**Error**: If no repos are indexed: descriptive message with fix instruction.

---

### `load_context`

Load a targeted context slice for a given scope.

**Input**:

| Field | Type | Required | Description |
|---|---|---|---|
| `scope` | string | yes | What to load (see scope values below) |
| `repo_id` | string | no | Target repo (see REPO_PATH resolution) |

**Output**: Formatted string:

```
# Context: {scope}
~{N} tokens

{content}
```

**Scope values (Phase 1)**:

| Scope | Returns | Source |
|---|---|---|
| `"orientation"` | Repo name, stack, symbol count, recent events (~200 tokens) | `sensei.repos`, `sensei.symbols`, `sensei.events` |
| `"<file_path>"` | L0 signatures for all symbols in the file | `sensei.symbols` where `file_path = <file_path>` |

**Error**: Unknown scope:
```
Unknown scope. Available: orientation, <file_path>
```

---

## Future Tool Contracts (Phase 2+)

These tools are planned but not yet implemented. Phase 1 uses `sensei init` CLI for indexing.

---

### `reindex_repo`

Trigger the Scan → Parse → Index pipeline for a repo.

**Input**:

| Field | Type | Required | Description |
|---|---|---|---|
| `path` | string | no | Repo path to index (defaults to REPO_PATH resolution) |
| `force` | boolean | no | Force full re-index even if no changes detected |

**Output**: Summary string:
```
Indexed {file_count} files, {symbol_count} symbols in {ms}ms.
```

**Note**: Phase 1 uses `sensei init` CLI instead; this tool is scheduled for Phase 2.

---

### `check_drift`

Detect documentation or context that may have drifted from indexed code.

**Input**:

| Field | Type | Required | Description |
|---|---|---|---|
| `repo_id` | string | no | Target repo (see REPO_PATH resolution) |

**Output**: Drift report listing changed files and affected docs.

```
# Drift Report

## Changed Files
- packages/collector/src/install.ts (modified)

## Affected Docs
- docs/design/30-implementation-phases.md
```

**Reads from**: `sensei.scan_state`, `sensei.symbols`, git diff.

---

### `get_file_context`

Return a file's symbols at a specified resolution level. See `06-compression.md` for level semantics.

**Input**:

| Field | Type | Required | Description |
|---|---|---|---|
| `path` | string | yes | File path relative to repo root |
| `level` | `"L0"` \| `"L1"` \| `"L2"` \| `"L3"` | yes | Resolution level |
| `repo_id` | string | no | Target repo (see REPO_PATH resolution) |

**Output**: File content at the requested level.

| Level | Content | Source |
|---|---|---|
| L0 | Symbol signatures only | `sensei.symbols.signature` |
| L1 | Signatures + I/O patterns | `sensei.symbols.io_pattern` |
| L2 | Signatures + logic flows | `sensei.symbols.logic_flow` |
| L3 | Full file content | Read from disk |

---

### `checkpoint`

Save a session summary and decisions to project memory.

**Input**:

| Field | Type | Required | Description |
|---|---|---|---|
| `summary` | string | yes | Session summary text |
| `decisions` | string[] | no | Architectural decisions made this session |
| `patterns` | string[] | no | Patterns observed or established |

**Output**: Confirmation string with event ID.

**Writes to**: `sensei.events` with `event_type = 'checkpoint'`.

---

### `add_decision`

Record an architectural decision for future sessions.

**Input**:

| Field | Type | Required | Description |
|---|---|---|---|
| `text` | string | yes | Decision text |

**Output**: Confirmation string with event ID.

**Writes to**: `sensei.events` with `event_type = 'decision'`.

---

## Error Handling

All tools follow a consistent error pattern:

| Condition | Behavior |
|---|---|
| No repos indexed | Descriptive message with fix: `"No repos indexed. Run \`sensei init\` to get started."` |
| Invalid input | Clear validation message naming the invalid field |
| Unknown scope / path | Message listing available options |
| No results | Return empty array or empty content — never silently fail due to a missing index |

No tool silently returns empty results when the underlying cause is a missing index. The distinction between "no matches" and "not indexed" must be surfaced explicitly.

---

## Non-Functional Requirements

| NFR | Requirement |
|---|---|
| Performance | Each tool call must return in under 500ms for indexed repos |
| Reliability | MCP server handles malformed input without crashing |
| Token efficiency | Tool responses include only data requested by the scope/level |
