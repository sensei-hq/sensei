set search_path to sensei, activity, extensions;

-- Aggregates the captured tool calls in activity.hook_events (written by the
-- pre/post-tool-use hooks via POST /hook/event). Counts completed calls
-- (PostToolUse rows, which carry the success flag); PreToolUse rows are the
-- paired "about to run" signal and are excluded so each call is counted once.
-- (Was previously reading activity.events WHERE event_type='tool_call', which
--  the hooks never populate — so the view was always empty despite thousands
--  of captured calls. See issue #58.)
CREATE OR REPLACE VIEW sensei.tool_usage_stats AS
SELECT h.tool_name                                        AS tool_name
     , COUNT(*)                                            AS call_count
     , COUNT(*) FILTER (WHERE h.success = false)           AS error_count
     , NULL::numeric                                       AS avg_duration_ms
     , to_timestamp(MAX(h.ts) / 1000.0)                    AS last_used_at
  FROM activity.hook_events h
 WHERE h.event_type = 'PostToolUse'
   AND h.tool_name IS NOT NULL
 GROUP BY h.tool_name;

comment on view tool_usage_stats is
'Aggregated tool usage statistics across all captured sessions.
Powers the Instruments health dashboard and tool popularity ranking.
Source: activity.hook_events (PostToolUse rows).
- call_count: completed invocations of this tool
- error_count: calls whose hook reported success=false
- avg_duration_ms: not yet captured per-call (PreToolUse/PostToolUse ts pairing
  is a follow-up); NULL for now
- last_used_at: most recent invocation timestamp';
