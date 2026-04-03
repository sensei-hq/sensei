-- supabase/migrations/20260402000001_platform_phase_d_sync_and_api_keys.sql
-- Platform Phase D: Data sync payload table + API key storage

-- ─── 1. Sync sessions table (receives scrubbed summaries from collector) ───

create table if not exists sensei.sync_sessions (
  id               uuid primary key default gen_random_uuid(),
  account_id       uuid not null references core.accounts(id) on delete cascade,
  session_id       text not null,
  repo_slug        text not null,   -- SHA-256 hash, no plaintext
  stack            text[],
  ftr_score        numeric(5,3),
  token_cost       numeric(12,6),
  duration_ms      int,
  tool_call_count  int not null default 0,
  error_count      int not null default 0,
  snapshot_count   int not null default 0,
  completed_cleanly boolean not null default false,
  recorded_at      timestamptz not null,
  received_at      timestamptz not null default now(),
  unique (account_id, session_id)
);

create index if not exists sync_sessions_account_idx     on sensei.sync_sessions (account_id);
create index if not exists sync_sessions_recorded_at_idx on sensei.sync_sessions (recorded_at);

-- ─── 2. API keys table (encrypted values, never returned in GET) ───────────

create table if not exists public.api_keys (
  id              uuid primary key default gen_random_uuid(),
  account_id      uuid not null references core.accounts(id) on delete cascade,
  provider        text not null,          -- 'anthropic', 'openai', etc.
  label           text not null,
  encrypted_value bytea not null,         -- encrypted with account DEK
  created_at      timestamptz not null default now()
);

create index if not exists api_keys_account_idx on public.api_keys (account_id);

-- Row-level security: only account members can see their own keys
alter table public.api_keys enable row level security;

create policy "account_members_own_keys" on public.api_keys
  using (
    account_id in (
      select account_id from core.profile_accounts where user_id = auth.uid()
    )
  );
