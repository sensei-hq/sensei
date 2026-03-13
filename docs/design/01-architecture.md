---
id: architecture
type: design
implements: []
---

# Architecture

> Sensei is a TypeScript/Bun monorepo that makes AI coding agents more effective through codebase intelligence, documentation traceability, and session continuity — exposed via an MCP server, a CLI, and a web dashboard.

## Overview

Sensei organises its capabilities into three layers. **Layer 1 (Codebase Intelligence)** handles the foundational work: scanning files, parsing ASTs, extracting symbols and call edges, computing embeddings, and indexing everything into Supabase. **Layer 2 (Documentation & Traceability)** builds on that index to track relationships between code and documentation — detecting drift, measuring quality, and generating docs from the symbol graph. **Layer 3 (System Intelligence)** composes the lower layers across multiple repositories to produce a cross-repo dependency graph, a service map, and conformance checks against shared architectural standards.

```
Layer 3: System Intelligence        — workspace, cross-repo graph, service map, conformance, system gaps
              builds on ↓
Layer 2: Documentation & Traceability — per-repo traceability, drift, quality metrics, doc generation
              builds on ↓
Layer 1: Codebase Intelligence      — scan, parse, index, symbol extraction, call graph, search
```

Cross-cutting concerns serve all layers: **Smart Context Delivery** (ranking and slicing code into token-budgeted packs), **Session Continuity** (crash recovery, snapshots, decision memory), **Multi-Agent Support** (skill generation for Claude, opencode, and others), **Analytics** (FTR scoring, coaching), and **Identity/Auth** (GitHub OAuth, team isolation, RLS).

All persistent state lives in **Supabase** (PostgreSQL + pgvector). There are no JSON artifacts written to disk for indexed data — only `.sensei/config.yaml` lives on disk per project. This enables cross-session queries, real-time dashboard updates, cross-repo analysis, and future team sharing without file sync complexity.

## Package Structure

```
/                                   ← repo root
  package.json                      ← bun workspace root ("workspaces": ["packages/*", "apps/*"])
  bun.lock
  README.md
  .gitignore

  config/                           ← shared build configuration (not a workspace package)
    tsconfig.base.json              ← base TypeScript config (strict, ESNext, bundler resolution)
    eslint.config.js                ← shared ESLint rules
    vitest.config.base.ts           ← base Vitest config (extended per package)

  packages/
    shared/                         ← Shared types, Supabase client, config schema, interfaces
    engine/                         ← All pipeline computation (Scan → Parse → Index → Rank → Slice → Assemble)
    collector/                      ← Event daemon (localhost:51789), hook scripts, session tracking
    server/                         ← MCP server, tool registration, stdio transport
    cli/                            ← CLI entry point, all sensei commands

  apps/
    dashboard/                      ← SvelteKit web app (analytics, traceability, quality, FTR)
    site/                           ← Marketing site (future)

  docs/
    features/                       ← What and why (feature specs)
    design/                         ← How (this directory — architecture, design decisions)

  supabase/                         ← Supabase local config, migrations, seed data
    config.toml
    migrations/

~/.config/sensei/                   ← Global developer config
  config.yaml                       ← Global config (model backend, auth tokens)

<repo>/.sensei/                     ← Per-project config (created by sensei init)
  config.yaml                       ← Project config (custom_libs, ranking strategy, agent setup)
```

## Package Responsibilities

### `packages/shared`

**Owns:**
- Supabase client factory and connection management (single place to configure the client)
- Config schema: types and parsing for `~/.config/sensei/config.yaml` and `.sensei/config.yaml`
- TypeScript types for all domain entities: `Repo`, `Symbol`, `CallEdge`, `Import`, `DocSection`, `TraceabilityLink`, `Session`, `TaskTurn`, `Workspace`, `ServiceNode`, `ConformanceResult`
- `ModelBackend` interface — provider-agnostic; implementations for Ollama, vLLM, LM Studio, and any OpenAI-compatible endpoint
- `TokenCounter` interface — provider-aware; implementations for Anthropic, OpenAI, local (tiktoken), and estimate (character-based)

**Depends on:** nothing internal (only npm packages)

