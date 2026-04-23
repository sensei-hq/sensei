set search_path to activity, extensions;

create table if not exists snapshots (
  id                       uuid        primary key default gen_random_uuid()
, session_id               uuid        not null references activity.sessions(id) on delete cascade
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, kind                     text        not null
                                       check (kind in ('manual', 'checkpoint'))
, progress_summary         text        not null
, next_step_hint           text
, completed_steps          text[]      not null default '{}'
, in_flight_files          text[]      not null default '{}'
, worktree_refs            jsonb       not null default '[]'
, diff_stat_summary        text
, created_at               timestamptz not null default now()
);

create index if not exists snapshots_session_id_idx
    on snapshots(session_id, created_at desc);

create index if not exists snapshots_folder_id_idx
    on snapshots(folder_id, created_at desc);

comment on table snapshots is
'Point-in-time session state for crash recovery and history.
- kind=manual: take_snapshot at a step boundary
- kind=checkpoint: task complete marker
- in_flight_files: flagged for consistency check on recovery
- worktree_refs: [{branch, path, status}] active git worktrees';

comment on column snapshots.id
     is 'Surrogate primary key (UUID).';
comment on column snapshots.session_id
     is 'Foreign key to sessions.';
comment on column snapshots.folder_id
     is 'Foreign key to folders.';
comment on column snapshots.kind
     is 'Snapshot type: manual or checkpoint.';
comment on column snapshots.progress_summary
     is 'Summary of work completed, used for recovery.';
comment on column snapshots.next_step_hint
     is 'Optional hint for next action when resuming.';
comment on column snapshots.completed_steps
     is 'Ordered list of completed step descriptions.';
comment on column snapshots.in_flight_files
     is 'Paths of files being actively modified at snapshot time.';
comment on column snapshots.worktree_refs
     is 'JSON array of active git worktrees [{branch, path, status}].';
comment on column snapshots.diff_stat_summary
     is 'Human-readable diff-stat summary at snapshot time.';
comment on column snapshots.created_at
     is 'Timestamp when snapshot was taken.';
