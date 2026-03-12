# Supabase Database Setup Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `database/` folder with dbd project config and DDL files for the `sensei` schema (9 tables), start local Supabase, apply the schema, and add seed data for web app development.

**Architecture:** Use `dbd` (same pattern as `~/Developer/strategos/solution/database/`) — one DDL file per table in `database/ddl/table/sensei/`. Run `supabase start` for local Postgres (port 54322), apply with `dbd apply`, seed with SQL inserts. No migrations — DDL is applied directly.

**Tech Stack:** Supabase CLI, dbd 2.0.3, PostgreSQL 15, pgvector, uuid-ossp

**Spec:** `docs/superpowers/specs/2026-03-12-supabase-central-store-design.md`

---

## Chunk 1: Bootstrap — Supabase init + dbd project

### Task 1: Initialize Supabase project and dbd database folder

**Files:**
- Create: `database/design.yaml`
- Create: `database/.env.local` (gitignored)
- Create: `database/.gitignore`
- Modify: `.gitignore` (root) — add `database/.env.local`

**Note:** `database/` lives at the repo root, alongside `packages/` and `apps/`.

- [ ] **Step 1: Initialize Supabase**

```bash
cd /path/to/sensei
supabase init
```

Expected: creates `supabase/` folder with `config.toml`.

- [ ] **Step 2: Create the database folder**

```bash
mkdir -p database/ddl/table/sensei
mkdir -p database/seed
```

- [ ] **Step 3: Create `database/design.yaml`**

```yaml
project:
  name: sensei
  database: PostgreSQL
  extensionSchema: extensions

schemas:
  - extensions
  - sensei

extensions:
  - uuid-ossp
  - vector
```

**Note:** `extensions` must come before `sensei` in the schemas list so `uuid-ossp` and `vector` are available when the `sensei` DDL files run. dbd applies schemas in the listed order.

- [ ] **Step 4: Create `database/.gitignore`**

```
.env.local
```

- [ ] **Step 5: Add `database/.env.local` to root `.gitignore`**

Open `/.gitignore` and add a line:
```
database/.env.local
```

- [ ] **Step 6: Start local Supabase**

```bash
supabase start
```

Expected output includes:
```
API URL: http://localhost:54321
DB URL:  postgresql://postgres:postgres@localhost:54322/postgres
Studio:  http://localhost:54323
```

- [ ] **Step 7: Create `database/.env.local`** with the DB URL from the output above:

```
DATABASE_URL=postgresql://postgres:postgres@localhost:54322/postgres
```

- [ ] **Step 8: Create the `sensei` schema, enable extensions, and grant anon access**

```bash
cd database
psql "$DATABASE_URL" -c "create schema if not exists sensei; create extension if not exists \"uuid-ossp\" schema extensions; create extension if not exists vector schema extensions;"
```

Then grant the Supabase `anon` role read access so the dashboard (which uses the anon key in the browser) can query `sensei.*` tables:

```bash
psql "$DATABASE_URL" -c "grant usage on schema sensei to anon; grant select on all tables in schema sensei to anon; alter default privileges in schema sensei grant select on tables to anon;"
```

Or connect via `supabase db connect` if psql is not in PATH.

Expected: no errors.

- [ ] **Step 9: Commit**

```bash
git add supabase/ database/design.yaml database/.gitignore
git commit -m "feat(database): init Supabase project and dbd config for sensei schema"
```

---

## Chunk 2: Core tables — repos, symbols, chunks, docs

### Task 2: DDL for repos, symbols, chunks, docs

**Files:**
- Create: `database/ddl/table/sensei/repos.ddl`
- Create: `database/ddl/table/sensei/symbols.ddl`
- Create: `database/ddl/table/sensei/chunks.ddl`
- Create: `database/ddl/table/sensei/docs.ddl`