**Does NOT:** contain any pipeline logic, CLI parsing, MCP handling, or Supabase schema definitions

---

### `packages/engine`

**Owns:**

*Scan stage* — file discovery via fast-glob, git diff integration for changed-file detection, mtime/hash fingerprinting, exclusion rules (node_modules, .git, build outputs)

*Parse stage* — `LanguageAdapterRegistry` (AST parsers keyed by file extension: TypeScript/JavaScript, Python, Go, Rust, others); `DocSystemAdapterRegistry` (Confluence, Notion, local Markdown); `ContractAdapterRegistry` (OpenAPI, Protobuf, AsyncAPI). Each adapter fails safely — a crash in one adapter never aborts the pipeline.

*Index stage* — receives parse output, writes symbols, call edges, imports, embeddings, doc sections, and traceability links to Supabase. This is the single write boundary to Supabase for indexed data.

*Rank stage* — `RankingStrategyChain` applies configurable strategies in order: `DiffFirstBFS` (changed files first, then BFS through call graph), `TraceabilityBoost` (symbols linked to active tasks ranked higher), `ExternalDocs` (boosts symbols with indexed doc sections), `Semantic` (cosine similarity via pgvector), `BM25` (lexical), `RelevanceLearning` (feedback from completed sessions)

*Slice stage* — `ASTSlicer` extracts precise line ranges for a symbol and its immediate dependencies; `SectionSlicer` extracts doc section content by heading path

*Assemble stage* — enforces token budget, deduplicates against current session context, and assembles a `ContextPack` (ordered list of slices with metadata)

*Skill generation* — generates skill markdown files from `project_profile` + `project_config` via a local model; output is agent-specific (Claude skills, opencode skills, Generic)

*Doc generation* — generates feature/design/API documentation from symbol and call graph analysis; writes to `docs/` in the target repo

*Quality analysis* — cyclomatic complexity, test coverage linkage, dead code detection (symbols with no callers and no exports), pattern consistency, gap detection

*Workspace indexer* — orchestrates per-repo indexers across a workspace, builds the cross-repo call graph, generates the service map, runs conformance checks, identifies system gaps

**Depends on:** `shared`

**Does NOT:** contain any CLI argument parsing, MCP protocol handling, HTTP routes, or direct Supabase schema definitions

---

### `packages/collector`

**Owns:**
- HTTP daemon on `localhost:51789` (started by `sensei init`, auto-started by hooks)
- Hook scripts installed to agent hook directories: `PreToolUse`, `PostToolUse`, `UserPromptSubmit`
- JSONL fallback writer: if the daemon is unreachable, events are written to `.sensei/events.jsonl` and drained on reconnect
- Supabase event writer: writes to `task_sessions`, `task_turns`, `events` tables
- Session snapshot writer: triggered by agent `snapshot` tool call or automatically every N turns
- FTR (First-Time-Right) calculator: runs after a session completes, scores each requirement by whether the implementing code was accepted without rework
- Crash detection: `process.on('exit')` hook + heartbeat to mark sessions interrupted vs completed

**Depends on:** `shared`

**Does NOT:** call engine, parse code, or make MCP calls

---

### `packages/server`

**Owns:**
- MCP server entry point, stdio transport setup
- Tool registration for all MCP tools
- Tool handlers — thin wrappers that validate inputs, call engine or shared, and return formatted responses
- MCP tool categories:
  - *Orientation:* `get_session_context`, `get_llmspec`
  - *Indexing:* `index`, `reindex`
  - *Context:* `load_context`, `context_pack`, `recommend_next`, `token_stats`
  - *Search:* `search`, `find_doc`, `get_lib_docs`
  - *Memory:* `add_decision`, `add_pattern`, `snapshot`
  - *Traceability:* `check_drift`, `find_coverage`
  - *Workspace:* `workspace_status`, `workspace_graph`, `find_service`

**Depends on:** `engine`, `shared`

**Does NOT:** contain any business logic — all computation is delegated to engine; does not write to Supabase directly except via engine or collector

---

### `packages/cli`

