set search_path to dojo, extensions;

-- Track (or untrack) a branch of a repository's shared code graph, as the
-- signed-in person (#225; docs/spec/dojo/graph-sync.md §9) — e.g. production
-- beside the default, or develop for a team that treats it as the latest.
--
-- Decided from the caller's own verdict row of dojo.all_my_repositories, the
-- rule set_repository_election uses:
--   no row for the caller          → no_data_found                    (404)
--   no authority yet (no capture)  → object_not_in_prerequisite_state (409)
--   not configurable_by_me         → insufficient_privilege           (403)
--   an empty branch name           → invalid_parameter_value          (400)
-- Tracking is idempotent: a re-track updates only the label. Untracking deletes
-- the branch's row, and the foreign keys take that branch's graph with it —
-- which is why it is an explicit act, never a side effect.
--
-- SECURITY DEFINER: dojo.repository_graph_branches takes no session write.
create or replace function dojo.set_graph_branch(
  p_repo_key text,
  p_branch   text,
  p_tracked  boolean,
  p_label    text
) returns table (repo_key text, branch text, tracked boolean, is_default boolean)
language plpgsql
volatile
security definer
set search_path = dojo, pg_temp
as $$
#variable_conflict use_column
declare
    v_principal uuid := dojo.current_principal_id();
    v_branch    text := nullif(btrim(p_branch), '');
    mine        dojo.all_my_repositories;
begin
    if v_branch is null then
        raise exception 'a branch needs a name' using errcode = 'invalid_parameter_value';
    end if;

    select * into mine
      from dojo.all_my_repositories v
     where v.principal_id = v_principal
       and v.repo_key = p_repo_key;
    if not found then
        raise exception 'no repository % for this account', p_repo_key using errcode = 'no_data_found';
    end if;
    if mine.authority is null then
        raise exception '% has no authority yet: %', p_repo_key,
              coalesce(mine.reason, mine.reason_code, 'forge visibility is not captured')
            using errcode = 'object_not_in_prerequisite_state';
    end if;
    if not mine.configurable_by_me then
        raise exception 'you may not change which branches % tracks', p_repo_key
              || case when mine.reason_actor is not null then ' (' || mine.reason_actor || ' can change it)' else '' end
            using errcode = 'insufficient_privilege';
    end if;

    if p_tracked then
        insert into dojo.repository_graph_branches as b (repository_id, tenant_id, branch, label, added_by)
        values (mine.repository_id, mine.tenant_id, v_branch, p_label, v_principal)
        on conflict (repository_id, branch) do update set label = excluded.label;
    else
        delete from dojo.repository_graph_branches b
         where b.repository_id = mine.repository_id and b.branch = v_branch;
    end if;

    return query
    select mine.repo_key, v_branch, p_tracked,
           coalesce((select b.is_default from dojo.repository_graph_branches b
                      where b.repository_id = mine.repository_id and b.branch = v_branch), false);
end;
$$;

revoke all on function dojo.set_graph_branch(text, text, boolean, text) from public;
grant execute on function dojo.set_graph_branch(text, text, boolean, text) to authenticated;

comment on function dojo.set_graph_branch(text, text, boolean, text) is
'Track or untrack a branch of a repository''s shared code graph, decided from the caller''s own verdict row (authority, configurable_by_me) as elections are. Refuses no_data_found, object_not_in_prerequisite_state, insufficient_privilege, invalid_parameter_value (empty name). Untracking deletes the branch''s graph (#225).';
