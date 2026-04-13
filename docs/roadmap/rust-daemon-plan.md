# Rust Daemon Implementation Plan

> Status: Planning | Resolves: #47
> Replaces: packages/server + packages/graph-indexer + packages/cli/src/daemon.ts

## Architecture

```
senseid (single Rust binary)
├── HTTP server (axum)           — all /api/* endpoints
├── Tree-sitter parsers          — per-language AST extraction
├── Kuzu (single instance)       — one DB for ALL repos, partitioned by project
├── SQLite (rusqlite)            — activity log, job queue, lib docs
├── File watcher (notify)        — debounced, lazy connect
├── Manifest manager             — incremental indexing
└── CLI subcommands              — start/stop/status/logs/clear-logs
```

### Key Design Decisions

1. **Single Kuzu database** for all repos — partitioned by `project` field on every node/edge. No more 72 separate DB instances.
2. **Tree-sitter native** — grammars compiled into the binary. No WASM, no Node native modules.
3. **Single binary** — no runtime deps, no `bun`, no `node_modules`.
4. **Same HTTP API** — desktop app and MCP server work unchanged.
5. **Same file layout** — `~/.sensei/` structure preserved for migration.

---

## CLI Commands

```
senseid start [--port <n>]    Start as background daemon (default 7744)
senseid stop                  Stop the running daemon
senseid status                Show daemon status + project count
senseid logs                  Tail ~/.sensei/senseid.log
senseid clear-logs            Truncate log file
senseid                       Start in foreground (dev mode)
```

**PID file:** `~/.sensei/serve.pid`
**Log file:** `~/.sensei/senseid.log`

---

## HTTP API Endpoints

### Health & Lifecycle

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Daemon status, version, queue, progress map |
| POST | `/stop` | Graceful shutdown |
| OPTIONS | `*` | CORS preflight |

**GET /health response:**
```json
{
  "ok": true,
  "name": "senseid",
  "version": "1.0.0",
  "backend": "ollama",
  "ollamaRunning": true,
  "indexing": ["repo-id-1"],
  "queue": [{"repoId": "...", "status": "running", "attempts": 1}],
  "progress": {
    "repo-id-1": {
      "currentFile": "src/main.rs",
      "filesProcessed": 42,
      "filesTotal": 150,
      "filesUnchanged": 10,
      "startedAt": "2026-04-12T..."
    }
  }
}
```

### Project Management

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/projects` | List all registered projects |
| POST | `/api/projects` | Register a new project |
| PUT | `/api/projects/:repoId` | Update project metadata (name, tags) |
| DELETE | `/api/projects/:repoId` | Unregister a project |

**GET /api/projects response:**
```json
[{
  "repoId": "kavach",
  "name": "kavach",
  "path": "/Users/dev/kavach",
  "indexedAt": "2026-04-12T...",
  "lastError": null,
  "partiallyIndexed": false,
  "libs": ["svelte", "zod", "hono"]
}]
```

**POST /api/projects body:**
```json
{
  "repoId": "kavach",
  "name": "kavach",
  "path": "/Users/dev/kavach"
}
```

### Solution Management (NEW)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/solutions` | List all solutions |
| POST | `/api/solutions` | Create a solution |
| PUT | `/api/solutions/:id` | Update solution (name, category, client) |
| DELETE | `/api/solutions/:id` | Delete a solution |
| POST | `/api/solutions/:id/repos` | Add a repo to a solution |
| DELETE | `/api/solutions/:id/repos/:repoId` | Remove a repo from a solution |
| PUT | `/api/solutions/:id/repos/:repoId` | Update repo role/label within solution |
| POST | `/api/solutions/detect` | Auto-detect solutions from registered projects |

**POST /api/solutions body:**
```json
{
  "name": "Acme Platform",
  "category": "active",
  "client": "Acme Corp",
  "repos": [
    { "repoId": "acme-api", "role": "backend", "label": "API" },
    { "repoId": "acme-ui", "role": "frontend", "label": "Dashboard" }
  ]
}
```

**POST /api/solutions/detect** — auto-groups by:
- Monorepo workspace detection
- Name-prefix clustering (acme-api, acme-ui → "Acme")
- GitHub org grouping
- Cross-repo import analysis

