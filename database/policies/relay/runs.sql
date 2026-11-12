-- RLS for relay.runs: a row is readable by its owner, and by others only by an
-- explicit share (none yet). Writes are the daemon owner's.
alter table relay.runs enable row level security;

drop policy if exists runs_owner_read on relay.runs;
create policy runs_owner_read on relay.runs
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay.runs to authenticated;
