-- Make the grant targets in this schema resolvable.
--
-- WHY THIS EXISTS. Twelve DDL files in the `sensei` and `activity` schemas —
-- so inside the daemon's `default` scope, not skippable the way the
-- `policies/dojo/*` files are — grant SELECT to `anon`, `authenticated` and
-- `service_role`. Those are Supabase's roles. `GRANT` resolves its grantee at
-- apply time, so on a Postgres that has never hosted Supabase the very first
-- such file fails:
--
--     role "authenticated" does not exist
--
-- which took the whole deploy with it. A first install therefore could not
-- provision its database at all (issue #196), and the failure was invisible on
-- any machine that already had the roles — which is every machine that had ever
-- run the dōjō locally.
--
-- WHAT THIS DOES AND DOES NOT DO. Its entire job is to make the grantee names
-- resolvable. Each role is created NOLOGIN with no members, so nothing can
-- connect or `SET ROLE` to one that could not already. It deliberately does NOT
-- reproduce Supabase's full role attributes — notably `service_role`'s
-- BYPASSRLS. Anything that depends on those attributes reads the dōjō's
-- Supabase-hosted Postgres, where the roles already exist with them and the
-- guard below skips creation entirely. Minting a locally-privileged lookalike of
-- a role whose real definition lives elsewhere is how a local run starts
-- disagreeing with production.
--
-- ON A REAL SUPABASE TARGET THIS IS A NO-OP, and that matters for more than
-- tidiness: the connecting role there may well lack CREATEROLE. Because the
-- guard checks existence first, no `CREATE ROLE` is ever attempted, so the
-- privilege is never needed. Where a role genuinely is absent AND we cannot
-- create it, this fails loudly with a permission error rather than continuing
-- into the confusing "does not exist" further down.
--
-- Idempotent: `CREATE ROLE` has no `IF NOT EXISTS`, hence the explicit check.
do $$
declare
    wanted text[] := array['anon', 'authenticated', 'service_role'];
    r text;
begin
    foreach r in array wanted loop
        if not exists (select 1 from pg_catalog.pg_roles where rolname = r) then
            execute format('create role %I nologin', r);
            raise notice 'created grant-target role %', r;
        end if;
    end loop;
end
$$;
