-- Drop dojo.reconcile_unit_members while it still returns a bare integer
-- (#308 R5c): it now answers how many it ended AND how many it joined, and
-- Postgres cannot replace a function's return type in place. The DDL recreates
-- it, with its grants. No-op afterwards, and where dojo is absent (the daemon's
-- own database).
do $$
begin
    if exists (select 1
                 from pg_proc p
                 join pg_namespace n on n.oid = p.pronamespace
                where n.nspname = 'dojo'
                  and p.proname = 'reconcile_unit_members'
                  and p.prorettype = 'integer'::regtype) then
        drop function dojo.reconcile_unit_members(uuid, dojo.forge_provider, text, dojo.auth_method, text[]);
    end if;
end $$;