### Indexing

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/index` | Queue a repo for indexing |
| POST | `/api/reindex-all` | Force re-index all repos |
| GET | `/api/index/status` | Current indexing queue and progress |

**POST /api/index body:**
```json
{
  "repoId": "kavach",
  "repoPath": "/Users/dev/kavach",
  "force": false
}
```

### Graph Data

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/graph` | Aggregated graph stats (communities, god nodes, rationale) |
| GET | `/api/graph/nodes` | Raw nodes + edges for D3 visualization |
| GET | `/api/graph/symbol` | Get a single symbol by name (L0-L5 depth) |
| GET | `/api/graph/search` | Search symbols by name |
| GET | `/api/graph/callers` | Get callers of a function |
| GET | `/api/graph/callees` | Get callees of a function |

**GET /api/graph?repoId=X response:**
```json
{
  "summary": { "totalSymbols": 342, "totalEdges": 520, "communities": 8 },
  "communities": [{ "id": "src/routes", "label": "Routes", "symbolCount": 45, "godNodes": ["Router"] }],
  "godNodes": [{ "name": "Router", "degree": 23, "file": "src/routes.ts", "community": "src/routes" }],
  "rationale": [{ "tag": "WHY", "text": "...", "file": "src/auth.ts" }]
}
```

**GET /api/graph/nodes?repoId=X response:**
```json
{
  "nodes": [
    { "id": "fn:src/auth.ts:validate:42", "name": "validate", "kind": "function", "file": "src/auth.ts", "line": 42, "complexity": 5 }
  ],
  "edges": [
    { "source": "fn:...", "target": "fn:...", "type": "CALLS" }
  ]
}
```

### Sessions & Activity

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/sessions` | List sessions with stats |
| POST | `/api/sessions` | Create/update a session |
| GET | `/api/sessions/:id` | Get session detail |

### Traceability

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/trace` | Doc → code → test traceability chain |
| GET | `/api/drift` | Code-doc drift detection |

### Library Docs

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/libraries` | List all indexed libraries with metadata |
| GET | `/api/lib-docs` | Get docs for a specific library |
| POST | `/api/lib-docs` | Index/update docs for a library |

### Solution Context (for MCP)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/solution-context` | Get solution context for a repo (MCP reads this) |
| POST | `/api/solution-context` | Push solution definitions (desktop writes this) |

### Tags & Metadata (NEW)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/tags` | List all tags |
| POST | `/api/projects/:repoId/tags` | Add tags to a project |
| DELETE | `/api/projects/:repoId/tags/:tag` | Remove a tag |
| POST | `/api/solutions/:id/tags` | Add tags to a solution |

---

## Database Schema

### Kuzu (Single Instance)

All repos in ONE database at `~/.sensei/graph.kuzu`. Every node has a `project` field for partitioning.

```sql
CREATE NODE TABLE Function(
  id STRING, name STRING, file STRING, line INT64,
  sig STRING, body STRING, docstring STRING,
  complexity INT64 DEFAULT 1, project STRING,
  PRIMARY KEY(id)
)

CREATE NODE TABLE File(
  id STRING, path STRING, module STRING, lang STRING, project STRING,
  PRIMARY KEY(id)
)

CREATE NODE TABLE Type(
  id STRING, name STRING, file STRING, line INT64, kind STRING, project STRING,
  PRIMARY KEY(id)
)

CREATE NODE TABLE Comment(
  id STRING, text STRING, tag STRING, line INT64, file STRING, project STRING,
  PRIMARY KEY(id)
)

CREATE NODE TABLE Doc(
  id STRING, path STRING, title STRING, project STRING,
  PRIMARY KEY(id)
)

-- Relationships
CREATE REL TABLE CALLS(FROM Function TO Function, weight DOUBLE)
CREATE REL TABLE IMPORTS(FROM File TO File)
CREATE REL TABLE EXPORTS_FN(FROM File TO Function)
CREATE REL TABLE EXPORTS_TYPE(FROM File TO Type)
CREATE REL TABLE ANNOTATES_FN(FROM Comment TO Function)
CREATE REL TABLE ANNOTATES_TYPE(FROM Comment TO Type)
CREATE REL TABLE COVERS(FROM Doc TO File)
CREATE REL TABLE MENTIONS_FN(FROM Doc TO Function)
CREATE REL TABLE SUPERSEDES(FROM Doc TO Doc)
```

### SQLite (Single Instance)

At `~/.sensei/sensei.db` — replaces per-repo activity.db files.

