-- coordinator_installs
-- Tracks which coordinator integrations are configured for each project.
-- One row per (project, coordinator) pair.
-- Populated by sensei init / the Tauri settings view.
-- Used by the daemon to know which event sources to listen for per project.
--
-- coordinator: claude-code | opencode | copilot | kiro | codex | generic
-- scope:       global (installed in ~/.<coordinator>/) | local (<repo>/.<coordinator>/)
-- status:      active | stale (config exists but coordinator not detected at last check)

create table if not exists coordinator_installs (
  id                  text not null primary key
, project_id          text not null references projects(id) on delete cascade
, coordinator         text not null
                           check (coordinator in ('claude-code','opencode','copilot',
                                                  'kiro','codex','generic'))
, scope               text not null default 'global'
                           check (scope in ('global','local'))
, status              text not null default 'active'
                           check (status in ('active','stale'))
, mcp_registered      integer not null default 0   -- 1 if MCP server registered
, hooks_installed     integer not null default 0   -- 1 if event capture hooks installed
, skills_installed    integer not null default 0   -- 1 if skill files written
, context_file        text                         -- path to CLAUDE.md / AGENTS.md etc.
, last_checked_at     text
, installed_at        text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at         text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by         text not null default 'system'
, unique(project_id, coordinator)
);

create index if not exists coordinator_installs_project_idx on coordinator_installs(project_id);
