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
);

comment on table shared_libs is
'Global catalog of promoted shared libraries (one row per lib).
- Promoted via: sensei update-registry --global --lib <name>
- section_count and indexed_at updated after every successful global index run
- Referenced by repo_libs.shared_lib_id for repos that link to the shared pool';
