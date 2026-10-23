set search_path to dojo, extensions;

-- One row per rate-limited attempt, per person and action (#311, generalised in
-- #308 R5b). A route counts a person's attempts at its action in the window and
-- refuses past the limit (dojo/src/lib/server/rate-limit.ts). The system's
-- alone: no session reads or writes it (policies/dojo/request_attempts.sql).
create table if not exists dojo.request_attempts (
  id           uuid               primary key default gen_random_uuid()
, principal_id uuid               not null
, action       dojo.limited_action not null
, at           timestamptz        not null default now()
);

create index if not exists request_attempts_principal_idx
    on dojo.request_attempts(principal_id, action, at desc);

comment on table dojo.request_attempts is
'Rate-limited attempts, per person and action (#311 invite accepts, #308 R5b Azure DevOps connects): the route refuses further attempts after too many in a short window. service_role only.';
comment on column dojo.request_attempts.principal_id
     is 'The person who made the attempt.';
comment on column dojo.request_attempts.action
     is 'Which limited action: invite_accept (a failed accept) or azure_connect (a connect-flow step).';
comment on column dojo.request_attempts.at
     is 'When the attempt was made; only attempts inside the window count.';

alter table dojo.request_attempts enable row level security;