**Owns:**
- CLI entry point and command router (`sensei` binary)
- Commands with options, validation, and progress output via `@clack/prompts`:
  - `sensei init` — stack detection, Supabase setup, first index run, CLAUDE.md + AGENTS.md generation, hook installation
  - `sensei index` / `sensei reindex` — trigger engine index/reindex pipeline
  - `sensei setup --agent <name>` — generate agent-specific skill files, configure hooks
  - `sensei workspace init/add/index/analyse` — workspace management
  - `sensei analyse goals/quality/gaps/refactor-readiness` — quality and gap analysis
  - `sensei doctor` — check doc coverage, traceability completeness, config validity
  - `sensei doc new/generate` — scaffold or generate documentation
  - `sensei stats` — session and FTR statistics
  - `sensei traceability link/export/promote` — traceability management
  - `sensei benchmark run/compare` — benchmark harness
  - `sensei quality report` — quality metrics report
  - `sensei update-registry` — update external library registry

**Depends on:** `engine`, `shared`, `collector`

**Does NOT:** contain business logic — all computation is delegated to engine; does not implement MCP protocol

---

### `apps/dashboard`

**Owns:**
- SvelteKit web app served at `localhost:3000` during development
- Views: Repos, Symbol Browser, Traceability, Quality, Analytics, FTR, Coaching, Service Map, Workspace Health, Sessions, Settings
- Reads Supabase directly via the shared Supabase client (no intermediate API layer)
- Renders Mermaid diagrams (service map, call graph, traceability graph)
- Auth via Supabase GitHub OAuth + magic link

**Depends on:** `shared` (Supabase client and types)

**Does NOT:** call the MCP server, call the CLI, or run engine computation

## Key Interfaces

These are the primary TypeScript interfaces at package boundaries. Implementations in `engine` satisfy scanner/parser/indexer/ranker/slicer/assembler interfaces. `shared` defines ModelBackend and TokenCounter so both `engine` and `server` can use them without coupling to a specific provider.

```typescript
// packages/shared — provider-agnostic model inference
interface ModelBackend {
  generate(prompt: string, options?: GenerateOptions): Promise<string>;
  embed(text: string): Promise<number[]>;
  isAvailable(): Promise<boolean>;
}

// packages/shared — token counting (provider-aware)
interface TokenCounter {
  count(text: string): number;
  countMessages(messages: Message[]): number;
  budget(maxTokens: number): TokenBudget;
}

// packages/engine — Scan stage output
interface ScanResult {
  files: FileEntry[];           // path, mtime, hash, size
  changed: string[];            // paths changed since last scan
  deleted: string[];            // paths removed since last scan
}

// packages/engine — Parse stage contract
interface LanguageAdapter {
  extensions: string[];
  parse(file: FileEntry): Promise<ParsedFile>;
  extractSymbols(parsed: ParsedFile): Symbol[];
  extractEdges(parsed: ParsedFile): CallEdge[];
}

// packages/engine — Index stage write interface
interface Indexer {
  indexRepo(repo: Repo, scanResult: ScanResult, parsed: ParsedFile[]): Promise<IndexResult>;
  reindex(repo: Repo, changedFiles: string[]): Promise<IndexResult>;
}

// packages/engine — Rank stage
interface RankingStrategy {
  name: string;
  score(symbol: Symbol, context: RankContext): number;
}

interface RankContext {
  task: string;
  changedFiles: string[];
  sessionSymbols: string[];   // already loaded in current session
  traceabilityLinks: TraceabilityLink[];
}

// packages/engine — Slice stage
interface ASTSlicer {
  slice(symbol: Symbol, depth: number): Promise<CodeSlice>;
}

interface SectionSlicer {
  slice(docSection: DocSection): Promise<DocSlice>;
}

// packages/engine — Assemble stage output
interface ContextPack {
  slices: (CodeSlice | DocSlice)[];
  totalTokens: number;
  budget: TokenBudget;
  sessionDeduped: string[];   // symbol IDs already seen in session
}

// packages/engine — Agent adapter (for skill generation)
interface AgentAdapter {
  agentName: string;
  skillsDir: string;          // e.g. ~/.claude/skills/ or ~/.opencode/skills/
  hooksDir: string;
  generateSkill(profile: ProjectProfile): Promise<SkillFile>;
  installHooks(daemonUrl: string): Promise<void>;
}
```

