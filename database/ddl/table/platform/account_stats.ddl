set search_path to platform, extensions;

create or replace view account_stats as
select
  a.id                                                                      as account_id
, a.slug                                                                    as account_slug
, a.account_type
, count(distinct r.id)                                                      as repo_count
, count(distinct s.id) filter (where s.created_at >= now() - interval '30 days') as sessions_30d
, round(avg(ts.ftr_score)::numeric, 3)                                      as avg_ftr
, round(sum(ar.cost_usd)::numeric, 6)                                       as total_cost_usd
from core.accounts a
left join sensei.repos r         on r.account_id = a.id
left join sensei.sessions s      on s.account_id = a.id
left join sensei.task_sessions ts on ts.account_id = a.id and ts.ftr_score is not null
left join sensei.api_requests ar  on ar.account_id = a.id
group by a.id, a.slug, a.account_type
order by sessions_30d desc;

comment on view account_stats is
'Per-account aggregates for platform admin analytics. No personal identifiers beyond account slug.';
