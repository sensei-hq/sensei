set search_path to activity, extensions;

-- A session's API cost: its priced messages summed, with what could not be
-- priced beside it (docs/spec/daemon/session-api-cost.md C4). `unpriced` =
-- messages with no price row; `hook_only_turns` = turns only the hook saw
-- (subagents), which have no messages to price. A session with no messages has
-- no row.
create or replace view activity.session_api_cost as
select c.session_id
     , sum(c.cost)                  as api_cost
     , count(c.cost)                as priced
     , count(*) - count(c.cost)     as unpriced
     , ( select count(*)
           from activity.turn_usage tu
          where tu.session_id = c.session_id
            and not tu.seen_by_transcript ) as hook_only_turns
  from activity.message_api_cost c
 group by c.session_id;

comment on view activity.session_api_cost is
'Per session: API cost of its priced messages, counts of priced/unpriced messages, and hook-only (subagent) turns.';