- [ ] **Step 1: Create `database/ddl/table/sensei/repos.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists repos (
  id                  uuid primary key default gen_random_uuid()
, name                text not null
, remote_url          text
, default_branch      text
, description         text
, stack               text[]
, entry_points        jsonb
, last_indexed_commit text
, last_indexed_at     timestamptz
, is_public           boolean not null default false
, owner_id            uuid
, created_at          timestamptz not null default now()
, modified_at         timestamptz not null default now()
);

create unique index if not exists repos_remote_url_ukey on repos(remote_url)
  where remote_url is not null;

-- Note: This is a partial unique index. The seed SQL and application code must use
-- ON CONFLICT (remote_url) WHERE remote_url IS NOT NULL to match this index.
-- Alternatively: omit the WHERE clause to make it a plain unique constraint, at the
-- cost of allowing only one NULL remote_url. The partial index is preferred.

comment on table repos is
'Central registry of indexed repos.
- One row per repo, identified by remote_url (git remote)
- entry_points is [{path, role, inferredRole}] — matches IndexSummary shape
- is_public stub for future multi-tenant RLS';
```

- [ ] **Step 2: Create `database/ddl/table/sensei/symbols.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists symbols (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid not null references sensei.repos(id) on delete cascade
, file_path   text not null
, l0          text[]
, l1          text
, l2          text
, modified_at timestamptz not null default now()
, unique(repo_id, file_path)
);

create index if not exists symbols_repo_id_idx on symbols(repo_id);

comment on table symbols is
'Per-file symbol index. Replaces .sensei/symbol-map.json.
- l0: exported names (for fast listing)
- l1: signatures (for context loading)
- l2: with docstrings (for deep context)
- l3 is NOT stored — served from local filesystem on demand';
```

- [ ] **Step 3: Create `database/ddl/table/sensei/chunks.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists chunks (
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

create index if not exists chunks_repo_id_idx on chunks(repo_id);

create index if not exists chunks_embedding_hnsw_idx
  on chunks using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64);

comment on table chunks is
'Chunked content with embeddings. Replaces .sensei/chunks.json + .sensei/embeddings.json.
- HNSW index for fast vector similarity search (cosine distance)
- 384-dim embeddings from local model (nomic-embed-text or similar)
- metadata: {type: "symbol"|"doc", contentHash, tf}';
```

- [ ] **Step 4: Create `database/ddl/table/sensei/docs.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists docs (
  id            uuid primary key default gen_random_uuid()
, repo_id       uuid not null references sensei.repos(id) on delete cascade
, doc_path      text not null
, covers        text[]
, auto_detected boolean not null default false
, modified_at   timestamptz not null default now()
, unique(repo_id, doc_path)
);

create index if not exists docs_repo_id_idx on docs(repo_id);

comment on table docs is
'Traceability: which source files each doc covers. Replaces .sensei/traceability.json.
- covers: array of source file paths this doc documents
- auto_detected: true when coverage was inferred from doc content, not declared in llmspec';
```

- [ ] **Step 5: Apply the schema**

```bash
cd database
dbd apply
```

Expected: 4 tables created, no errors.

- [ ] **Step 6: Verify**

```bash
psql "$DATABASE_URL" -c "\dt sensei.*"
```

Expected:
```
 sensei | chunks  | table | postgres
 sensei | docs    | table | postgres
 sensei | repos   | table | postgres
 sensei | symbols | table | postgres
```

- [ ] **Step 7: Commit**

```bash
git add database/ddl/table/sensei/
git commit -m "feat(database): add core DDL — repos, symbols, chunks, docs"
```

---

## Chunk 3: Analytics and benchmarks

### Task 3: DDL for events and benchmark_reports

**Files:**
- Create: `database/ddl/table/sensei/events.ddl`
- Create: `database/ddl/table/sensei/benchmark_reports.ddl`

- [ ] **Step 1: Create `database/ddl/table/sensei/events.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists events (
  id           uuid primary key default gen_random_uuid()
, user_uuid    text not null
, session_id   text
, repo_id      uuid references sensei.repos(id)
, phase        text not null check (phase in ('pre', 'post'))
, tool         text not null
, project_path text
, input        jsonb
, ts           timestamptz not null
, created_at   timestamptz not null default now()
);

create index if not exists events_user_uuid_idx on events(user_uuid);
create index if not exists events_ts_idx        on events(ts desc);
create index if not exists events_tool_idx      on events(tool);
create index if not exists events_repo_id_idx   on events(repo_id)
  where repo_id is not null;

comment on table events is
'Telemetry events from collector daemon. Replaces ~/.sensei/<uuid>/analytics.db.
- repo_id nullable: events before repo registration have no FK
- input: jsonb for structured tool input (replaces text column in SQLite)
- ts: original event timestamp; created_at: Supabase insert time';
```

