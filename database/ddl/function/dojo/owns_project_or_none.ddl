set search_path to dojo, extensions;

-- Whether a project reference is the signed-in principal's own project, or no
-- project at all — what a relay session may be filed under (#252), so nobody
-- posts their run into another person's (and so another team's) project.
create or replace function dojo.owns_project_or_none(p_project uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select p_project is null
        or exists (select 1 from dojo.projects p
                    where p.id = p_project and p.user_id = dojo.current_principal_id());
$$;

revoke all on function dojo.owns_project_or_none(uuid) from public;
grant execute on function dojo.owns_project_or_none(uuid) to authenticated;

comment on function dojo.owns_project_or_none(uuid) is
'True for null or a project the signed-in principal owns — the check on a relay
session''s project_id (#252).';
