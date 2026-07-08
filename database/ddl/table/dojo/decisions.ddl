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