## Data Flow

### Flow 1: Indexing

```
sensei index (CLI) or index tool (MCP)
       │
       ▼
engine: Scanner
  ├── glob all files matching include/exclude rules
  ├── compute mtime + hash fingerprint per file
  └── diff against last scan state in Supabase → ScanResult{files, changed, deleted}
       │
       ▼
engine: Parser (per changed file, in parallel)
  ├── LanguageAdapterRegistry.get(ext) → LanguageAdapter
  ├── adapter.parse(file) → ParsedFile (AST or token stream)
  ├── adapter.extractSymbols(parsed) → Symbol[]
  └── adapter.extractEdges(parsed) → CallEdge[]
       │
       ▼
engine: Indexer
  ├── upsert symbols → supabase: symbols
  ├── upsert call edges → supabase: call_edges
  ├── upsert imports → supabase: imports
  ├── generate embeddings via ModelBackend.embed() → supabase: embeddings
  ├── upsert doc sections (if DocSystemAdapter present) → supabase: doc_sections
  └── update scan fingerprint → supabase: scan_state
       │
       ▼
  IndexResult { symbolCount, edgeCount, duration, errors[] }
```

### Flow 2: Context Pack

```
context_pack(task, maxTokens) (MCP tool or CLI)
       │
       ▼
engine: Ranker
  ├── load RankingStrategyChain from repo config
  ├── query supabase: symbols for candidate set
  ├── apply strategies in order:
  │   ├── DiffFirstBFS:       score by proximity to changed files in call graph
  │   ├── TraceabilityBoost:  score by linkage to active task requirements
  │   ├── Semantic:           cosine similarity via pgvector (embedding of task)
  │   ├── BM25:               lexical match against symbol names and docs
  │   └── RelevanceLearning:  boost symbols accepted in past similar sessions
  └── return ranked Symbol[]
       │
       ▼
engine: Slicer (per top-N symbols)
  ├── ASTSlicer.slice(symbol, depth=1) → CodeSlice{path, startLine, endLine, content, tokens}
  └── SectionSlicer.slice(linkedDocSection) → DocSlice{source, heading, content, tokens}
       │
       ▼
engine: Assembler
  ├── enforce token budget (maxTokens)
  ├── deduplicate against session context (symbols already loaded)
  ├── order slices (most relevant first)
  └── return ContextPack{slices[], totalTokens, sessionDeduped[]}
       │
       ▼
  MCP response or CLI output
```

### Flow 3: Agent Session

```
Agent starts task
  → get_session_context()         ← orientation + recovery context if previous session interrupted
  → recommend_next(task)          ← ranking prescription: which scope, which depth
  → context_pack(task, 8000)      ← ranked, sliced, token-budgeted code + doc slices

  [Agent works on code]

  → search(query)                 ← symbol search for unfamiliar APIs
  → get_lib_docs(lib, component)  ← external library doc chunks
  → add_decision(text)            ← persist architectural decision to session
  → snapshot()                    ← write session snapshot to Supabase

  [Task complete]

  → collector daemon              ← PostToolUse hook fires on each tool use
  → FTR calculator runs           ← scores requirements after session end
  → Analytics updated             ← dashboard shows updated FTR, session timeline
```

## Architectural Decisions

