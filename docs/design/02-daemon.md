# Daemon -- senseid

## Overview

Rust HTTP server. The core engine of Sensei. Owns the code graph, session store, task queue, intelligence pipeline, analytics engine, and all derived analysis. Every other component (desktop app, MCP server, CLI) is a consumer of the daemon's API.

Port 7744 (release) / 7745 (dev). Mode is compile-time via Cargo feature flag (`--features dev`), not a runtime environment variable. Dev builds connect to `sensei_dev`, release builds connect to `sensei`.

See [ideas/03-observatory](../ideas/03-observatory.md) and [ideas/04-project](../ideas/04-project.md) for the "what." This document covers the "how."

---

## Architecture

### Crate structure

```
crates/
├── senseid/        HTTP daemon (binary)
│   ├── src/
│   │   ├── main.rs         Entry point, Axum server setup
│   │   ├── routes/         HTTP route handlers
│   │   ├── indexer/        Indexing pipeline
│   │   ├── tasks/          Task queue + workers
│   │   ├── watcher/        File system watchers
│   │   ├── intelligence/   Compression, patterns, context, recommendations
│   │   ├── analytics/      FTR computation, session metrics
│   │   └── db/             PostgreSQL pool + queries (sqlx)
│   └── Cargo.toml
├── cli/            sensei CLI (binary: sensei)
├── mcp/            MCP server (binary: sensei-mcp)
├── bootstrap/      Prereq checker (library, no binary, no DB access)
├── gateway/        LLM routing library
├── logger/         Structured logging (library)
└── sensei-config/  Shared config types (library)
```

### Axum HTTP server

The daemon is a single Axum process. On startup it:

1. Reads `SenseiConfig::from_env()` (compile-time port and DB name).
2. Creates a PostgreSQL connection pool (`sqlx::PgPool`).
3. Registers route handlers grouped by screen (bootstrap, setup, observatory, project).
4. Spawns task queue workers (default 3 threads).
5. Spawns file watchers (one per scanned root).
6. Binds the port and serves.

### PostgreSQL pool

All state lives in PostgreSQL. There is no in-memory cache that outlives a request, no file-based storage, and no separate vector database. pgvector handles semantic search natively alongside relational queries.

The connection pool is shared across all handlers via Axum state extraction. Queries use `sqlx` with compile-time checked SQL where possible.

---

## API surface

Every endpoint exists because a screen needs it. Grouped by the screen that consumes them.

### Bootstrap endpoints

Called by the desktop app sidecar during bootstrap, and by the daemon's own health check.

| Method | Route | Purpose |
|--------|-------|---------|
| GET | `/health` | Daemon status, version, uptime, component health |
| GET | `/api/health/components` | Full component status aligned with bootstrap types |

### Setup endpoints

Called by the wizard after the daemon is running.

| Method | Route | Purpose |
|--------|-------|---------|
| GET | `/api/config` | Read app config key-value pairs |
| PUT | `/api/config` | Set config (preferences, setup_complete, etc.) |
| POST | `/api/scan/roots` | Add a watch root directory |
| DELETE | `/api/scan/roots` | Remove a watch root |
| GET | `/api/scan/roots` | List watch roots |
| POST | `/api/scan` | Trigger scan for a root path |
| GET | `/api/scan/events` | SSE stream (project + activity entities) |
| GET | `/api/index/status` | Current indexing status |
| GET | `/api/projects` | List discovered projects |
| POST | `/api/projects` | Create project |
| PUT | `/api/projects/:id` | Update project (name, icon, metadata) |
| DELETE | `/api/projects/:id` | Delete project |
| POST | `/api/projects/:id/folders` | Add folder to project |
| DELETE | `/api/projects/:id/folders/:fid` | Remove folder from project |
| POST | `/api/projects/merge` | Merge projects |
| GET | `/api/assistants/detect` | Detect installed ACPs |
| POST | `/api/assistants/configure` | Register MCP + extensions with ACPs |
| GET | `/api/libraries` | List libraries (auto-detected + imported) |

