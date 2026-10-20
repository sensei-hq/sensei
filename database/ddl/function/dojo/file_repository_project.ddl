set search_path to dojo, extensions;

-- File one synced repository under its sensei project (#299; decided
-- 2026-10-20: one dōjō project per sensei project).
--
-- Before this, `dojo.projects` was written only by a relay run
-- (`upsertProjectFromRun`) and `dojo.repositories_in_projects` by nothing, so a
-- repository that synced never appeared under any project.
--
-- The project is the CALLER's, keyed `(user_id, slug)` — the same row a relay
-- run for that project writes, so the two paths converge on one project. A
-- `personal` project carries no tenant, as a relay run files it; every other
-- classification belongs to the repository's tenant. The link is one project per
-- repository per tenant (the table's unique key): filing a repository under a
-- different project MOVES it.
--
-- SECURITY DEFINER because `repositories_in_projects` admits no direct writes;
-- the caller must be a member of the repository's tenant, which this checks
-- before writing anything.
create or replace function dojo.file_repository_project(
    p_repository_id  uuid,
    p_slug           text,
    p_name           text,
    p_classification dojo.project_classification
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
    v_project uuid;
begin
    select r.tenant_id into v_tenant from dojo.repositories r where r.id = p_repository_id;
    if v_me is null or v_tenant is null or not dojo.is_member(v_tenant) then
        raise exception 'not a member of this repository''s tenant'
            using errcode = 'insufficient_privilege';
    end if;
    if btrim(coalesce(p_slug, '')) = '' or btrim(coalesce(p_name, '')) = '' then
        raise exception 'a project needs a slug and a name'
            using errcode = 'invalid_parameter_value';
    end if;

    insert into dojo.projects as p (user_id, tenant_id, slug, name, classification)
    values (v_me,
            case when p_classification = 'personal' then null else v_tenant end,
            p_slug, p_name, p_classification)
    on conflict on constraint projects_user_slug_unique
        do update set name = excluded.name, updated_at = now()
    returning p.id into v_project;

    insert into dojo.repositories_in_projects (project_id, repository_id, tenant_id)
    values (v_project, p_repository_id, v_tenant)
    on conflict (repository_id, tenant_id) do update set project_id = excluded.project_id;

    return v_project;
end;
$$;

revoke all on function dojo.file_repository_project(uuid, text, text, dojo.project_classification) from public;
grant execute on function dojo.file_repository_project(uuid, text, text, dojo.project_classification) to authenticated;

comment on function dojo.file_repository_project(uuid, text, text, dojo.project_classification) is
'File a synced repository under the caller''s project (#299): upsert the
(user_id, slug) project a relay run would write, and link the repository to it
(one project per repository per tenant — a re-file moves it). Refuses a caller
who is not a member of the repository''s tenant.';
