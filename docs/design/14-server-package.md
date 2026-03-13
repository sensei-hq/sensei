---
id: server-package
type: design
implements: []
---

> **SUPERSEDED** by `01-architecture.md`. The package split described here (`@sensei/server`, `@sensei/mcp`, `@sensei/cli`, `@sensei/shared`) maps to the current `packages/` structure, but the deployment model has changed: inference is no longer a local HTTP server — all persistent state lives in Supabase. The deployment configuration sections and model recommendations remain useful context.

---

# Server Package

## What Changed and Why

The v1 architecture (`01-architecture.md`) treated sensei as a single package with CLI + MCP server co-located. This worked for a solo developer but has a fundamental limitation: inference (the work of analyzing code, generating summaries, extracting symbols) is tightly coupled to the process that runs the CLI command.

Three forces push toward a server-mediated architecture:

1. **Local model management is heavyweight** — Ollama, ONNX models, disk/RAM checks, model downloads. This belongs in a persistent server process, not re-initialized on every CLI call.

2. **Shared org infrastructure** — teams want to run one inference server on shared hardware instead of every developer downloading 2 GB of models. A server-based design supports local, on-premise org, and cloud endpoints with the same client interface.

3. **Separation of concerns** — CLI (developer UX), MCP server (LLM tool interface), and inference server (AI tasks + telemetry) have different deployment models, scaling characteristics, and update cycles. They should be separate packages.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| scalability | Server package must support concurrent requests from multiple agents |
| reliability | Server must restart cleanly after crash without index corruption |

---

## Package Structure

```
packages/
  cli/              @sensei/cli       — thin CLI, talks to server
  server/           @sensei/server    — inference engine + telemetry receiver
  mcp/              @sensei/mcp       — MCP tool server (served to Claude)
  shared/           @sensei/shared    — types, constants, API contracts
```

Each package is independently installable. In the common case a developer installs all three. In an org context a team might run one shared `@sensei/server` and every developer only installs `@sensei/cli` + `@sensei/mcp`.

---

## Component Roles

### `@sensei/server` — The Brain

Runs as a persistent local HTTP server (default: `localhost:7744`).

**Responsibilities:**
- **Model management** — detect Ollama, pull required models, download ONNX models, check disk/RAM, manage the model lifecycle
- **Inference** — analyze files via local LLM (Ollama), embed text via ONNX, extract symbols/summaries/flows/relations per file
- **Index management** — own the write side of `.sensei/` artifacts (symbol-map, folder-map, traceability, analysis-cache)
- **Telemetry** — collect benchmark reports (existing SQLite receiver, extended for inference telemetry)
- **Backend resolution** — transparently route inference to local model, org endpoint, or cloud fallback based on availability and config

The server holds warm models across requests. A CLI call that triggers indexing does not pay model cold-start cost because the server is already running.

**Inference preference order:**
```
1. local Ollama  (localhost:11434 — default, always tried first)
2. local ONNX    (Transformers.js, embedded in server process)
3. org server    (SENSEI_SERVER_URL env or ~/.config/sensei/config.yaml)
4. cloud         (future — requires explicit opt-in)
```

### `@sensei/cli` — The Developer Interface

Thin binary. No model management, no inference, no file analysis.

**Responsibilities:**
- Parse commands and flags
- Find repo root (`findRepoRoot`)
- Send requests to `@sensei/server` HTTP API
- Display results via `@clack/prompts`
- Manage local config (`sensei setup --mcp`, `sensei setup --server`)

The CLI knows the server URL from (in order): `--server` flag → `SENSEI_SERVER_URL` env → `~/.config/sensei/config.yaml` → `localhost:7744`.

If the server is not running, `sensei index` offers to start it:
```
sensei: server not running at localhost:7744
  Start it?  [sensei server start]
```

### `@sensei/mcp` — The LLM Tool Interface

MCP server exposing tools to Claude (or any MCP-compatible LLM agent).

**Responsibilities:**
- Expose query tools: `get_llmspec`, `get_file_context`, `list_exports`, `find_pattern`, `get_shortcuts`
- Expose context tools: `load_context`, `recommend_next`
- Expose drift tool: `check_drift`
- Expose project memory tools: `get_session_context`, `checkpoint`, `add_decision`, ...
- Proxy inference-heavy requests to `@sensei/server` (e.g., `reindex_repo` triggers a server-side re-index)

The MCP server is read-heavy — most tools just read `.sensei/` artifacts that the server already wrote. Write operations (reindex, checkpoint) go through the server API to ensure consistent state.

### `@sensei/shared` — Contracts

Shared TypeScript types, API request/response schemas, constants.

