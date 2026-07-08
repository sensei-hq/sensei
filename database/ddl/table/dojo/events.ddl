set search_path to dojo, extensions;

create table if not exists dojo.events (
  id          bigserial   primary key
, tenant_id   uuid        not null references dojo.tenants(id)
, ts          timestamptz not null default now()
, actor_id    uuid
, artifact_id uuid        references dojo.artifacts(id)
, action      text        not null
, detail      jsonb       not null default '{}'
);

create index if not exists events_tenant_idx on dojo.events(tenant_id);
create index if not exists events_artifact_idx on dojo.events(artifact_id);

comment on table dojo.events is
'Append-only operational log of the loop — triage, evaluate, decide, distribute,
and attribution actions. Serves the maintainer console audit trail and the Today
downstream lane. Row count for a maintainer equals the number of actions they
performed. (dojo.audit_events is the separate compliance/non-repudiation log for
admin + client-lead consoles.)';

comment on column dojo.events.actor_id
     is 'user_id of the actor (maintainer/system) that produced the event.';
comment on column dojo.events.action
     is 'Event verb (e.g. queued, evaluated, approved, revised, declined, distributed, dereferenced).';
comment on column dojo.events.detail
     is 'Structured event context (scope, reason, counts, before/after).';
