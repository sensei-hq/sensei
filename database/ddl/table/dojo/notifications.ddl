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