| Decision | Why | Alternatives Considered |
|---|---|---|
| **Supabase as the single data store** | Enables cross-session persistence, cross-repo queries, real-time dashboard, and multi-user team access without file sync. pgvector handles semantic search natively. | File-based artifacts (JSON/YAML in repo): no cross-repo queries, breaks team sharing, no real-time dashboard. SQLite: no multi-user, no cloud sync, no pgvector. |
| **Engine package separated from CLI and server** | Both CLI and MCP server need identical pipeline computation. Separating engine prevents duplication, allows engine stages to be unit-tested without transport concerns, and means adding a new transport (e.g. REST API) requires no engine changes. | Co-locating logic in CLI: duplicated code in server. Co-locating in server: CLI becomes thin wrapper with no local execution. |
| **Pipeline stages as distinct interfaces** | Allows language adapters to be added (new `LanguageAdapter`) without modifying ranking or slicing. Allows ranking strategies to be swapped via config without touching parsing. Each stage can be tested independently. | Monolithic indexer function: impossible to swap adapters, hard to test ranking without running parse. |
| **Adapters fail safely** | A broken Python adapter must not abort indexing a TypeScript repo. Each adapter is wrapped; errors are logged and the file is skipped. | Fail-fast: simpler but makes the pipeline brittle for multi-language repos. |
| **Collector as a standalone daemon** | Telemetry must not block agent tool calls. The daemon decouples event capture from event processing. JSONL fallback ensures no events are lost during daemon outages. | In-process event capture: adds latency to every tool call. Direct Supabase writes from hooks: blocks the hook execution path. |
| **MCP server is thin (delegates to engine)** | Tool handlers that contain business logic cannot be unit-tested without an MCP client. Thin handlers mean engine stages are tested directly; MCP layer is tested with a lightweight harness. | Fat handlers: logic lives in server, hard to reuse in CLI, hard to test. |
| **Dashboard reads Supabase directly** | Avoids a REST API layer between dashboard and data. Supabase Row Level Security handles authorisation. Supabase Realtime enables live updates. One fewer service to operate. | REST API in front of Supabase: extra latency, extra service, duplicated query logic. GraphQL layer: significant complexity for no clear gain. |
| **ModelBackend is provider-agnostic** | No hard dependency on Ollama. Teams can use cloud models for quality-sensitive tasks (extraction, doc generation) and local models for cost-sensitive tasks (embedding). Swapping providers requires only config change, not code change. | Hard-coded Ollama: breaks in air-gapped environments, forces local GPU for all tasks. Hard-coded OpenAI: adds cost and network dependency for every embed call. |

## Technology Choices

| Component | Choice | Reason |
|---|---|---|
| Language | TypeScript | Native MCP SDK support, type safety across all package boundaries |
| Runtime / package manager | Bun | Fast installs, built-in test runner, workspace support, ESM-native |
| Monorepo | Bun workspaces | Single lockfile, shared dependencies, no Nx/Turborepo overhead |
| MCP SDK | `@modelcontextprotocol/sdk` | Standard SDK maintained by Anthropic |
| Database | Supabase (PostgreSQL + pgvector) | Persistent storage, semantic search, Realtime, RLS, local Docker dev mode |
| Dashboard framework | SvelteKit | Lightweight, fast SSR, native TypeScript, familiar to the team |
| CSS / design system | UnoCSS + Rokkit | Atomic utility CSS, consistent component library |
| CLI prompts | `@clack/prompts` | Accessible spinners, selects, confirms; clean DX |
| File globbing | `fast-glob` | Fastest glob implementation available for Node/Bun |
| YAML parser | `js-yaml` | Lightweight, well-maintained, handles config files |
| Unit tests | Vitest | Fast, ESM-native, `*.spec.ts` files co-located with source |
| E2E tests | Playwright | Full CLI and MCP tool integration tests |
| Local model inference | Ollama (default) | Easy local setup; any OpenAI-compatible endpoint works via ModelBackend |
| Semantic embeddings | pgvector (via Supabase) | Eliminates a separate vector database; co-located with relational data |

## Non-Functional Requirements

| NFR | Requirement |
|---|---|
| Indexing latency | Incremental reindex of a changed file must complete within 2 seconds on a 50K-line repo |
| Context pack latency | `context_pack` must return within 500ms for a 8K-token pack on an already-indexed repo |
| Token accuracy | Token counts must be within 5% of provider-reported counts for budget enforcement |
| Adapter isolation | A crash in any language or doc adapter must not abort the pipeline for other files |
| Event loss | No agent events lost during collector daemon restarts (JSONL fallback + drain) |
| Dashboard load | Repo view and Traceability view must load within 2 seconds on a 100K-symbol index |
| Maintainability | Any pipeline stage can be replaced without modifying other stages |
| Testability | Every package can be tested independently with Vitest using Supabase local Docker |
| Extensibility | New language adapters added without modifying Scan, Rank, Slice, or Assemble stages |
| Multi-repo | Workspace indexer must handle up to 20 repos without sequential bottleneck (parallel per-repo indexing) |
