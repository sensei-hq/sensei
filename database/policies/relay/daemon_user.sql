-- RLS for relay.daemon_user: the machine-local id, readable by its owner like
-- every relay table (the leases precedent — also never synced). In practice no
-- dōjō JWT sub ever equals a machine id, so the dōjō reads nothing here; the
-- daemon reads as owner. Writes are the daemon owner's.
alter table relay.daemon_user enable row level security;

drop policy if exists daemon_user_owner_read on relay.daemon_user;
create policy daemon_user_owner_read on relay.daemon_user
    for select to authenticated
    using (user_id::text = (nullif(current_setting('request.jwt.claims', true), '')::json ->> 'sub'));

grant select on relay.daemon_user to authenticated;
