set search_path to sensei, activity, extensions;

-- Session tool-call timeline: one row per tool invocation, its request beside
-- ITS response, for the Instruments Replay tab.
--
-- Pairing is by Claude Code's `tool_use_id`, which both the PreToolUse and the
-- PostToolUse payload carry. It used to be by firing order within
-- (session, tool), which parallel calls break: measured 2026-10-08, 183,376 of
-- 264,589 paired rows (69%) joined two different tool_use_ids (#238).
--
-- One event per id on each side (`distinct on`, earliest row): the 2026-06-01..15
-- double ingestion left identical copies of 7,184 calls, and a join would
-- multiply them. An event with NO id pairs with nothing — it is never matched by
-- order as a guess, so its response and duration stay NULL.
--
-- The response side takes PostToolUse OR PostToolUseFailure: Claude Code fires
-- the failure event INSTEAD of PostToolUse when a call fails, so pairing only
-- PostToolUse left every failed call looking in flight forever (#238).
--
-- Both keys are the table's stored columns, written with the event: reading
-- them out of `payload` here opened every PostToolUse's tool response and made
-- workspace Instruments take 11–28s to open (#298).
--
-- `duration_ms` is the ENGINE's measure from the PostToolUse payload, not the
-- gap between the two hooks' timestamps (which included hook-process delay and,
-- on a mis-paired row, another call's time).
create or replace view session_tool_calls as
with pre as (
  select distinct on (session_id, coalesce(tool_use_id, id::text))
      session_id
    , family
    , tool_name
    , id                         as pre_id
    , ts                         as pre_ts
    , created_at                 as pre_at
    , payload                    as request
    , tool_use_id
    from activity.assistant_events
   where event_type = 'PreToolUse'
     and tool_name is not null
     and session_id <> ''
   order by session_id, coalesce(tool_use_id, id::text), id
),
post as (
  select distinct on (session_id, tool_use_id)
      session_id
    , tool_use_id
    , id                                          as post_id
    , ts                                          as post_ts
    , created_at                                  as post_at
    , payload                                     as response
    , success
    , duration_ms
    , payload->>'agent_id'                        as agent_id
    , payload->>'agent_type'                      as agent_type
    from activity.assistant_events
   where event_type in ('PostToolUse', 'PostToolUseFailure')
     and session_id <> ''
     and tool_use_id is not null
   order by session_id, tool_use_id, id
)
select
    pre.session_id
  , pre.family
  , pre.tool_name
  , pre.pre_id                                              as call_id
  , pre.request
  , post.response
  , post.success
  , pre.pre_ts                                              as started_at_ms
  , post.post_ts                                            as completed_at_ms
  , post.duration_ms
  , pre.pre_at                                              as started_at
  , post.post_at                                            as completed_at
  -- Appended last: `create or replace view` may only add columns at the end.
  , pre.tool_use_id
  , coalesce(post.agent_id, pre.request->>'agent_id')       as agent_id
  , coalesce(post.agent_type, pre.request->>'agent_type')   as agent_type
  , post.post_id                                            as response_id
  from pre
  left join post
    on post.session_id  = pre.session_id
   and post.tool_use_id = pre.tool_use_id
;

comment on view session_tool_calls is
'One row per tool invocation in an assistant session: the PreToolUse request
beside ITS PostToolUse / PostToolUseFailure response, paired by `tool_use_id`
(never by order); `success` is false for a failure.
`duration_ms` is the engine''s measured duration from the PostToolUse payload.
`agent_id`/`agent_type` name the subagent that made the call (NULL for the main
loop). `response_id` is the PostToolUse event id — what `tool_call_verdicts`
keys on. A call with no `tool_use_id`, or still in flight, has NULL response
fields. Session_id is the assistant string session id, not a DB UUID.';