**Contains:**
- `FileAnalysis`, `AnalyzedSymbol`, `Flow`, `Relation` (from `16-local-model-indexer.md`)
- `PackageInfo`, `PackageAdapter`, `DocLink` (from `15-package-adapters.md`)
- `ServerAPI` — typed request/response shapes for all server endpoints
- `SENSEI_DIR`, `senseiPath` (currently in `constants.ts`, promoted to shared)
- `IndexConfig`, `ModelBackendId` types

---

## Server API

The server exposes a REST/JSON API. The CLI and MCP server are both clients.

```
POST /analyze          Analyze one or more files → FileAnalysis[]
POST /index            Full or incremental index run → IndexSummary
POST /reports          Submit telemetry report (existing endpoint)

GET  /health           { ok, backend, models }
GET  /status           { lastIndexed, fileCount, backend, ollamaModel }
GET  /artifacts/:name  Serve a .sensei/ artifact by name (symbol-map.json, etc.)

POST /setup/ollama     Check + install Ollama + pull required model (interactive via SSE)
POST /setup/onnx       Check + download ONNX embedding model
GET  /setup/status     { ollama: bool, ollamaModel: bool, onnx: bool, diskGB: number, ramGB: number }
```

The `/analyze` endpoint is the core of the 16-local-model-indexer.md design. The CLI sends file content; the server returns `FileAnalysis`. The server manages the model backend selection transparently.

The `/setup/*` endpoints enable the `sensei init` prerequisite check flow designed in the previous design doc, but now the checks and downloads happen server-side, not inside the CLI process.

---

## Deployment Configurations

### Solo Developer (default)

```
Developer machine
  ├── sensei server      running on localhost:7744
  │     └── Ollama       running on localhost:11434
  ├── sensei cli         installed globally (bun i -g)
  └── sensei mcp         registered in ~/.claude/mcp.json
```

`sensei setup` (or `sensei init` on first use) starts the server, checks Ollama, downloads models. Everything local.

### Org / Team (shared inference)

```
Shared inference server  (on-premise, e.g. GPU machine)
  └── sensei server      running on inference.myorg.internal:7744
        └── Ollama        GPU-accelerated, larger model (e.g. llama3.1:70b)

Developer machines (each)
  ├── sensei cli         installed globally
  │     └── SENSEI_SERVER_URL=http://inference.myorg.internal:7744
  └── sensei mcp         registered in ~/.claude/mcp.json
        └── SENSEI_SERVER_URL=...
```

No model downloads per developer. All inference is fast (GPU-backed). The server handles auth (future: `X-Sensei-Token` header).

### CI / Headless

```
CI runner
  └── sensei cli    ← runs `sensei index --ci` → calls server API
        └── SENSEI_SERVER_URL=http://sensei-server:7744  (Docker service)
```

---

## `sensei init` Flow with Server

The prerequisites check (designed in `16-local-model-indexer.md`) moves to the server:

```
sensei init (CLI)
  │
  ├── is server running? (GET /health)
  │     no  → "Start sensei server first: sensei server start"
  │            or: auto-start if --auto flag
  │
  ├── GET /setup/status
  │     → { ollama, ollamaModel, onnx, diskGB, ramGB }
  │
  ├── if !ollama or !ollamaModel or !onnx:
  │     present setup report via @clack/prompts
  │     "The following are needed:"
  │       [ ] Ollama installed      (2.0 GB disk)
  │       [ ] llama3.2:3b pulled    (2.0 GB disk, 4 GB RAM)
  │       [ ] Embedding model       (22 MB disk)
  │     "Available: X GB disk, Y GB RAM"
  │     confirm → POST /setup/ollama (SSE stream → spinner updates)
  │             → POST /setup/onnx
  │
  └── POST /index { repoPath, force: false }
        → streams progress (SSE or polling)
        → on complete: display IndexSummary
```

This keeps the CLI dumb — it never shells out to `ollama pull` itself. The server owns all model lifecycle.

---

## Index Format (Output of Inference)

Artifact format decisions informed by LLM navigation patterns (progressive zoom from orientation to detail):

| Artifact | Format | Why |
|---|---|---|
| `symbol-map.json` | JSON | Queried by key (file path), machine-generated, schema-validated |
| `folder-map.json` | JSON | Package tree, traversed by code, not read narratively |
| `traceability.json` | JSON | Graph structure, traversed by code |
| `doc-index.json` | JSON | Fingerprints, queried by path |
| `index-config.json` | JSON | Machine-written config, rarely read by humans |
| `stack.md` | Markdown | Read top-to-bottom by LLM in one pass, human-editable |
| `shortcuts.md` | Markdown | Cheat-sheet, scanned visually, human-editable |
| `patterns.md` | Markdown | Narrative conventions, human-edited and LLM-read |
| `llmspec.yaml` | YAML | Mixed narrative + structure, human-edited, comment-friendly |
| `llms.txt` | Text | llmstxt.org standard, LLM reads whole file |
| `analysis-cache/<sha>.json` | JSON | Per-file model output, large, gitignored |

