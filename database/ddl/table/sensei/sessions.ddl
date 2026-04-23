set search_path to sensei, extensions;

create table if not exists sessions (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, project_id               uuid        references sensei.projects(id) on delete set null
, task                     text        not null default ''
, acp_id                   text
, outcome                  text
                                       check (outcome in ('completed', 'corrected', 'blocked', 'partial', 'abandoned'))
, ftr                      boolean
, turns                    integer     not null default 0
, corrections              integer     not null default 0
, tokens_in                integer
, tokens_out               integer
, duration_ms              integer
, module                   text
, props                    jsonb       not null default '{}'
, started_at               timestamptz not null default now()
, completed_at             timestamptz
);

create index if not exists sessions_folder_id_idx
    on sessions(folder_id, started_at desc);

create index if not exists sessions_project_id_idx
    on sessions(project_id, started_at desc)
 where project_id is not null;

create index if not exists sessions_ftr_idx
    on sessions(ftr)
 where ftr is not null;

comment on table sessions is
'AI coding sessions captured by hooks.
- outcome: completed (no corrections), corrected (corrections made), blocked, partial (crash), abandoned
- ftr: true if corrections == 0 (First-Try Rate)
- module: primary code module touched (for per-module FTR tracking)
- props: extensible — {patterns_matched, personas_applied, ...}';

comment on column sessions.id
     is 'Surrogate primary key (UUID).';
comment on column sessions.folder_id
     is 'Foreign key to folders — which folder this session ran in.';
comment on column sessions.project_id
     is 'Foreign key to projects — which project this session belongs to. Derived from folder.';
comment on column sessions.task
     is 'Task description passed by the agent at get_session_context.';
comment on column sessions.acp_id
     is 'Agent client identifier: claude-code, cursor, codex, aider, etc.';
comment on column sessions.outcome
     is 'Session outcome: completed, corrected, blocked, partial (crash), abandoned.';
comment on column sessions.ftr
     is 'First-Try Rate: true if session completed with zero corrections.';
comment on column sessions.turns
     is 'Number of user turns in this session.';
comment on column sessions.corrections
     is 'Number of times the user corrected the assistant (FTR detractor).';
comment on column sessions.tokens_in
     is 'Total input tokens consumed during the session.';
comment on column sessions.tokens_out
     is 'Total output tokens generated during the session.';
comment on column sessions.duration_ms
     is 'Session duration in milliseconds.';
comment on column sessions.module
     is 'Primary code module touched during the session (e.g. "auth/refresh.ts"). For per-module FTR.';
comment on column sessions.props
     is 'Extensible metadata: {patterns_matched, personas_applied, workflow_phase, ...}.';
comment on column sessions.started_at
     is 'Timestamp when this session started.';
comment on column sessions.completed_at
     is 'Timestamp when this session ended.';
