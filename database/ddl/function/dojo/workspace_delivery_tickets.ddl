set search_path to dojo, extensions;

-- A board's tickets from its kept copy, for a reader of the workspace (spec
-- tracker-delivery-metrics D4; #352): all of them, one sprint's (p_sprint, an
-- external id), or the backlog's (p_backlog: in no non-removed sprint), in the
-- app's order — key prefix, then its number, then the key — so AUTH-2 comes
-- before AUTH-10 in both apps. Paged; the caller reads every page.
create or replace function dojo.workspace_delivery_tickets(
    p_workspace uuid,
    p_board     uuid,
    p_sprint    text,
    p_backlog   boolean,
    p_limit     integer,
    p_offset    integer
)
returns table (
    key              text,
    title            text,
    status_category  text,
    points           numeric,
    sprints          text[],
    issue_created_at timestamptz,
    team             jsonb,
    team_words       jsonb,
    contributors     integer,
    last_report      timestamptz
)
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
    select t.key, t.title, t.status_category, t.points, t.sprints, t.issue_created_at,
           m.team, dojo.delivery_reason_words(m.team -> 'reasons'), m.contributors, m.last_report
      from dojo.delivery_tickets t
      join dojo.delivery_boards b on b.id = t.board_id
      cross join lateral dojo.delivery_team(b.id, 'ticket', t.key, t.issue_created_at) m
     where b.id = p_board
       and b.workspace_id = p_workspace
       and dojo.can_read_workspace(p_workspace)
       and (p_sprint is null or p_sprint = any(t.sprints))
       and (p_backlog is not true
            or not exists (select 1 from dojo.delivery_sprints s
                            where s.board_id = t.board_id and not s.removed
                              and s.external_id = any(t.sprints)))
     order by regexp_replace(t.key, '[0-9]+$', ''),
              nullif(substring(t.key from '([0-9]+)$'), '')::numeric nulls first,
              t.key
     limit p_limit offset p_offset;
$$;

revoke all on function dojo.workspace_delivery_tickets(uuid, uuid, text, boolean, integer, integer) from public;
grant execute on function dojo.workspace_delivery_tickets(uuid, uuid, text, boolean, integer, integer) to authenticated;

comment on function dojo.workspace_delivery_tickets(uuid, uuid, text, boolean, integer, integer) is
'A board''s tickets from its kept copy — all, one sprint''s, or the backlog''s — with their titles and the team''s merged effort under B16 (team, team_words, contributors, last_report), in the app''s key order, for a reader of the workspace (dojo.can_read_workspace); paged (#352, #353).';
