set search_path to platform, extensions;

create or replace view tool_usage as
select
  model
, count(*)                                           as total_calls
, sum(input_tokens)                                  as total_input_tokens
, sum(output_tokens)                                 as total_output_tokens
, round(avg(cost_usd)::numeric, 6)                   as avg_cost_usd
, round(avg(duration_ms)::numeric, 0)                as avg_duration_ms
from sensei.api_requests
group by model
order by total_calls desc;

comment on view tool_usage is
'Aggregated tool call stats across all accounts — no personal identifiers.
Used for platform health monitoring and tool performance tracking.';
