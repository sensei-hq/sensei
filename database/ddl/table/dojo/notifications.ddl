set search_path to dojo, extensions;

create table if not exists dojo.notifications (
  id            uuid        primary key default gen_random_uuid()
, tenant_id     uuid        not null references dojo.tenants(id)
, membership_id uuid        references dojo.memberships(id)
, user_id       uuid
, kind          text        not null
, title         text        not null
, body          text
, payload       jsonb       not null default '{}'
, read_at       timestamptz
, created_at    timestamptz not null default now()
);

create index if not exists notifications_tenant_idx on dojo.notifications(tenant_id);
create index if not exists notifications_membership_idx on dojo.notifications(membership_id);
create index if not exists notifications_unread_idx on dojo.notifications(user_id) where read_at is null;

comment on table dojo.notifications is
'Per-member console notifications about Dōjō activity — e.g. "3 new items in
your queue" (maintainer), "your contribution was approved" (contributor),
"queue depth high" / "incident open past SLA" (admin/lead). read_at gates
the unread badge. (Daemon-side "new upgrade arrived" toasts are a separate
sensei-side concern handled when downstream artifacts land — C7.)';

comment on column dojo.notifications.membership_id
     is 'Recipient membership (null for a tenant-wide broadcast).';
comment on column dojo.notifications.user_id
     is 'Recipient user_id (denormalised for unread lookups across memberships).';
comment on column dojo.notifications.kind
     is 'Notification type (e.g. triage_pending, contribution_approved, queue_depth, incident_sla).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.notifications enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists notifications_service_only on dojo.notifications;
create policy notifications_service_only on dojo.notifications
    for all to authenticated, anon
    using (false) with check (false);
