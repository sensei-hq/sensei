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
