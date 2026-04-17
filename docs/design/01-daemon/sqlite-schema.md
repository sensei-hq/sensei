# Database Schema — SQLite Local-First

> DDL files: `database/ddl/sqlite/`
> Design config: `database/design.yaml` (sqlite section)

---

## SQLite vs Local Postgres

**SQLite is the default.** For sensei's workload — personal developer tool, repos up to
~200K symbols, single writer (daemon) with concurrent readers (desktop app) — SQLite in
WAL mode is fast enough across all query patterns:

| Query | Postgres approach | SQLite equivalent | Notes |
|---|---|---|---|
| Keyword search | `rank_bm25` (LIKE-based) | FTS5 virtual table | FTS5 is *better* — real inverted index vs LIKE |
| BFS traversal | `rank_bfs` (recursive CTE) | Recursive CTE | SQLite has supported this since 3.8.3 |
| Vector similarity | pgvector `<=>`, HNSW index | sqlite-vec F32_BLOB | Adequate for personal scale; HNSW not available |
| Symbol lookup | B-tree index on (repo, file, name) | Same | No difference |
| Session queries | Standard SQL | Same | No difference |

**When to prefer local Postgres:** Repos with >500K symbols, or multi-user team scenarios
where a shared local Postgres makes sense. Supported via the `DatabaseAdapter` interface
in `packages/shared` — switch is a config change, not a code change.

### sqlite-vec is optional

If sqlite-vec is not installed, vector similarity ranking degrades gracefully to FTS5 only.
The `chunks.embedding` column is left null and the application skips the vector step.
Functional, just slightly less precise for semantic queries.

---

## Two Databases

### `~/.sensei/sensei.db` — Global

One file per developer machine. Shared across all projects.

```
projects             — unified registry of repos and ideas (kind discriminator)
phases               — optional phase containers per project
cards                — atomic units of thinking within a phase
card_links           — typed graph edges between cards, symbols, files, sessions
sessions             — one row per MCP server process (agent session)
task_sessions        — one row per task (get_session_context → checkpoint)
task_turns           — one row per tool call within a task session
api_requests         — OTLP token/cost data (Claude Code; null for others)
snapshots            — point-in-time agent state for interruption recovery
memory_items         — cross-session knowledge (decisions, patterns, questions)
libraries            — known third-party and internal libraries
project_libraries    — join: which libraries does each project use
lib_doc_sections     — documentation chunks for indexed libraries
coordinator_installs — which coordinators are configured per project
settings             — key-value config (global and per-project overrides)
```

### `<repo>/.sensei/index.db` — Per-Repo

One file per indexed repository. Self-contained — moving the repo folder moves its index.

```
symbols           — code symbols (L0-L3 resolution, community, degree)
call_edges        — function call graph (confidence-tagged)
imports           — import graph (source_file → target_path)
chunks            — vector embeddings for semantic search (sqlite-vec or FTS5 fallback)
scan_state        — per-file fingerprints for incremental indexing
doc_sections      — markdown documentation sections
graph_edges       — full confidence-tagged graph (all node types and relations)
rationale         — "why" nodes from annotated comments (NOTE/WHY/REASON/etc.)
communities       — Leiden community detection results
hyperedges        — 3+ node group relationships
hyperedge_members — members of each hyperedge
```

---

## Key Design Decisions

### 1. `projects` replaces `repos` and covers ideas

The Postgres schema had a `repos` table. In the new model, projects are the top-level
container for both repos (`kind='repo'`) and pre-code ideas (`kind='idea'`). They share
phases, cards, maturity tracking, and coordinator installs. When an idea graduates to a
repo, `graduated_from` links the two rows.

### 2. `repo_id` removed from index.db tables

The Postgres schema put `repo_id` on every table. In SQLite, the per-repo database *is*
the scope — there are no cross-repo queries within `index.db`. Removing `repo_id`
simplifies every query and write. Cross-repo queries happen via `sensei.db` through
`projects`.

### 3. Type mapping from Postgres

| Postgres | SQLite | Notes |
|---|---|---|
| `uuid` | `text` | UUID string; generated in application via `crypto.randomUUID()` |
| `timestamptz` | `text` | ISO 8601 via `strftime('%Y-%m-%dT%H:%M:%fZ', 'now')` |
| `text[]` | `text` | JSON array string; `'[]'` default |
| `jsonb` | `text` | JSON string |
| `boolean` | `integer` | `0`=false, `1`=true |
| `vector(n)` | `blob` | sqlite-vec F32_BLOB format; null when extension absent |
| `numeric(p,s)` | `real` | adequate for costs and FTR scores |
| `current_user` | `'system'` | application provides the actual identity |

### 4. Stored functions become application code

The three Postgres query functions move to the engine package as TypeScript:

```
packages/engine/src/db/queries.ts

rankBfs(db, changedFiles, maxDepth)   — recursive CTE, same logic as rank_bfs()
rankBm25(db, query)                   — FTS5 MATCH query (replaces LIKE-based approach)
matchEmbeddings(db, embedding, threshold) — sqlite-vec vec_distance_cosine()
```

SQLite FTS5 `MATCH` syntax is more powerful than the current LIKE-based approximation.
Recursive CTEs handle BFS traversal with the same logic in a different dialect.

### 5. FTS5 virtual tables

Every text-heavy table has an FTS5 companion for full-text search. FTS5 uses an
inverted index — substantially faster than LIKE, and no extension required beyond
standard SQLite 3.x.

```sql
-- replaces the LIKE-based rank_bm25 function
select id from symbols_fts where symbols_fts match 'authenticate token' order by rank;
```

### 6. Confidence tagging on all graph edges

Both `call_edges` and `graph_edges` carry a `confidence` column:
`EXTRACTED` (directly observed in AST), `INFERRED` (deduced, with score), `AMBIGUOUS`
(flagged for review). Every edge in the graph is honest about how certain it is.

### 7. `coordinator` column on sessions and api_requests

Sessions record which coordinator generated them. This enables coordinator-specific
analytics — `api_requests` is only populated for coordinators with OTLP support (Claude
Code today), shown as null/unavailable for others.

---

## WAL Mode Configuration

Both databases opened with WAL mode. Single writer (daemon), concurrent readers (desktop):

```typescript
db.exec('PRAGMA journal_mode = WAL');
db.exec('PRAGMA synchronous = NORMAL');
db.exec('PRAGMA foreign_keys = ON');
db.exec('PRAGMA busy_timeout = 5000');
```

---

## Schema Initialization

The application creates both databases on first run from the DDL files:

```typescript
// packages/shared/src/db/init.ts
export function initGlobalDb(path: string): Database
export function initIndexDb(path: string): Database
```

Schema migrations use a `schema_version` pragma plus a migrations table.
dbd manages the DDL source of truth in `database/ddl/sqlite/`.
Migrations are generated from diffs between DDL versions.
