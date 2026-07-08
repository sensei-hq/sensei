set search_path to sensei, extensions;

-- Unified per-assistant TOOL INVENTORY — the registry half of the
-- Instruments · Health share grid. One row per registered tool that can appear
-- in `sensei.tool_usage_stats`, so the grid can compute
--   share_invoked = tools_invoked_14d / tools_registered
-- per source. This table is the REGISTRY (what tools exist); tool_usage_stats
-- is the USAGE. Config is NOT here — it stays on the typed source tables
-- (`mcp_servers` for MCP). Populated by the per-assistant ToolDiscovery trait:
-- MCP tools from probing (mcp_tool_manifests), built-in tools from the
-- per-harness catalog. Plugins (skills/commands/agents) live in `extensions`,
-- not here — they are not invoked as distinct tools in usage stats.
create table if not exists assistant_tools (
  id                uuid          primary key default gen_random_uuid()
, assistant_family  text          not null                  -- 'claude' | 'zed' | 'cursor' | ...
, source_type       text          not null check (source_type in ('mcp', 'builtin'))
, source_key        text          not null                  -- mcp server key for mcp; 'builtin' for built-ins
, tool_name         text          not null                  -- bare tool name (from probe / catalog)
, invoked_name      text          not null                  -- harness-qualified name; joins tool_usage_stats.tool_name
, description       text
, server_id         uuid          references sensei.mcp_servers(id) on delete cascade  -- set when source_type='mcp'
, discovered_at     timestamptz   not null default now()
, updated_at        timestamptz   not null default now()
);

-- One row per (assistant, source, tool). Re-discovery upserts in place.
create unique index if not exists assistant_tools_unique
    on assistant_tools (assistant_family, source_type, source_key, tool_name);

-- The usage join key (assistant_tools.invoked_name = tool_usage_stats.tool_name).
create index if not exists assistant_tools_invoked_idx
    on assistant_tools (invoked_name);

-- Per-source grouping for the L1 grid (one card per source).
create index if not exists assistant_tools_source_idx
    on assistant_tools (assistant_family, source_type, source_key);

create index if not exists assistant_tools_server_idx
    on assistant_tools (server_id);

comment on table assistant_tools is
'Unified per-assistant tool inventory (MCP + built-in) — the registry half of
the Instruments · Health share grid. tools_registered per source. Config stays
on the typed source tables (mcp_servers). `invoked_name` is the harness-qualified
name that joins sensei.tool_usage_stats.tool_name (e.g.
mcp__plugin_playwright_playwright__browser_click, or a bare built-in like Bash).
Plugins are captured in sensei.extensions, not here.';
