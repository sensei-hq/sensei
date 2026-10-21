set search_path to dojo, extensions;

-- Whether the signed-in principal holds at least `p_role` on the repository
-- (#307 D5): the admin override when set, else the forge's role — and only with an
-- active membership in the repository's dōjō.
create or replace function dojo.repo_role_at_least(p_repository uuid, p_role dojo.member_role)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1
          from dojo.repository_roles rr
          join dojo.repositories r on r.id = rr.repository_id
          join dojo.memberships m
            on m.tenant_id = r.tenant_id
           and m.user_id = rr.principal_id
           and m.disabled_at is null
         where rr.repository_id = p_repository
           and rr.principal_id = dojo.current_principal_id()
           and (case when rr.overridden then rr.override_role else rr.forge_role end) >= p_role
    );
$$;

revoke all on function dojo.repo_role_at_least(uuid, dojo.member_role) from public;
grant execute on function dojo.repo_role_at_least(uuid, dojo.member_role) to authenticated;

comment on function dojo.repo_role_at_least(uuid, dojo.member_role) is
'Whether the signed-in principal holds at least p_role on the repository: the admin
override when set, else the forge''s; only with an active membership in its dōjō.';
