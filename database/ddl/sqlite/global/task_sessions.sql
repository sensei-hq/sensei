-- task_sessions
-- One row per agent task (get_session_context → checkpoint boundary).
-- task_type auto-detected from task_description keywords.
-- ftr_score computed at checkpoint time from ftr_signals.

create table if not exists task_sessions (
  id               text not null primary key
, session_id       text references sessions(id) on delete set null
, project_id       text not null references projects(id) on delete cascade
, task_description text
, task_type        text default 'unknown'
                        check (task_type in ('feat','fix','refactor','docs','test','chore','unknown'))
, status           text not null default 'in_progress'
                        check (status in ('in_progress','completed','abandoned'))
, ftr_score        real                  -- 0.0–1.0; null until checkpoint
, ftr_signals      text                  -- JSON: raw signals for auditability
, completed_at     text                  -- ISO 8601
, created_at       text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at      text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by      text not null default 'system'
);

create index if not exists task_sessions_project_idx    on task_sessions(project_id, created_at desc);
create index if not exists task_sessions_session_idx    on task_sessions(session_id)
  where session_id is not null;
create index if not exists task_sessions_status_idx     on task_sessions(status, project_id);
