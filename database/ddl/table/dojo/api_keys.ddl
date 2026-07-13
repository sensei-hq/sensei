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

comment on table dojo.api_keys is
'Bearer API keys. Only the sha256 hash is stored; the plaintext is shown once at
issue. Lookups compare hashes (the compared value is itself a hash, so timing
leaks nothing about the key). revoked_at/disabled_at gate validity.';
