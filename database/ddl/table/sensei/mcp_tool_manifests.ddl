set search_path to sensei, extensions;

-- #84 T2 Slice B — Cached tool manifests per MCP server. The Playground tab
-- lists every configured server's tools; live-probing on each open would
-- add up to seconds of stdio + subprocess churn per view. Instead we cache
-- the response of the server's `tools/list` JSON-RPC call here and refresh
-- on TTL expiry or explicit user request.
--
-- One row per mcp_server. `tools` is the raw JSON-RPC response from the
-- server (an array of tool descriptors, each with name, description,
-- inputSchema). `probed_at` + `ttl_seconds` drive staleness. `error`
-- carries a short failure message when the last probe didn't complete.
create table if not exists mcp_tool_manifests (
  id              uuid          primary key default gen_random_uuid()
, server_id       uuid          not null unique references sensei.mcp_servers(id) on delete cascade
, tools           jsonb         not null default '[]'
, tool_count      int           not null default 0
, probed_at       timestamptz   not null default now()
, ttl_seconds     int           not null default 900     -- 15 min default
, error           text                                    -- short string when the last probe failed
, protocol_version text                                   -- MCP protocol version reported by initialize
, server_name     text                                    -- serverInfo.name from initialize response
, server_version  text                                    -- serverInfo.version from initialize response
);

create index if not exists mcp_tool_manifests_probed_at_idx
    on mcp_tool_manifests (probed_at);

comment on table mcp_tool_manifests is
'#84 T2 Slice B. Cached tool_list JSON-RPC responses per MCP server. Refreshed
on TTL expiry or explicit request. `server_id` is UNIQUE so a manifest re-probe
overwrites in place rather than accumulating.';
