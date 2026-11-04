set search_path to dojo, extensions;

-- Whether text is an ISO-8601 instant with an offset that the timestamptz
-- cast accepts (F4/M2): the `repository_metrics_authored_at_iso` CHECK agrees
-- with the cast the workspace rollup relies on, so a row the CHECK admits
-- never fails the read. The shape test lives inside (a regex alone admits
-- Feb 30; a bare cast depends on DateStyle for non-ISO shapes), which is what
-- makes immutable sound: the answer never depends on session settings. Plain
-- function (no data access), so PUBLIC execute stays as Postgres ships it —
-- revoking it could fail the CHECK for whichever role writes.
create or replace function dojo.is_iso_instant(p_value text)
    returns boolean
    language plpgsql
    immutable
    set search_path = dojo, pg_temp
as $$
begin
    if p_value !~ '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$' then
        return false;
    end if;
    perform p_value::timestamptz;
    return true;
exception
    when invalid_datetime_format or datetime_field_overflow
      or invalid_time_zone_displacement_value then
        return false;
end;
$$;

grant execute on function dojo.is_iso_instant(text) to authenticated;

comment on function dojo.is_iso_instant(text) is
'Whether text is an ISO-8601 instant with an offset that parses as a
timestamptz (F4/M2). The authored_at CHECK calls it so the constraint agrees
with the cast the workspace rollup relies on.';
