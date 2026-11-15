set search_path to relay, extensions;

-- A task's actions, newest first (id breaks stamp ties), with the folds'
-- primary structures unpacked: verdict/findings for reviews, files/gates for
-- reports, options for questions, to for messages. Security-invoker: the
-- caller's RLS applies. Run totals are daemon-computed over this view.
create or replace view relay.task_history with (security_invoker = true) as
select a.user_id, a.crew_id, a.task_id, a.id as action_id, l.code as lane_code,
  a.kind, a.round, a.phase, a.body_md, a.reply_to, a.created_at, a.props,
  a.props->>'verdict' as verdict,
  a.props->'findings' as findings,
  a.props->'files' as files,
  a.props->'gates' as gates,
  a.props->'options' as options,
  a.props->'to' as "to"
from relay.actions a
left join relay.lanes l on l.user_id = a.user_id and l.id = a.lane_id
where a.task_id is not null
order by a.created_at desc, a.id desc;

comment on view relay.task_history is
'Task history, newest first as the view orders it. lane_code is null when the person acted.';
