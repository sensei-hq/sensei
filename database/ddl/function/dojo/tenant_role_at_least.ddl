set search_path to dojo, extensions;

create or replace function dojo.tenant_role_at_least(p_tenant uuid, p_role dojo.member_role)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1
          from dojo.memberships m
         where m.tenant_id = p_tenant
           and m.user_id = dojo.current_principal_id()
           and m.disabled_at is null
           and m.role >= p_role
    );
$$;

revoke all on function dojo.tenant_role_at_least(uuid, dojo.member_role) from public;
grant execute on function dojo.tenant_role_at_least(uuid, dojo.member_role) to authenticated;

comment on function dojo.tenant_role_at_least(uuid, dojo.member_role) is
'Whether the signed-in principal''s ACTIVE membership in the tenant carries at
least `p_role`. Compared by the ENUM''s declared order — contributor < maintainer
< lead < admin — never as text, which is the order the access matrix and the
entity & access model state.';
