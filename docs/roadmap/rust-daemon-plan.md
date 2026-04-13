# Rust Daemon Implementation Plan

> Status: Planning | Resolves: #47
> Replaces: packages/server + packages/graph-indexer + packages/cli/src/daemon.ts

## Architecture

```
senseid (single Rust binary)
├── HTTP server (axum)           — all /api/* endpoints
├── Tree-sitter parsers          — per-language AST extraction
├── Config file parsers          — package.json, Cargo.toml, pyproject.toml, etc.
├── Kuzu (single instance)       — one DB for ALL repos, partitioned by project
├── SQLite (rusqlite)            — activity log, job queue, lib docs, solutions
├── File watcher (notify)        — debounced, lazy connect
├── Manifest manager             — incremental indexing
└── CLI subcommands              — start/stop/status/logs/clear-logs
```

### Key Design Decisions

1. **Single Kuzu database** for all repos — partitioned by `project` field on every node/edge. No more separate DB instances.
2. **Single SQLite database** — `~/.sensei/sensei.db` for everything: projects, solutions, sessions, queue, lib docs.
3. **Tree-sitter native** — grammars compiled into the binary. No WASM, no Node native modules.
4. **Single binary** — no runtime deps, no `bun`, no `node_modules`.
5. **Same HTTP API** — desktop app and MCP server work unchanged.
6. **Config files as first-class data** — package.json, Cargo.toml, pyproject.toml parsed for deps, scripts, workspace info.
7. **SQL is SQL** — Rust has `sqlparser` crate which handles PostgreSQL, MySQL, SQLite, and ANSI SQL dialects. Auto-detect from file content or project context.

---

## CLI Commands

```
senseid start [--port <n>]    Start as background daemon (default 7744)
senseid stop                  Stop the running daemon
senseid status                Show daemon status + project/solution counts
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

### Project Management

Projects = repos. Each registered repo is a project.

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/projects` | List all registered projects |
| POST | `/api/projects` | Register a new project |
| PUT | `/api/projects/:repoId` | Update project (name, tags) |
| DELETE | `/api/projects/:repoId` | Unregister a project (removes from graph too) |
| POST | `/api/projects/:repoId/tags` | Add tags to a project |
| DELETE | `/api/projects/:repoId/tags/:tag` | Remove a tag |

**GET /api/projects response:**
```json
[{
  "repoId": "kavach",
  "name": "kavach",
  "path": "/Users/dev/kavach",
  "indexedAt": "2026-04-12T...",
  "lastError": null,
  "partiallyIndexed": false,
  "libs": ["svelte", "zod", "hono"],
  "tags": ["typescript", "auth"],
  "duplicateOf": null,
  "stack": ["sveltekit", "typescript"],
  "status": "active"
}]
```

**Duplicate detection:** on project registration, compare:
- Git remote URL (normalized) — exact match = duplicate
- Name + structure similarity — fuzzy match = potential duplicate (flagged, not auto-merged)
- `duplicateOf` field links to the canonical repo if detected

