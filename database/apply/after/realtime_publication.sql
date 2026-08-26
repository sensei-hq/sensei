-- Realtime publication membership for the relay tables.
--
-- WHY THIS EXISTS AT ALL. Being in `supabase_realtime` is what makes a table
-- eligible for Realtime; RLS (authored with each table) still decides which rows
-- a subscriber receives, so membership widens eligibility, never access.
--
-- WHY IT LIVES WITH THE SCHEMA rather than in supabase/migrations/. It used to be
-- a standalone Supabase migration, on the reasoning that `supabase_realtime` is a
-- Supabase-owned object. The flaw is lifecycle, not ownership: DROPPING a table
-- removes it from a publication, and RE-CREATING it does not put it back. So a
-- `dbd reset` + redeploy of the dojo scope left every relay table out of the
-- publication — Realtime silently delivering nothing, no error anywhere — until
-- somebody remembered to re-run the migration by hand. Verified: drop
-- dojo.relay_inbox and the membership count falls 3 → 2; a redeploy alone left it
-- at 2. Riding the deploy, it comes back.
--
-- WHY AN AFTER-SCRIPT rather than a table DDL file: publication membership is not
-- an entity dbd can diff — it is a one-line side effect that has to run once the
-- table exists. `apply.after` is exactly that hook, and it runs after the schema
-- is applied but before imports, so the tables are there and nothing has loaded
-- yet.
--
-- (dbd < 0.12.0 also could not PARSE a `do $$ … $$` block, and silently dropped
-- any file containing one — the same failure that hid dojo.set_pack_adoption and
-- dojo.can_read_repository_metric. Fixed in 0.12.0; the hook is still the right
-- home regardless.)
--
-- Guarded three ways, because this runs for EVERY scope: skip when the
-- publication is absent (a plain Postgres — the daemon's own database), skip when
-- the table is absent (any scope that excludes `dojo`), and skip when the table is
-- already a member (`alter publication … add table` ERRORS on a duplicate, which
-- would fail every deploy after the first).
do $$
declare
  t text;
begin
  if not exists (select 1 from pg_publication where pubname = 'supabase_realtime') then
    return;
  end if;

  foreach t in array array['relay_sessions', 'relay_segments', 'relay_inbox'] loop
    if to_regclass('dojo.' || t) is not null
       and not exists (
         select 1 from pg_publication_tables
          where pubname = 'supabase_realtime'
            and schemaname = 'dojo'
            and tablename = t)
    then
      execute format('alter publication supabase_realtime add table dojo.%I', t);
    end if;
  end loop;
end $$;
