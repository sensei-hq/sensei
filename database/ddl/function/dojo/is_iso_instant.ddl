set search_path to dojo, extensions;

-- Whether text is an instant the timestamptz cast accepts (F4/M2): the
-- `repository_metrics_authored_at_iso` CHECK agrees with the cast the
-- workspace rollup relies on, so a row the CHECK admits never fails the read.
-- A shape regex alone admits calendar-invalid instants (Feb 30) that the cast
-- refuses. Immutable is sound: with an offset required, the answer never
-- depends on the session TimeZone. Plain function (no data access), so PUBLIC
-- execute stays as Postgres ships it — revoking it could fail the CHECK for
-- whichever role writes.
create or replace function dojo.is_iso_instant(p_value text)
    returns boolean
    language plpgsql
    immutable
    set search_path = dojo, pg_temp
as $$
begin
    perform p_value::timestamptz;
    return true;
exception
    when invalid_datetime_format or datetime_field_overflow then
        return false;
end;
$$;

grant execute on function dojo.is_iso_instant(text) to authenticated;

comment on function dojo.is_iso_instant(text) is
'Whether text parses as a timestamptz (F4/M2). The authored_at CHECK calls it
so the constraint agrees with the cast the workspace rollup relies on.';
