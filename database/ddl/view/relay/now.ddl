set search_path to relay, extensions;

-- The Now screen's grain, one view in six sections: one row per lane with
-- its active task, newest lease and the review it holds; open person
-- decisions; crew → person messages the person has not read; the manual
-- triage count; today's completed tasks; open sprint runs counted by state;
-- reports waiting for a reviewer. Security-invoker: the caller's RLS applies.
-- The daemon selects sections and shapes RelayNow; each section carries the
-- full grain (no per-row re-join). Tracker-side triage is composed by the
-- daemon per side — this view counts manual triage items only.
create or replace view relay.now with (security_invoker = true) as
select l.user_id, 'lane'::text as section, l.crew_id, l.id,
  jsonb_build_object(
    'code', l.code, 'title', l.title, 'suggested_name', l.suggested_name,
    'role', l.role, 'family', l.family,
    'version', l.version, 'model', l.model,
    'limit_scope', l.limit_scope, 'limit_resumes_at', l.limit_resumes_at,
    'limit_note', l.limit_note,
    'lease', case when ls.id is null then null
      else jsonb_build_object('state', ls.state, 'expires_at', ls.expires_at) end,
    'task', case when t.id is null then null
      else jsonb_build_object(
        'id', t.id, 'title', t.title, 'repo_key', t.repo_key, 'workspace_id', t.workspace_id,
        'phase', t.phase, 'state', t.state, 'round', t.round,
        'state_changed_at', t.state_changed_at,
        'needs_answer', exists (
          select 1 from relay.actions a
           where a.user_id = l.user_id and a.kind = 'asked'
             and a.props->>'asked_of' = 'person'
             and (a.task_id = t.id or a.props->'blocks' @> to_jsonb(t.id::text))
             and not exists (
               select 1 from relay.actions an
                where an.user_id = l.user_id and an.kind = 'answered' and an.reply_to = a.id))) end,
    'reviewing', case when tr.id is null then null
      else jsonb_build_object(
        'id', tr.id, 'title', tr.title, 'phase', tr.phase, 'round', tr.round,
        'claimed_at', tr.review_claimed_at) end
  ) as payload
from relay.lanes l
left join lateral (
  select * from relay.leases ll
   where ll.user_id = l.user_id and ll.crew_id = l.crew_id and ll.lane_id = l.id and ll.kind = 'lane'
   order by ll.id desc limit 1
) ls on true
left join relay.tasks t on t.user_id = l.user_id and t.lane_id = l.id
  and t.state in ('assigned', 'working', 'in_review', 'rework', 'blocked')
left join relay.tasks tr on tr.user_id = l.user_id and tr.review_claimed_by = l.id
  and tr.state = 'in_review'
union all
select a.user_id, 'open_asked', a.crew_id, a.id,
  jsonb_build_object(
    'task_id', a.task_id, 'task_title', t.title, 'question', a.body_md,
    'context', a.props->'context', 'options', a.props->'options',
    'recommended', a.props->'recommended', 'asked_at', a.created_at)
from relay.actions a
left join relay.tasks t on t.user_id = a.user_id and t.id = a.task_id
where a.kind = 'asked' and a.props->>'asked_of' = 'person'
  and not exists (
    select 1 from relay.actions an
     where an.user_id = a.user_id and an.kind = 'answered' and an.reply_to = a.id)
union all
-- Q2 part 2: to.person mail for the Needs-you section. A `read` receipt
-- with a null lane (the person is no lane) retires the row; the daemon's
-- person read walks delivered → read exactly like the lane envelope.
select m.user_id, 'person_message', m.crew_id, m.id,
  jsonb_build_object(
    'task_id', m.task_id, 'task_title', t.title,
    'from_lane_code', l.code, 'text', m.body_md, 'sent_at', m.created_at)
from relay.actions m
left join relay.tasks t on t.user_id = m.user_id and t.id = m.task_id
left join relay.lanes l on l.user_id = m.user_id and l.id = m.lane_id
where m.kind = 'message' and m.props->'to'->>'kind' = 'person'
  and not exists (
    select 1 from relay.actions r
     where r.user_id = m.user_id and r.kind = 'read' and r.reply_to = m.id
       and r.lane_id is null)
union all
-- One row per open sprint run with its tasks counted by state: the band
-- above the lanes. Planning runs count their Task 0; completed runs leave.
select r.user_id, 'sprint_summary'::text as section, r.crew_id, r.id,
  jsonb_build_object(
    'run_id', r.id, 'title', r.title, 'goal', r.goal, 'state', r.state,
    'created_at', r.created_at, 'started_at', r.started_at,
    'total_tasks', count(t.id),
    'ready_tasks', count(t.id) filter (where t.state = 'ready'),
    'working_tasks', count(t.id) filter (where t.state in ('assigned', 'working', 'rework')),
    'in_review_tasks', count(t.id) filter (where t.state = 'in_review'),
    'floating_reviews', count(t.id) filter (where t.state = 'in_review' and t.review_claimed_by is null),
    'approved_tasks', count(t.id) filter (where t.state = 'approved'),
    'done_tasks', count(t.id) filter (where t.state = 'done'),
    'blocked_tasks', count(t.id) filter (where t.state = 'blocked'),
    'interrupted_tasks', count(t.id) filter (where t.state = 'interrupted')) as payload
from relay.runs r
left join relay.tasks t on t.user_id = r.user_id and t.run_id = r.id
where r.state in ('planning', 'active')
group by r.user_id, r.crew_id, r.id
union all
-- One row per report waiting for a reviewer: the review queue's head, which
-- any non-author lane claims next.
select t.user_id, 'floating_review'::text as section, t.crew_id, t.id,
  jsonb_build_object(
    'task_id', t.id, 'run_id', t.run_id, 'title', t.title, 'phase', t.phase, 'round', t.round,
    'repo_key', t.repo_key, 'author_lane_code', l.code, 'author_lane_title', l.title,
    'reported_at', t.state_changed_at) as payload
from relay.tasks t
left join relay.lanes l on l.user_id = t.user_id and l.id = t.lane_id
where t.state = 'in_review' and t.review_claimed_by is null
union all
select i.user_id, 'triage_count', null::uuid, null::uuid,
  jsonb_build_object('provider', 'manual', 'sprint', null, 'count', count(*))
from relay.items i
where i.state = 'triage' and i.ticket_provider is null
group by i.user_id having count(*) > 0
union all
select t.user_id, 'completed_today', t.crew_id, t.id,
  jsonb_build_object(
    'task_id', t.id, 'crew_id', t.crew_id, 'repo_key', t.repo_key, 'title', t.title,
    'lane_code', l.code, 'lane_title', l.title,
    'outcome', case
      when exists (
        select 1 from relay.actions m
         where m.user_id = t.user_id and m.task_id = t.id and m.kind = 'merged')
      then 'merged'
      when t.state = 'approved' then 'approved'
      else 'done' end,
    'completed_at', coalesce(t.completed_at, t.state_changed_at))
from relay.tasks t
left join relay.lanes l on l.user_id = t.user_id and l.id = t.lane_id
where t.state in ('approved', 'done')
  -- current_timestamp, never now(): dbd resolves now() to this view.
  and coalesce(t.completed_at, t.state_changed_at)
      >= date_trunc('day', current_timestamp);

comment on view relay.now is
'Now grain: lane rows (task + lease + reviewing), open person decisions, unread crew → person messages, sprint_summary per open run, floating_review per unclaimed report, manual triage count, session-local-day completions. Approved tasks fall back to state_changed_at for the stamp.';
