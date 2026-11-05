set search_path to dojo, extensions;

-- One board item's team effort as THIS reader may see it (spec
-- tracker-delivery-metrics D4 "Who sees", B16, A4; #353, #354).
--
-- A reader who may see every contributor's own numbers on the item —
-- dojo.can_read_repository_metric(r, 'user', principal, tenant) for every
-- repository r of every row: the person themself, a tenant admin, a maintainer
-- on those repositories, or the person's team lead — gets the merge in full
-- (cost and a single contributor included), the latest report, and the people:
-- each contributor's rows across their installs merged, named, with their latest
-- report and whether it is older than 7 days. Anyone else gets B16's view
-- (dojo.delivery_team_view: nothing from one contributor, no cost), the latest
-- report only from two contributors on, no people — and one `stale` flag, true
-- when any contributor last reported over 7 days ago, never whom, and only from
-- two contributors on.
create or replace function dojo.delivery_team(
    p_board            uuid,
    p_kind             text,
    p_ref              text,
    p_issue_created_at timestamptz
)
    returns table (team jsonb, contributors integer, last_report timestamptz, people jsonb, stale boolean)
    language plpgsql
    stable
    set search_path = dojo, pg_temp
as $$
declare
    v_tenant  uuid;
    v_tickets integer;
    v_rows    jsonb;
    v_k       integer;
    v_latest  timestamptz;
    v_full    boolean;
    v_stale   boolean;
    v_people  jsonb;
begin
    select b.tenant_id into v_tenant from dojo.delivery_boards b where b.id = p_board;
    -- The project's per-ticket denominator: the board's tickets with a row from anyone.
    select count(distinct e.item_ref)::integer into v_tickets
      from dojo.delivery_effort e
      join dojo.delivery_tickets t on t.board_id = e.board_id and t.key = e.item_ref
     where e.board_id = p_board and e.item_kind = 'ticket';

    select jsonb_agg(jsonb_build_object('values', e."values", 'reasons', e.reasons)),
           count(distinct e.principal_id)::integer,
           max(e.reported_at),
           bool_and(not exists (select 1 from unnest(e.repository_ids) r
                                 where not dojo.can_read_repository_metric(r, 'user', e.principal_id, v_tenant)))
      into v_rows, v_k, v_latest, v_full
      from dojo.delivery_effort e
     where e.board_id = p_board and e.item_kind = p_kind and e.item_ref = p_ref
       and (e."values" ->> 'sessions')::numeric >= 1;

    if v_k = 0 then
        return query select null::jsonb, 0, null::timestamptz, null::jsonb, false;
        return;
    end if;

    select bool_or(q.latest < now() - interval '7 days') into v_stale
      from (select max(e.reported_at) as latest
              from dojo.delivery_effort e
             where e.board_id = p_board and e.item_kind = p_kind and e.item_ref = p_ref
               and (e."values" ->> 'sessions')::numeric >= 1
             group by e.principal_id) q;

    if not v_full then
        return query select dojo.delivery_team_view(
                                dojo.delivery_effort_merged(p_kind, v_rows, p_issue_created_at, v_tickets), v_k),
                            v_k,
                            case when v_k >= 2 then v_latest end,
                            null::jsonb,
                            -- one contributor's report age is theirs, like their report time
                            v_k >= 2 and v_stale;
        return;
    end if;

    select jsonb_agg(jsonb_build_object(
               'principal_id', q.principal_id,
               'display_name', pr.display_name,
               'team', dojo.delivery_effort_merged(p_kind, q.rows, p_issue_created_at, v_tickets),
               'reported_at', q.latest,
               'stale', q.latest < now() - interval '7 days')
             order by pr.display_name, q.principal_id)
      into v_people
      from (select e.principal_id,
                   jsonb_agg(jsonb_build_object('values', e."values", 'reasons', e.reasons)) as rows,
                   max(e.reported_at) as latest
              from dojo.delivery_effort e
             where e.board_id = p_board and e.item_kind = p_kind and e.item_ref = p_ref
               and (e."values" ->> 'sessions')::numeric >= 1
             group by e.principal_id) q
      left join dojo.principals pr on pr.id = q.principal_id;

    return query select dojo.delivery_effort_merged(p_kind, v_rows, p_issue_created_at, v_tickets),
                        v_k, v_latest, v_people, v_stale;
end
$$;

revoke all on function dojo.delivery_team(uuid, text, text, timestamptz) from public;

comment on function dojo.delivery_team(uuid, text, text, timestamptz) is
'One board item''s team effort as the reader may see it (#353, #354; B16, A4): in full with each contributor''s merged rows (people) when can_read_repository_metric admits the reader to every row''s repositories; otherwise B16''s restricted merge, no people, and the latest report only from two contributors on. stale: any contributor''s last report is over 7 days old.';
