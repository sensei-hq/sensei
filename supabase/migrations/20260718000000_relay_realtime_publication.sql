-- Relay P4.1 — add the relay tables to the Supabase Realtime publication.
--
-- WHY a supabase/migrations/*.sql (and NOT a dbd DDL file): `supabase_realtime`
-- is a Supabase-OWNED object (created + managed by the Supabase platform stack,
-- not by our dbd `dojo` scope). dbd owns the `dojo.*` tables (incl. the RLS
-- policies, which live with each table's DDL); Supabase owns the publication.
-- Keeping the publication membership on the Supabase side means a `dbd reset` /
-- redeploy of the dojo schema never has to know about, or clobber, the realtime
-- publication. This file runs via `supabase db reset` / `supabase migration up`.
--
-- The phone subscribes Supabase Realtime directly with its JWT; RLS (authored in
-- the relay table DDL) still governs which rows a subscriber actually receives —
-- being in the publication only makes a table ELIGIBLE for realtime, it does not
-- widen row access. So this pairs with the SELECT-only own-rows RLS policies.
--
-- Idempotent: `alter publication ... add table` ERRORS if the table is already a
-- member, so guard each add with a pg_publication_tables check. Re-running this
-- migration (or applying it on a DB that already has some/all tables) is a no-op
-- for the tables already present.
do $$
begin
  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime'
      and schemaname = 'dojo' and tablename = 'relay_sessions'
  ) then
    alter publication supabase_realtime add table dojo.relay_sessions;
  end if;

  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime'
      and schemaname = 'dojo' and tablename = 'relay_segments'
  ) then
    alter publication supabase_realtime add table dojo.relay_segments;
  end if;

  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime'
      and schemaname = 'dojo' and tablename = 'relay_inbox'
  ) then
    alter publication supabase_realtime add table dojo.relay_inbox;
  end if;
end $$;
