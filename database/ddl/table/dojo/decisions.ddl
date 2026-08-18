set search_path to dojo, extensions;

create table if not exists dojo.decisions (
  id                 uuid                 primary key default gen_random_uuid()
, tenant_id          uuid                 not null references dojo.tenants(id)
, artifact_id        uuid                 not null references dojo.artifacts(id)
, triage_id          uuid                 references dojo.triage_queue(id)
, maintainer_id      uuid
, status             dojo.decision_status not null
, reason             text
, distribution_scope jsonb
, regression_note    text
, automated          boolean              not null default false
, created_at         timestamptz          not null default now()
);

create index if not exists decisions_tenant_idx on dojo.decisions(tenant_id);
create index if not exists decisions_artifact_idx on dojo.decisions(artifact_id);
-- Covering index for the triage_id FK (nullable) — avoids a seq-scan on
-- dojo.triage_queue deletes (Supabase unindexed_foreign_keys).
create index if not exists decisions_triage_idx on dojo.decisions(triage_id) where triage_id is not null;

comment on table dojo.decisions is
'A named triage verdict (approve/revise/decline) with a clear trail. Approve
requires distribution_scope; decline requires a non-empty reason (enforced in
the API). automated=true marks the collective auto-approve path (maintainer_id
null). Publishing an approval sets the artifact live and fans out downstream.';

comment on column dojo.decisions.maintainer_id
     is 'user_id of the deciding maintainer (null when automated).';
comment on column dojo.decisions.status
     is 'approve | revise | decline.';
comment on column dojo.decisions.reason
     is 'Required (non-empty) for decline; optional note otherwise.';
comment on column dojo.decisions.distribution_scope
     is 'Required for approve — who receives it (all-org | team X | stack Y). No default (unsafe to broadcast).';
comment on column dojo.decisions.regression_note
     is 'The regression note published alongside an approval.';
comment on column dojo.decisions.automated
     is 'True when the automated trust process auto-approved this (no human maintainer).';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.decisions enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists decisions_service_only on dojo.decisions;
create policy decisions_service_only on dojo.decisions
    for all to authenticated, anon
    using (false) with check (false);
