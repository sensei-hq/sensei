set search_path to dojo, extensions;

-- Whether the signed-in principal is in the dōjō as a viewer (#307 D8).
create or replace function dojo.is_dojo_viewer(p_tenant uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (select 1 from dojo.dojo_viewers v
                    where v.tenant_id = p_tenant and v.principal_id = dojo.current_principal_id());
$$;

revoke all on function dojo.is_dojo_viewer(uuid) from public;
grant execute on function dojo.is_dojo_viewer(uuid) to authenticated;

comment on function dojo.is_dojo_viewer(uuid) is
'Whether the signed-in principal is in the dōjō as a viewer (not a member).';
