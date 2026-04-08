-- snapshots
-- Point-in-time agent state for interruption recovery.
-- kind=manual: agent called take_snapshot at a step boundary
-- kind=checkpoint: agent called checkpoint (task complete)
-- Recovery: most recent snapshot from a crashed session is surfaced on next get_session_context.

create table if not exists snapshots (
  id                text not null primary key
, session_id        text not null references sessions(id) on delete cascade
, project_id        text not null references projects(id) on delete cascade
, kind              text not null check (kind in ('manual','checkpoint'))
, progress_summary  text not null
, next_step_hint    text
, completed_steps   text not null default '[]'  -- JSON: ["step1","step2"]
, in_flight_files   text not null default '[]'  -- JSON: ["src/auth.ts"]
, worktree_refs     text not null default '[]'  -- JSON: [{branch,path,status}]
, diff_stat_summary text
, created_at        text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at       text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by       text not null default 'system'
);

create index if not exists snapshots_session_idx  on snapshots(session_id, created_at desc);
create index if not exists snapshots_project_idx  on snapshots(project_id, created_at desc);
