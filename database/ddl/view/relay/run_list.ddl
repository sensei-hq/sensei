set search_path to relay, extensions;

-- One row per sprint run, open or ended (#419; CONTRACT §1i, §3k): its tasks
-- counted, the lanes that took part, and when an ended run is deleted —
-- 30 days after it completed or was cancelled. Open runs (draft, planning,
-- active) have no delete date. Security-invoker: the caller's RLS applies.
create or replace view relay.run_list with (security_invoker = true) as
select
  r.user_id,
  r.id,
  r.crew_id,
  r.title,
  r.goal,
  r.state,
  r.repo_key,
  r.workspace_id,
  r.created_at,
  r.started_at,
  r.completed_at,
  case when r.state in ('completed', 'cancelled')
    then r.completed_at + interval '30 days' end as delete_after,
  (select count(*) from relay.tasks t
    where t.user_id = r.user_id and t.run_id = r.id) as total_tasks,
  (select count(*) from relay.tasks t
    where t.user_id = r.user_id and t.run_id = r.id and t.state = 'done') as done_tasks,
  -- Who took part: every lane that wrote on one of its tasks.
  (select count(distinct a.lane_id) from relay.actions a
     join relay.tasks t on t.user_id = a.user_id and t.id = a.task_id
    where t.user_id = r.user_id and t.run_id = r.id and a.lane_id is not null) as members
from relay.runs r;

comment on view relay.run_list is
'Sprint runs, open and ended: task counts, members, and delete_after (completed_at + 30 days) once ended.';
