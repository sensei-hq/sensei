set search_path to dojo, extensions;

create or replace function dojo.is_member(p_tenant uuid)
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
    );
$$;

revoke all on function dojo.is_member(uuid) from public;
grant execute on function dojo.is_member(uuid) to authenticated;

comment on function dojo.is_member(uuid) is
'Whether the signed-in principal holds an ACTIVE membership in the tenant — the
floor of every tenant-scoped cell in docs/spec/dojo/access-matrix.md. A disabled
or missing membership is no access (never "level 0"). SECURITY DEFINER so a
policy can ask without granting the caller a read on memberships.';
