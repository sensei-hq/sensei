set search_path to relay, extensions;

-- What a lane works on. Repository required (the headline rule: work on a
-- workspace-less repository must still appear); workspace optional. One lane
-- holds one active task; a task never reviews itself; a blocked task remembers
-- where it was (Q5) so the resume lands back there.
create table if not exists relay.tasks (
  user_id            uuid              not null,
  id                 uuid              not null,
  crew_id            uuid              not null,
  item_id            uuid,
  repo_key           text              not null,
  workspace_id       uuid,
  lane_id            uuid,
  reviewer_lane_id   uuid,
  title              text              not null,
  phase              relay.task_phase  not null,
  state              relay.task_state  not null default 'assigned',
  round              int               not null default 1,
  owned_paths        text[]            not null default '{}',
  acceptance         text[]            not null default '{}',
  resources          text[]            not null default '{}',
  deps               uuid[]            not null default '{}',
  blocked_from_state relay.task_state,
  assigned_at        timestamptz       not null default now(),
  state_changed_at   timestamptz       not null default now(),
  completed_at       timestamptz,
  constraint tasks_pkey primary key (user_id, id),
  constraint tasks_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint tasks_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id),
  constraint tasks_item_fk foreign key (user_id, item_id) references relay.items (user_id, id),
  constraint tasks_lane_fk foreign key (user_id, lane_id) references relay.lanes (user_id, id),
  constraint tasks_reviewer_fk foreign key (user_id, reviewer_lane_id) references relay.lanes (user_id, id),
  constraint tasks_no_self_review check
    (reviewer_lane_id is null or lane_id is null or reviewer_lane_id <> lane_id),
  constraint tasks_round_from_1 check (round >= 1),
  constraint tasks_blocked_memory check ((state = 'blocked') = (blocked_from_state is not null))
);

create unique index if not exists tasks_one_active_per_lane
  on relay.tasks (user_id, lane_id) where lane_id is not null
  and state in ('assigned', 'working', 'in_review', 'rework', 'blocked');

alter table relay.tasks enable row level security;

comment on table relay.tasks is
'Tasks. The active-task unique is (user_id, lane_id) over the five lane-holding states.';
comment on column relay.tasks.repo_key is 'Required: the repository this work belongs to. No row without one.';
comment on column relay.tasks.deps is 'Task ids this one waits on. Plain uuids, no FK: graphs, not trees.';
comment on column relay.tasks.blocked_from_state is 'Where the task was when it blocked (Q5). Set on block, cleared on resume.';
comment on column relay.tasks.state_changed_at is 'Every transition stamps it. completed_today falls back to it when completed_at is null.';
