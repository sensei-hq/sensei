set search_path to sensei, extensions;

create table if not exists snapshots (
  id                       uuid        primary key default gen_random_uuid()
, session_id               uuid        not null references sensei.sessions(id) on delete cascade
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
, modified_at              timestamptz not null default now()
);

create index if not exists snapshots_session_id_idx
    on snapshots(session_id, created_at desc);

create index if not exists snapshots_folder_id_idx
    on snapshots(folder_id, created_at desc);

comment on table snapshots is
'Point-in-time state saved by the agent during a session.
- kind=manual: take_snapshot at a step boundary
- kind=checkpoint: task complete marker
- in_flight_files: paths being modified, flagged for consistency check on recovery
- worktree_refs: [{branch, path, status}] active git worktrees at snapshot time';

comment on column snapshots.id
     is 'Surrogate primary key (UUID).';
comment on column snapshots.session_id
     is 'Foreign key to sessions — the session that created this snapshot.';
comment on column snapshots.folder_id
     is 'Foreign key to folders — which folder this snapshot covers.';
comment on column snapshots.kind
     is 'Snapshot type: manual (take_snapshot) or checkpoint (task completion).';
comment on column snapshots.progress_summary
     is 'Free-text summary of work completed, used for interruption recovery.';
comment on column snapshots.next_step_hint
     is 'Optional hint for the next action when resuming from this snapshot.';
comment on column snapshots.completed_steps
     is 'Ordered list of completed step descriptions.';
comment on column snapshots.in_flight_files
     is 'Paths of files being actively modified at snapshot time.';
comment on column snapshots.worktree_refs
     is 'JSON array of active git worktrees [{branch, path, status}].';
comment on column snapshots.diff_stat_summary
     is 'Human-readable diff-stat summary at snapshot time.';
comment on column snapshots.created_at
     is 'Timestamp when the row was first created.';
comment on column snapshots.modified_at
     is 'Timestamp of the last modification to this row.';
