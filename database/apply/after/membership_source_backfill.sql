-- Mark the memberships org provisioning made before #309 recorded a source.
-- Invites sign in as `sso`; provisioning records the forge sign-in method — so
-- every non-sso membership is provisioned. Idempotent: rows already marked, and
-- every invited row, are untouched. Safe to run on every deploy; a no-op where
-- dojo.memberships is absent (the daemon's own database).
do $$
begin
    if to_regclass('dojo.memberships') is not null then
        update dojo.memberships
           set source = 'provisioned'
         where source = 'invited'
           and authenticated_via <> 'sso';
    end if;
end $$;

-- Record which forge provisioned the memberships made before #308 R5b recorded
-- it: every provisioned membership of an organization dōjō signed in through
-- GitHub was made by GitHub org provisioning, from the dōjō's GitHub connection
-- (source_unit when the dōjō has exactly one). Idempotent; rows that already
-- name a source are untouched. A no-op where the columns are absent.
do $$
begin
    if exists (select 1 from information_schema.columns
                where table_schema = 'dojo' and table_name = 'memberships' and column_name = 'source_provider') then
        update dojo.memberships m
           set source_provider = 'github',
               source_unit = (select min(c.external_id) from dojo.tenant_connections c
                               where c.tenant_id = m.tenant_id and c.provider = 'github'
                              having count(*) = 1)
          from dojo.tenants t
         where t.id = m.tenant_id
           and t.origin = 'organization'
           and m.source = 'provisioned'
           and m.authenticated_via = 'github_oauth'
           and m.source_provider is null;
    end if;
end $$;
