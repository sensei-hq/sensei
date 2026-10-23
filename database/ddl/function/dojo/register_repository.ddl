set search_path to dojo, extensions;

-- Register one repository the signed-in person's daemon found (#236; access
-- matrix "Workspaces and repositories": a member registers).
--
-- Maps the repository to the ONE tenant of the caller's that its forge org is
-- connected to, and files it there — or returns why not:
--   no_connection  the org is connected to no tenant at all
--   not_a_member   it is connected, but to no tenant the caller is in
--   ambiguous      its slug resolves to two of the caller's tenants (an org
--                  renamed upstream and its old name re-registered) — attaching
--                  code to the wrong governance boundary is worse than refusing
--
-- SECURITY DEFINER because telling the first two apart reads every tenant's
-- connections; what leaves is a reason code, never another tenant's id. The
-- caller parses (provider, org) from the key (repo-mapping.ts knows the forge
-- hosts); this refuses a key whose path does not carry that org, so a direct
-- call cannot file someone else's repository under the caller's tenant.
-- Idempotent: a second registration returns the existing row.
--
-- THE UNIT'S MAPPING DECIDES FIRST (#308 R5a). When an admin has mapped the
-- org (dojo.forge_unit_mappings), its repositories file into the mapped dōjō —
-- and, mapped as a workspace, into that workspace — for members of that dōjō,
-- and into no other dōjō (not_a_member for anyone outside it). An org with no
-- mapping files by its connection, as provisioning made it.
--
-- The returned workspace_id is the mapping's: the caller does not re-file a
-- repository its unit's mapping already placed.
--
-- AN AZURE DEVOPS PROJECT IS A UNIT TOO (#308 R5b, D36). An organization is a
-- dōjō and each project a workspace in it: an import maps each project, named
-- `{org}/{project}`. The caller passes the project the key names (p_project,
-- repo-mapping.ts parses it); its mapping decides first, then the
-- organization's, then the connection. The key must carry the org and that
-- project AT THEIR PLACES (dojo.repo_key_names), so a direct call cannot file a
-- repository under another org or another project's workspace.
drop function if exists dojo.register_repository(text, dojo.forge_provider, text, text, text);
create or replace function dojo.register_repository(
    p_repo_key   text,
    p_provider   dojo.forge_provider,
    p_org        text,
    p_name       text,
    p_remote_url text,
    p_project    text default null
)
    returns table (tenant_id uuid, tenant_key text, repository_id uuid, reason text, workspace_id uuid)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_key  text := lower(btrim(p_repo_key));
    v_org  text := lower(btrim(p_org));
    v_proj text := nullif(lower(btrim(coalesce(p_project, ''))), '');
    v_hits uuid[];
    v_mine uuid[];
    v_id   uuid;
    v_map_tenants uuid[];
    v_map_ws      uuid;
    v_target      uuid;
begin
    -- The org (and an Azure project) must sit at its own place in the key — not
    -- merely appear in it: a repository named like an org or a project would
    -- otherwise file under it (#308 R5b security review).
    if v_key = '' or not dojo.repo_key_names(v_key, v_org, v_proj) then
        raise exception 'repository key % does not carry the org % (or the project %) at its place', p_repo_key, p_org, p_project
            using errcode = 'invalid_parameter_value';
    end if;

    -- One entry per distinct PLACE (dōjō, workspace) — not per mapping row: two
    -- units that share a name and file to the same place are not ambiguous.
    if v_proj is not null then
        select array_agg(x.tenant_id), (array_agg(x.workspace_id))[1] into v_map_tenants, v_map_ws
          from (select distinct m.tenant_id, m.workspace_id
                  from dojo.forge_unit_mappings m
                 where m.provider = p_provider
                   and lower(m.external_slug) = v_org || '/' || v_proj) x;
    end if;

    if v_map_tenants is null then
        select array_agg(x.tenant_id), (array_agg(x.workspace_id))[1] into v_map_tenants, v_map_ws
          from (select distinct m.tenant_id, m.workspace_id
                  from dojo.forge_unit_mappings m
                 where m.provider = p_provider
                   and lower(m.external_slug) = v_org) x;
    end if;

    if v_map_tenants is not null then
        -- Two units have held the same name (renamed upstream, name re-taken) and
        -- file to different places: which one this remote means is unknowable.
        if cardinality(v_map_tenants) > 1 then
            return query select null::uuid, null::text, null::uuid, 'ambiguous'::text, null::uuid;
            return;
        end if;
        if not dojo.is_member(v_map_tenants[1]) then
            return query select null::uuid, null::text, null::uuid, 'not_a_member'::text, null::uuid;
            return;
        end if;
        v_target := v_map_tenants[1];
    else
        select array_agg(distinct c.tenant_id) into v_hits
          from dojo.tenant_connections c
         where c.provider = p_provider
           and lower(c.external_slug) = v_org;
        if v_hits is null then
            return query select null::uuid, null::text, null::uuid, 'no_connection'::text, null::uuid;
            return;
        end if;

        select array_agg(h) into v_mine from unnest(v_hits) h where dojo.is_member(h);
        if v_mine is null then
            return query select null::uuid, null::text, null::uuid, 'not_a_member'::text, null::uuid;
            return;
        end if;
        if cardinality(v_mine) > 1 then
            return query select null::uuid, null::text, null::uuid, 'ambiguous'::text, null::uuid;
            return;
        end if;
        v_target := v_mine[1];
        v_map_ws := null;
    end if;

    insert into dojo.repositories as r (tenant_id, repo_key, name, remote_url, provider)
    values (v_target, v_key, p_name, p_remote_url, p_provider)
    on conflict on constraint repositories_tenant_id_repo_key_key do nothing
    returning r.id into v_id;
    if v_id is null then
        select r.id into v_id from dojo.repositories r where r.tenant_id = v_target and r.repo_key = v_key;
    end if;

    if v_map_ws is not null then
        insert into dojo.repositories_in_workspaces as riw (workspace_id, repository_id, tenant_id)
        values (v_map_ws, v_id, v_target)
        on conflict on constraint repositories_in_workspaces_repository_id_tenant_id_key
            do update set workspace_id = excluded.workspace_id;
    end if;

    return query select v_target, t.key, v_id, null::text, v_map_ws from dojo.tenants t where t.id = v_target;
end;
$$;

revoke all on function dojo.register_repository(text, dojo.forge_provider, text, text, text, text) from public;
grant execute on function dojo.register_repository(text, dojo.forge_provider, text, text, text, text) to authenticated;

comment on function dojo.register_repository(text, dojo.forge_provider, text, text, text, text) is
'Register one repository under the dōjō its forge unit is mapped to (and into the
mapped workspace) — an Azure DevOps repository by its project''s mapping first
(#308 R5b) — or, with no mapping, the ONE tenant of the caller''s its org
is connected to; or say why not (no_connection | not_a_member | ambiguous). Reads
every tenant''s connections and mappings; returns only a reason, never another
tenant''s id. Refuses a key whose path does not carry the org (#236, #308 R5a).';
