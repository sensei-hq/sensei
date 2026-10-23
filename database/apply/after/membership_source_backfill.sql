-- Membership provenance, recorded and never guessed. Idempotent; a no-op where
-- dojo.memberships (or its columns) is absent (the daemon's own database).
--
-- WHO WRITES source = 'provisioned' (read from the code, #308 R5b review 2):
-- only dojo/src/lib/server/provisioning.ts ensureMembership. Its callers pass
--   * github_oauth — GitHub org provisioning (ensureOrgTenant), and the person's
--     personal dōjō on a GitHub sign-in;
--   * azure_devops — the Azure DevOps import (ensureOrgTenant), which since
--     #308 R5b also writes source_provider / source_unit itself;
--   * sso — ONLY the personal dōjō of a sign-in with no forge token.
-- device_code never reaches it: admin-added and invited rows (admin-data.ts
-- addMember, invites-data.ts) are 'invited'.
do $$
begin
    if to_regclass('dojo.memberships') is null then
        return;
    end if;

    -- #309's relabel: provisioning wrote no source before #309, so every non-sso
    -- row was taken to be provisioned. Only for rows from BEFORE #309 shipped
    -- (v0.16.0, 2026-10-21 19:01:28 UTC): after it, provisioning writes the source
    -- itself, and relabelling a newer admin-added row would hand it to a forge's
    -- reconcile.
    update dojo.memberships
       set source = 'provisioned'
     where source = 'invited'
       and authenticated_via <> 'sso'
       and created_at < timestamptz '2026-10-21 19:01:28+00';

    if not exists (select 1 from information_schema.columns
                    where table_schema = 'dojo' and table_name = 'memberships' and column_name = 'source_provider') then
        return;
    end if;

    -- Provenance (#308 R5b): which forge, and which unit, provisioned each org
    -- membership. Attributed only where the code says which forge it was, with
    -- the unit when the dōjō has exactly one connection to that forge.
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

    update dojo.memberships m
       set source_provider = 'azure_devops',
           source_unit = (select min(c.external_id) from dojo.tenant_connections c
                           where c.tenant_id = m.tenant_id and c.provider = 'azure_devops'
                          having count(*) = 1)
      from dojo.tenants t
     where t.id = m.tenant_id
       and t.origin = 'organization'
       and m.source = 'provisioned'
       and m.authenticated_via = 'azure_devops'
       and m.source_provider is null;

    -- Everything else provisioned stays unattributed — sso and device_code org
    -- rows (no forge provisions those), and every personal-dōjō row (no reconcile
    -- touches a personal dōjō). No reconcile acts on them; say how many there are.
    raise notice 'membership provenance: % provisioned organization membership(s) have no attributable forge and are left alone',
        (select count(*) from dojo.memberships m join dojo.tenants t on t.id = m.tenant_id
          where t.origin = 'organization' and m.source = 'provisioned' and m.source_provider is null);
end $$;
