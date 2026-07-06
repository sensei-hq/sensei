set search_path to sensei, extensions;

-- #83 T1 commands surface. Discoverable commands per folder — populated
-- from each folder's manifest via `ManifestAdapter::parse_commands`, read
-- by the project window's action buttons + the `get_commands` MCP tool +
-- AI-assistant "how do I run tests here?" queries.
--
-- One row per (folder_id, raw_name) so re-running the scan replaces the
-- set for a folder without accumulating stale entries. Category is the
-- canonical verb ('test' / 'build' / 'lint' / 'run' / 'format' /
-- 'typecheck' / 'e2e' / 'bench' / 'docs' / 'start' / 'dev'), or NULL when
-- the classifier couldn't decide.
create table if not exists project_commands (
  id             bigserial   primary key
, folder_id      uuid        not null references sensei.folders(id) on delete cascade
, raw_name       text        not null                        -- name as it appears in the manifest (e.g. 'test:unit')
, command_line   text        not null                        -- executable form ('npm run test:unit')
, category       text                                        -- canonical verb; NULL when unclassified
, ecosystem      text        not null                        -- 'npm' | 'cargo' | 'pypi' | 'go' | 'maven' | 'nuget' | ...
, source_file    text                                        -- absolute path to the manifest that produced this row
, discovered_at  timestamptz not null default now()
, unique (folder_id, raw_name)
);

create index if not exists project_commands_folder_idx    on project_commands (folder_id);
create index if not exists project_commands_category_idx  on project_commands (category);
create index if not exists project_commands_ecosystem_idx on project_commands (ecosystem);

comment on table project_commands is
'#83 T1 commands surface. Discoverable commands per folder — populated from
manifest adapters. One row per (folder_id, raw_name); re-scan replaces the
per-folder set atomically via delete-then-insert.';
