set search_path to dojo, extensions;

create table if not exists dojo.triage_queue (
  id                  uuid              primary key default gen_random_uuid()
, tenant_id           uuid              not null references dojo.tenants(id)
, artifact_id         uuid              not null references dojo.artifacts(id)
, signature           text              not null
, owner_scope         jsonb             not null default '{}'
, confidence          numeric(4,3)
, contributor_count   integer           not null default 1
, similarity          numeric(4,3)
, nearest_artifact_id uuid              references dojo.artifacts(id)
, state               dojo.triage_state not null default 'queued'
, created_at          timestamptz       not null default now()
, updated_at          timestamptz       not null default now()
);

create index if not exists triage_queue_tenant_idx on dojo.triage_queue(tenant_id);
create index if not exists triage_queue_state_idx on dojo.triage_queue(tenant_id, state);
create index if not exists triage_queue_signature_idx on dojo.triage_queue(tenant_id, signature);
create index if not exists triage_queue_artifact_idx on dojo.triage_queue(artifact_id);

comment on table dojo.triage_queue is
'The accumulate/triage stage: candidate artifacts pooled on the Dōjō server,
clustered and deduped by signature, waiting for a maintainer. A maintainer sees
only rows within scopes they own (owner_scope). Scored for the queue view
(confidence, contributor_count, similarity to the nearest existing artifact).';

comment on column dojo.triage_queue.owner_scope
     is 'The scope that owns this candidate — filters the queue to a maintainer''s owned scopes.';
comment on column dojo.triage_queue.confidence
     is 'Aggregate confidence 0..1 used for the confidence filter/sort.';
comment on column dojo.triage_queue.contributor_count
     is 'Distinct contributing users (drives collective scoring and the auto-approve bar).';
comment on column dojo.triage_queue.similarity
     is 'Similarity 0..1 to the nearest existing artifact (the similarity chip); high values offer a merge.';
comment on column dojo.triage_queue.nearest_artifact_id
     is 'The existing artifact this candidate most resembles (the merge target when similarity is high).';
comment on column dojo.triage_queue.state
     is 'queued | in_review | auto_approved | resolved | merged.';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.triage_queue enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists triage_queue_service_only on dojo.triage_queue;
create policy triage_queue_service_only on dojo.triage_queue
    for all to authenticated, anon
    using (false) with check (false);
