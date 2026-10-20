set search_path to activity, extensions;

-- One row per turn of a session, with its token usage counted ONCE (#280,
-- docs/decisions.md 2026-10-19).
--
-- Two sources see a turn and share no id: the hook's `TurnComplete` (the usage
-- mod, live, every request of the turn summed, subagent turns included) and the
-- transcript (history, one row per human-prompt turn, sidechains not read). The
-- hook's `turn_id` is not in the transcript and the transcript's `requestId` is
-- not in the hook, so a main-loop hook turn links to the transcript turn whose
-- span — `[started_at, next turn's started_at)` — holds the hook's end. Measured
-- on the first live pair: 68 ms apart, equal usage.
--
-- A turn both saw is ONE row. `agreed` says whether their four counts match; on
-- a disagreement the row carries the hook's figures (the engine's own sum of its
-- requests) and `agreed = false` says so — never an average, never silent. A turn
-- one source saw carries that source's figures and `agreed` is null. Subagent
-- turns (`agent_id` set) come from hooks alone.
create or replace view turn_usage as
with transcript as (
    select t.session_id
         , t.turn_index
         , t.started_at
         , lead(t.started_at) over (partition by t.source, t.session_id order by t.turn_index) as next_started_at
         , t.model
         , t.tokens_in   as fresh
         , t.cache_read
         , t.cache_write
         , t.tokens_out  as output
      from activity.transcript_turns t
     where not coalesce(t.is_sidechain, false)
       and t.tokens_in is not null
), hook as (
    select e.session_id
         , e.payload ->> 'turn_id'                                          as hook_turn_id
         , nullif(e.payload ->> 'agent_id', '')                             as agent_id
         , to_timestamp(e.ts / 1000.0)                                      as ended_at
         , e.payload -> 'usage' ->> 'model'                                 as model
         , coalesce((e.payload -> 'usage' ->> 'input_tokens')::bigint, 0)                as fresh
         , coalesce((e.payload -> 'usage' ->> 'cache_read_input_tokens')::bigint, 0)     as cache_read
         , coalesce((e.payload -> 'usage' ->> 'cache_creation_input_tokens')::bigint, 0) as cache_write
         , coalesce((e.payload -> 'usage' ->> 'output_tokens')::bigint, 0)               as output
      from activity.assistant_events e
     where e.event_type = 'TurnComplete'
       and e.payload ? 'usage'
), hook_linked as (
    -- A main-loop hook turn takes the transcript turn whose span holds its end.
    select h.*
         , (select t.turn_index
              from transcript t
             where h.agent_id is null
               and t.session_id = h.session_id
               and h.ended_at >= t.started_at
               and (t.next_started_at is null or h.ended_at < t.next_started_at)) as turn_index
      from hook h
), hook_by_turn as (
    -- Two hook turns inside one transcript turn are summed, then compared.
    select session_id, turn_index
         , sum(fresh)::bigint as fresh, sum(cache_read)::bigint as cache_read
         , sum(cache_write)::bigint as cache_write, sum(output)::bigint as output
         , max(model) as model
      from hook_linked
     where turn_index is not null
     group by session_id, turn_index
)
-- Transcript turns, with the hook's figures where a hook saw the same turn.
select t.session_id
     , t.turn_index
     , null::text                                  as agent_id
     , t.started_at
     , coalesce(h.model, t.model)                  as model
     , h.session_id is not null                    as seen_by_hook
     , true                                        as seen_by_transcript
     , case when h.session_id is null then null
            else (h.fresh, h.cache_read, h.cache_write, h.output)
               = (t.fresh, coalesce(t.cache_read, 0), coalesce(t.cache_write, 0), coalesce(t.output, 0))
       end                                         as agreed
     , coalesce(h.fresh, t.fresh)                  as fresh
     , coalesce(h.cache_read, t.cache_read, 0)     as cache_read
     , coalesce(h.cache_write, t.cache_write, 0)   as cache_write
     , coalesce(h.output, t.output, 0)             as output
  from transcript t
  left join hook_by_turn h on h.session_id = t.session_id and h.turn_index = t.turn_index
union all
-- Hook turns no transcript turn holds: subagent turns, and main-loop turns of a
-- session whose transcript was not read.
select h.session_id
     , null::integer                               as turn_index
     , h.agent_id
     , h.ended_at                                  as started_at
     , h.model
     , true                                        as seen_by_hook
     , false                                       as seen_by_transcript
     , null::boolean                               as agreed
     , h.fresh, h.cache_read, h.cache_write, h.output
  from hook_linked h
 where h.turn_index is null;

comment on view turn_usage is
'One row per turn with its token usage counted once (#280). A main-loop hook TurnComplete links to the transcript
turn whose span holds its end; agreed = their counts match (on a disagreement the hook''s figures, flagged). Turns
one source saw carry that source; subagent turns are hook-only.';

grant select on turn_usage to authenticated, service_role;
