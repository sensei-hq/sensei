-- supabase/migrations/20260315000002_phase8_shared_lib_pool.sql

-- Global shared library catalog (one row per promoted lib)
create table if not exists sensei.shared_libs (
  id            uuid primary key default gen_random_uuid(),
  name          text not null unique,
  source_type   text not null check (source_type in ('llms.txt', 'http', 'local')),
  base_url      text,
  local_path    text,
  section_count int not null default 0,
  indexed_at    timestamptz,
  created_at    timestamptz not null default now()
);

-- Indexed doc sections for shared libs (no repo_id — global)
create table if not exists sensei.shared_lib_sections (
  id            uuid primary key default gen_random_uuid(),
  shared_lib_id uuid not null references sensei.shared_libs(id) on delete cascade,
  title         text not null,
  url           text,
  local_path    text,
  description   text not null,
  content       text,
  source_type   text not null check (source_type in ('llms.txt', 'http', 'local')),
  component     text,
  embedding     vector(768),
  last_fetched  timestamptz not null default now(),
  created_at    timestamptz not null default now()
);

create index if not exists shared_lib_sections_shared_lib_id_idx
  on sensei.shared_lib_sections(shared_lib_id);

create index if not exists shared_lib_sections_embedding_idx
  on sensei.shared_lib_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

-- FK from repo_libs → shared_libs (nullable: null = per-repo indexed)
alter table sensei.repo_libs
  add column if not exists shared_lib_id uuid references sensei.shared_libs(id) on delete set null;

-- RPC: semantic search over shared lib sections
create or replace function sensei.match_shared_lib_sections(
  p_shared_lib_id uuid,
  p_component     text,
  query_embedding vector(768),
  match_count     int default 10
)
returns table (
  id          uuid,
  title       text,
  url         text,
  local_path  text,
  description text,
  content     text,
  source_type text,
  component   text,
  similarity  float
)
language sql stable
as $$
  select
    id, title, url, local_path, description, content, source_type, component,
    1 - (embedding <=> query_embedding) as similarity
  from sensei.shared_lib_sections
  where shared_lib_id = p_shared_lib_id
    and (p_component is null or component = p_component)
  order by embedding <=> query_embedding
  limit match_count;
$$;

-- NOTE: grants are managed separately in database/ddl/grants.ddl — do not add here
