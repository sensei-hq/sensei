set search_path to dojo, extensions;

-- One row per failed invite accept, per person (#311). The accept route counts a
-- person's failures in the last 15 minutes and refuses further attempts past the
-- limit, so a token cannot be guessed at speed. The system's alone: no session
-- reads or writes it (policies/dojo/invite_accept_failures.sql).
create table if not exists dojo.invite_accept_failures (
  id           uuid        primary key default gen_random_uuid()
, principal_id uuid        not null
, failed_at    timestamptz not null default now()
);

create index if not exists invite_accept_failures_principal_idx
    on dojo.invite_accept_failures(principal_id, failed_at desc);

comment on table dojo.invite_accept_failures is
'Failed invite accepts, per person (#311): the accept route refuses further attempts after too many in a short window. service_role only.';
