set search_path to dojo, extensions;

create table if not exists dojo.upstream_queue (
  id            uuid                primary key default gen_random_uuid()
, tenant_id     uuid                not null references dojo.tenants(id)
, membership_id uuid                not null references dojo.memberships(id)
, artifact_id   uuid                references dojo.artifacts(id)
, kind          dojo.artifact_kind  not null
, title         text                not null
, body          text                not null
, scope         jsonb               not null default '{}'
, attribution   jsonb               not null default '{}'
, signature     text                not null
, state         dojo.upstream_state not null default 'queued'
, created_at    timestamptz         not null default now()
, updated_at    timestamptz         not null default now()
, sent_at       timestamptz
);

create index if not exists upstream_queue_tenant_idx on dojo.upstream_queue(tenant_id);
create index if not exists upstream_queue_membership_idx on dojo.upstream_queue(membership_id);
create index if not exists upstream_queue_state_idx on dojo.upstream_queue(tenant_id, state);

comment on table dojo.upstream_queue is
'Pending upstream contributions from a membership, before a batch fires. Backs
the personal Share-review surface (what is about to leave, scoped + attributed).
Held items re-queue for the next batch; edited items ship with the edit; the
mandatory client-work dereference cannot be overridden here. Durability note:
the daemon holds its own local durable copy so a federation drop never loses
in-flight items (C6); this is the server-side view of the queue.';

comment on column dojo.upstream_queue.artifact_id
     is 'Set once the contribution has been materialised into dojo.artifacts; null while still a raw queued item.';
comment on column dojo.upstream_queue.state
     is 'queued | held | edited | sent.';
comment on column dojo.upstream_queue.sent_at
     is 'When the item was pushed upstream (state=sent).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.upstream_queue enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists upstream_queue_service_only on dojo.upstream_queue;
create policy upstream_queue_service_only on dojo.upstream_queue
    for all to authenticated, anon
    using (false) with check (false);
