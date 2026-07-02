set search_path to sensei, activity, extensions;

-- Session tool-call timeline: pair every PreToolUse event with its matching
-- PostToolUse event so the Instruments Replay tab can render one row per
-- tool invocation with request + response + duration side by side.
--
-- Pairing rule: within a session, PreToolUse and PostToolUse events for the
-- same tool are matched sequentially by their firing order. Row 1 of
-- PreToolUse pairs with row 1 of PostToolUse for that (session, tool), row
-- 2 with row 2, etc. If a PreToolUse has no matching PostToolUse yet (call
-- still in-flight or aborted), post_at / response / success stay NULL.
--
-- Duration is in-flight-safe: milliseconds only when both events exist,
-- otherwise NULL.
create or replace view session_tool_calls as
with pre as (
  select
      session_id
    , family
    , tool_name
    , id                       as pre_id
    , ts                       as pre_ts
    , created_at               as pre_at
    , payload                  as request
    , row_number() over (partition by session_id, tool_name order by ts, id) as seq
    from activity.assistant_events
   where event_type = 'PreToolUse'
     and tool_name is not null
     and session_id <> ''
),
post as (
  select
      session_id
    , tool_name
    , id                       as post_id
    , ts                       as post_ts
    , created_at               as post_at
    , payload                  as response
    , success
    , row_number() over (partition by session_id, tool_name order by ts, id) as seq
    from activity.assistant_events
   where event_type = 'PostToolUse'
     and tool_name is not null
     and session_id <> ''
)
select
    pre.session_id
  , pre.family
  , pre.tool_name
  , pre.pre_id                                        as call_id
  , pre.request
  , post.response
  , post.success
  , pre.pre_ts                                        as started_at_ms
  , post.post_ts                                      as completed_at_ms
  , case when post.post_ts is null then null
         else greatest(post.post_ts - pre.pre_ts, 0)
    end                                               as duration_ms
  , pre.pre_at                                        as started_at
  , post.post_at                                      as completed_at
  from pre
  left join post
    on pre.session_id = post.session_id
   and pre.tool_name  = post.tool_name
   and pre.seq        = post.seq
;

comment on view session_tool_calls is
'Paired PreToolUse ↔ PostToolUse timeline for the Instruments Replay tab.
One row per tool invocation in a session with request payload, response
payload, success flag, and duration_ms. Unpaired PreToolUse rows (in-flight
or aborted calls) have NULL response / success / completed_at fields.
Pairing is by sequential row_number within (session_id, tool_name) — so a
session that calls `search` three times gets its three PreToolUse rows
paired with three PostToolUse rows in order. Session_id is the assistant
string session ID, not a DB UUID.';
