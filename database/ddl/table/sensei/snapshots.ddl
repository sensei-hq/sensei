set search_path to sensei, extensions;

create table if not exists snapshots (
  id                 uuid        primary key default gen_random_uuid()
, session_id         uuid        not null references sensei.sessions(id) on delete cascade
, repo_id            uuid        not null references sensei.repos(id) on delete cascade
, kind               text        not null
                                 check (kind in ('manual', 'checkpoint'))
, progress_summary   text        not null
, next_step_hint     text
, completed_steps    text[]      not null default '{}'
, in_flight_files    text[]      not null default '{}'
, worktree_refs      jsonb       not null default '[]'
, diff_stat_summary  text
, created_at         timestamptz not null default now()
, modified_at        timestamptz not null default now()
, modified_by        text        not null default current_user
);

create index if not exists snapshots_session_id_idx on snapshots(session_id, created_at desc);
create index if not exists snapshots_repo_id_idx    on snapshots(repo_id, created_at desc);

comment on table snapshots is
'Point-in-time state saved by the agent during a session.
- kind=manual: agent called take_snapshot at a step boundary
- kind=checkpoint: agent called checkpoint, marking a task complete
- worktree_refs: [{branch, path, status}] — active git worktrees at snapshot time
- in_flight_files: paths being modified, so recovery can flag them for consistency check
- Recovery uses the most recent snapshot from a crashed session
- Multiple snapshots per session are retained for history';

comment on column snapshots.id is 'Surrogate primary key (UUID).';
comment on column snapshots.session_id is 'Foreign key to sensei.sessions — links this row to the agent session that created it.';
comment on column snapshots.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column snapshots.kind is 'Snapshot type: manual (take_snapshot call) or checkpoint (task completion boundary).';
comment on column snapshots.progress_summary is 'Free-text summary of work completed up to this snapshot, used for interruption recovery.';
comment on column snapshots.next_step_hint is 'Optional hint for the next action to take when resuming from this snapshot.';
comment on column snapshots.completed_steps is 'Ordered list of step descriptions that have been fully completed before this snapshot.';
comment on column snapshots.in_flight_files is 'Paths of files being actively modified at snapshot time; flagged for consistency check on recovery.';
comment on column snapshots.worktree_refs is 'JSON array of active git worktrees at snapshot time [{branch, path, status}].';
comment on column snapshots.diff_stat_summary is 'Human-readable diff-stat summary (e.g. "3 files changed, 42 insertions") at snapshot time.';
comment on column snapshots.created_at is 'Timestamp when the row was first created.';
comment on column snapshots.modified_at is 'Timestamp of the last modification to this row.';
comment on column snapshots.modified_by is 'Identity (user, role, or service) that last modified this row.';
