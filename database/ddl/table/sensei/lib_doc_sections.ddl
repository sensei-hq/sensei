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
, modified_at   timestamptz not null default now()
, modified_by   text        not null default current_user
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

comment on column lib_doc_sections.id is 'Surrogate primary key (UUID).';
comment on column lib_doc_sections.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column lib_doc_sections.lib_name is 'Library name as declared in custom_libs[].name in .sensei/config.yaml.';
comment on column lib_doc_sections.title is 'Title of this documentation section.';
comment on column lib_doc_sections.url is 'Remote URL this section was fetched from, if sourced over HTTP.';
comment on column lib_doc_sections.local_path is 'Local filesystem path this section was read from, if sourced locally.';
comment on column lib_doc_sections.description is 'Short summary of this documentation section''s content.';
comment on column lib_doc_sections.content is 'Full text content of this documentation section.';
comment on column lib_doc_sections.source_type is 'How this section was obtained: llms.txt, http, or local.';
comment on column lib_doc_sections.component is 'Optional sub-component or topic label within the library.';
comment on column lib_doc_sections.embedding is '768-dimensional vector embedding for semantic search via match_lib_doc_sections RPC.';
comment on column lib_doc_sections.last_fetched is 'Timestamp when this section''s content was last fetched or refreshed.';
comment on column lib_doc_sections.created_at is 'Timestamp when the row was first created.';
comment on column lib_doc_sections.modified_at is 'Timestamp of the last modification to this row.';
comment on column lib_doc_sections.modified_by is 'Identity (user, role, or service) that last modified this row.';
