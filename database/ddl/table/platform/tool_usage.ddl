set search_path to platform, extensions;

create or replace view tool_usage as
select
  tool_name
, count(*)                                                         as total_calls
, round(
    100.0 * count(*) filter (where status = 'error') / count(*), 2
  )                                                                as error_rate_pct
, round(avg(duration_ms)::numeric, 0)                             as avg_duration_ms
from sensei.api_requests
group by tool_name
order by total_calls desc;

comment on view tool_usage is
'Aggregated tool call stats across all accounts — no personal identifiers.
Used for platform health monitoring and tool performance tracking.';
