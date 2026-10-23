set search_path to dojo, extensions;

-- Give back an attempt dojo.take_attempt recorded (#311, #308 R5b review 2): an
-- invite accept that succeeded counts no failure. A no-op for an id already
-- gone. service_role only.
create or replace function dojo.release_attempt(p_id uuid)
    returns void
    language sql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
    delete from dojo.request_attempts a where a.id = p_id;
$$;

revoke all on function dojo.release_attempt(uuid) from public, anon, authenticated;
grant execute on function dojo.release_attempt(uuid) to service_role;

comment on function dojo.release_attempt(uuid) is
'Give back one rate-limited attempt (a successful invite accept counts no failure) (#311, #308 R5b). service_role only.';
