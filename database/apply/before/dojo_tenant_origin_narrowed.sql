-- Let `dojo.tenant_origin` lose its `team` value (#307 D11: a dōjō is only an
-- organization's or a person's) on a database that still has it.
--
-- WHY THIS EXISTS. Dropping an enum value means rebuilding the type, and dbd
-- drops and recreates the views that use it — `dojo.all_my_repositories` among
-- them. But `dojo.my_repositories()` RETURNS that view's row type, so the view
-- cannot be dropped while the function exists ("cannot drop view
-- all_my_repositories because other objects depend on it"). This drops the
-- function first, and the retired `dojo.create_dojo` with it, ONLY while the
-- enum still has `team`: once narrowed it is a no-op. The reconcile then
-- re-applies `my_repositories()` from its DDL, before anything can call it.
--
-- No-op where `dojo.tenant_origin` is absent (the daemon's own database).
do $$
begin
    if exists (select 1 from pg_enum e
                 join pg_type t on t.oid = e.enumtypid
                 join pg_namespace n on n.oid = t.typnamespace
                where n.nspname = 'dojo' and t.typname = 'tenant_origin' and e.enumlabel = 'team') then
        drop function if exists dojo.my_repositories();
        drop function if exists dojo.create_dojo(text, text, dojo.membership_kind);
        delete from dojo.tenants where origin::text = 'team';
    end if;
end $$;
