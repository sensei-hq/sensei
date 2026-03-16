-- supabase/migrations/20260315000001_phase7_repo_libs.sql

create table if not exists sensei.repo_libs (
  id                  uuid primary key default gen_random_uuid(),
  repo_id             uuid not null references sensei.repos(id) on delete cascade,
  name                text not null,
  source_type         text not null check (source_type in ('llms.txt', 'http', 'local')),
  base_url            text,
  local_path          text,
  skill_path          text,
  skill_generated_at  timestamptz,
  created_at          timestamptz not null default now(),
  unique(repo_id, name)
);

create index if not exists repo_libs_repo_id_idx on sensei.repo_libs(repo_id);

-- NOTE: grants are managed separately in database/ddl/grants.ddl — do not add here
