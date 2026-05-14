set search_path to sensei, activity, extensions;

CREATE OR REPLACE VIEW sensei.tool_usage_stats AS
SELECT e.data->>'tool_name'                            AS tool_name
     , COUNT(*)                                         AS call_count
     , COUNT(*) FILTER (
         WHERE e.data->>'error' IS NOT NULL
            OR (e.data->>'exit_code' IS NOT NULL
                AND e.data->>'exit_code' != '0')
       )                                                AS error_count
     , AVG((e.data->>'duration_ms')::numeric)           AS avg_duration_ms
     , MAX(e.created_at)                                AS last_used_at
  FROM activity.events e
 WHERE e.event_type = 'tool_call'
   AND e.data->>'tool_name' IS NOT NULL
 GROUP BY e.data->>'tool_name';

comment on view tool_usage_stats is
'Aggregated tool usage statistics across all sessions.
Powers Instruments health dashboard and tool popularity ranking.
- call_count: total invocations of this tool
- error_count: calls that returned an error or non-zero exit code
- avg_duration_ms: average response time (from data.duration_ms)
- last_used_at: most recent invocation timestamp';
