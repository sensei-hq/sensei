-- supabase/migrations/20260315000000_phase5_lib_doc_sections.sql

create table if not exists sensei.lib_doc_sections (
  id            uuid primary key default gen_random_uuid(),
  repo_id       uuid not null references sensei.repos(id) on delete cascade,
  lib_name      text not null,
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

create index if not exists lib_doc_sections_repo_lib_idx
  on sensei.lib_doc_sections(repo_id, lib_name);

create index if not exists lib_doc_sections_embedding_idx
  on sensei.lib_doc_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

-- RPC for semantic search (mirrors match_embeddings from Phase 2)
create or replace function sensei.match_lib_doc_sections(
  p_repo_id       uuid,
  p_lib_name      text,
  p_component     text DEFAULT NULL,
  query_embedding vector(768),
  match_count     int
)
returns table(
  title       text,
  url         text,
  local_path  text,
  description text,
  content     text,
  source_type text,
  component   text
)
language sql
security definer
as $$
  SELECT title, url, local_path, description, content, source_type, component
  FROM sensei.lib_doc_sections
  WHERE repo_id = p_repo_id
    AND lib_name = p_lib_name
    AND (p_component IS NULL OR component = p_component)
  ORDER BY embedding <=> query_embedding
  LIMIT match_count
$$;

-- NOTE: grants are managed separately in database/ddl/grants.ddl — do not add here
