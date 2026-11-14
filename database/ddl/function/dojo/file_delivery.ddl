set search_path to dojo, extensions;

-- File one board push for the signed-in person's daemon, both parts in ONE
-- transaction (spec tracker-delivery-metrics D4, A2; #353): the facts as
-- dojo.file_delivery_facts files them, then — when p_effort is given — the
-- caller's own effort rows for the board, REPLACING everything they reported
-- for it before from the same install (B13; p_effort.source, the install's id). Effort is filed even when the facts answer 'stale': it is
-- the caller's alone.
--
-- p_effort = { project: Row|null, sprints: [{external_id, …Row}], tickets: [{key, …Row}] },
-- Row = { repo_keys, values, reasons }. A row's repo_keys must be among the
-- request's (invalid_parameter_value); its repository_ids are their verified ids.
create or replace function dojo.file_delivery(
    p_tenant    uuid,
    p_repo_keys text[],
    p_board     jsonb,
    p_effort    jsonb
)
    returns jsonb
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_principal uuid := dojo.current_principal_id();
    v_facts     text;
    v_board     uuid;
    v_rows      jsonb;
begin
    v_facts := dojo.file_delivery_facts(p_tenant, p_repo_keys, p_board);
    if p_effort is null then
        return jsonb_build_object('facts', v_facts);
    end if;
    if jsonb_typeof(p_effort) <> 'object'
       or jsonb_typeof(p_effort -> 'sprints') is distinct from 'array'
       or jsonb_typeof(p_effort -> 'tickets') is distinct from 'array'
       or (p_effort -> 'project' is not null and jsonb_typeof(p_effort -> 'project') not in ('object', 'null'))
       or (p_effort ->> 'source') is null
       or (p_effort ->> 'source') !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' then
        raise exception 'effort carries its source install, project, sprints and tickets' using errcode = 'invalid_parameter_value';
    end if;
    -- The bounds hold here, whoever calls (D3): the board's own, for its effort too.
    if jsonb_array_length(p_effort -> 'tickets') > 5000 or jsonb_array_length(p_effort -> 'sprints') > 1000
       or octet_length(p_effort::text) > 4 * 1024 * 1024 then
        raise exception 'effort carries at most 5000 tickets, 1000 sprints and 4 MB' using errcode = 'invalid_parameter_value';
    end if;

    select b.id into strict v_board
      from dojo.delivery_boards b
     where b.board_key = p_board ->> 'board_key'
       and b.workspace_id = (select distinct sc.workspace_id from dojo.delivery_scope(p_tenant, p_repo_keys) sc);

    -- Every row, tagged with its kind and reference (none: v_rows stays NULL and
    -- the caller's rows are simply cleared).
    select jsonb_agg(q.x) into v_rows
      from (select (p_effort -> 'project') || '{"item_kind": "project", "item_ref": ""}'::jsonb as x
             where jsonb_typeof(p_effort -> 'project') = 'object'
            union all
            select e || jsonb_build_object('item_kind', 'sprint', 'item_ref', e ->> 'external_id')
              from jsonb_array_elements(p_effort -> 'sprints') e
            union all
            select e || jsonb_build_object('item_kind', 'ticket', 'item_ref', e ->> 'key')
              from jsonb_array_elements(p_effort -> 'tickets') e) q;

    if exists (
        select 1 from jsonb_array_elements(v_rows) x
         where x ->> 'item_ref' is null
            or length(x ->> 'item_ref') > 255
            or (x ->> 'item_kind' <> 'project' and x ->> 'item_ref' = '')
            or not dojo.delivery_effort_row_ok(x ->> 'item_kind', x -> 'values', x -> 'reasons')
            or jsonb_typeof(x -> 'repo_keys') is distinct from 'array'
            or jsonb_array_length(x -> 'repo_keys') = 0
            or exists (select 1 from jsonb_array_elements_text(x -> 'repo_keys') k
                        where k.value <> all(p_repo_keys))) then
        raise exception 'each effort row names its item and repositories among the request''s, with the wire''s values and reasons'
            using errcode = 'invalid_parameter_value';
    end if;

    delete from dojo.delivery_effort e
     where e.board_id = v_board and e.principal_id = v_principal and e.source = (p_effort ->> 'source')::uuid;
    insert into dojo.delivery_effort (board_id, item_kind, item_ref, principal_id, source, repository_ids, "values", reasons)
    select v_board, x ->> 'item_kind', x ->> 'item_ref', v_principal, (p_effort ->> 'source')::uuid,
           array(select sc.repository_id
                   from dojo.delivery_scope(p_tenant, p_repo_keys) sc
                  where sc.repo_key in (select jsonb_array_elements_text(x -> 'repo_keys'))
                  order by sc.repository_id),
           x -> 'values', x -> 'reasons'
      from jsonb_array_elements(v_rows) x;

    return jsonb_build_object('facts', v_facts, 'effort', 'accepted');
end
$$;

revoke all on function dojo.file_delivery(uuid, text[], jsonb, jsonb) from public;
grant execute on function dojo.file_delivery(uuid, text[], jsonb, jsonb) to authenticated;

comment on function dojo.file_delivery(uuid, text[], jsonb, jsonb) is
'File one board push in one transaction (#353, tracker-delivery-metrics D4 A2): the facts as dojo.file_delivery_facts, then the caller''s effort rows for the board, replacing all they reported before (B13), even when the facts are stale. A row''s repo_keys must be among the request''s; its repository_ids are their verified ids. Answers {facts, effort}.';