- [ ] **Step 2: Create `database/ddl/table/sensei/benchmark_reports.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists benchmark_reports (
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

create index if not exists benchmark_reports_repo_id_idx on benchmark_reports(repo_id)
  where repo_id is not null;
create index if not exists benchmark_reports_created_idx on benchmark_reports(created_at desc);

comment on table benchmark_reports is
'Benchmarking run results. Replaces .sensei/reports.db.
- promoted: true when a strategy was selected as the winner
- payload: full run details (questions, answers, scores per item)';
```

- [ ] **Step 3: Apply and verify**

```bash
cd database
dbd apply
psql "$DATABASE_URL" -c "\dt sensei.*"
```

Expected: 6 tables (repos, symbols, chunks, docs, events, benchmark_reports).

- [ ] **Step 4: Commit**

```bash
git add database/ddl/table/sensei/
git commit -m "feat(database): add analytics and benchmark DDL — events, benchmark_reports"
```

---

## Chunk 4: Knowledge base tables

### Task 4: DDL for libraries, references, repo_libraries

**Files:**
- Create: `database/ddl/table/sensei/libraries.ddl`
- Create: `database/ddl/table/sensei/references.ddl`
- Create: `database/ddl/table/sensei/repo_libraries.ddl`

- [ ] **Step 1: Create `database/ddl/table/sensei/libraries.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists libraries (
  id                  uuid primary key default gen_random_uuid()
, name                text not null
, ecosystem           text not null check (ecosystem in ('npm','pypi','cargo','go'))
, version             text
, description         text
, homepage_url        text
, docs_url            text
, llms_txt_url        text
, llms_txt            text
, llms_txt_fetched_at timestamptz
, embedding           vector(384)
, modified_at         timestamptz not null default now()
, unique(ecosystem, name)
);

create index if not exists libraries_ecosystem_idx on libraries(ecosystem);
create index if not exists libraries_embedding_hnsw_idx
  on libraries using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
  where embedding is not null;

comment on table libraries is
'Package catalog with cached llms.txt content.
- ecosystem + name: unique key (e.g. npm + @supabase/supabase-js)
- llms_txt: cached content from llms_txt_url, refreshed periodically
- embedding: on description field, for semantic library search';
```

- [ ] **Step 2: Create `database/ddl/table/sensei/references.ddl`**

**Important:** `references` is a reserved SQL keyword. The table name must be quoted everywhere in DDL and queries.

```sql
set search_path to sensei, extensions;

create table if not exists "references" (
  id          uuid primary key default gen_random_uuid()
, url         text not null unique
, title       text
, description text
, tags        text[]
, content     text
, embedding   vector(384)
, fetched_at  timestamptz
, modified_at timestamptz not null default now()
);

create index if not exists references_tags_idx on "references" using gin(tags);
create index if not exists references_embedding_hnsw_idx
  on "references" using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
  where embedding is not null;

comment on table "references" is
'Freeform URL catalog — docs pages, RFCs, blog posts.
- url: unique identifier
- content: cached page content (may be large — consider size limits)
- embedding: on title + description for semantic search';
```

- [ ] **Step 3: Create `database/ddl/table/sensei/repo_libraries.ddl`**

```sql
set search_path to sensei, extensions;

create table if not exists repo_libraries (
  repo_id      uuid not null references sensei.repos(id) on delete cascade
, library_id   uuid not null references sensei.libraries(id) on delete cascade
, version_used text
, primary key (repo_id, library_id)
);

create index if not exists repo_libraries_library_id_idx on repo_libraries(library_id);

comment on table repo_libraries is
'Junction: which repos use which libraries.
- version_used: the version string from package.json / requirements.txt etc.
- Cascade delete when repo is deleted';
```

- [ ] **Step 4: Apply and verify all 9 tables**

```bash
cd database
dbd apply
psql "$DATABASE_URL" -c "\dt sensei.*"
```

Expected output (9 rows):
```
 sensei | benchmark_reports | table | postgres
 sensei | chunks            | table | postgres
 sensei | docs              | table | postgres
 sensei | events            | table | postgres
 sensei | libraries         | table | postgres
 sensei | references        | table | postgres
 sensei | repo_libraries    | table | postgres
 sensei | repos             | table | postgres
 sensei | symbols           | table | postgres
```

