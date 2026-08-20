set search_path to activity, sensei, extensions;
create table if not exists sessions (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        references sensei.folders(id) on delete set null   -- raw cwd provenance (nullable): a folder prune must NEVER delete a session (spec 2026-08-18 I2)
, repo_folder_id           uuid        references sensei.folders(id) on delete set null   -- the DURABLE repo anchor: nearest repo-kind ancestor via sensei.repo_anchor_for
, repo_key                 text                                                           -- repo identity (primary git remote, else abs_path) — survives a full folder-row rebuild
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
, meta_synced_at           timestamptz -- when the transcript backfill last attempted model+token capture for this session; gates the one-time metadata backfill so a token-less source isn't re-read every cycle
, process_analyzed_at      timestamptz -- when the LLM process-quality analyzer last scored this session (spec 2026-08-20); gates the daily incremental pass so a session is judged at most once until its transcript changes
);

create index if not exists sessions_folder_id_idx
    on sessions(folder_id, started_at desc);

create index if not exists sessions_project_id_idx
    on sessions(project_id, started_at desc)
 where project_id is not null;

create index if not exists sessions_repo_folder_id_idx
    on sessions(repo_folder_id, started_at desc)
 where repo_folder_id is not null;

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
     is 'Raw cwd provenance — the exact folder the session ran in. Nullable + ON DELETE
SET NULL so pruning that folder can never delete the session (spec 2026-08-18, I2).
Attribution keys off repo_folder_id, not this.';
comment on column sessions.repo_folder_id
     is 'The DURABLE repo anchor: the nearest repo-kind ancestor (git/subtree/standalone
project-root) resolved by sensei.repo_anchor_for. Recency, FTR, metrics, and the
project rollup key off this so a session belongs to a repo, never a transient folder.';
comment on column sessions.repo_key
     is 'Stable repo identity — the primary git remote URL (else the repo abs_path). Lets
attribution survive a full folder-row rebuild (spec 2026-08-18, D2).';
comment on column sessions.project_id
     is 'Foreign key to projects — derived from the repo anchor.';
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
comment on column sessions.meta_synced_at
     is 'When the transcript backfill last attempted to capture this session''s
inference model + token usage from the source transcript. Set once the attempt
runs (whether or not the source carried the values), so a session whose source has
no tokens (e.g. a Zed thread) is not re-read on every backfill cycle. NULL ⇒ never
attempted ⇒ eligible for the one-time metadata backfill.';
comment on column sessions.process_analyzed_at
     is 'When the LLM process-quality analyzer last scored this session (spec
2026-08-20). Gates the daily incremental pass: NULL ⇒ never scored ⇒ eligible;
set to now() after a successful pass. Cleared when a transcript re-ingest bumps
the session so its judgments are recomputed. props.process holds the judgments;
activity.session_process_evidence holds the cited turns.';