```sql
-- Projects registry (replaces projects.json)
CREATE TABLE projects(
  repo_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  indexed_at TEXT,
  last_error TEXT,
  libs TEXT,  -- JSON array of lib names
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Solutions (replaces localStorage)
CREATE TABLE solutions(
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  client TEXT,
  category TEXT NOT NULL DEFAULT 'active',
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE solution_repos(
  solution_id TEXT NOT NULL REFERENCES solutions(id) ON DELETE CASCADE,
  repo_id TEXT NOT NULL REFERENCES projects(repo_id),
  role TEXT NOT NULL DEFAULT 'unknown',
  label TEXT,
  PRIMARY KEY(solution_id, repo_id)
);

-- Tags
CREATE TABLE tags(
  entity_type TEXT NOT NULL,  -- 'project' or 'solution'
  entity_id TEXT NOT NULL,
  tag TEXT NOT NULL,
  PRIMARY KEY(entity_type, entity_id, tag)
);

-- Sessions
CREATE TABLE sessions(
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL,
  task TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  outcome TEXT,
  summary TEXT,
  cost REAL,
  tokens_in INTEGER,
  tokens_out INTEGER
);

-- Activity log (actions, decisions, backlog, snapshots, etc.)
-- Same tables as current activity-log.ts
CREATE TABLE decisions(id TEXT PRIMARY KEY, repo_id TEXT, text TEXT, context TEXT, timestamp TEXT, tags TEXT);
CREATE TABLE backlog(id TEXT PRIMARY KEY, repo_id TEXT, title TEXT, description TEXT, status TEXT, priority TEXT, created_at TEXT, updated_at TEXT);
CREATE TABLE snapshots(id TEXT PRIMARY KEY, session_id TEXT, repo_id TEXT, progress_summary TEXT, next_step_hint TEXT, in_flight_files TEXT, completed_steps TEXT, diff_stat_summary TEXT, created_at TEXT);

-- Index queue (replaces SQLite queue.db)
CREATE TABLE index_jobs(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repo_id TEXT NOT NULL UNIQUE,
  repo_path TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  attempts INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  error TEXT
);

-- Shared library docs (replaces ~/.sensei/libraries/*/docs.db)
CREATE TABLE lib_docs(
  id TEXT PRIMARY KEY,
  lib_name TEXT NOT NULL,
  title TEXT NOT NULL,
  url TEXT,
  local_path TEXT,
  summary TEXT NOT NULL DEFAULT '',
  content TEXT,
  source_type TEXT NOT NULL,
  component TEXT,
  indexed_at TEXT NOT NULL
);
CREATE INDEX lib_docs_lib ON lib_docs(lib_name);

CREATE TABLE lib_meta(
  name TEXT PRIMARY KEY,
  source_type TEXT NOT NULL,
  base_url TEXT,
  used_by TEXT,  -- JSON array of repo_ids
  indexed_at TEXT
);
```

---

## Tree-sitter Language Grammars

Compiled into the binary (no runtime loading):

| Language | Crate | Extensions |
|----------|-------|-----------|
| Rust | `tree-sitter-rust` | .rs |
| Python | `tree-sitter-python` | .py |
| TypeScript | `tree-sitter-typescript` | .ts, .tsx |
| JavaScript | `tree-sitter-javascript` | .js, .jsx, .mjs, .cjs |
| Java | `tree-sitter-java` | .java |
| Kotlin | `tree-sitter-kotlin` | .kt, .kts |
| Swift | `tree-sitter-swift` | .swift |
| Svelte | `tree-sitter-svelte` | .svelte (extract script, parse as TS) |
| SQL | regex fallback | .sql |

### Extraction Pattern (per language)

Each language implements a `LanguageAdapter` trait:

```rust
trait LanguageAdapter {
    fn language(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn parse(&self, source: &str, path: &str) -> ParsedFile;
}

struct ParsedFile {
    file_path: String,
    language: String,
    symbols: Vec<ParsedSymbol>,
    edges: Vec<ParsedEdge>,
    imports: Vec<ParsedImport>,
}

struct ParsedSymbol {
    name: String,
    kind: SymbolKind,       // Function, Class, Type, Interface, Enum, Const, Method, Component
    signature: Option<String>,
    docstring: Option<String>,
    line_start: u32,
    line_end: u32,
    is_exported: bool,
}
```

---

## Indexing Pipeline

### 5-Pass Architecture

```
Pass 0: Schema initialization (CREATE NODE TABLE IF NOT EXISTS ...)
Pass 1: File parsing + node creation (incremental via manifest)
  For each file:
    1. Check manifest (mtime + sha256 hash) — skip if unchanged
    2. Parse with tree-sitter → ParsedFile
    3. MERGE Function/Type/File/Comment nodes into Kuzu
    4. Update manifest entry
    5. Write progress.json
Pass 2: IMPORTS edges (resolve relative paths → file IDs)
Pass 3: CALLS edges (resolve callee names → function IDs)
Pass 4: Doc indexing (.md/.mdx → Doc nodes + COVERS/MENTIONS_FN/SUPERSEDES edges)
Pass 5: External lib detection (collect non-workspace imports, group by scope)
```