- [ ] **Step 5: Commit**

```bash
git add database/ddl/table/sensei/
git commit -m "feat(database): add knowledge base DDL — libraries, references, repo_libraries"
```

---

## Chunk 5: Seed data

### Task 5: Insert seed rows for web app development

**Files:**
- Create: `database/seed/repos.sql`
- Create: `database/seed/libraries.sql`
- Create: `database/seed/events.sql`

These are development seed rows so the web app has data to display. They are applied manually (not via `dbd import`) — just run the SQL files against the local DB.

- [ ] **Step 1: Create `database/seed/repos.sql`**

```sql
-- Development seed: sample repo rows
insert into sensei.repos (id, name, remote_url, default_branch, description, stack, is_public, last_indexed_at)
values
  (
    '00000000-0000-0000-0000-000000000001'::uuid,
    'sensei',
    'git@github.com:yourorg/sensei.git',
    'main',
    'AI-native dev tool: indexer, MCP server, CLI, collector daemon',
    array['TypeScript','Bun','SvelteKit'],
    false,
    now() - interval '1 hour'
  ),
  (
    '00000000-0000-0000-0000-000000000002'::uuid,
    'strategos',
    'git@github.com:yourorg/strategos.git',
    'main',
    'AI routing and model gateway',
    array['TypeScript','Bun','PostgreSQL'],
    false,
    now() - interval '2 hours'
  )
on conflict (remote_url) where remote_url is not null do nothing;
```

- [ ] **Step 2: Create `database/seed/libraries.sql`**

```sql
-- Development seed: sample library rows
insert into sensei.libraries (name, ecosystem, version, description, homepage_url, llms_txt_url)
values
  ('typescript',       'npm',  '5.9.3',  'TypeScript language', 'https://typescriptlang.org', null),
  ('vite',             'npm',  '6.0.0',  'Frontend build tool', 'https://vitejs.dev',         null),
  ('@supabase/supabase-js', 'npm', '2.47.0', 'Supabase client library', 'https://supabase.com', null),
  ('sveltekit',        'npm',  '2.0.0',  'SvelteKit web framework', 'https://kit.svelte.dev', null),
  ('zod',              'npm',  '3.24.0', 'TypeScript schema validation', 'https://zod.dev',    null)
on conflict (ecosystem, name) do nothing;
```

- [ ] **Step 3: Create `database/seed/events.sql`**

```sql
-- Development seed: 10 sample events for stats page
insert into sensei.events (user_uuid, session_id, repo_id, phase, tool, project_path, ts)
select
  'dev-user-seed'::text,
  'sess-' || gs::text,
  '00000000-0000-0000-0000-000000000001'::uuid,
  case when gs % 2 = 0 then 'pre' else 'post' end,
  (array['Read','Edit','Bash','Write','Grep'])[1 + (gs % 5)],
  '/Users/dev/sensei',
  now() - (gs * interval '10 minutes')
from generate_series(1, 20) as gs;
```

- [ ] **Step 4: Apply seed data**

```bash
cd database
psql "$DATABASE_URL" -f seed/repos.sql
psql "$DATABASE_URL" -f seed/libraries.sql
psql "$DATABASE_URL" -f seed/events.sql
```

Expected: no errors; INSERT messages.

- [ ] **Step 5: Verify seed data**

```bash
psql "$DATABASE_URL" -c "select name, stack from sensei.repos;"
psql "$DATABASE_URL" -c "select count(*) from sensei.events;"
```

Expected: 2 repos, 20 events.

- [ ] **Step 6: Commit**

```bash
git add database/seed/
git commit -m "feat(database): add seed data for web app development"
```

---

## Chunk 6: Update traceability

### Task 6: Register database design in traceability.yaml

**Files:**
- Modify: `docs/traceability.yaml`

- [ ] **Step 1: Add `supabase-central-store` design entry to `docs/traceability.yaml`**

In the `design:` section, add after `analytics-collector`:

```yaml
  supabase-store:
    doc: docs/superpowers/specs/2026-03-12-supabase-central-store-design.md
    title: Supabase Central Store
    implements: []
```

- [ ] **Step 2: Commit**

```bash
git add docs/traceability.yaml
git commit -m "docs(traceability): register supabase-central-store design"
```
