set search_path to dojo, sensei, extensions;

create table if not exists dojo.members (
  id           uuid        primary key default gen_random_uuid()
, name         text        not null
, email        text
, role         text        not null default 'member'
, disabled_at  timestamptz
, created_at   timestamptz not null default now()
);

comment on table dojo.members is
'A federation participant. role gates the REST API: member=pull, publisher=pull+publish,
admin=publisher+manage members/keys/namespaces+audit. Instance-global roles (one dojo
instance == one org); per-namespace ACLs are a deferred extension.';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.members enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists members_service_only on dojo.members;
create policy members_service_only on dojo.members
    for all to authenticated, anon
    using (false) with check (false);
