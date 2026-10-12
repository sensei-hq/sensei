set search_path to dojo, extensions;

-- Whether a principal holds an ACTIVE membership in a tenant — asked by the
-- incidents write policy of an incident's owner (an owner outside the tenant
-- would be a read of their name through a side door).
--
-- Answers only for a tenant the CALLER is a member of: it is not a way to ask
-- who belongs to someone else's dōjō.
create or replace function dojo.principal_is_member(p_tenant uuid, p_principal uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select dojo.is_member(p_tenant)
       and exists (
        select 1
          from dojo.memberships m
         where m.tenant_id = p_tenant
           and m.user_id = p_principal
           and m.disabled_at is null
    );
$$;

revoke all on function dojo.principal_is_member(uuid, uuid) from public;
grant execute on function dojo.principal_is_member(uuid, uuid) to authenticated;

comment on function dojo.principal_is_member(uuid, uuid) is
'Whether the principal holds an ACTIVE membership in the tenant, answered only to
a member of that tenant. Used by the incidents write policy for an incident''s
owner (#236, docs/spec/dojo/access-matrix.md "Client work and compliance").';
