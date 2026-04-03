set search_path to platform, extensions;

create or replace view ftr_distribution as
select
  width_bucket(ftr_score, 0, 1, 10) as bucket
, round(min(ftr_score), 1)          as bucket_min
, round(max(ftr_score), 1)          as bucket_max
, count(*)                          as session_count
from sensei.task_sessions
where ftr_score is not null
  and status = 'completed'
group by 1
order by 1;

comment on view ftr_distribution is
'Anonymized FTR score histogram across all accounts (10 equal-width buckets 0–1).
No personal identifiers — safe for platform admin display.';
