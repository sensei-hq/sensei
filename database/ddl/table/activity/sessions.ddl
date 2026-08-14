set search_path to activity, sensei, extensions;
create table if not exists sessions (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, project_id               uuid        references sensei.projects(id) on delete set null
, task                     text        not null default ''
, acp_id                   text
, client_session_id        text
, outcome                  session_outcome
, ftr                      boolean
, turns                    integer     not null default 0
, corrections              integer     not null default 0
, tokens_in                integer
, tokens_out               integer
, duration                 interval
, module                   text
, summary                  text
, provider                 text
, model                    text
, props                    jsonb       not null default '{}'
, evidence                 jsonb       -- Phase C: deterministic transcript-sourced evidence (moments) grounding the drill-down; NULL until enriched
, started_at               timestamptz not null default now()
, completed_at             timestamptz
, analyzed_at              timestamptz
, backfilled               boolean     not null default false  -- synthesized from a historical transcript (#75), not live-captured
);

create index if not exists sessions_folder_id_idx
    on sessions(folder_id, started_at desc);

create index if not exists sessions_project_id_idx
    on sessions(project_id, started_at desc)
 where project_id is not null;

create index if not exists sessions_ftr_idx
    on sessions(ftr)
 where ftr is not null;

-- One session row per assistant session id (hook-derived sessions upsert by it;
-- MCP-created sessions leave it NULL and are excluded from the unique index).
create unique index if not exists sessions_client_session_id_uniq
    on sessions(client_session_id)
 where client_session_id is not null;

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
     is 'Assistant family name (matches sensei.assistants.family): claude, cursor, codex, aider, etc.';
comment on column sessions.client_session_id
     is 'The assistant''s own session id (e.g. Claude session_id from hooks). Correlates all hook events of one session to a single row; NULL for MCP-created sessions.';
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
comment on column sessions.duration
     is 'Gap-aware active work time (excludes idle gaps > threshold). The full
wall-clock span is started_at..completed_at; per-turn/segment detail lives in
activity.turns.';
comment on column sessions.module
     is 'Primary code module touched. For per-module FTR.';
comment on column sessions.provider
     is 'Inference provider that ran this session (e.g. anthropic, openai, copilot_chat, ollama). Captured from the transcript at synthesis (#75); NULL for live hook-captured sessions whose model the hook stream doesn''t carry. Powers effectiveness-by-model.';
comment on column sessions.model
     is 'Specific model that ran this session (e.g. claude-opus-4, GPT-5, Grok Code Fast 1). See provider. Powers effectiveness-by-model insights.';
comment on column sessions.summary
     is 'Brief summary of what happened in this session. Populated at checkpoint time by the assistant.';
comment on column sessions.props
     is 'Extensible: {patterns_matched, personas_applied, workflow_phase, ...}.';
comment on column sessions.started_at
     is 'When this session started.';
comment on column sessions.completed_at
     is 'When this session ended.';
comment on column sessions.analyzed_at
     is 'When the analyzer last enriched this session (#67). The scheduler skips a
session whose assistant_events are no newer than this — incremental re-analysis.';