### Solution Management

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/solutions` | List all solutions |
| POST | `/api/solutions` | Create a solution |
| PUT | `/api/solutions/:id` | Update solution (name, category, client, description) |
| DELETE | `/api/solutions/:id` | Delete a solution |
| POST | `/api/solutions/:id/repos` | Add repo(s) to a solution |
| DELETE | `/api/solutions/:id/repos/:repoId` | Remove a repo from a solution |
| PUT | `/api/solutions/:id/repos/:repoId` | Update repo role/label within solution |
| POST | `/api/solutions/:id/tags` | Add tags to a solution |
| DELETE | `/api/solutions/:id/tags/:tag` | Remove a tag |
| POST | `/api/solutions/detect` | Auto-detect solutions from registered projects |
| POST | `/api/solutions/merge-duplicates` | Combine duplicate repos into a solution tagged "duplicates" |

**POST /api/solutions body:**
```json
{
  "name": "Acme Platform",
  "category": "active",
  "client": "Acme Corp",
  "description": "Main product platform",
  "tags": ["production", "priority"],
  "repos": [
    { "repoId": "acme-api", "role": "backend", "label": "API" },
    { "repoId": "acme-ui", "role": "frontend", "label": "Dashboard" }
  ]
}
```

All fields except `name` and `repos` are optional.

**POST /api/solutions/detect** — auto-groups by:
- Monorepo workspace detection (package.json workspaces, Cargo.toml [workspace])
- Name-prefix clustering (acme-api, acme-ui → "Acme")
- GitHub org grouping (same org in remote URL)
- Cross-repo import analysis (shared library references)
- Duplicate detection (same remote URL → merge suggestion)

**POST /api/solutions/merge-duplicates** — finds repos with same git remote, creates a solution tagged `["duplicates"]` containing all copies. User can then choose which to keep.

### Indexing & Progress

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/index` | Queue a single project for indexing |
| POST | `/api/index/solution/:id` | Queue all repos in a solution |
| POST | `/api/index/all` | Queue ALL unindexed projects |
| POST | `/api/reindex` | Force re-index a project (resets manifest) |
| POST | `/api/reindex/all` | Force re-index everything |
| GET | `/api/index/status` | Current queue + per-project progress |
| GET | `/api/index/errors` | All indexing errors across all projects |
| GET | `/api/index/errors/:repoId` | Errors for a specific project |

**POST /api/index body:**
```json
{
  "repoId": "kavach",
  "repoPath": "/Users/dev/kavach",
  "force": false
}
```

**GET /api/index/status response:**
```json
{
  "queue": [
    { "repoId": "kavach", "status": "running", "attempts": 1 },
    { "repoId": "rokkit", "status": "pending", "attempts": 0 }
  ],
  "progress": {
    "kavach": {
      "repoId": "kavach",
      "currentFile": "src/routes/auth.ts",
      "filesProcessed": 42,
      "filesTotal": 150,
      "filesUnchanged": 80,
      "filesSkipped": 3,
      "filesFailed": 1,
      "startedAt": "2026-04-12T..."
    }
  },
  "summary": {
    "total": 72,
    "indexed": 65,
    "running": 2,
    "pending": 3,
    "failed": 2
  }
}
```

**GET /api/index/errors response:**
```json
[{
  "repoId": "kavach",
  "file": "src/legacy/old.js",
  "error": "SyntaxError: Unexpected token at line 42",
  "timestamp": "2026-04-12T...",
  "adapter": "javascript"
}, {
  "repoId": "rokkit",
  "file": "src/broken.svelte",
  "error": "No script block found",
  "timestamp": "2026-04-12T...",
  "adapter": "svelte"
}]
```

Errors persist across restarts. Each re-index clears errors for that project. The desktop Indexer page reads these to show per-file failures with drill-down.

### Graph Data

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/graph` | Aggregated stats (communities, god nodes, rationale) |
| GET | `/api/graph/nodes` | Raw nodes + edges for D3 visualization |
| GET | `/api/graph/symbol` | Get a single symbol by name (L0-L5 depth) |
| GET | `/api/graph/search` | Search symbols by name |
| GET | `/api/graph/callers/:name` | Get callers of a function |
| GET | `/api/graph/callees/:name` | Get callees of a function |

All graph endpoints accept `?repoId=X` or `?solutionId=X` to scope the query.

### Sessions & Activity

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/sessions` | List sessions (filterable by repoId, solutionId) |
| POST | `/api/sessions` | Create/update a session |
| GET | `/api/sessions/:id` | Session detail |
| GET | `/api/decisions` | Recent decisions across projects |
| GET | `/api/backlog` | Open backlog items |

### Traceability & Drift

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
| GET | `/api/solution-context` | Get solution context for a repo |
| POST | `/api/solution-context` | Push solution definitions |

---

## Config File Parsing

Beyond source code, these files contain critical project metadata:

