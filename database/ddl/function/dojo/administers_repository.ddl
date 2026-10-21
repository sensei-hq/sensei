set search_path to dojo, extensions;

-- Whether the signed-in principal is an admin of the repository's dōjō. SECURITY
-- DEFINER so it does not depend on dojo.repositories' own RLS.
create or replace function dojo.administers_repository(p_repository uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1 from dojo.repositories r
         where r.id = p_repository
           and dojo.tenant_role_at_least(r.tenant_id, 'admin')
    );
$$;

revoke all on function dojo.administers_repository(uuid) from public;
grant execute on function dojo.administers_repository(uuid) to authenticated;

comment on function dojo.administers_repository(uuid) is
'Whether the signed-in principal is an admin of the repository''s dōjō.';
