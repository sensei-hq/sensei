set search_path to dojo, extensions;

-- Take one rate-limited attempt (#311, #308 R5b review 2): count the person's
-- attempts at the action inside the window and record this one, atomically —
-- serialised per (person, action) by a transaction-scoped advisory lock, so
-- concurrent requests cannot all pass a check made before any of them inserts.
-- Answers the new attempt's id, or NULL when none are left (the caller says
-- 429). Purges the action's rows older than the window on the way, so the table
-- holds only what a window can count. service_role only; the limits are the
-- server's (rate-limit.ts).
create or replace function dojo.take_attempt(
    p_principal uuid,
    p_action    dojo.limited_action,
    p_limit     integer,
    p_window    interval
)
    returns uuid
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_id uuid;
begin
    if p_principal is null or coalesce(p_limit, 0) < 1 or p_window is null or p_window <= interval '0' then
        raise exception 'an attempt needs a person, a limit and a window'
            using errcode = 'invalid_parameter_value';
    end if;

    perform pg_advisory_xact_lock(hashtext(p_principal::text || ':' || p_action::text));

    delete from dojo.request_attempts a where a.action = p_action and a.at < now() - p_window;

    if (select count(*) from dojo.request_attempts a
         where a.principal_id = p_principal and a.action = p_action and a.at >= now() - p_window) >= p_limit then
        return null;
    end if;

    insert into dojo.request_attempts (principal_id, action) values (p_principal, p_action)
    returning id into v_id;
    return v_id;
end;
$$;

revoke all on function dojo.take_attempt(uuid, dojo.limited_action, integer, interval) from public, anon, authenticated;
grant execute on function dojo.take_attempt(uuid, dojo.limited_action, integer, interval) to service_role;

comment on function dojo.take_attempt(uuid, dojo.limited_action, integer, interval) is
'Count and record one rate-limited attempt atomically (advisory lock per person and action), purging the action''s rows older than the window; NULL when none are left (#311, #308 R5b). service_role only.';
