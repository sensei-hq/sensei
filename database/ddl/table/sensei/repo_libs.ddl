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
, created_at          timestamptz not null default now()
, unique(repo_id, name)
);

create index if not exists repo_libs_repo_id_idx on repo_libs(repo_id);

comment on table repo_libs is
'Per-repo custom library configuration, synced from .sensei/config.yaml by update-registry.
- One row per lib per repo; upserted after every successful update-registry run
- Source of truth for the dashboard libraries page (avoids reading local config files)
- skill_path and skill_generated_at track the generated Agent Skill file for each lib';
