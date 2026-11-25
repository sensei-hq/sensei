set search_path to relay_archive, extensions;

-- Event rows of ended runs and closed crews (#393): same columns as
-- relay.actions plus archived_at; same primary key and row-local check; no
-- foreign keys. A row moves with its task's run, or with its crew at close.
create table if not exists relay_archive.actions (
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
  created_at timestamptz       not null default current_timestamp,
  archived_at timestamptz      not null default current_timestamp,
  constraint actions_pkey primary key (user_id, id),
  constraint actions_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint actions_summary_short check
    (kind not in ('reported', 'reviewed') or body_md is null or char_length(body_md) <= 600)
);

create index if not exists actions_archived_at_idx
  on relay_archive.actions (archived_at);

alter table relay_archive.actions enable row level security;

comment on table relay_archive.actions is
'Event rows of ended runs and closed crews. Moved here at run completion (task rows) or crew close; purged past 30 days.';
comment on column relay_archive.actions.archived_at is 'When the row moved here; the purge horizon counts from it.';