### Observatory endpoints

Called by the daily-use dashboard and session screens.

| Method | Route | Purpose |
|--------|-------|---------|
| GET | `/api/metrics/:project_id` | FTR trend, turn count, rework rate, tool adherence |
| GET | `/api/sessions` | Cross-project session list with filters |
| GET | `/api/sessions/:id` | Session detail + event timeline |
| GET | `/api/recommendations` | Active recommendations with evidence |
| GET | `/api/recommendations/:id` | Recommendation detail + MOE reasoning trace |
| GET | `/api/memories` | Memories with scope, strength, status |
| GET | `/api/memories/:id` | Memory detail + evidence + examples |
| GET | `/api/insights` | Collective intelligence sharing history |

### Project endpoints

Called by project detail screens.

| Method | Route | Purpose |
|--------|-------|---------|
| GET | `/api/projects/:id/graph` | Code graph nodes + edges for visualization |
| GET | `/api/projects/:id/patterns` | Detected patterns + anti-patterns |
| GET | `/api/projects/:id/communities` | Leiden communities with god-node flags |
| GET | `/api/projects/:id/drift` | Doc drift items (current / drifted / broken) |
| GET | `/api/projects/:id/duplicates` | Duplicate code clusters |
| GET | `/api/projects/:id/sessions` | Sessions scoped to project |
| PUT | `/api/projects/:id/settings` | Guidelines, links, exclusions, privacy, skills |

### Workflow endpoints (MCP-facing)

Called by the MCP server on behalf of AI assistants during sessions.

| Method | Route | Purpose |
|--------|-------|---------|
| POST | `/api/events` | Log a workflow event |
| GET | `/api/events/:project_id` | List events for a project |
| GET | `/api/state/:project_id` | Current workflow state (phase, task, issue) |
| PUT | `/api/state/:project_id` | Update workflow state |
| GET | `/api/phases/:project_id` | Phase transition history |

---

## Indexing pipeline

The indexing pipeline transforms raw source files into a searchable code graph. It is deterministic (no LLM required for the core path) with optional local inference enrichment when Ollama is available.

### Pipeline stages

```
ScanRoot(path)
  --> ProcessRepo(repo_path)
        --> ProcessFolder(folder_path)
              --> ProcessFile(file_path)  [language adapter]
        --> ResolveEdges(repo_id)         [barrier: all files done]
        --> BuildConnections(repo_id)     [after resolve]
```

### Stage details

**ScanRoot** -- walks directory tree, finds `.git` folders, registers each as a project in PostgreSQL, enqueues `ProcessRepo` per repo.

**ProcessRepo** -- detects git remote (duplicate check), detects workspace members (npm, Cargo, pnpm, go.work), creates virtual grouping nodes (`repo:`, `code:`, `docs:`), enqueues `ProcessFolder` per directory plus barrier tasks.

**ProcessFolder** -- creates module node, wires containment edge, enumerates files (non-recursive), enqueues `ProcessFile` per supported file.

**ProcessFile** -- routes to language adapter by extension, extracts symbols (functions, classes, interfaces, types, constants, enums), raw imports, raw calls, parent tracking. Stores unresolved references for later resolution. Updates `scan_state` fingerprint (mtime + content_hash).

**ResolveEdges** -- barrier task, runs after all file tasks complete. Resolves unresolved references: `./bar` to `file:{path}/bar.ts`, `helperFn` to `fn:...:helperFn`. Creates CALLS, IMPORTS, HAS_METHOD, CONTAINS_FN edges. Runs framework tagging and community detection.

**BuildConnections** -- runs after edge resolution. Creates doc-to-code traceability links (SPECIFIES, IMPLEMENTS, DOCUMENTS, COVERS, MENTIONS_FN). Creates cross-repo links for multi-repo projects (SHARED_SYMBOLS, SHARED_DEPS). Flags doc drift.

### Adapter IR

Language adapters produce a common intermediate representation. This separates parsing from processing -- new languages only need a new adapter.

```
Source file --> Language adapter --> ParsedFile (IR) --> Graph writer --> PostgreSQL
```

