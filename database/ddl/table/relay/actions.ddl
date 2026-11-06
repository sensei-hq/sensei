set search_path to relay, extensions;

-- Everything that happens: one table, categorised by kind. Task actions carry
-- the task; crew-level messages leave it null. The lane is null when the
-- person acted. Summaries (reported/reviewed) stay short; the full report
-- text rides report_md, which syncs to the dōjō only when sharing is on.
create table if not exists relay.actions (
  user_id    uuid              not null,
  id         uuid              not null,
  crew_id    uuid              not null,
  task_id    uuid,
  lane_id    uuid,
  kind       relay.action_kind not null,
  round      int,
  phase      relay.task_phase,
  body_md    text,
  report_md  text,
  props      jsonb             not null default '{}',
  reply_to   uuid,
  created_at timestamptz       not null default now(),
  constraint actions_pkey primary key (user_id, id),
  constraint actions_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint actions_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id),
  constraint actions_task_fk foreign key (user_id, task_id) references relay.tasks (user_id, id),
  constraint actions_lane_fk foreign key (user_id, lane_id) references relay.lanes (user_id, id),
  constraint actions_reply_fk foreign key (user_id, reply_to) references relay.actions (user_id, id),
  constraint actions_summary_short check
    (kind not in ('reported', 'reviewed') or body_md is null or char_length(body_md) <= 600)
);

alter table relay.actions enable row level security;

comment on table relay.actions is
'The event log: briefs, turns, reviews, decisions, messages, receipts, merges. props keys are fixed per kind, daemon-validated.';
comment on column relay.actions.body_md is 'The shared text. Capped at 600 chars for reported/reviewed summaries.';
comment on column relay.actions.report_md is 'The full report. Dōjō-bound only when sharing is on.';
comment on column relay.actions.reply_to is 'Answers and receipts point at what they answer. Null otherwise.';
