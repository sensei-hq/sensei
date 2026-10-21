set search_path to dojo, extensions;

-- Create a team in a dōjō (#307 A3, D7). Admins only. Returns its id.
create or replace function dojo.create_team(p_tenant uuid, p_name text)
    returns uuid
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_id uuid;
begin
    if not dojo.tenant_role_at_least(p_tenant, 'admin') then
        raise exception 'only an admin of this dōjō creates its teams'
            using errcode = 'insufficient_privilege';
    end if;
    if nullif(btrim(p_name), '') is null then
        raise exception 'a team needs a name' using errcode = 'invalid_parameter_value';
    end if;
    insert into dojo.teams (tenant_id, name) values (p_tenant, btrim(p_name)) returning id into v_id;
    return v_id;
end;
$$;

revoke all on function dojo.create_team(uuid, text) from public;
grant execute on function dojo.create_team(uuid, text) to authenticated;

comment on function dojo.create_team(uuid, text) is
'Create a team (a grouping, #307 D7) in a dōjō. Admins only.';
