set search_path to dojo, extensions;

-- Whether a relay session may name this project (#252, #299): none at all, or a
-- project in a dōjō the signed-in principal is a member of. Projects are shared
-- items with no owner, so the runner's membership is what matters — a run is never
-- filed under a project in somebody else's dōjō.
create or replace function dojo.project_in_my_dojo_or_none(p_project uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select p_project is null
        or exists (select 1 from dojo.projects p
                    where p.id = p_project and dojo.is_member(p.tenant_id));
$$;

revoke all on function dojo.project_in_my_dojo_or_none(uuid) from public;
grant execute on function dojo.project_in_my_dojo_or_none(uuid) to authenticated;

comment on function dojo.project_in_my_dojo_or_none(uuid) is
'True for null or a project in a dōjō the signed-in principal is a member of —
the check on a relay session''s project_id (#252, #299).';
