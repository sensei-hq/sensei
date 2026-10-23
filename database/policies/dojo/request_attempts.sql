-- RLS for dojo.request_attempts (#311, #308 R5b): the system's alone — the
-- rate-limited routes (service_role) count and record attempts; no session reads
-- or writes them. Covered by database/tests/dojo/invites_access.sql.
alter table dojo.request_attempts enable row level security;

revoke all on dojo.request_attempts from authenticated, anon;

drop policy if exists request_attempts_service_only on dojo.request_attempts;
create policy request_attempts_service_only on dojo.request_attempts
    for all to authenticated, anon using (false) with check (false);
