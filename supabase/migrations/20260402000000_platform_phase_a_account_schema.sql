-- supabase/migrations/20260402000000_platform_phase_a_account_schema.sql
-- Platform Phase A: Multi-account schema foundation

-- ─── 1. Core schema ────────────────────────────────────────────────────────

create schema if not exists core;

-- ─── 2. Accounts ──────────────────────────────────────────────────────────

create table if not exists core.accounts (
  id           uuid primary key default gen_random_uuid(),
  account_type text not null check (account_type in ('individual', 'team', 'platform')),
  slug         text unique,
  domain       text,          -- email domain for auto-assignment (team accounts)
  oss_shard    smallint,      -- 0–15, set by trigger for individual accounts
  is_platform  boolean not null default false,
  status       text not null default 'active' check (status in ('active', 'suspended', 'deleted')),
  created_at   timestamptz not null default now()
);

create index if not exists accounts_domain_idx on core.accounts (domain) where domain is not null;
create index if not exists accounts_type_idx   on core.accounts (account_type);

-- ─── 3. Profile → Account mapping ─────────────────────────────────────────

create table if not exists core.profile_accounts (
  id          uuid primary key default gen_random_uuid(),
  user_id     uuid not null references auth.users(id) on delete cascade,
  account_id  uuid not null references core.accounts(id) on delete cascade,
  role        text not null default 'user' check (role in ('platform_admin', 'account_admin', 'user')),
  created_at  timestamptz not null default now(),
  unique (user_id, account_id)
);

create index if not exists profile_accounts_user_idx    on core.profile_accounts (user_id);
create index if not exists profile_accounts_account_idx on core.profile_accounts (account_id);

-- ─── 4. Per-account encryption keys (DEK) ─────────────────────────────────

create table if not exists core.account_keys (
  id             uuid primary key default gen_random_uuid(),
  account_id     uuid not null unique references core.accounts(id) on delete cascade,
  wrapped_dek    bytea not null,   -- AES-256-GCM: [12-byte IV][16-byte tag][32-byte ciphertext]
  kek_version    int  not null default 1,
  created_at     timestamptz not null default now(),
  rotated_at     timestamptz
);

-- ─── 5. Team invitations ───────────────────────────────────────────────────

create table if not exists core.invitations (
  id          uuid primary key default gen_random_uuid(),
  account_id  uuid not null references core.accounts(id) on delete cascade,
  email       text not null,
  token       text not null unique default encode(gen_random_bytes(24), 'base64'),
  role        text not null default 'user' check (role in ('account_admin', 'user')),
  invited_by  uuid references auth.users(id),
  accepted_at timestamptz,
  expires_at  timestamptz not null default now() + interval '7 days',
  created_at  timestamptz not null default now()
);

create index if not exists invitations_account_idx on core.invitations (account_id);
create index if not exists invitations_token_idx   on core.invitations (token);

-- ─── 6. Add account_id to existing tables (nullable initially) ───────────

alter table sensei.repos           add column if not exists account_id uuid references core.accounts(id);
alter table sensei.sessions        add column if not exists account_id uuid references core.accounts(id);
alter table sensei.task_sessions   add column if not exists account_id uuid references core.accounts(id);
alter table sensei.snapshots       add column if not exists account_id uuid references core.accounts(id);
alter table sensei.symbol_map      add column if not exists account_id uuid references core.accounts(id);
alter table sensei.chunks          add column if not exists account_id uuid references core.accounts(id);
alter table sensei.doc_sections    add column if not exists account_id uuid references core.accounts(id);
alter table sensei.task_turns      add column if not exists account_id uuid references core.accounts(id);
alter table sensei.memories        add column if not exists account_id uuid references core.accounts(id);
alter table sensei.pattern_usages  add column if not exists account_id uuid references core.accounts(id);
alter table sensei.api_requests    add column if not exists account_id uuid references core.accounts(id);

-- ─── 7. OSS shard assignment trigger (individual accounts) ────────────────

create or replace function core.assign_oss_shard()
returns trigger language plpgsql as $$
begin
  if new.account_type = 'individual' then
    new.oss_shard := abs(hashtext(new.id::text)) % 16;
  end if;
  return new;
end;
$$;

drop trigger if exists assign_oss_shard_trigger on core.accounts;
create trigger assign_oss_shard_trigger
  before insert on core.accounts
  for each row execute function core.assign_oss_shard();

-- ─── 8. Domain auto-assignment trigger ────────────────────────────────────

create or replace function core.assign_account_by_domain()
returns trigger language plpgsql security definer as $$
declare
  v_domain     text;
  v_account_id uuid;
begin
  -- Extract email domain
  v_domain := split_part(new.email, '@', 2);

  -- Try domain match → team account
  select id into v_account_id
  from core.accounts
  where domain = v_domain and account_type = 'team' and status = 'active'
  limit 1;

  -- No match → create individual account
  if v_account_id is null then
    insert into core.accounts (account_type)
    values ('individual')
    returning id into v_account_id;
  end if;

  -- Map profile → account
  insert into core.profile_accounts (user_id, account_id, role)
  values (new.id, v_account_id, 'user')
  on conflict (user_id, account_id) do nothing;

  return new;
end;
$$;

drop trigger if exists assign_account_by_domain_trigger on auth.users;
create trigger assign_account_by_domain_trigger
  after insert on auth.users
  for each row execute function core.assign_account_by_domain();

-- ─── 9. Seed: platform + team + individual accounts ───────────────────────

do $$
declare
  v_platform_id uuid;
  v_team_id     uuid;
  v_ind_id      uuid;
begin
  -- Platform account (singleton)
  insert into core.accounts (account_type, slug, is_platform)
  values ('platform', 'sensei-platform', true)
  on conflict (slug) do nothing
  returning id into v_platform_id;

  -- Example team account (domain-matched)
  insert into core.accounts (account_type, slug, domain)
  values ('team', 'acme-team', 'acme.example')
  on conflict (slug) do nothing
  returning id into v_team_id;

  -- Example individual account
  insert into core.accounts (account_type, slug)
  values ('individual', 'demo-individual')
  on conflict (slug) do nothing
  returning id into v_ind_id;
end;
$$;
