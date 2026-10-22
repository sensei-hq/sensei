-- A dōjō has no Clients section (#307 R7, D34, user 2026-10-22): a client is
-- just another dōjō. Drops the engagement and incident tables and the
-- confidentiality ledger built on them, rows included (pre-v1: dropping dōjō
-- data is approved).
--
-- WHY A HOOK. The release runs `dbd reconcile --scope dojo --allow-destructive`
-- WITHOUT `--prune`, so a table that leaves the design is never dropped by dbd —
-- `dojo.engagements` and `dojo.incidents` would sit orphaned in production, RLS
-- on and no policy. `--prune` is not added to the release: it drops EVERY
-- orphaned table in a managed schema, a reach this one change has no business
-- having.
--
-- WHAT IT LEAVES TO THE RECONCILE. dbd plans its changes BEFORE this runs and
-- then executes them without IF EXISTS, so anything it plans must still be here
-- for it: the `engagement_id` columns on audit_events/artifacts and their
-- indexes are dropped by the reconcile itself (rehearsed: dropping a column here
-- failed the reconcile with "column does not exist"), and the three enums by
-- apply/after/retired_types.sql (the reconcile leaves an orphaned enum). This
-- drops only what the reconcile cannot or would not:
--   - the ledger view, which depends on `audit_events.engagement_id` and would
--     block the column drop;
--   - the lead's ledger read policy, while the function it calls still exists
--     (retired_functions.sql drops the function after; the policies step
--     recreates `audit_events_read` admin-only — until then the table has RLS on
--     and no read policy, so it reads EMPTY, never open);
--   - the two tables, and the foreign keys into dojo.engagements. The DDL never
--     named those constraints, so dbd cannot drop them ("has no constraint name
--     — drop it manually"); they are found by what they reference.
-- With the tables gone, nothing uses the enums.
--
-- No-op once it has run, and where the `dojo` schema is absent (the daemon's
-- own database). Covered by database/tests/dojo/clients_retired.sql.
do $$
declare
    fk record;
begin
    if to_regnamespace('dojo') is null then
        return;
    end if;

    drop view if exists dojo.confidentiality_ledger;

    if to_regprocedure('dojo.is_confidentiality_action(text)') is not null
       and to_regclass('dojo.audit_events') is not null then
        drop policy if exists audit_events_read on dojo.audit_events;
    end if;

    drop table if exists dojo.incidents;

    if to_regclass('dojo.engagements') is not null then
        for fk in
            select c.conrelid::regclass as tbl, c.conname
              from pg_constraint c
             where c.contype = 'f'
               and c.confrelid = 'dojo.engagements'::regclass
               and c.conrelid <> 'dojo.engagements'::regclass
        loop
            execute format('alter table %s drop constraint %I', fk.tbl, fk.conname);
        end loop;
        drop table dojo.engagements;
    end if;
end $$;
