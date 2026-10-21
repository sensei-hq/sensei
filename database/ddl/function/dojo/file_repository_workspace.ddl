set search_path to dojo, extensions;

-- The first cut took the classification as an argument; it is now derived.
drop function if exists dojo.file_repository_workspace(uuid, text, text, dojo.workspace_classification);

-- File one synced repository under its sensei workspace (#299; decided
-- 2026-10-20: one dōjō workspace per sensei workspace).
--
-- Before this, `dojo.workspaces` was written only by a relay run
-- (`upsertProjectFromRun`) and `dojo.repositories_in_workspaces` by nothing, so a
-- repository that synced never appeared under any workspace.
--
-- The workspace is ONE SHARED item in the repository's dōjō, keyed
-- `(tenant_id, slug)` (#299): every member who syncs the same workspace converges
-- on the one row. A personal dōjō's workspaces live in it — they are the workspaces
-- under the owner's own GitHub account, never under an org.
--
-- Its classification is set when the workspace is first filed and follows where
-- it lives: a personal dōjō files `personal`; an org dōjō files by the filer's
-- membership kind there (employer → `company`, client, community). A later
-- filing by another member does not re-classify it. The link is one workspace per
-- repository per tenant (the table's unique key): filing a repository under a
-- different workspace MOVES it.
--
-- SECURITY DEFINER because `repositories_in_workspaces` admits no direct writes;
-- the caller must be a member of the repository's tenant, which this checks
-- before writing anything.
create or replace function dojo.file_repository_workspace(
    p_repository_id  uuid,
    p_slug           text,
    p_name           text
)
    returns uuid
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_me      uuid := dojo.current_principal_id();
    v_tenant  uuid;
    v_class   dojo.workspace_classification;
    v_workspace uuid;
begin
    select r.tenant_id into v_tenant from dojo.repositories r where r.id = p_repository_id;
    if v_me is null or v_tenant is null or not dojo.is_member(v_tenant) then
        raise exception 'not a member of this repository''s tenant'
            using errcode = 'insufficient_privilege';
    end if;
    if btrim(coalesce(p_slug, '')) = '' or btrim(coalesce(p_name, '')) = '' then
        raise exception 'a workspace needs a slug and a name'
            using errcode = 'invalid_parameter_value';
    end if;

    select case when t.origin = 'personal' then 'personal'
                when m.kind = 'client'     then 'client'
                when m.kind = 'community'  then 'community'
                else 'company'
           end::dojo.workspace_classification
      into v_class
      from dojo.tenants t
      join dojo.memberships m on m.tenant_id = t.id and m.user_id = v_me
     where t.id = v_tenant;

    insert into dojo.workspaces as p (tenant_id, slug, name, classification)
    values (v_tenant, p_slug, p_name, v_class)
    on conflict on constraint workspaces_tenant_slug_unique
        do update set name = excluded.name, updated_at = now()
    returning p.id into v_workspace;

    insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id)
    values (v_workspace, p_repository_id, v_tenant)
    on conflict (repository_id, tenant_id) do update set workspace_id = excluded.workspace_id;

    return v_workspace;
end;
$$;

revoke all on function dojo.file_repository_workspace(uuid, text, text) from public;
grant execute on function dojo.file_repository_workspace(uuid, text, text) to authenticated;

comment on function dojo.file_repository_workspace(uuid, text, text) is
'File a synced repository under its shared workspace (#299): find or create the
(tenant_id, slug) workspace in the repository''s dōjō and link the repository to it
(one workspace per repository per tenant — a re-file moves it). Refuses a caller
who is not a member of the repository''s tenant.';
