-- Platform admin views (anonymized cross-account analytics)
create schema if not exists platform;

-- FTR distribution histogram (score bucketed, no personal data)
create or replace view platform.ftr_distribution as
select
  width_bucket(ftr_score, 0, 1, 10) as bucket,
  round(min(ftr_score), 1) as bucket_min,
  round(max(ftr_score), 1) as bucket_max,
  count(*) as session_count
from sensei.task_sessions
where ftr_score is not null
  and status = 'completed'
group by 1
order by 1;

-- Per-account aggregates (no personal identifiers)
create or replace view platform.account_stats as
select
  a.id as account_id,
  a.slug as account_slug,
  a.account_type,
  count(distinct r.id) as repo_count,
  count(distinct s.id) filter (where s.created_at >= now() - interval '30 days') as sessions_30d,
  round(avg(ts.ftr_score)::numeric, 3) as avg_ftr,
  round(sum(ar.cost_usd)::numeric, 6) as total_cost_usd
from core.accounts a
left join sensei.repos r on r.account_id = a.id
left join sensei.sessions s on s.account_id = a.id
left join sensei.task_sessions ts on ts.account_id = a.id and ts.ftr_score is not null
left join sensei.api_requests ar on ar.account_id = a.id
where a.is_platform = false
group by a.id, a.slug, a.account_type;

-- Tool usage frequency + error rate
create or replace view platform.tool_usage as
select
  tool,
  count(*) as total_calls,
  round(count(*) filter (where success = false)::numeric / nullif(count(*), 0), 3) as error_rate,
  round(avg(duration_ms)::numeric) as avg_duration_ms
from sensei.task_turns
group by tool
order by total_calls desc;