The IR defines three node types with a shared base:

- **IRDoc** -- markdown/text files with frontmatter, sections, code blocks, file/symbol/doc references.
- **IRModule** -- functional files with functions, constants, imports, type aliases.
- **IRClass** -- OO constructs (class, struct, interface, trait, enum, component) with methods, properties, implements/extends.

All fields are `Option<>` -- adapters populate what they can. Processing gracefully handles missing data.

### Language adapters

| Adapter | Extensions | Parser |
|---------|-----------|--------|
| Rust | `.rs` | tree-sitter-rust |
| TypeScript | `.ts`, `.tsx`, `.js`, `.jsx` | tree-sitter-typescript |
| Python | `.py` | tree-sitter-python |
| SQL | `.sql`, `.ddl` | tree-sitter-sql |
| Svelte | `.svelte` | tree-sitter-svelte |
| Swift | `.swift` | tree-sitter-swift |
| Markdown | `.md`, `.mdx` | Frontmatter + heading parser |

Each adapter fails safely -- a crash in one adapter never aborts the pipeline for other files. Errors are logged to `sensei.index_errors`.

### Optional local inference pass

When Ollama is available, an enrichment pass runs after the deterministic pipeline:

| Task | Model | Output | Storage |
|------|-------|--------|---------|
| Embeddings | all-MiniLM-L6-v2 | 384-dim per file | `nodes.embedding` (HNSW index) |
| L2 logic flow | gemma3 | Plain-English function summaries | `nodes.props` |
| Pattern classification | gemma3 | Pattern type + confidence | `inference.detected_patterns` |
| Testability scoring | gemma3 | Complexity + composability | `nodes.props` |

If Ollama is not running, the pipeline completes without these enrichments. Semantic search falls back to BM25 lexical matching.

---

## Task queue

The task queue replaces a monolithic indexer with a hierarchical system where each unit of work is small and focused. It is backed by PostgreSQL `LISTEN/NOTIFY` for coordination.

### Task schema

```rust
pub struct Task {
    pub id: u64,
    pub kind: TaskKind,
    pub repo_id: String,
    pub path: String,
    pub parent_task_id: Option<u64>,
    pub status: TaskStatus,
    pub depends_on: Vec<u64>,
    pub created_at: Instant,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
    pub error: Option<String>,
}

pub enum TaskKind {
    ScanRoot, ProcessRepo, ProcessFolder, ProcessFile,
    DeleteFile, DeleteFolder, ResolveEdges, BuildConnections,
}

pub enum TaskStatus {
    Pending, Blocked, Running, Completed, Failed,
}
```

### Hierarchical tasks

Tasks form a tree: `ScanRoot` creates `ProcessRepo` tasks, which create `ProcessFolder` tasks, which create `ProcessFile` tasks. Barrier tasks (`ResolveEdges`, `BuildConnections`) have `depends_on` lists that prevent them from running until all prerequisite tasks complete.

### Barrier mechanism

When a task completes:
1. Mark task as `Completed`.
2. For each task that `depends_on` this task: remove from its dependency list.
3. If `depends_on` is now empty, move from `Blocked` to `Pending`.

### Parallelism

- **Workers**: N worker threads (default 3) pull tasks from queue.
- **Repo limit**: Max M repos processing concurrently (default 3).
- **File parallelism**: File tasks for the same repo run in parallel across workers.
- **Watcher threads**: One per scanned root (not per repo).

### Incremental updates

Single file change: `ProcessFile` + `ResolveEdges` -- two tasks, sub-second. New folder: `ProcessFolder` + N x `ProcessFile` + `ResolveEdges` -- parallel file processing. Full re-index: all stages, but file tasks run in parallel.

### Priority

Tasks are dequeued by priority. `ProcessFile` tasks from watcher events (real-time changes) get higher priority than bulk scan tasks. Barrier tasks inherit the priority of their parent.

### File watcher integration

