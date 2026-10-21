-- RLS for dojo.invite_accept_failures (#311): the system's alone — the accept
-- route (service_role) counts and records failures; no session reads or writes
-- them. Covered by database/tests/dojo/invites_access.sql.
alter table dojo.invite_accept_failures enable row level security;

revoke all on dojo.invite_accept_failures from authenticated, anon;

drop policy if exists invite_accept_failures_service_only on dojo.invite_accept_failures;
create policy invite_accept_failures_service_only on dojo.invite_accept_failures
    for all to authenticated, anon using (false) with check (false);
