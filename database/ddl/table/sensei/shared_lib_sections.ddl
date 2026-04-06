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
, modified_at   timestamptz not null default now()
, modified_by   text        not null default current_user
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

comment on column shared_lib_sections.id is 'Surrogate primary key (UUID).';
comment on column shared_lib_sections.shared_lib_id is 'Foreign key to shared_libs — identifies which global library this section belongs to.';
comment on column shared_lib_sections.title is 'Title of this documentation section.';
comment on column shared_lib_sections.url is 'Remote URL this section was fetched from, if sourced over HTTP.';
comment on column shared_lib_sections.local_path is 'Local filesystem path this section was read from, if sourced locally.';
comment on column shared_lib_sections.description is 'Short summary of this documentation section''s content.';
comment on column shared_lib_sections.content is 'Full text content of this documentation section.';
comment on column shared_lib_sections.source_type is 'How this section was obtained: llms.txt, http, or local.';
comment on column shared_lib_sections.component is 'Optional sub-component or topic label within the library.';
comment on column shared_lib_sections.embedding is '768-dimensional vector embedding for semantic search via match_shared_lib_sections RPC.';
comment on column shared_lib_sections.last_fetched is 'Timestamp when this section''s content was last fetched or refreshed.';
comment on column shared_lib_sections.created_at is 'Timestamp when the row was first created.';
comment on column shared_lib_sections.modified_at is 'Timestamp of the last modification to this row.';
comment on column shared_lib_sections.modified_by is 'Identity (user, role, or service) that last modified this row.';
