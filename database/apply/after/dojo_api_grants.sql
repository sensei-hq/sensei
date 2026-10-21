-- The API roles' reach into the `dojo` schema.
--
-- PostgREST serves `dojo` (supabase `[api] schemas`), but a role can address
-- nothing in a schema without USAGE on it — the table grants and RLS policies in
-- policies/dojo/*.sql never even evaluate. Without this, every route that reads
-- as the signed-in person (`authenticated`, #236) fails "permission denied for
-- schema dojo", and every route that reads as the system (`service_role`) does too.
--
-- WHY A HOOK, and not a manual deploy step. It used to be one ("expose + grant
-- the dojo schema", docs/history/plan/decisions.md), kept out of the dbd tree on two
-- grounds that no longer hold: the Supabase roles may not exist (apply/before/
-- roles.sql now makes them resolvable everywhere), and dbd could not parse a
-- `do` block (fixed in 0.12.0). A step a person must remember is a step a fresh
-- database misses — verified: a `dbd apply --scope dojo --with-policies` left
-- `authenticated`, `anon` and `service_role` with no USAGE on `dojo` at all.
--
-- WHAT IS GRANTED:
--   * USAGE on the schema to anon, authenticated, service_role — reach only;
--     what each may read or write is the per-table grants and policies.
--   * service_role: all on the schema's tables and sequences, now and for objects
--     created later. service_role is the system (the Worker's own work — the
--     access matrix's service-role table) and bypasses RLS by definition.
--
-- Guarded: runs for EVERY scope, and is a no-op where `dojo` does not exist (the
-- daemon's own database). Idempotent: GRANT repeats harmlessly.
do $$
begin
    if not exists (select 1 from pg_namespace where nspname = 'dojo') then
        return;
    end if;
    grant usage on schema dojo to anon, authenticated, service_role;
    grant all on all tables in schema dojo to service_role;
    grant all on all sequences in schema dojo to service_role;
    alter default privileges in schema dojo grant all on tables to service_role;
    alter default privileges in schema dojo grant all on sequences to service_role;
end
$$;
