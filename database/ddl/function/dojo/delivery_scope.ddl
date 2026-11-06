set search_path to dojo, extensions;

-- Where the signed-in caller's board goes (spec tracker-delivery-metrics D4; #352,
-- #353): every repo_key must be their own verdict row of dojo.all_my_repositories
-- in p_tenant with sync_enabled, else insufficient_privilege; the workspace is the
-- one those verified repositories are filed in, resolved from their ids, never by
-- matching a repo_key across tenants — none → no_data_found ('workspace_not_filed'),
-- more than one → object_not_in_prerequisite_state ('workspace_ambiguous').
-- Answers each key with its verified repository and the workspace.
create or replace function dojo.delivery_scope(
    p_tenant    uuid,
    p_repo_keys text[]
)
    returns table (repo_key text, repository_id uuid, workspace_id uuid)
    language plpgsql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_principal  uuid := dojo.current_principal_id();
    v_repos      uuid[];
    v_keys       text[];
    v_workspaces uuid[];
begin
    if v_principal is null then
        raise exception 'no signed-in principal' using errcode = 'insufficient_privilege';
    end if;
    if p_repo_keys is null or cardinality(p_repo_keys) = 0 then
        raise exception 'a board names at least one repository' using errcode = 'invalid_parameter_value';
    end if;

    select array_agg(v.repository_id order by v.repo_key), array_agg(v.repo_key order by v.repo_key)
      into v_repos, v_keys
      from dojo.all_my_repositories v
     where v.principal_id = v_principal
       and v.tenant_id = p_tenant
       and v.sync_enabled
       and v.repo_key = any(p_repo_keys);
    if v_repos is null or cardinality(v_repos) <> (select count(distinct k) from unnest(p_repo_keys) k) then
        raise exception 'a board names a repository the caller may not sync in this dōjō'
            using errcode = 'insufficient_privilege';
    end if;

    select array_agg(distinct riw.workspace_id) into v_workspaces
      from dojo.repositories_in_workspaces riw
     where riw.tenant_id = p_tenant
       and riw.repository_id = any(v_repos);
    if v_workspaces is null then
        raise exception 'workspace_not_filed' using errcode = 'no_data_found';
    end if;
    if cardinality(v_workspaces) > 1 then
        raise exception 'workspace_ambiguous' using errcode = 'object_not_in_prerequisite_state';
    end if;

    return query select k, r, v_workspaces[1] from unnest(v_keys, v_repos) as u(k, r);
end
$$;

revoke all on function dojo.delivery_scope(uuid, text[]) from public;

comment on function dojo.delivery_scope(uuid, text[]) is
'The caller''s consent-checked repositories for a board push and the one workspace they are filed in (#352, #353): insufficient_privilege unless every key is the caller''s own sync_enabled repository in p_tenant; no_data_found when filed in no workspace, 55000 when in several. Used by dojo.file_delivery_facts and dojo.file_delivery; not callable by clients.';
