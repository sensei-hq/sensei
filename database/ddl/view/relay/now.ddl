set search_path to relay, extensions;

-- The Now screen's grain, one view in four sections: one row per lane with
-- its active task and newest lease; open person decisions; the manual triage
-- count; today's completed tasks. Security-invoker: the caller's RLS applies.
-- The daemon selects sections and shapes RelayNow; each section carries the
-- full grain (no per-row re-join). Tracker-side triage is composed by the
-- daemon per side — this view counts manual triage items only.
create or replace view relay.now with (security_invoker = true) as
select l.user_id, 'lane'::text as section, l.crew_id, l.id,
  jsonb_build_object(
    'code', l.code, 'title', l.title, 'role', l.role, 'family', l.family,
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
                where an.user_id = l.user_id and an.kind = 'answered' and an.reply_to = a.id))) end
  ) as payload
from relay.lanes l
left join lateral (
  select * from relay.leases ll
   where ll.user_id = l.user_id and ll.crew_id = l.crew_id and ll.lane_id = l.id and ll.kind = 'lane'
   order by ll.id desc limit 1
) ls on true
left join relay.tasks t on t.user_id = l.user_id and t.lane_id = l.id
  and t.state in ('assigned', 'working', 'in_review', 'rework', 'blocked')
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
'Now grain: lane rows, open person decisions, manual triage count, session-local-day completions. Approved tasks fall back to state_changed_at for the stamp.';
