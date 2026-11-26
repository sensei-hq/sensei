-- Loosen two live FKs the archive outgrows (#393): a live backlog item may
-- name a lane that has moved to the archive, and a live action may answer an
-- archived one. Drops the constraints only where they still exist: a no-op on
-- a fresh deploy (whose DDL never declares them) and once landed.
do $$
begin
    if exists (select 1 from pg_catalog.pg_constraint where conname = 'items_adder_fk') then
        alter table relay.items drop constraint items_adder_fk;
    end if;
    if exists (select 1 from pg_catalog.pg_constraint where conname = 'actions_reply_fk') then
        alter table relay.actions drop constraint actions_reply_fk;
    end if;
end $$;