| File | What we extract | Stored as |
|------|----------------|-----------|
| `package.json` | name, version, dependencies, devDependencies, scripts, workspaces, type, main/exports | Project metadata + lib detection |
| `Cargo.toml` | name, version, dependencies, [workspace] members, edition | Project metadata + lib detection |
| `pyproject.toml` | name, version, dependencies, [tool.poetry] deps | Project metadata + lib detection |
| `requirements.txt` | dependency list | Lib detection |
| `go.mod` | module name, require directives | Project metadata + lib detection |
| `pom.xml` | groupId, artifactId, dependencies | Project metadata + lib detection |
| `build.gradle` / `build.gradle.kts` | dependencies, plugins | Lib detection |
| `Gemfile` | gem dependencies | Lib detection |
| `docker-compose.yml` | services, networks, ports, depends_on | Deployment diagram |
| `Dockerfile` | FROM base, EXPOSE ports | Deployment info |
| `k8s/*.yaml` | Deployment, Service, Ingress specs | Deployment diagram |
| `.env.example` | Environment variable names | Config surface |
| `tsconfig.json` | paths, references, compilerOptions | Project structure |
| `Makefile` / `Taskfile.yml` | Available tasks/commands | Developer workflow |

These are stored as structured metadata in the `project_configs` SQLite table:

```sql
CREATE TABLE project_configs(
  repo_id TEXT NOT NULL,
  file_path TEXT NOT NULL,        -- relative path (package.json, Cargo.toml, etc.)
  file_type TEXT NOT NULL,        -- 'package.json', 'cargo.toml', 'dockerfile', etc.
  parsed_data TEXT NOT NULL,      -- JSON of extracted fields
  updated_at TEXT NOT NULL,
  PRIMARY KEY(repo_id, file_path)
);
```

This powers:
- **Lib detection** without reading source imports (faster, more complete)
- **Stack detection** (what frameworks/languages)
- **Deployment diagram derivation** (from docker-compose + k8s)
- **Workspace detection** (monorepo structure)
- **Script/task discovery** (what commands are available)

### SQL Dialect Detection

For `.sql` files, use the `sqlparser-rs` crate which supports:
- PostgreSQL
- MySQL
- SQLite
- ANSI SQL
- MS SQL
- BigQuery

Auto-detect dialect from:
1. File content heuristics (e.g., `SERIAL` → PostgreSQL, `AUTO_INCREMENT` → MySQL, `AUTOINCREMENT` → SQLite)
2. Project context (if `docker-compose.yml` has a `postgres` service → PostgreSQL)
3. Config files (`dbd` config, `.env` with `DATABASE_URL`)
4. Default: ANSI SQL

---

## Database Schema

### Kuzu (Single Instance at `~/.sensei/graph.kuzu`)

All repos in ONE database. Every node has a `project` field.

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
  id STRING, path STRING, title STRING, doc_type STRING, project STRING,
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

### SQLite (Single Instance at `~/.sensei/sensei.db`)

