---
name: ADR-005 — Migrate from SQLite + Kuzu to PostgreSQL
status: accepted
date: 2026-04-22
---

# ADR-005: Migrate from SQLite + Kuzu to PostgreSQL

## Context

The daemon currently uses:
- **SQLite** for relational data (repos, projects, sessions, events, config, patterns)
- **Kuzu** for graph data (code graph — functions, types, edges, communities)
- **In-memory task queue** with broadcast channels for SSE

Problems observed:
1. **N+1 queries** — SQLite has no JOINs on JSON arrays, forcing per-row tag lookups
2. **Kuzu maintenance risk** — small project, no longer actively maintained, embedded C++ dependency
3. **No vector support** — future embedding search requires a separate system
4. **Queue is in-memory** — task state lost on daemon restart
5. **Two databases** — SQLite + Kuzu with manual coordination, no transactions across both
6. **Limited concurrency** — SQLite WAL helps but still single-writer

## Decision

Migrate to **PostgreSQL** with extensions:
- **Apache AGE** — graph database extension, Cypher query language (replaces Kuzu)
- **pgvector** — vector similarity search (future: embeddings)
- **PGMQ** or LISTEN/NOTIFY — durable task queue with SSE support

## Consequences

### Positive
- Single database for relational + graph + vectors + queue
- Proper JOINs, views, materialized views for dashboard metrics
- Concurrent connections — desktop app, MCP server, CLI can all query simultaneously
- `DATABASE_URL` configuration — dev/test/prod environments via connection string
- PostgreSQL is a brew prerequisite, well-understood, widely supported
- AGE supports Cypher — our existing graph queries translate directly
- pgvector ready for when we add embeddings

### Negative
- Requires PostgreSQL installed (brew dependency)
- Migration effort: rewrite store.rs, graph.rs, queue system
- Startup is "connect to PG" not "open file" — slightly more setup
- Need to handle PG not running (daemon should check/start or show error)

### Neutral
- Database URL makes dev/test separation trivial (different databases, same server)
- Can keep SQLite as an optional "lite mode" for users who don't want PG

## Migration Plan

### Phase 1: PostgreSQL foundation
1. Add `sqlx` with PostgreSQL driver (replace rusqlite)
2. Create migration files for all tables (repos, projects, config, events, sessions, etc.)
3. Add `DATABASE_URL` config (env var + CLI flag + config file)
4. Update `store.rs` — rewrite all queries using sqlx
5. Setup wizard Step 2 checks for PostgreSQL (brew install postgresql@16)
6. Daemon start creates database if not exists

### Phase 2: Graph migration (AGE)
7. Install Apache AGE extension
8. Migrate graph schema — nodes, edges, hierarchy as AGE vertices and edges
9. Rewrite graph queries — Kuzu Cypher → AGE Cypher (mostly compatible)
10. Remove Kuzu dependency

### Phase 3: Queue + SSE
11. Replace in-memory queue with PGMQ or custom PG-backed queue
12. Use LISTEN/NOTIFY for SSE events (replaces broadcast channels)
13. Task state persists across daemon restarts

### Phase 4: Views + Performance
14. Create materialized views for dashboard metrics (FTR, sessions, patterns)
15. Add pgvector extension for future embedding search
16. Index optimization based on query patterns

## Configuration

```
# Default — local PostgreSQL
DATABASE_URL=postgres://localhost/sensei

# Dev mode
DATABASE_URL=postgres://localhost/sensei_dev

# Test mode
DATABASE_URL=postgres://localhost/sensei_test

# Custom (remote, cloud, etc.)
DATABASE_URL=postgres://user:pass@host:5432/sensei
```

The daemon reads `DATABASE_URL` from:
1. `--database-url` CLI flag
2. `SENSEI_DATABASE_URL` env var
3. `~/.sensei/config.json` `database_url` key
4. Default: `postgres://localhost/sensei` (prod) or `postgres://localhost/sensei_dev` (dev)