### Manifest Format

`~/.sensei/projects/{repoId}/manifest.json`:
```json
{
  "/abs/path/to/file.ts": { "mtime": 1712934000, "hash": "a1b2c3d4e5f67890" }
}
```

### Node ID Formats

```
fn:{absPath}:{name}:{lineStart}     — Function
type:{absPath}:{name}:{lineStart}   — Type
file:{absPath}                      — File
comment:{absPath}:{tag}:{line}      — Comment
doc:{absPath}                       — Doc
```

---

## File Watcher

Using `notify` crate with debounce:

```rust
struct RepoWatcher {
    repo_id: String,
    repo_path: PathBuf,
    include: Vec<GlobPattern>,
    exclude: Vec<GlobPattern>,
    debounce_ms: u64,           // default 300
    // NO permanent DB connection — opens lazily per batch
}
```

On file change:
1. Accumulate changes for `debounce_ms`
2. Filter by include/exclude patterns
3. Open Kuzu connection
4. For deleted files → remove from graph
5. For new/changed files → re-parse and re-index
6. Close Kuzu connection
7. Update manifest

---

## Crates

| Crate | Purpose |
|-------|---------|
| `axum` | HTTP server |
| `tokio` | Async runtime |
| `kuzu` | Graph database (single instance) |
| `rusqlite` | SQLite for activity/queue/libs |
| `tree-sitter` + grammar crates | AST parsing |
| `notify` | File system watcher |
| `serde` / `serde_json` | JSON serialization |
| `clap` | CLI argument parsing |
| `sha2` | File hashing |
| `glob` / `globset` | Pattern matching |
| `fast-glob` or `walkdir` | File discovery |
| `tracing` | Structured logging |
| `yaml-rust2` | YAML parsing (config, frontmatter) |
| `pulldown-cmark` | Markdown parsing (doc indexer) |

---

## Migration Path

1. Build `senseid-rs` alongside existing `senseid`
2. Run both in parallel — verify API parity
3. Migration script: move per-repo Kuzu DBs → single shared DB
4. Feature flag: `SENSEI_DAEMON=rust` to switch
5. Default to Rust when parity confirmed
6. Remove TypeScript daemon

### Data Migration

```
~/.sensei/projects/*/graph.kuzu (72 separate DBs)
  ↓ migrate
~/.sensei/graph.kuzu (single DB, project field distinguishes repos)

~/.sensei/projects/*/activity.db (72 separate DBs)
  ↓ migrate
~/.sensei/sensei.db (single SQLite, repo_id field distinguishes repos)

~/.sensei/libraries/*/docs.db (per-lib DBs)
  ↓ migrate
~/.sensei/sensei.db lib_docs table (single table, lib_name field)
```

---

## Implementation Phases

### Phase 1: Core daemon + indexer (2-3 weeks)
- CLI: start/stop/status/logs
- HTTP: /health, /api/projects, /api/index
- Single Kuzu DB with schema
- Tree-sitter parsers for TypeScript + Python
- 5-pass indexing pipeline
- Manifest-based incremental indexing
- Progress reporting

### Phase 2: Query layer + remaining adapters (1-2 weeks)
- HTTP: /api/graph, /api/graph/nodes, /api/graph/symbol, /api/graph/search
- All tree-sitter adapters (Rust, Java, Kotlin, Swift)
- Doc indexer (COVERS, MENTIONS_FN)
- Drift detection

### Phase 3: Activity log + solutions (1 week)
- SQLite activity log (sessions, decisions, backlog)
- HTTP: /api/sessions, /api/solutions, /api/solution-context
- Solution auto-detection
- Tags

### Phase 4: File watcher + lib detection (1 week)
- notify-based watcher with debounce
- Lazy Kuzu connections (open per batch, close after)
- External lib detection from imports
- Shared lib docs

### Phase 5: Distribution (1 week)
- Cross-compile: macOS arm64/x86, Linux x86_64
- `brew install sensei`
- Migration script from TypeScript daemon data
- Feature flag for daemon selection

---

## Build & Test

```bash
# Build
cargo build --release

# Run
./target/release/senseid start

# Test against TypeScript daemon
# Start both, compare API responses
diff <(curl -s localhost:7744/api/projects) <(curl -s localhost:7745/api/projects)
```

### Benchmark Corpus Validation

Run `sensei benchmark indexer --all` against both daemons:
- Same symbol counts?
- Same edge counts?
- Same god nodes?

The benchmark spec (`sensei-benchmark.yaml`) is the parity contract.