```sql
-- ── Projects (repos) ────────────────────────────────────────────────────────

CREATE TABLE projects(
  repo_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  remote_url TEXT,                    -- git remote origin
  indexed_at TEXT,
  last_error TEXT,
  duplicate_of TEXT,                  -- repo_id of canonical if detected as dup
  stack TEXT,                         -- JSON ["typescript", "sveltekit"]
  libs TEXT,                          -- JSON ["svelte", "zod"]
  status TEXT DEFAULT 'active',       -- active/recent/stale/archived/abandoned
  last_commit_days INTEGER,
  commit_count INTEGER DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ── Solutions ───────────────────────────────────────────────────────────────

CREATE TABLE solutions(
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  client TEXT,                        -- optional
  category TEXT NOT NULL DEFAULT 'active',  -- active/side/idea
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE solution_repos(
  solution_id TEXT NOT NULL REFERENCES solutions(id) ON DELETE CASCADE,
  repo_id TEXT NOT NULL REFERENCES projects(repo_id),
  role TEXT NOT NULL DEFAULT 'unknown',  -- backend/frontend/mobile/infra/docs/library/shared
  label TEXT,
  PRIMARY KEY(solution_id, repo_id)
);

-- ── Tags (for both projects and solutions) ──────────────────────────────────

CREATE TABLE tags(
  entity_type TEXT NOT NULL,          -- 'project' or 'solution'
  entity_id TEXT NOT NULL,
  tag TEXT NOT NULL,
  PRIMARY KEY(entity_type, entity_id, tag)
);

-- ── Config files ────────────────────────────────────────────────────────────

CREATE TABLE project_configs(
  repo_id TEXT NOT NULL REFERENCES projects(repo_id) ON DELETE CASCADE,
  file_path TEXT NOT NULL,
  file_type TEXT NOT NULL,
  parsed_data TEXT NOT NULL,          -- JSON
  updated_at TEXT NOT NULL,
  PRIMARY KEY(repo_id, file_path)
);

-- ── Index queue ─────────────────────────────────────────────────────────────

CREATE TABLE index_jobs(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repo_id TEXT NOT NULL UNIQUE,
  repo_path TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',  -- pending/running/done/failed
  attempts INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  error TEXT
);

-- ── Index errors (persist across restarts) ──────────────────────────────────

CREATE TABLE index_errors(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repo_id TEXT NOT NULL,
  file_path TEXT NOT NULL,
  error TEXT NOT NULL,
  adapter TEXT,                       -- which language adapter failed
  timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_errors_repo ON index_errors(repo_id);

-- ── Sessions & Activity ─────────────────────────────────────────────────────

CREATE TABLE sessions(
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL,
  task TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  outcome TEXT,                       -- completed/partial/blocked
  summary TEXT,
  cost REAL,
  tokens_in INTEGER,
  tokens_out INTEGER
);

CREATE TABLE decisions(
  id TEXT PRIMARY KEY, repo_id TEXT, text TEXT, context TEXT,
  timestamp TEXT, tags TEXT
);

CREATE TABLE backlog(
  id TEXT PRIMARY KEY, repo_id TEXT, title TEXT, description TEXT,
  status TEXT DEFAULT 'open', priority TEXT DEFAULT 'medium',
  created_at TEXT, updated_at TEXT
);

CREATE TABLE snapshots(
  id TEXT PRIMARY KEY, session_id TEXT, repo_id TEXT,
  progress_summary TEXT, next_step_hint TEXT, in_flight_files TEXT,
  completed_steps TEXT, diff_stat_summary TEXT, created_at TEXT
);

-- ── Library docs (shared) ───────────────────────────────────────────────────

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
  used_by TEXT,                       -- JSON array of repo_ids
  indexed_at TEXT
);
```

---

## Tree-sitter Language Grammars

Compiled into the binary:

| Language | Crate | Extensions |
|----------|-------|-----------|
| Rust | `tree-sitter-rust` | .rs |
| Python | `tree-sitter-python` | .py |
| TypeScript | `tree-sitter-typescript` | .ts, .tsx |
| JavaScript | `tree-sitter-javascript` | .js, .jsx, .mjs, .cjs |
| Java | `tree-sitter-java` | .java |
| Kotlin | `tree-sitter-kotlin` | .kt, .kts |
| Swift | `tree-sitter-swift` | .swift |
| SQL | `sqlparser-rs` | .sql (multi-dialect) |
| Svelte | extract `<script>`, parse as TS | .svelte |

### Config file parsers (not tree-sitter — structured format parsers)

| Format | Crate | Files |
|--------|-------|-------|
| JSON | `serde_json` | package.json, tsconfig.json, composer.json |
| TOML | `toml` | Cargo.toml, pyproject.toml |
| YAML | `serde_yaml` | docker-compose.yml, k8s manifests, .github/workflows |
| XML | `quick-xml` | pom.xml, build.xml |
| Plain text | regex | requirements.txt, Gemfile, .env.example, Makefile |

---

## Indexing Pipeline

### 6-Pass Architecture