One watcher per scanned root (stored in `sensei.folders_to_watch`, recreated on daemon restart). On file change: identify repo by path prefix, enqueue `ProcessFile` + `ResolveEdges`. On deletion: enqueue `DeleteFile`/`DeleteFolder` + `ResolveEdges`.

---

## Intelligence layer

The intelligence layer builds on the indexed code graph and session data to provide compressed context, pattern detection, recommendations, and semantic search.

### Compression L0-L3

Code is stored at four resolution levels. The MCP server serves the requested level based on task type.

| Level | Name | Content | Tokens (typical function) |
|-------|------|---------|--------------------------|
| L0 | Signature | Export declaration line, stripped of body | 8-15 |
| L1 | IO Pattern | Assignment notation: `result = fn(param1, param2)` | 20-40 |
| L2 | Logic Flow | Plain-English logic description (LLM-generated) | 30-80 |
| L3 | Full Source | Actual file content, read from disk | 100-500+ |

L0 and L1 are computed deterministically during indexing. L2 requires local inference (optional). L3 is never stored -- always read live.

### Context manager

The context manager exposes targeted slices of the indexed codebase, each sized to fit within a defined token budget. Key operations:

- **`load_context(scope, level)`** -- returns content for a scope (orientation, module, patterns) at the requested resolution level.
- **`context_pack(task, max_tokens)`** -- ranks symbols by relevance to the task, slices them, deduplicates against current session context, and assembles a token-budgeted pack.
- **`recommend_next(task)`** -- classifies the task (discovery/understanding/edit) and returns a prescription: which scope, which level, which tools to call.

Ranking strategies applied in order:
1. **DiffFirstBFS** -- changed files first, then BFS through call graph.
2. **TraceabilityBoost** -- symbols linked to active tasks ranked higher.
3. **Semantic** -- cosine similarity via pgvector embeddings.
4. **BM25** -- lexical match against symbol names and docs.
5. **RelevanceLearning** -- boost symbols accepted in past similar sessions.

Token budget target: orientation < 250 tokens, module < 150 tokens, `get_session_context()` < 400 tokens regardless of project age.

### Pattern store

Patterns are detected during indexing and stored in `inference.detected_patterns`. Each pattern has a lifecycle: `suggested` (auto-detected, 2+ usages) -> `gap` (identified but not followed) -> `rule` (enforced). Anti-patterns link to their constructive fix via `fix_pattern_id`.

Detection methods:
- **Naming heuristics** -- file/directory naming conventions (adapter/, factory/, etc.).
- **Structural analysis** -- implements/extends relationships from the IR.
- **Duplicate detection** -- normalized body hash comparison.
- **Convention analysis** -- recurring export patterns, file layout similarity.

### Metadata model

The code graph stores four categories of metadata:

1. **Orientation** -- project name, stack, entry points (derived from `sensei.projects` + `sensei.folders`).
2. **Symbol graph** -- `sensei.nodes` (unified node table, 16 kinds) with `embedding vector(384)` and HNSW index.
3. **Relationships** -- `sensei.edges` (11 edge kinds: calls, implements, extends, imports, depends_on, traces_to, references, covers, rationale_for, duplicates, similar_to).
4. **Indexing fingerprints** -- `sensei.scan_state` (mtime + content_hash per file).

### Semantic search (pgvector)

Embeddings are stored directly in the `nodes` table (`embedding vector(384)`) with an HNSW index. Library documentation embeddings use a larger dimension (`library_pages.embedding vector(768)`). Semantic search is a single SQL query combining cosine similarity with relational filters (project scope, node kind, etc.).

---

## Analytics engine

The analytics engine computes metrics from raw session events and surfaces coaching insights.

### Event types

The `activity.events` table stores a unified log with 12 event types:

