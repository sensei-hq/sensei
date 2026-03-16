set search_path to sensei, extensions;

create table if not exists lib_doc_sections (
  id            uuid primary key default gen_random_uuid()
, repo_id       uuid not null references repos(id) on delete cascade
, lib_name      text not null
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

create index if not exists lib_doc_sections_repo_lib_idx
  on lib_doc_sections(repo_id, lib_name);

create index if not exists lib_doc_sections_embedding_idx
  on lib_doc_sections
  using ivfflat (embedding vector_cosine_ops) with (lists = 100);

comment on table lib_doc_sections is
'Indexed documentation sections for custom libraries configured per repo.
- One row per content chunk fetched from a lib source (llms.txt, HTTP page, or local path)
- lib_name matches custom_libs[].name from .sensei/config.yaml (synced to repo_libs)
- embedding: 768-dim vector for semantic search via match_lib_doc_sections RPC';
