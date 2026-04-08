-- settings
-- Key-value configuration store. Global settings have project_id = null.
-- Per-project settings override global defaults.
-- value is always a JSON-encoded string (even for scalars: '"value"', '42', 'true').

create table if not exists settings (
  id          text not null primary key
, project_id  text references projects(id) on delete cascade  -- null = global
, key         text not null
, value       text not null   -- JSON-encoded
, modified_at text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text not null default 'system'
, unique(project_id, key)
);

-- Well-known keys (application enforces these):
--
--  Global:
--    default_coordinator          "claude-code"
--    ollama_url                   "http://localhost:11434"
--    ollama_model                 "gemma3:4b"
--    local_inference_enabled      true
--    telemetry_enabled            false
--    theme                        "system"
--
--  Per-project:
--    index_excludes               ["node_modules","dist",".git"]
--    ranking_strategy             "diff-first-bfs"
--    token_budget                 8000

create index if not exists settings_project_key_idx on settings(project_id, key);
