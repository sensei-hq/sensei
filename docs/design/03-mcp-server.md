# MCP Server

## Overview

`repo-index-server` is a local MCP server that reads from a repo's `.index/` directory and `.llmspec.yaml` and serves targeted slices on demand. It also performs compute offload — generating docs, checking drift, running benchmarks — so LLMs don't spend context tokens on deterministic work.

---

## Tool Categories

### Query Tools (read from index)

| Tool | Inputs | Output | Token cost |
|---|---|---|---|
| `get_llmspec` | `section?: string` | Full spec or named section | ~500 (full) / ~50 (section) |
| `get_file_context` | `path: string, level: L0\|L1\|L2\|L3` | File at resolution level | Varies by level |
| `list_exports` | `module?: string` | L0 signatures, all or scoped | ~50–200 |
| `find_pattern` | `name?: string` | Named pattern or all patterns | ~50–300 |
| `get_shortcuts` | — | Dev commands | ~50 |

### Reindex Tool (write to index)

| Tool | Inputs | Output |
|---|---|---|
| `reindex_repo` | `path?: string, force?: boolean` | Confirmation + artifact list |

### Context Tools (in-session)

| Tool | Inputs | Output |
|---|---|---|
| `load_context` | `scope: string` | Content slice + token estimate |
| `get_context_summary` | — | Available scopes with token estimates |
| `checkpoint` | `summary: string, decisions?: string[], patterns?: string[]` | Archive session, return resume instruction |
| `recommend_next` | `task: string` | Context prescription for the task |

### Project Memory Tools (cross-session)

| Tool | Inputs | Output |
|---|---|---|
| `get_session_context` | — | Compressed project snapshot ~300 tokens |
| `add_decision` | `text: string` | Appends to memory.yaml, deduped |
| `add_pattern` | `name: string, convention: string` | Appends to patterns.yaml |
| `ask_question` | `question: string` | Adds to open-items.yaml, returns id |
| `get_open_items` | — | All open questions and next steps |
| `close_item` | `id: string, resolution?: string` | Marks open item resolved |

### Drift Tools

| Tool | Inputs | Output |
|---|---|---|
| `check_drift` | — | Drift report: drifted files + summary |

### Generation Tools

| Tool | Inputs | Output |
|---|---|---|
| `generate_llms_txt` | — | Writes llms.txt, returns content |
| `generate_changelog` | `since?: string` | Changelog from git log since ref |

### Benchmark Tools

| Tool | Inputs | Output |
|---|---|---|
| `run_benchmark` | `corpus: string, config: string` | Benchmark result JSON |
| `compare_results` | `fileA: string, fileB: string` | Comparison report |
| `get_metrics_summary` | — | Trend summary across all runs |

---

## Tool Contracts

### `get_llmspec`

```typescript
input: { section?: string }
output: string  // YAML-formatted spec or section

// section values: "entry_points", "concepts", "patterns", "api_surface",
//                 "doc_layers", "shortcuts", "stack"
// if section not found: throws "Section 'x' not found in llmspec"
// if no .llmspec.yaml: throws "No .llmspec.yaml found. Run codebase-indexer first."
```

### `get_file_context`

```typescript
input: { path: string, level: "L0" | "L1" | "L2" | "L3" }
output: string  // file content at requested level

// L0: signatures only (from symbol-map.json)
// L1: IO patterns (from symbol-map.json)
// L2: logic flow bullets (from symbol-map.json)
// L3: full source (reads actual file from disk)
// if file not in symbol map and level < L3: throws "File not in symbol map. Run reindex_repo first."
```

### `load_context`

```typescript
input: { scope: string }
output: string  // formatted: "# Context: {scope}\n~{N} tokens\n\n{content}"

// scope values:
//   "orientation" → project name, description, stack, entry points (~200 tokens)
//   "patterns"    → .index/patterns.md content
//   "stack"       → .index/stack.md content
//   "<path>"      → L0 exports from files matching the path prefix
```

### `check_drift`

```typescript
input: {}
output: string  // human-readable drift report

// report format:
//   "No drift detected. All indexed docs match current state."
//   OR
//   "3 file(s) drifted since last index:\n
//    README.md: modified since last index\n
//    docs/old.md: deleted (was in index)"
```

### `recommend_next`

```typescript
input: { task: string }
output: string  // context prescription as human-readable guidance

// prescription logic (keyword-based, V1):
//   "list" | "find" | "search"     → L0 via list_exports
//   "explain" | "what" | "how"     → L1/L2 via get_file_context
//   "trace" | "flow" | "walk"      → L2 via get_file_context
//   "fix" | "bug" | "error"        → L2 then L3
//   "add" | "implement" | "create" → patterns first, then L0 for context, L3 for target
//   default                        → get_llmspec for orientation
```

---

## Error Handling

All tools follow a consistent error pattern:

```typescript
// Missing index: descriptive message with fix instruction
"No .llmspec.yaml found at /path. Run codebase-indexer first."
"File 'src/auth.ts' not in symbol map. Run reindex_repo first."

// Missing section: clear identification
"Section 'nonexistent' not found in llmspec."

// Missing file (L3): propagates fs error with context
"File not found: src/nonexistent.ts"
```

No tool silently returns empty results when the underlying cause is a missing index.

---

## REPO_PATH Resolution

The server resolves the target repo path in this order:
1. `path` parameter on tools that accept it (`reindex_repo`, `load_context`)
2. `REPO_PATH` environment variable (set at MCP registration time)
3. `process.cwd()` (fallback)

---

## Server Entry Point

`src/index.ts` registers all tools and connects via stdio transport:

```typescript
const server = new McpServer({ name: "repo-index-server", version: "0.1.0" });
const REPO = process.env.REPO_PATH ?? process.cwd();

// Register tools (see tools/ directory)
// Each tool file exports pure functions; index.ts wires them to MCP SDK

const transport = new StdioServerTransport();
await server.connect(transport);
```

---

## Testing Strategy

Each tool module (`tools/query.ts`, `tools/reindex.ts`, etc.) has a co-located test file. Tests use real filesystem operations on temp directories — no mocking of fs, since the real value of these tests is verifying file system behaviour.

```
src/tools/query.ts        ← implementation
src/tools/query.test.ts   ← tests using /tmp/test-* directories
```

Test setup: `beforeEach` creates a temp dir with realistic fixture files. `afterEach` removes it. No shared state between tests.

Run all tests: `cd mcp/repo-index-server && npx vitest run`
