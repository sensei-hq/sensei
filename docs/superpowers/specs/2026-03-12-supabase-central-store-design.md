# Supabase Central Store — Design

## Goal

Replace sensei's scattered SQLite + JSON file artifacts with a central Supabase (PostgreSQL) instance. A SvelteKit + rokkit web app reads from Supabase directly. Auth and route protection via kavach (magic link / anonymous). The `.sensei/` folder shrinks to two committed files; everything computed lives in Supabase.

## Architecture

```
.sensei/llmspec.yaml       human-authored config (committed)
.sensei/config.yaml        repo_id + supabase_url (committed, not secret)
~/.config/sensei/credentials.yaml   service key (global, never committed)

Supabase (local dev: supabase start, prod: hosted)
  schema: sensei
  extensions: vector, uuid-ossp

sensei CLI / MCP  →  writes to Supabase  (replaces file writes)
collector daemon  →  writes sensei.events (replaces analytics.db)
web app (apps/dashboard)  →  reads Supabase via supabase-js
```

## Database Setup

- Tool: `dbd` (same pattern as strategos)
- Folder: `database/`
- Schema: `sensei` (isolated — safe on shared Supabase instances)
- Extensions: `vector` (pgvector), `uuid-ossp`

## Schema

### `sensei.repos`
Central registry of indexed repos.

```sql
create table if not exists sensei.repos (
  id                    uuid primary key default gen_random_uuid()
, name                  text not null
, remote_url            text                          -- git remote (stable ID)
, default_branch        text
, description           text
, stack                 text[]
, entry_points          jsonb                         -- [{path, role, inferredRole}]
, last_indexed_commit   text
, last_indexed_at       timestamptz
, is_public             boolean not null default false
, owner_id              uuid                          -- auth.users ref (future)
, created_at            timestamptz not null default now()
, modified_at           timestamptz not null default now()
);
```

### `sensei.symbols`
Replaces `.sensei/symbol-map.json`. L0–L2 per file (L3 served from local filesystem).

```sql
create table if not exists sensei.symbols (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid not null references sensei.repos(id) on delete cascade
, file_path   text not null
, l0          text[]    -- exported names
, l1          text      -- signatures
, l2          text      -- with docstrings
, modified_at timestamptz not null default now()
, unique(repo_id, file_path)
);
```

### `sensei.chunks`
Replaces `.sensei/chunks.json` + `.sensei/embeddings.json`. pgvector HNSW index.

```sql
create table if not exists sensei.chunks (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid not null references sensei.repos(id) on delete cascade
, file_path   text not null
, chunk_index integer not null
, content     text not null
, embedding   vector(384)
, token_count integer
, metadata    jsonb
, modified_at timestamptz not null default now()
, unique(repo_id, file_path, chunk_index)
);

create index if not exists idx_chunks_embedding_hnsw
  on sensei.chunks using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64);
```

### `sensei.docs`
Replaces `.sensei/traceability.json`.

```sql
create table if not exists sensei.docs (
  id            uuid primary key default gen_random_uuid()
, repo_id       uuid not null references sensei.repos(id) on delete cascade
, doc_path      text not null
, covers        text[]    -- source file paths
, auto_detected boolean not null default false
, modified_at   timestamptz not null default now()
, unique(repo_id, doc_path)
);
```

### `sensei.events`
Replaces `~/.sensei/<uuid>/analytics.db`. Collector daemon writes here.

```sql
create table if not exists sensei.events (
  id           uuid primary key default gen_random_uuid()
, user_uuid    text not null
, session_id   text
, repo_id      uuid references sensei.repos(id)   -- nullable
, phase        text not null check (phase in ('pre', 'post'))
, tool         text not null
, project_path text
, input        jsonb
, ts           timestamptz not null
, created_at   timestamptz not null default now()
);

create index if not exists idx_events_user_uuid on sensei.events(user_uuid);
create index if not exists idx_events_ts on sensei.events(ts desc);
create index if not exists idx_events_tool on sensei.events(tool);
```

### `sensei.benchmark_reports`
Replaces `reports.db`.

```sql
create table if not exists sensei.benchmark_reports (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid references sensei.repos(id)
, run_name    text not null
, strategy    text not null
, score       numeric
, tokens      integer
, elapsed_ms  integer
, payload     jsonb
, promoted    boolean not null default false
, created_at  timestamptz not null default now()
);
```

### `sensei.libraries`
Package catalog with cached llms.txt.

```sql
create table if not exists sensei.libraries (
  id                  uuid primary key default gen_random_uuid()
, name                text not null
, ecosystem           text not null check (ecosystem in ('npm','pypi','cargo','go'))
, version             text
, description         text
, homepage_url        text
, docs_url            text
, llms_txt_url        text
, llms_txt            text          -- cached content
, llms_txt_fetched_at timestamptz
, embedding           vector(384)   -- on description
, modified_at         timestamptz not null default now()
, unique(ecosystem, name)
);
```

### `sensei.references`
Freeform URL catalog — docs pages, RFCs, blog posts.

```sql
create table if not exists sensei.references (
  id          uuid primary key default gen_random_uuid()
, url         text not null unique
, title       text
, description text
, tags        text[]
, content     text          -- cached page content
, embedding   vector(384)   -- on title + description
, fetched_at  timestamptz
, modified_at timestamptz not null default now()
);
```

### `sensei.repo_libraries`
Junction — which repos use which libraries.

```sql
create table if not exists sensei.repo_libraries (
  repo_id      uuid not null references sensei.repos(id) on delete cascade
, library_id   uuid not null references sensei.libraries(id) on delete cascade
, version_used text
, primary key (repo_id, library_id)
);
```

## Committed Repo Files (post-migration)

`.sensei/config.yaml`:
```yaml
repo_id: <uuid assigned on first index>
supabase_url: http://localhost:54321   # or hosted URL
```

`.sensei/llmspec.yaml`: unchanged — human-authored config.

## Global Credentials

`~/.config/sensei/credentials.yaml`:
```yaml
supabase_service_key: <service_role_key>
```

Or via env: `SUPABASE_SERVICE_KEY`.

## Web App — `apps/dashboard`

- **Stack:** SvelteKit + rokkit + kavach
- **Auth:** kavach init → magic link or anonymous
- **Data:** supabase-js client, reads `sensei.*` tables directly
- **Pages:**
  1. Dashboard — repo count, recent events, index health
  2. Stats — tool usage charts, gaps report (`events` table)
  3. Repos — indexed repos, drift status, last commit
  4. Benchmark Reports — run results, strategy comparison
  5. Libraries — package catalog, llms.txt viewer
  6. References — URL catalog, search

## Implementation Order

1. **Database** — `dbd init`, DDL files, `supabase start`, apply schema
2. **Web app** — SvelteKit + rokkit + kavach, pages with seed data
3. **Backend migration** — collector writes to Supabase, indexer writes to Supabase, MCP queries Supabase
4. **Config** — `.sensei/config.yaml` + `~/.config/sensei/credentials.yaml` support

## Non-Goals

- Real-time subscriptions (future)
- Multi-tenant RLS policies (future — `is_public` stub only for now)
- `sensei sync` / `traceability-status-sync` (separate feature)