```
Pass 0: Schema init (Kuzu tables + SQLite tables)
Pass 1: Config file parsing (package.json, Cargo.toml, etc. → project_configs)
Pass 2: Source file parsing + node creation (incremental via manifest)
  For each file:
    1. Check manifest (mtime + sha256) — skip if unchanged
    2. Select adapter by extension
    3. Parse with tree-sitter → ParsedFile
    4. MERGE Function/Type/File/Comment nodes into Kuzu
    5. Update manifest
    6. Write progress
    7. On error: log to index_errors table, continue
Pass 3: IMPORTS edges (resolve relative paths → file IDs)
Pass 4: CALLS edges (resolve callee names → function IDs across all files)
Pass 5: Doc indexing (.md/.mdx → Doc nodes + COVERS/MENTIONS_FN/SUPERSEDES)
Pass 6: Lib detection (collect external imports, cross-ref with config deps)
```

### Error Handling

Every error is:
1. Logged to `index_errors` SQLite table (persists across restarts)
2. Does NOT stop the indexing of other files
3. The failed file stays out of the manifest (will retry on next index)
4. Accessible via `GET /api/index/errors` and `GET /api/index/errors/:repoId`
5. Cleared when the project is successfully re-indexed

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
    // NO permanent DB connection — opens Kuzu lazily per batch
}
```

**Key difference from TypeScript:** no persistent Kuzu connection. Opens on file change event, processes batch, closes. This allows 72+ watchers without resource exhaustion.

---

## Crates

| Crate | Purpose |
|-------|---------|
| `axum` | HTTP server |
| `tokio` | Async runtime |
| `kuzu` | Graph database (single instance) |
| `rusqlite` | SQLite (single instance) |
| `tree-sitter` + grammar crates | AST parsing |
| `sqlparser` | SQL dialect parsing |
| `notify` | File system watcher |
| `serde` / `serde_json` | JSON |
| `toml` | TOML parsing |
| `serde_yaml` | YAML parsing |
| `quick-xml` | XML parsing (pom.xml) |
| `clap` | CLI |
| `sha2` | File hashing |
| `globset` | Pattern matching |
| `walkdir` | File discovery |
| `tracing` | Structured logging |
| `pulldown-cmark` | Markdown parsing |

---

## Migration Path

1. Build `senseid-rs` alongside existing `senseid`
2. Migration script:
   - Merge 72 per-repo Kuzu DBs → single `~/.sensei/graph.kuzu`
   - Merge per-repo `activity.db` + `projects.json` + `queue.db` → `~/.sensei/sensei.db`
   - Merge per-lib `docs.db` → `sensei.db` lib_docs table
3. Feature flag: `SENSEI_DAEMON=rust`
4. Validate API parity via benchmark corpus
5. Default to Rust
6. Remove TypeScript daemon

---

## Implementation Phases

### Phase 1: Core daemon + indexer (2-3 weeks)
- CLI: start/stop/status/logs
- HTTP: /health, /api/projects, /api/index, /api/index/status, /api/index/errors
- Single Kuzu DB with schema
- Single SQLite with projects + queue + errors tables
- Tree-sitter: TypeScript + Python adapters
- Config parsing: package.json, Cargo.toml
- 6-pass indexing pipeline
- Manifest-based incremental indexing
- Progress reporting + error logging

### Phase 2: Query layer + all adapters (1-2 weeks)
- HTTP: /api/graph/*, /api/trace, /api/drift
- All tree-sitter adapters (Rust, Java, Kotlin, Swift, JS, Svelte)
- SQL: sqlparser-rs with dialect detection
- Doc indexer
- Config parsers: pyproject.toml, docker-compose.yml, k8s yaml

### Phase 3: Solutions + activity (1-2 weeks)
- HTTP: /api/solutions/*, /api/sessions, /api/tags
- Solution CRUD + auto-detection + duplicate merging
- SQLite: solutions, solution_repos, tags tables
- Activity log (sessions, decisions, backlog)
- Solution context for MCP

### Phase 4: File watcher + libs (1 week)
- notify-based watcher with lazy Kuzu connections
- Lib detection from imports + config files
- Shared lib docs
- HTTP: /api/libraries, /api/lib-docs

### Phase 5: Distribution (1 week)
- Cross-compile: macOS arm64/x86, Linux x86_64
- `brew install sensei`
- Migration script
- Benchmark validation against TypeScript daemon
