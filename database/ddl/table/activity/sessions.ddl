set search_path to activity, extensions;

create type if not exists session_outcome
    as enum ('completed', 'corrected', 'blocked', 'partial', 'abandoned');


create table if not exists sessions (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, project_id               uuid        references sensei.projects(id) on delete set null
, task                     text        not null default ''
, acp_id                   text
, outcome                  session_outcome
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
- outcome: completed (no corrections), corrected, blocked, partial (crash), abandoned
- ftr: true if corrections == 0 (First-Try Rate)
- module: primary code module touched (for per-module FTR tracking)
- props: {patterns_matched, personas_applied, workflow_phase, ...}';

comment on column sessions.id
     is 'Surrogate primary key (UUID).';
comment on column sessions.folder_id
     is 'Foreign key to folders — which folder this session ran in.';
comment on column sessions.project_id
     is 'Foreign key to projects — derived from folder.';
comment on column sessions.task
     is 'Task description passed at get_session_context.';
comment on column sessions.acp_id
     is 'Agent client: claude-code, cursor, codex, aider, etc.';
comment on column sessions.outcome
     is 'Session outcome: completed, corrected, blocked, partial, abandoned.';
comment on column sessions.ftr
     is 'First-Try Rate: true if zero corrections.';
comment on column sessions.turns
     is 'Number of user turns.';
comment on column sessions.corrections
     is 'Number of user corrections (FTR detractor).';
comment on column sessions.tokens_in
     is 'Total input tokens consumed.';
comment on column sessions.tokens_out
     is 'Total output tokens generated.';
comment on column sessions.duration_ms
     is 'Session duration in milliseconds.';
comment on column sessions.module
     is 'Primary code module touched. For per-module FTR.';
comment on column sessions.props
     is 'Extensible: {patterns_matched, personas_applied, workflow_phase, ...}.';
comment on column sessions.started_at
     is 'When this session started.';
comment on column sessions.completed_at
     is 'When this session ended.';
