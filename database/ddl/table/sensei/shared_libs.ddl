set search_path to sensei, extensions;

create table if not exists shared_libs (
  id            uuid primary key default gen_random_uuid()
, name          text not null unique
, source_type   text not null check (source_type in ('llms.txt', 'http', 'local'))
, base_url      text
, local_path    text
, section_count int not null default 0
, indexed_at    timestamptz
, created_at    timestamptz not null default now()
, modified_at   timestamptz not null default now()
, modified_by   text        not null default current_user
);

comment on table shared_libs is
'Global catalog of promoted shared libraries (one row per lib).
- Promoted via: sensei update-registry --global --lib <name>
- section_count and indexed_at updated after every successful global index run
- Referenced by repo_libs.shared_lib_id for repos that link to the shared pool';

comment on column shared_libs.id is 'Surrogate primary key (UUID).';
comment on column shared_libs.name is 'Unique name identifying this shared library in the global catalog.';
comment on column shared_libs.source_type is 'How sections are sourced for this library: llms.txt, http, or local.';
comment on column shared_libs.base_url is 'Base URL used to fetch documentation sections when source_type is http or llms.txt.';
comment on column shared_libs.local_path is 'Local filesystem path used to read documentation sections when source_type is local.';
comment on column shared_libs.section_count is 'Number of indexed sections in shared_lib_sections for this library; updated after each index run.';
comment on column shared_libs.indexed_at is 'Timestamp of the most recent successful global index run for this library.';
comment on column shared_libs.created_at is 'Timestamp when the row was first created.';
comment on column shared_libs.modified_at is 'Timestamp of the last modification to this row.';
comment on column shared_libs.modified_by is 'Identity (user, role, or service) that last modified this row.';
