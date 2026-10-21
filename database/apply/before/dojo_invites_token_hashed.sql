-- Let dojo.invites store only a hash of its token (#311) on a database that still
-- has the plain `token` column.
--
-- WHY THIS EXISTS. The design drops `token` and adds `token_hash`, with a check
-- that every PENDING invite has a hash. A pending invite issued before #311 has
-- only its plain token, which the reconcile drops — it could never be accepted
-- again, and it would fail the check. So pending invites are deleted first, ONLY
-- while the old column exists (a no-op afterwards); accepted invites are history
-- and are kept. An admin re-issues any invite still wanted (pre-v1, approved).
--
-- No-op where dojo.invites is absent (the daemon's own database).
do $$
begin
    if exists (select 1 from information_schema.columns
                where table_schema = 'dojo' and table_name = 'invites' and column_name = 'token') then
        delete from dojo.invites where accepted_at is null;
    end if;
end $$;
