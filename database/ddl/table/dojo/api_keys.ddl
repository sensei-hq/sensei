set search_path to dojo, sensei, extensions;

create table if not exists dojo.api_keys (
  id           uuid        primary key default gen_random_uuid()
, member_id    uuid        not null references dojo.members(id)
, key_hash     text        not null
, label        text
, last_used_at timestamptz
, revoked_at   timestamptz
, created_at   timestamptz not null default now()
);

create index if not exists api_keys_key_hash_idx on dojo.api_keys(key_hash);
-- Covering index for the member_id FK: without it, deleting a dojo.members row
-- seq-scans api_keys to enforce the constraint (Supabase unindexed_foreign_keys).
create index if not exists api_keys_member_idx on dojo.api_keys(member_id);

comment on table dojo.api_keys is
'Bearer API keys. Only the sha256 hash is stored; the plaintext is shown once at
issue. Lookups compare hashes (the compared value is itself a hash, so timing
leaks nothing about the key). revoked_at/disabled_at gate validity.';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.api_keys enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists api_keys_service_only on dojo.api_keys;
create policy api_keys_service_only on dojo.api_keys
    for all to authenticated, anon
    using (false) with check (false);
