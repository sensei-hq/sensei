set search_path to sensei, extensions;

create table if not exists knowledge_sources (
  id             uuid        primary key default gen_random_uuid()
, kind           text        not null      -- hive_mind | mcp | rest | webhook (only hive_mind wired @ MVP)
, name           text        not null
, url            text        not null      -- Dōjō rules tenant base {registry}/v1/t/{origin}/{org}; the daemon appends /rules (D1)
, namespace_id   uuid        references sensei.namespaces(id) on delete set null  -- null = all shareable namespaces
, credential_ref text        not null      -- Keychain entry id; the per-membership device token lives in the OS keychain, never in PG
, direction      text        not null default 'both'   -- push | pull | both
, last_seq       bigint      not null default 0         -- pull cursor for this source
, enabled        boolean     not null default true
, created_at     timestamptz not null default now()
);

-- Covering index for the namespace_id FK (nullable) — avoids a seq-scan when a
-- referenced namespace is deleted (on delete set null).
create index if not exists knowledge_sources_namespace_id_idx
    on knowledge_sources(namespace_id) where namespace_id is not null;

comment on table knowledge_sources is
'Registered rules-federation endpoints (governance P4). Mirrors gateway-router
registration: the row holds connection metadata; the credential is in the OS
Keychain referenced by credential_ref. D1: url is the Worker tenant base
{registry}/v1/t/{origin}/{org} (the daemon appends /rules) and the credential is
the per-membership device token — the same tenant-path + device-token plane the
artifacts client (dojo_memberships) uses. direction gates push vs pull; last_seq
is the per-source monotonic pull cursor.';
