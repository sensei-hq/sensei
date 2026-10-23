set search_path to dojo, extensions;

-- Decide where one forge unit's repositories file (#308 R5a, D20): its own dōjō,
-- or a workspace in a dōjō the mapper administers. One mapping per unit; a second
-- call replaces it.
--
-- The caller is the server (service_role). It has read the forge with the
-- mapper's OWN token, so it knows the mapper can see the unit and passes
-- p_proven = whether the forge calls them an admin of it. What this enforces:
--   * the mapper is an ACTIVE admin of the target dōjō (p_tenant);
--   * ITS OWN DŌJŌ (p_workspace_slug null) needs the proof, and p_tenant must be
--     the dōjō the unit's connection names;
--   * A WORKSPACE (p_workspace_slug given) needs the proof only when the unit is
--     already an admin's to place — its own dōjō is claimed, or the current
--     mapping was made with proof. Otherwise a member who can see an unclaimed
--     unit may file it into a dōjō they administer (the spec's "units they cannot
--     prove can only become workspaces of a dōjō they already administer").
-- The workspace is found or made by (tenant, slug), as filing does. Audited in
-- the dōjō it files into, and — when it moved — in the dōjō it left.
--
-- p_keep_elsewhere (#308 R5b review): an IMPORT files a unit only where nobody
-- has placed it yet. With it set, a unit already mapped into ANOTHER dōjō stays
-- there — nothing is written or audited, and the answer names the dōjō it is in
-- — so a concurrent re-map cannot be overwritten by an import's stale pre-read.
drop function if exists dojo.map_forge_unit(uuid, dojo.forge_provider, text, text, uuid, text, text, boolean);
create or replace function dojo.map_forge_unit(
    p_principal      uuid,
    p_provider       dojo.forge_provider,
    p_external_id    text,
    p_external_slug  text,
    p_tenant         uuid,
    p_workspace_slug text,
    p_workspace_name text,
    p_proven         boolean,
    p_keep_elsewhere boolean default false
)
    returns table (mapping_id uuid, tenant_id uuid, workspace_id uuid)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_own         uuid;
    v_own_claimed timestamptz;
    v_prev        dojo.forge_unit_mappings%rowtype;
    v_origin      dojo.tenant_origin;
    v_workspace   uuid;
    v_id          uuid;
begin
    if btrim(coalesce(p_external_id, '')) = '' or btrim(coalesce(p_external_slug, '')) = '' then
        raise exception 'a unit needs its forge id and name'
            using errcode = 'invalid_parameter_value';
    end if;

    if not exists (
        select 1 from dojo.memberships m
         where m.tenant_id = p_tenant and m.user_id = p_principal
           and m.disabled_at is null and m.role = 'admin'
    ) then
        raise exception 'only an admin of the dōjō may file a unit into it'
            using errcode = 'insufficient_privilege';
    end if;

    select c.tenant_id, t.claimed_at into v_own, v_own_claimed
      from dojo.tenant_connections c
      join dojo.tenants t on t.id = c.tenant_id
     where c.provider = p_provider and c.external_id = p_external_id;

    select m.* into v_prev
      from dojo.forge_unit_mappings m
     where m.provider = p_provider and m.external_id = p_external_id
       for update;

    if coalesce(p_keep_elsewhere, false) and v_prev.tenant_id is not null and v_prev.tenant_id <> p_tenant then
        return query select v_prev.id, v_prev.tenant_id, v_prev.workspace_id;
        return;
    end if;

    if p_workspace_slug is null then
        if not coalesce(p_proven, false) then
            raise exception 'only an admin of the organization can make it its own dōjō'
                using errcode = 'insufficient_privilege';
        end if;
        if v_own is distinct from p_tenant then
            raise exception 'a unit''s own dōjō is the one connected to it'
                using errcode = 'invalid_parameter_value';
        end if;
    else
        if btrim(p_workspace_slug) = '' or btrim(coalesce(p_workspace_name, '')) = '' then
            raise exception 'a workspace needs a slug and a name'
                using errcode = 'invalid_parameter_value';
        end if;
        if not coalesce(p_proven, false) and (v_own_claimed is not null or v_prev.proven_at is not null) then
            raise exception 'only an admin of the organization can move a claimed organization''s repositories'
                using errcode = 'insufficient_privilege';
        end if;

        select t.origin into v_origin from dojo.tenants t where t.id = p_tenant;
        insert into dojo.workspaces as w (tenant_id, slug, name, classification)
        values (p_tenant, btrim(p_workspace_slug), btrim(p_workspace_name),
                case when v_origin = 'personal' then 'personal' else 'company' end::dojo.workspace_classification)
        on conflict on constraint workspaces_tenant_slug_unique do update set updated_at = now()
        returning w.id into v_workspace;
    end if;

    insert into dojo.forge_unit_mappings as m
        (provider, external_id, external_slug, tenant_id, workspace_id, proven_at, mapped_by, mapped_at)
    values (p_provider, p_external_id, btrim(p_external_slug), p_tenant, v_workspace,
            case when p_proven then now() end, p_principal, now())
    on conflict on constraint forge_unit_mappings_unit_unique do update
        set external_slug = excluded.external_slug,
            tenant_id     = excluded.tenant_id,
            workspace_id  = excluded.workspace_id,
            proven_at     = excluded.proven_at,
            mapped_by     = excluded.mapped_by,
            mapped_at     = excluded.mapped_at
    returning m.id into v_id;

    perform dojo.append_audit_event(p_tenant, p_principal, 'forge_unit_mapped',
        jsonb_build_object('provider', p_provider, 'external_id', p_external_id,
                           'external_slug', btrim(p_external_slug),
                           'as', case when v_workspace is null then 'dojo' else 'workspace' end,
                           'workspace_id', v_workspace, 'proven', coalesce(p_proven, false)));
    if v_prev.tenant_id is not null and v_prev.tenant_id <> p_tenant then
        perform dojo.append_audit_event(v_prev.tenant_id, p_principal, 'forge_unit_unmapped',
            jsonb_build_object('provider', p_provider, 'external_id', p_external_id,
                               'external_slug', btrim(p_external_slug), 'moved_to', p_tenant));
    end if;

    return query select v_id, p_tenant, v_workspace;
end;
$$;

revoke all on function dojo.map_forge_unit(uuid, dojo.forge_provider, text, text, uuid, text, text, boolean, boolean) from public;
grant execute on function dojo.map_forge_unit(uuid, dojo.forge_provider, text, text, uuid, text, text, boolean, boolean) to service_role;

comment on function dojo.map_forge_unit(uuid, dojo.forge_provider, text, text, uuid, text, text, boolean, boolean) is
'Map a forge unit to its own dōjō or to a workspace in a dōjō the mapper
administers (#308 R5a). One mapping per unit; audited. service_role only — the
server reads the forge with the mapper''s own token and passes whether it proved
them an admin of the unit. p_keep_elsewhere leaves a unit mapped into another
dōjō where it is (an import).';