| Event type | Source | Purpose |
|-----------|--------|---------|
| `tool_call` | Pre/post-tool hooks | Tool name, params, response, duration |
| `api_request` | Hook capture | External API calls |
| `correction` | Turn classification | User corrected the AI (affects FTR) |
| `turn` | UserPromptSubmit hook | Turn counter per task |
| `phase_transition` | Phase commands | Workflow phase changes |
| `checkpoint` | Session end / manual | Session state snapshot |
| `task_start` | Build command | Task boundary marker |
| `task_end` | Build command | Task completion marker |
| `context_loaded` | Context commands | Files loaded, token count, level |
| `edit` | File modification | Files changed, lines added/removed |
| `test` | Test execution | Test results |
| `error` | Error capture | Errors during session |

### FTR computation

FTR (First-Time-Right) is the hero metric. A session is FTR when `corrections == 0`.

**Turn classification** determines corrections. Every user message is classified:

| Classification | Detection | FTR impact |
|---------------|-----------|------------|
| `correction` | Keywords: "no", "wrong", "not what I", "try again", "fix this" | Negative |
| `continuation` | "now", "next", "also", "and then" | Neutral |
| `clarification` | "what about", "how does", "can you explain" | Neutral |
| `new_request` | Default | Neutral |

Phase 1 uses regex heuristics. Phase 2 uses local model classification for higher accuracy.

**Project FTR** is a rolling 14-day window: `sessions with ftr=true / total sessions`.

### Correction classification

When a correction is detected, the event captures:
- Which turn number it occurred at.
- Which module was being worked on.
- What tool was called before and after the correction.

This feeds the module correction rate (which modules cause the most rework) and the recommendation engine (patterns to extract, personas to create).

### Project memory

Cross-session knowledge layer. Core principle: distil at session end, load compressed at session start. Context budget stays flat (~300 tokens) regardless of project age.

- **Decisions** are deduplicated (not an append log).
- Only the last 2 session summaries are in active memory; the rest are archived.
- Open items shrink as items are closed.

### Recommendations

The recommendation pipeline: signal accumulation -> threshold check -> MOE consensus panel -> recommendation with evidence and projected impact.

Recommendations track their own effectiveness via a measurement window: `baseline_ftr` at `acted_at` timestamp vs `current_ftr` rolling. Verdicts: `positive`, `negative`, `neutral`, `pending`.

Recommendation types: `promote-pattern`, `create-agent`, `write-skill`, `archive-memory`, `enrich-memory`, `cross-project`.

---

## Traceability

### Traceability matrix

The traceability matrix maps documentation files to the source files they describe. Stored in PostgreSQL and combined with `git diff` for precise drift detection.

**Population strategy:**

1. **Manual** -- declared in project configuration (`covers:` per doc). Authoritative, never overwritten.
2. **Auto-detection** -- during indexing, scan doc content for code file paths (backtick-wrapped filenames) and symbol names matching the graph. Best-effort.
3. Manual declarations override auto-detection.

**Edge types created by BuildConnections:**

| Edge | Meaning |
|------|---------|
| SPECIFIES | Requirement doc -> design doc |
| IMPLEMENTS | Design doc -> code module |
| DOCUMENTS | Usage doc -> code module |
| COVERS | Doc -> file (backtick path references) |
| MENTIONS_FN | Doc -> function (backtick identifier references) |

### Drift detection

Drift detection uses `git diff` against the last indexed commit to identify changed files, then cross-references against the traceability matrix to flag docs whose linked code has changed.

```
checkDrift(repo):
  1. changedFiles = git diff <lastIndexedCommit>..HEAD --name-only
  2. Load traceability matrix from PostgreSQL
  3. For each (doc, coveredFiles):
     if ANY coveredFile in changedFiles AND doc NOT in changedFiles:
       -> doc is drifted
  4. Return { drifted: DriftEntry[], summary }
```

Drift items are stored in `inference.drift_items` with status: `current`, `drifted`, `broken`.

For non-git repos, falls back to mtime/size comparison against `sensei.scan_state` fingerprints.

### Doc tools

- **Pre-commit hook** -- blocks commit if docs are stale relative to changed code.
- **`sensei drift`** CLI command -- on-demand drift report.
- **MCP `check_drift`** tool -- AI can query drift status mid-session.

---

## Database schema

