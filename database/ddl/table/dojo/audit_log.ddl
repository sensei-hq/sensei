set search_path to dojo, sensei, extensions;

create table if not exists dojo.audit_log (
  id         bigserial    primary key
, ts         timestamptz  not null default now()
, member_id  uuid         references dojo.members(id)
, action     text         not null
, target     text
, detail     jsonb        not null default '{}'
);

-- Covering index for the member_id FK (nullable) so deleting a dojo.members row
-- doesn't seq-scan audit_log (Supabase unindexed_foreign_keys). Partial on
-- not-null since only referencing rows matter for FK enforcement.
create index if not exists audit_log_member_idx on dojo.audit_log(member_id) where member_id is not null;

comment on table dojo.audit_log is
'Append-only audit of mutating API actions, stamped by the auth middleware.';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.audit_log enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists audit_log_service_only on dojo.audit_log;
create policy audit_log_service_only on dojo.audit_log
    for all to authenticated, anon
    using (false) with check (false);
