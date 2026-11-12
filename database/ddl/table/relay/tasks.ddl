set search_path to relay, extensions;

-- What a lane works on. Repository required (the headline rule: work on a
-- workspace-less repository must still appear); workspace optional. One lane
-- holds one active task; a task never reviews itself; a blocked task remembers
-- where it was (Q5) so the resume lands back there. The queue's columns: the
-- run a task belongs to, who holds its review claim, who executes it, the
-- in-turn checkpoint.
create table if not exists relay.tasks (
  user_id            uuid                not null,
  id                 uuid                not null,
  crew_id            uuid                not null,
  run_id             uuid,
  item_id            uuid,
  repo_key           text                not null,
  workspace_id       uuid,
  lane_id            uuid,
  reviewer_lane_id   uuid,
  review_claimed_by  uuid,
  review_claimed_at  timestamptz,
  title              text                not null,
  phase              relay.task_phase    not null,
  state              relay.task_state    not null default 'assigned',
  round              int                 not null default 1,
  executor           relay.task_executor not null default 'agent',
  deterministic      jsonb               not null default '{}',
  owned_paths        text[]              not null default '{}',
  acceptance         text[]              not null default '{}',
  resources          text[]              not null default '{}',
  deps               uuid[]              not null default '{}',
  blocked_from_state relay.task_state,
  assigned_at        timestamptz         not null default current_timestamp,
  state_changed_at   timestamptz         not null default current_timestamp,
  completed_at       timestamptz,
  checkpoint         jsonb               not null default '{}',
  steps              jsonb               not null default '[]',
  constraint tasks_pkey primary key (user_id, id),
  constraint tasks_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint tasks_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id),
  constraint tasks_run_fk foreign key (user_id, run_id) references relay.runs (user_id, id),
  constraint tasks_item_fk foreign key (user_id, item_id) references relay.items (user_id, id),
  constraint tasks_lane_fk foreign key (user_id, lane_id) references relay.lanes (user_id, id),
  constraint tasks_reviewer_fk foreign key (user_id, reviewer_lane_id) references relay.lanes (user_id, id),
  constraint tasks_review_claim_fk foreign key (user_id, review_claimed_by) references relay.lanes (user_id, id),
  constraint tasks_no_self_review check
    (reviewer_lane_id is null or lane_id is null or reviewer_lane_id <> lane_id),
  constraint tasks_no_self_review_claim check
    (review_claimed_by is null or lane_id is null or review_claimed_by <> lane_id),
  constraint tasks_round_from_1 check (round >= 1),
  constraint tasks_blocked_memory check ((state = 'blocked') = (blocked_from_state is not null)),
  constraint tasks_broker_deterministic check (executor <> 'broker' or deterministic <> '{}'::jsonb)
);

create unique index if not exists tasks_one_active_per_lane
  on relay.tasks (user_id, lane_id) where lane_id is not null
  and state in ('assigned', 'working', 'in_review', 'rework', 'blocked');

create unique index if not exists tasks_one_active_review_per_lane
  on relay.tasks (user_id, review_claimed_by) where review_claimed_by is not null
  and state = 'in_review';

create index if not exists tasks_claim_queue_idx
  on relay.tasks (user_id, crew_id, state, state_changed_at)
  where executor = 'agent' and state in ('ready', 'in_review');

create index if not exists tasks_broker_queue_idx
  on relay.tasks (user_id, crew_id, state, state_changed_at)
  where executor = 'broker' and state = 'ready';

alter table relay.tasks enable row level security;

comment on table relay.tasks is
'Tasks. The active-task unique is (user_id, lane_id) over the five lane-holding states; the review-claim unique is (user_id, review_claimed_by) over in_review.';
comment on column relay.tasks.run_id is 'The sprint run this task belongs to. Null for tasks planned outside a run.';
comment on column relay.tasks.repo_key is 'Required: the repository this work belongs to. No row without one.';
comment on column relay.tasks.reviewer_lane_id is 'Slice 1: the picked reviewer. Retired by the floating review claim (review_claimed_by) at the queue cutover.';
comment on column relay.tasks.review_claimed_by is 'The lane reviewing this task. Null while the review floats on the queue.';
comment on column relay.tasks.review_claimed_at is 'When the reviewer claimed. The sweeper clears a claim whose lane lease lapsed.';
comment on column relay.tasks.executor is 'agent: a lane claims and works it. broker: the daemon runs tasks.deterministic.';
comment on column relay.tasks.deterministic is 'When executor = broker: {action, manifest, target, timeout_seconds, ...}. Empty for agent tasks.';
comment on column relay.tasks.deps is 'Task ids this one waits on. Plain uuids, no FK: graphs, not trees. A report is gated until every dep is done.';
comment on column relay.tasks.blocked_from_state is 'Where the task was when it blocked (Q5). Set on block, cleared on resume.';
comment on column relay.tasks.state_changed_at is 'Every transition stamps it. completed_today falls back to it when completed_at is null.';
comment on column relay.tasks.checkpoint is 'In-turn checkpoint ({current_step, notes, files, ...}) saved by the holder; kept across interrupt and re-offer.';
comment on column relay.tasks.steps is 'Flat checklist (F8): [{key, title, state, reason, updated_at}], insertion order. The holder posts it turn by turn; the task view serves it as-is.';
