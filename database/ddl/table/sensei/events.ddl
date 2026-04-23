set search_path to sensei, extensions;

create table if not exists events (
  id                       text primary key default gen_random_uuid()::text
, project                  text not null
, session_id               text references sensei.sessions(id)
, event_type               text not null
, data                     jsonb not null default '{}'
, created_at               timestamptz not null default now()
);

create index if not exists idx_events_project
    on events(project, created_at);

create index if not exists idx_events_type
    on events(project, event_type);

create index if not exists idx_events_session
    on events(session_id);

comment on table events is
'Session-level activity captured by hooks and MCP tools.
Types: turn, revision_requested, tool_used, phase_transition, mindset_applied, etc.
Source for FTR, pattern detection, and coaching metrics.';

comment on column events.id is 'Primary key — event_id from .sensei/sensei.json or generated UUID.';
comment on column events.project is 'Project identifier (matches repos.project_id or sensei.project).';
comment on column events.session_id is 'Foreign key to sensei.sessions — null for global events.';
comment on column events.event_type is 'Event type: turn, revision_requested, tool_used, phase_transition, mindset_applied, etc.';
comment on column events.data is 'Event payload stored as JSON.';
comment on column events.created_at is 'Timestamp when this event was recorded.';
