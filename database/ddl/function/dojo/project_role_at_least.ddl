set search_path to dojo, extensions;

-- Whether the signed-in principal may act on a project at `p_role` or above: the
-- higher of their ACTIVE dōjō role in the project's tenant and their role on the
-- project. A project role only raises (#307 A1) — and only with an active
-- membership in the project's dōjō, so leaving the dōjō ends it too.
create or replace function dojo.project_role_at_least(p_project uuid, p_role dojo.member_role)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1
          from dojo.projects p
          join dojo.memberships m
            on m.tenant_id = p.tenant_id
           and m.user_id = dojo.current_principal_id()
           and m.disabled_at is null
          left join dojo.project_members pm
            on pm.project_id = p.id
           and pm.principal_id = m.user_id
         where p.id = p_project
           and greatest(m.role, pm.role) >= p_role
    );
$$;

revoke all on function dojo.project_role_at_least(uuid, dojo.member_role) from public;
grant execute on function dojo.project_role_at_least(uuid, dojo.member_role) to authenticated;

comment on function dojo.project_role_at_least(uuid, dojo.member_role) is
'Whether the signed-in principal acts on the project at p_role or above: the
higher of their active dōjō role and their project role (which only raises).
Compared in the enum''s order — contributor < maintainer < lead < admin.';
