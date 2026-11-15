set search_path to relay, extensions;

-- What a lane works on. Repository required (the headline rule: work on a
-- workspace-less repository must still appear); workspace optional. A task
-- is minted ready (the pool: no lane); a claim or an assign puts it on a
-- lane; one lane works one task at a time; a report floats the task on the
-- review queue, which frees the lane; a lane never reviews its own task; a
-- blocked task remembers where it was (Q5) so the resume lands back there.
create table if not exists relay.tasks (
  user_id            uuid                not null,
  id                 uuid                not null,
  crew_id            uuid                not null,
  run_id             uuid,
  item_id            uuid,
  repo_key           text                not null,
  workspace_id       uuid,
  lane_id            uuid,
  review_claimed_by  uuid,
  review_claimed_at  timestamptz,
  title              text                not null,
  phase              relay.task_phase    not null,
  state              relay.task_state    not null default 'ready',
  round              int                 not null default 1,
  executor           relay.task_executor not null default 'agent',
  deterministic      jsonb               not null default '{}',
  owned_paths        text[]              not null default '{}',
  acceptance         text[]              not null default '{}',
  resources          text[]              not null default '{}',
  deps               uuid[]              not null default '{}',
  blocked_from_state relay.task_state,
  assigned_at        timestamptz,
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
  constraint tasks_review_claim_fk foreign key (user_id, review_claimed_by) references relay.lanes (user_id, id),
  constraint tasks_no_self_review_claim check
    (review_claimed_by is null or lane_id is null or review_claimed_by <> lane_id),
  constraint tasks_round_from_1 check (round >= 1),
  constraint tasks_blocked_memory check ((state = 'blocked') = (blocked_from_state is not null)),
  constraint tasks_broker_deterministic check (executor <> 'broker' or deterministic <> '{}'::jsonb),
  constraint tasks_lane_coherence check (
    case
      when state = 'ready' then lane_id is null
      when lane_id is null then state = 'cancelled' or executor = 'broker'
      else true
    end)
);

create unique index if not exists tasks_one_active_worker_per_lane
  on relay.tasks (user_id, lane_id) where lane_id is not null
  and state in ('assigned', 'working', 'rework', 'blocked');

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
'Tasks. The worker unique is (user_id, lane_id) over assigned/working/rework/blocked (in_review floats); the review-claim unique is (user_id, review_claimed_by) over in_review. A ready task has no lane; a laneless task is ready, cancelled, or broker-run.';
comment on column relay.tasks.run_id is 'The sprint run this task belongs to. Null for tasks planned outside a run.';
comment on column relay.tasks.repo_key is 'Required: the repository this work belongs to. No row without one.';
comment on column relay.tasks.lane_id is 'The lane working the task. Null in the pool (ready), after a cancel from the pool, and for broker-run tasks.';
comment on column relay.tasks.review_claimed_by is 'The lane reviewing this task. Null while the review floats on the queue; never the author.';
comment on column relay.tasks.review_claimed_at is 'When the reviewer claimed. The sweeper clears a claim whose lane lease lapsed.';
comment on column relay.tasks.executor is 'agent: a lane claims and works it. broker: the daemon runs tasks.deterministic.';
comment on column relay.tasks.deterministic is 'When executor = broker: {action, manifest, target, timeout_seconds, ...}. Empty for agent tasks.';
comment on column relay.tasks.deps is 'Task ids this one waits on. Plain uuids, no FK: graphs, not trees. A report is gated until every dep is done.';
comment on column relay.tasks.blocked_from_state is 'Where the task was when it blocked (Q5). Set on block, cleared on resume.';
comment on column relay.tasks.assigned_at is 'When a lane took the task (claim or assign). Null in the pool.';
comment on column relay.tasks.state_changed_at is 'Every transition stamps it. completed_today falls back to it when completed_at is null.';
comment on column relay.tasks.checkpoint is 'In-turn checkpoint ({current_step, notes, files, ...}) saved by the holder; kept across interrupt and re-offer.';
comment on column relay.tasks.steps is 'Flat checklist (F8): [{key, title, state, reason, updated_at}], insertion order. The holder posts it turn by turn; the task view serves it as-is.';
