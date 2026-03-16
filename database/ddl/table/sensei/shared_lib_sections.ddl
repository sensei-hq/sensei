set search_path to sensei, extensions;

create table if not exists shared_lib_sections (
  id            uuid primary key default gen_random_uuid()
, shared_lib_id uuid not null references shared_libs(id) on delete cascade
, title         text not null
, url           text
, local_path    text
, description   text not null
, content       text
, source_type   text not null check (source_type in ('llms.txt', 'http', 'local'))
, component     text
, embedding     vector(768)
, last_fetched  timestamptz not null default now()
, created_at    timestamptz not null default now()
);

create index if not exists shared_lib_sections_shared_lib_id_idx
  on shared_lib_sections(shared_lib_id);

create index if not exists shared_lib_sections_embedding_idx
  on shared_lib_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

comment on table shared_lib_sections is
'Indexed documentation sections for globally-shared libraries.
- One row per content chunk; no repo_id (global, not per-repo)
- shared_lib_id references shared_libs(id)
- embedding: 768-dim vector for semantic search via match_shared_lib_sections RPC';
