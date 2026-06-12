set search_path to sensei, extensions;

create table if not exists knowledge_sources (
  id             uuid        primary key default gen_random_uuid()
, kind           text        not null      -- hive_mind | mcp | rest | webhook (only hive_mind wired @ MVP)
, name           text        not null
, url            text        not null
, namespace_id   uuid        references sensei.namespaces(id) on delete set null  -- null = all shareable namespaces
, credential_ref text        not null      -- Keychain entry id; the API key lives in the OS keychain, never in PG
, direction      text        not null default 'both'   -- push | pull | both
, last_seq       bigint      not null default 0         -- pull cursor for this source
, enabled        boolean     not null default true
, created_at     timestamptz not null default now()
);

comment on table knowledge_sources is
'Registered federation endpoints (governance P4). Mirrors gateway-router
registration: the row holds connection metadata; the API key is in the OS
Keychain referenced by credential_ref. direction gates push vs pull; last_seq
is the per-source monotonic pull cursor.';
