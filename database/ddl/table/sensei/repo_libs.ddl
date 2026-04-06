set search_path to sensei, extensions;

create table if not exists repo_libs (
  id                  uuid primary key default gen_random_uuid()
, repo_id             uuid not null references repos(id) on delete cascade
, name                text not null
, source_type         text not null check (source_type in ('llms.txt', 'http', 'local'))
, base_url            text
, local_path          text
, skill_path          text
, skill_generated_at  timestamptz
, shared_lib_id       uuid references shared_libs(id) on delete set null
, created_at          timestamptz not null default now()
, modified_at         timestamptz not null default now()
, modified_by         text        not null default current_user
, unique(repo_id, name)
);

create index if not exists repo_libs_repo_id_idx on repo_libs(repo_id);

comment on table repo_libs is
'Per-repo custom library configuration, synced from .sensei/config.yaml by update-registry.
- One row per lib per repo; upserted after every successful update-registry run
- Source of truth for the dashboard libraries page (avoids reading local config files)
- skill_path and skill_generated_at track the generated Agent Skill file for each lib
- shared_lib_id (nullable): FK to shared_libs; null = per-repo indexed, non-null = linked to global shared pool';

comment on column repo_libs.id is 'Surrogate primary key (UUID).';
comment on column repo_libs.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column repo_libs.name is 'Library name as declared in custom_libs[].name in .sensei/config.yaml.';
comment on column repo_libs.source_type is 'How documentation sections are sourced for this lib: llms.txt, http, or local.';
comment on column repo_libs.base_url is 'Base URL used to fetch documentation when source_type is http or llms.txt.';
comment on column repo_libs.local_path is 'Local filesystem path used to read documentation when source_type is local.';
comment on column repo_libs.skill_path is 'Filesystem path to the generated Agent Skill file for this library.';
comment on column repo_libs.skill_generated_at is 'Timestamp when the Agent Skill file was last generated for this library.';
comment on column repo_libs.shared_lib_id is 'Optional FK to shared_libs; null means per-repo indexed, non-null means linked to the global shared pool.';
comment on column repo_libs.created_at is 'Timestamp when the row was first created.';
comment on column repo_libs.modified_at is 'Timestamp of the last modification to this row.';
comment on column repo_libs.modified_by is 'Identity (user, role, or service) that last modified this row.';