All state lives in PostgreSQL. Five schemas, approximately 39 tables. DDL source of truth is in `database/ddl/`, managed by `dbd`.

### Schema overview

| Schema | Tables | Purpose |
|--------|--------|---------|
| `gateway` | 6 | LLM routing engine (providers, models, routers, fallback chains) |
| `sensei` | 18 | Structural data (folders, projects, nodes, edges, libraries, extensions, config) |
| `inference` | 10 | Derived analysis (patterns, communities, recommendations, drift, reasoning traces) |
| `activity` | 4 | Session tracking (sessions, events, task_sessions, snapshots) |
| `history` | 1 | Audit trail (past_extensions, auto-populated by trigger) |

Additionally, `staging` (9 tables) provides import buffers for seed data with no PKs or FKs.

### Key tables

**`sensei.nodes`** -- unified node table. Every structural element: files, code symbols, doc sections, rationale comments. 16 `node_kind` values. Self-referential `parent_id` for containment hierarchy. Carries `embedding vector(384)` with HNSW index.

**`sensei.edges`** -- typed relationships. 11 `edge_kind` values (calls, implements, extends, imports, depends_on, traces_to, references, covers, rationale_for, duplicates, similar_to). `edge_confidence`: extracted, inferred, ambiguous.

**`sensei.projects`** -- independent grouping entity. `project_maturity`: discovery -> active -> maintenance -> archived. Carries icon, stack, commands, links, guidelines, preferred_acp, tags, backlog, privacy, excluded_globs.

**`sensei.folders`** -- discovered filesystem tree. `folder_kind`: parent, folder, git, subtree. Self-referential `parent_id` for hierarchy. `folders_to_watch` holds watched root directories with status: scanning -> watching -> paused.

**`activity.sessions`** -- session records with `session_outcome` enum (completed, corrected, blocked, partial, abandoned). Carries ftr boolean, corrections count, tokens_in/out, duration_ms, module, summary.

**`activity.events`** -- unified activity log. 12 `event_type` values. Payload in `data` JSONB.

**`inference.detected_patterns`** -- patterns and anti-patterns. `pattern_lifecycle`: suggested -> gap -> rule. `pattern_severity`: low, medium, high. Self-referential `fix_pattern_id` for anti-pattern -> constructive fix cross-link.

**`inference.recommendations`** -- full lifecycle: insight -> recommendation -> action -> measurement. Tracks `baseline_ftr` at `acted_at` vs `current_ftr` rolling. Verdicts: pending, positive, negative, neutral.

**`inference.reasoning_traces`** -- MOE consensus panel debates: `models_used`, `exchanges`, `consensus`, `action_proposed`.

**`inference.communities`** -- Leiden algorithm clusters with labels, descriptions, `god_node_ids`.

**`inference.drift_items`** -- doc-to-code traceability drift. Status: current, drifted, broken.

### Enum types

All constraints use proper PostgreSQL enums. Key enums:

| Enum | Values |
|------|--------|
| `node_kind` | file, module, package, class, interface, function, method, property, field, parameter, type, const, enum, enum_variant, section, rationale |
| `edge_kind` | calls, implements, extends, imports, depends_on, traces_to, references, covers, rationale_for, duplicates, similar_to |
| `session_outcome` | completed, corrected, blocked, partial, abandoned |
| `event_type` | tool_call, api_request, correction, turn, phase_transition, checkpoint, task_start, task_end, context_loaded, edit, test, error |
| `pattern_lifecycle` | suggested, gap, rule |
| `recommendation_status` | pending, accepted, dismissed, superseded |
| `recommendation_verdict` | pending, positive, negative, neutral |
| `extension_kind` | plugin, skill, command, agent, hook |
| `folder_kind` | parent, folder, git, subtree |
| `project_maturity` | discovery, active, maintenance, archived |

### DDL management

DDL files live in `database/ddl/` organized by type (enum, table, view, function, procedure). Files define the full table -- no ALTER statements. The `dbd` command handles reset, apply, and import. Staging tables and import procedures are used for seeding with timestamp-based guards to avoid overwriting production data.
