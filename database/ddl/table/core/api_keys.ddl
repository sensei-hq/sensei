set search_path to core, extensions;

create table if not exists api_keys (
  id              uuid        primary key default gen_random_uuid()
, account_id      uuid        not null references core.accounts(id) on delete cascade
, provider        text        not null
, label           text        not null
, encrypted_value bytea       not null
, created_at      timestamptz not null default now()
);

create index if not exists api_keys_account_idx on api_keys(account_id);

alter table api_keys enable row level security;

create policy "account_members_own_keys" on api_keys
  using (
    account_id in (
      select account_id from core.profile_accounts where user_id = auth.uid()
    )
  );

comment on table api_keys is
'LLM provider API keys encrypted with the account DEK.
- provider: e.g. ''anthropic'', ''openai''
- encrypted_value: AES-256-GCM ciphertext; never returned in plaintext via API
- RLS: only account members can see their own account keys';
