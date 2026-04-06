set search_path to core, extensions;

create table if not exists api_keys (
  id              uuid        primary key default gen_random_uuid()
, account_id      uuid        not null references core.accounts(id) on delete cascade
, provider        text        not null
, label           text        not null
, encrypted_value bytea       not null
, created_at      timestamptz not null default now()
, modified_at     timestamptz not null default now()
, modified_by     text        not null default current_user
);

create index if not exists api_keys_account_idx on api_keys(account_id);

comment on table api_keys is
'LLM provider API keys encrypted with the account DEK.
- provider: e.g. ''anthropic'', ''openai''
- encrypted_value: AES-256-GCM ciphertext; never returned in plaintext via API
- RLS: only account members can see their own account keys';

comment on column api_keys.id is 'Surrogate primary key (UUID).';
comment on column api_keys.account_id is 'Account that owns this API key.';
comment on column api_keys.provider is 'LLM provider name (e.g. anthropic, openai) this key authenticates against.';
comment on column api_keys.label is 'Human-readable label for the key, shown in the account settings UI.';
comment on column api_keys.encrypted_value is 'AES-256-GCM ciphertext of the raw API key; never returned in plaintext via the API.';
comment on column api_keys.created_at is 'Timestamp when the row was first created.';
comment on column api_keys.modified_at is 'Timestamp of the last modification to this row.';
comment on column api_keys.modified_by is 'Identity (user, role, or service) that last modified this row.';
