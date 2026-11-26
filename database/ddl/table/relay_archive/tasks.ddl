set search_path to relay_archive, extensions;

-- Tasks of ended runs (#393): same columns as relay.tasks plus archived_at;
-- same primary key and row-local checks; no foreign keys.
create table if not exists relay_archive.tasks (
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
  archived_at        timestamptz         not null default current_timestamp,
  constraint tasks_pkey primary key (user_id, id),
  constraint tasks_id_v7 check (substring(id::text from 15 for 1) = '7'),
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

create index if not exists tasks_archived_at_idx
  on relay_archive.tasks (archived_at);

alter table relay_archive.tasks enable row level security;

comment on table relay_archive.tasks is
'Tasks of ended runs. Moved here at run completion; purged past 30 days.';
comment on column relay_archive.tasks.archived_at is 'When the row moved here; the purge horizon counts from it.';
