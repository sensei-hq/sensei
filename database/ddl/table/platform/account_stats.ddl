set search_path to platform, extensions;

create or replace view account_stats as
select
  a.id                                                                         as account_id
, a.slug                                                                       as account_slug
, a.account_type
, count(distinct ss.id)                                                        as sync_session_count
, count(distinct ss.id) filter (where ss.recorded_at >= now() - interval '30 days') as sessions_30d
, round(avg(ss.ftr_score)::numeric, 3)                                         as avg_ftr
, round(avg(ss.token_cost)::numeric, 6)                                        as avg_token_cost
from core.accounts a
left join sensei.sync_sessions ss on ss.account_id = a.id
group by a.id, a.slug, a.account_type
order by sessions_30d desc;

comment on view account_stats is
'Per-account aggregates for platform admin analytics. No personal identifiers beyond account slug.
Uses sync_sessions (collector data) — not raw sensei.sessions (local only).';
