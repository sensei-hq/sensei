set search_path to sensei, extensions;

-- #84 Track 2 Slice A — MCP servers discovered from ACP config files.
--
-- Populated by scanning each detected ACP's MCP config (~/.claude/mcp.json,
-- <project>/.mcp.json, .zed/settings.json, etc.) on daemon start and manual
-- refresh. Enables per-project enable/disable + connection-state tracking
-- that the Playground / Replay / Health tabs consume.
--
-- One row per (acp_family, mcp_key, scope, project) tuple. User-scope entries
-- have NULL project_id; project-scope entries have a concrete project_id.
-- Partial unique indexes keep those two variants separate so a server can be
-- configured user-wide AND overridden per project without a conflict.
create table if not exists mcp_servers (
  id                uuid          primary key default gen_random_uuid()
, acp_family        text          not null                  -- 'claude' | 'zed' | 'cursor' | 'codex' | 'opencode' | 'other'
, mcp_key           text          not null                  -- key in the ACP config: 'postgres', 'sensei', 'svelte'
, scope             text          not null check (scope in ('user', 'project'))
, project_id        uuid          references sensei.projects(id) on delete cascade
, config_source     text          not null                  -- absolute path where discovered
, command           text          not null default ''       -- the executable
, args              jsonb         not null default '[]'
, env               jsonb         not null default '{}'
, enabled           boolean       not null default true
, connection_state  text          not null default 'unknown' check (connection_state in ('unknown', 'connected', 'error', 'disabled'))
, last_error        text
, last_seen_at      timestamptz   not null default now()
, discovered_at     timestamptz   not null default now()
, check ((scope = 'user' and project_id is null) or (scope = 'project' and project_id is not null))
);

-- Uniqueness: one user-scope row per (family, key); one project-scope row per
-- (family, key, project). Partial indexes let both variants coexist without a
-- three-column unique-key gymnastics with a sentinel UUID.
create unique index if not exists mcp_servers_user_unique
    on mcp_servers (acp_family, mcp_key)
    where scope = 'user';

create unique index if not exists mcp_servers_project_unique
    on mcp_servers (acp_family, mcp_key, project_id)
    where scope = 'project';

create index if not exists mcp_servers_project_idx     on mcp_servers (project_id);
create index if not exists mcp_servers_family_idx      on mcp_servers (acp_family);
create index if not exists mcp_servers_enabled_idx     on mcp_servers (enabled);
create index if not exists mcp_servers_last_seen_idx   on mcp_servers (last_seen_at);

comment on table mcp_servers is
'#84 T2 Slice A. MCP servers discovered from ACP config files (Claude Code,
Zed, Cursor, Codex, OpenCode, …). One row per (acp_family, mcp_key, scope,
project). Scanned on daemon start and manual refresh. Consumed by the
Instruments Playground / Replay / Health tabs.';