**Symbol-map schema extension** (additions from local model inference):

```typescript
// per file entry in symbol-map.json
{
  L0: string[];           // export signatures (existing)
  L1: string[];           // description + signature (existing)
  L2: string[];           // logic flows (now filled by local model)
  summary?: string;       // NEW: 1-2 sentence file purpose
  role?: string;          // NEW: "component" | "service" | "util" | "config" | "test"
  contentHash?: string;   // NEW: sha256 — incremental cache key
}
```

**Orientation blob** (~500 tokens) served by `load_context("orientation")`:
- Project name + description (from llmspec)
- Tech stack (flat list)
- Package list with role + one-sentence description each (from folder-map)
- Top 5 shortcuts (from shortcuts.md)
- Pattern headings only — not full content (H2 from patterns.md)

This is what the LLM loads first. It answers "what is this and how do I navigate it?" without requiring further tool calls in most cases.

---

## Recommended Models

| Purpose | Model | Pull command | Disk | Min RAM |
|---|---|---|---|---|
| Code analysis + extraction | `llama3.2:3b` | `ollama pull llama3.2:3b` | 2.0 GB | 4 GB |
| Fallback / stronger reasoning | `phi3.5:mini` | `ollama pull phi3.5:mini` | 2.2 GB | 4 GB |
| Semantic embeddings | `Xenova/all-MiniLM-L6-v2` | auto-downloaded by Transformers.js | 22 MB | 50 MB |

Total first-install footprint: ~2.3 GB (Ollama binary 200 MB + model 2.0 GB + ONNX 22 MB + npm 80 MB).

The server records the active model in `index-config.json`. Upgrading the model just changes config and triggers a `sensei index --force` to re-analyze with the better model.

---

## Implementation Plan

### Phase 0 — Shared Types Package

1. Create `packages/shared/` — `@sensei/shared`
   - `src/types.ts` — `FileAnalysis`, `AnalyzedSymbol`, `Flow`, `Relation`, `PackageInfo`, `DocLink`, `IndexConfig`, `ServerAPI` request/response shapes
   - `src/constants.ts` — `SENSEI_DIR`, `senseiPath` (moved from `packages/sensei/src/constants.ts`)

### Phase 1 — Server Package (new package or extend existing `serve.ts`)

2. Extract `packages/server/` from current `packages/sensei/src/commands/serve.ts`
   - Add `/health`, `/status`, `/setup/status` endpoints to existing Bun server
   - Add `/analyze` endpoint — receives file content + path, returns `FileAnalysis`
   - Add `/index` endpoint — triggers `reindexRepo` and streams progress
   - Add `/setup/ollama` and `/setup/onnx` endpoints
   - Implement `src/model/ollama-backend.ts`, `src/model/transformers-backend.ts`
   - Implement `src/model/system-check.ts` (server-side: disk/RAM checks, Ollama detection)
   - Implement `src/tools/file-analyzer.ts` — cache management + backend dispatch

### Phase 2 — CLI Thinning

3. Modify `packages/sensei/src/commands/init.ts` — replace inline `reindexRepo` call with HTTP call to `POST /index`; add setup check via `GET /setup/status`
4. Modify `packages/sensei/src/cli.ts` — add `server start` / `server stop` / `server status` subcommands; add `--server <url>` global flag

### Phase 3 — MCP Separation (lower priority)

5. Create `packages/mcp/` — extract MCP tool registrations from `packages/sensei/src/index.ts`
   - Read-only tools (`get_file_context` etc.) call server's `GET /artifacts/:name` or read local `.sensei/` directly (no inference needed)
   - `reindex_repo` tool calls server's `POST /index`

### Phase 4 — Package Adapters + Folder Map

6. Implement `packages/server/src/adapters/` — JS/TS, Python adapters (from `15-package-adapters.md`)
7. Implement `packages/server/src/tools/folder-map.ts` — PackageScanner
8. Wire folder-map into server's `/index` flow

### Phase 5 — Traceability Enhancement

9. Embedding-based traceability (server-side, uses warm ONNX model)
10. `GET /traceability` endpoint — returns enhanced traceability.json

---

## Open Questions

| Question | Current thinking |
|---|---|
| Server auth for org deployments | `X-Sensei-Token` header, validated server-side; CLI reads from config.yaml |
| Server auto-start | `sensei server start --daemon` via `Bun.spawn` + PID file; MCP server auto-starts it if down |
| Windows support | Ollama supports Windows; `statfs` needs `wmic` fallback; otherwise all Bun APIs are cross-platform |
| Model override per repo | `.sensei/llmspec.yaml` can specify `model: phi3.5:mini`; server respects per-repo config |
| Analysis cache gitignore | `.sensei/analysis-cache/` always gitignored; symbol-map.json committed for team sharing |
