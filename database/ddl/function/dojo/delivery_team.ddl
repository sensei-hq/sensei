set search_path to dojo, extensions;

-- One board item's team effort as a workspace reader sees it (spec
-- tracker-delivery-metrics D4, B16; #353): the contributors' rows merged
-- (dojo.delivery_effort_merged), restricted (dojo.delivery_team_view), how many
-- people contributed (one person's two installs are one contributor), and — only from two contributors on, since one person's report
-- time is theirs — the latest report.
create or replace function dojo.delivery_team(
    p_board            uuid,
    p_kind             text,
    p_ref              text,
    p_issue_created_at timestamptz
)
    returns table (team jsonb, contributors integer, last_report timestamptz)
    language sql
    stable
    set search_path = dojo, pg_temp
as $$
    with rows as (
        select e.principal_id, e."values", e.reasons, e.reported_at
          from dojo.delivery_effort e
         where e.board_id = p_board and e.item_kind = p_kind and e.item_ref = p_ref
           and (e."values" ->> 'sessions')::numeric >= 1
    ), n as (select count(distinct principal_id)::integer as k, max(reported_at) as latest from rows)
    select dojo.delivery_team_view(
               dojo.delivery_effort_merged(
                   p_kind,
                   (select jsonb_agg(jsonb_build_object('values', r."values", 'reasons', r.reasons)) from rows r),
                   p_issue_created_at,
                   (select count(distinct e.item_ref)::integer from dojo.delivery_effort e
                     join dojo.delivery_tickets t on t.board_id = e.board_id and t.key = e.item_ref
                     where e.board_id = p_board and e.item_kind = 'ticket')),
               n.k),
           n.k,
           case when n.k >= 2 then n.latest end
      from n;
$$;

revoke all on function dojo.delivery_team(uuid, text, text, timestamptz) from public;

comment on function dojo.delivery_team(uuid, text, text, timestamptz) is
'One board item''s merged team effort as a workspace reader sees it under B16 (NULL from fewer than two contributors, no cost), the contributor count, and the latest report only from two contributors on (#353).';
