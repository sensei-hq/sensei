set search_path to dojo, extensions;

-- A board's sprints from its kept copy, for a reader of the workspace (spec
-- tracker-delivery-metrics D4; #352): removed sprints hidden, in the daemon's
-- order (start date newest first, undated last, then seq, then the id) so the
-- dōjō and the app list one board alike. Paged; the caller reads every page.
create or replace function dojo.workspace_delivery_sprints(
    p_workspace uuid,
    p_board     uuid,
    p_limit     integer,
    p_offset    integer
)
returns table (
    external_id text,
    seq         bigint,
    name        text,
    state       text,
    start_at    timestamptz,
    end_at      timestamptz,
    complete_at timestamptz,
    sdlc        jsonb,
    reason_words jsonb
)
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
    select s.external_id, s.seq, s.name, s.state, s.start_at, s.end_at, s.complete_at, s.sdlc,
           (select jsonb_object_agg(r.code, r.summary)
              from sensei.reason_codes r
             where r.domain = 'sdlc_metrics'
               and r.code in (select x.value
                                from jsonb_each_text(case when jsonb_typeof(s.sdlc -> 'reasons') = 'object'
                                                          then s.sdlc -> 'reasons' end) x))
      from dojo.delivery_sprints s
      join dojo.delivery_boards b on b.id = s.board_id
     where b.id = p_board
       and b.workspace_id = p_workspace
       and not s.removed
       and dojo.can_read_workspace(p_workspace)
     order by s.start_at desc nulls last, s.seq desc, s.external_id
     limit p_limit offset p_offset;
$$;

revoke all on function dojo.workspace_delivery_sprints(uuid, uuid, integer, integer) from public;
grant execute on function dojo.workspace_delivery_sprints(uuid, uuid, integer, integer) to authenticated;

comment on function dojo.workspace_delivery_sprints(uuid, uuid, integer, integer) is
'A board''s non-removed sprints from its kept copy, in the daemon''s order (start_at desc nulls last, seq desc), each with the words of the reason codes in its figures (reason_words, code → summary), for a reader of the workspace (dojo.can_read_workspace); paged (#352).';
