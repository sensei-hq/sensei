set search_path to sensei, extensions;

-- A ticket's history as the tracker's changelog records it (spec tracker-sdlc-metrics
-- S4): sprint membership, status and story points over time. Insert-only; sprint
-- metrics replay it (S5).
create table if not exists tracker_task_changes (
  task_id     uuid                         not null references sensei.tracker_tasks(id) on delete cascade
, change_id   text                         not null
, field       sensei.tracked_change_field  not null
, at          timestamptz                  not null
, from_value  text
, to_value    text
, primary key (task_id, change_id, field)
);

comment on table tracker_task_changes is
'Tracker changelog entries per ticket for sprint, status and estimate (spec tracker-sdlc-metrics S4). Insert-only; read by the sprint replay.';

comment on column tracker_task_changes.task_id
     is 'The ticket.';
comment on column tracker_task_changes.change_id
     is 'The tracker''s changelog history id.';
comment on column tracker_task_changes.field
     is 'What changed.';
comment on column tracker_task_changes.at
     is 'When it changed (the history''s created time).';
comment on column tracker_task_changes.from_value
     is 'The raw value before: sprint ids comma-separated, a status id, or points as text.';
comment on column tracker_task_changes.to_value
     is 'The raw value after.';
