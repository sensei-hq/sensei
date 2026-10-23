-- Rate-limited attempts are taken atomically (#311, #308 R5b review 2).
--
-- dojo.take_attempt(principal, action, limit, window) counts the person's
-- attempts at the action inside the window and records this one IN ONE CALL,
-- serialised per (person, action) by a transaction-scoped advisory lock — a
-- check-then-insert from the server let concurrent requests all pass the check.
-- It answers the new attempt's id, or NULL when none are left (the route says
-- 429). dojo.release_attempt(id) gives a slot back (a successful invite accept
-- counts no failure). Each take purges the action's rows older than the window,
-- so the table does not grow without bound. service_role only.
--
-- WHAT BREAKS THIS TEST: a take past the limit; a take that does not record; a
-- release that does not free a slot; old rows kept; no lock held; a session
-- executing either function.

begin;

do $$
begin
    if has_function_privilege('authenticated', 'dojo.take_attempt(uuid, dojo.limited_action, integer, interval)', 'execute')
       or has_function_privilege('anon', 'dojo.take_attempt(uuid, dojo.limited_action, integer, interval)', 'execute')
       or has_function_privilege('authenticated', 'dojo.release_attempt(uuid)', 'execute') then
        raise exception 'attempts: a session may take or release attempts';
    end if;
end $$;

-- An old attempt — and another person's old one — sit in the table.
insert into dojo.request_attempts (principal_id, action, at) values
  ('a0000000-0000-4000-8000-000000000e01', 'azure_connect', now() - interval '2 hours'),
  ('a0000000-0000-4000-8000-000000000e02', 'azure_connect', now() - interval '2 hours'),
  ('a0000000-0000-4000-8000-000000000e02', 'invite_accept', now() - interval '2 hours');

set local role service_role;
do $$
declare
    k  bigint := hashtext('a0000000-0000-4000-8000-000000000e01:azure_connect')::bigint;
    a1 uuid;
    a2 uuid;
    a3 uuid;
begin
    a1 := dojo.take_attempt('a0000000-0000-4000-8000-000000000e01', 'azure_connect', 2, interval '15 minutes');
    if a1 is null then raise exception 'attempts: the first take was refused'; end if;
    if not exists (
        select 1 from pg_locks l
         where l.locktype = 'advisory' and l.pid = pg_backend_pid() and l.granted
           and l.objid::bigint = (k & 4294967295) and l.classid::bigint = ((k >> 32) & 4294967295)
    ) then
        raise exception 'attempts: no advisory lock on (person, action) is held';
    end if;
    -- The two-hour-old rows of this action were purged on the way; the other action's were not.
    if exists (select 1 from dojo.request_attempts where action = 'azure_connect' and at < now() - interval '1 hour') then
        raise exception 'attempts: rows older than the window were kept';
    end if;
    if not exists (select 1 from dojo.request_attempts where action = 'invite_accept') then
        raise exception 'attempts: another action''s rows were purged by this take';
    end if;

    a2 := dojo.take_attempt('a0000000-0000-4000-8000-000000000e01', 'azure_connect', 2, interval '15 minutes');
    a3 := dojo.take_attempt('a0000000-0000-4000-8000-000000000e01', 'azure_connect', 2, interval '15 minutes');
    if a2 is null or a3 is not null then
        raise exception 'attempts: expected the second take to pass and the third to be refused, got % / %', a2, a3;
    end if;
    if (select count(*) from dojo.request_attempts where principal_id = 'a0000000-0000-4000-8000-000000000e01') <> 2 then
        raise exception 'attempts: a refused take was recorded';
    end if;

    perform dojo.release_attempt(a2);
    if dojo.take_attempt('a0000000-0000-4000-8000-000000000e01', 'azure_connect', 2, interval '15 minutes') is null then
        raise exception 'attempts: a released slot was not given back';
    end if;

    -- Another person, the same action: their own budget.
    if dojo.take_attempt('a0000000-0000-4000-8000-000000000e03', 'azure_connect', 2, interval '15 minutes') is null then
        raise exception 'attempts: one person''s attempts limited another';
    end if;
end $$;
reset role;

rollback;
