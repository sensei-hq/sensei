set search_path to activity, extensions;

create type if not exists event_type
    as enum (
        'tool_call'
      , 'api_request'
      , 'correction'
      , 'turn'
      , 'phase_transition'
      , 'checkpoint'
      , 'task_start'
      , 'task_end'
      , 'context_loaded'
      , 'edit'
      , 'test'
      , 'error'
    );

create table if not exists events (
  id                       uuid        primary key default gen_random_uuid()
, session_id               uuid        not null references activity.sessions(id) on delete cascade
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, event_type               event_type  not null
, turn_number              integer
, data                     jsonb       not null default '{}'
, created_at               timestamptz not null default now()
);

create index if not exists events_session_id_idx
    on events(session_id, created_at);

create index if not exists events_folder_id_idx
    on events(folder_id, created_at desc);

create index if not exists events_type_idx
    on events(event_type);

comment on table events is
'Unified activity log for everything that happens during sessions.
Consolidates the former api_requests, task_turns, tool_calls, and events tables.

Event types and their data payloads:
  tool_call:        {tool_name, service_name, input_params, response, duration_ms, used_in_response}
  api_request:      {prompt_id, model, input_tokens, output_tokens, cache_read_tokens, cost_usd, duration_ms}
  correction:       {description, module}
  turn:             {role:"user"|"assistant", token_count}
  phase_transition: {from_phase, to_phase}
  checkpoint:       {summary, next_step}
  task_start:       {task_description, task_type}
  task_end:         {status, ftr_score, ftr_signals}
  context_loaded:   {files_loaded, token_count}
  edit:             {file_path, lines_changed}
  test:             {passed, failed, total}
  error:            {message, adapter, phase}';

comment on column events.id
     is 'Surrogate primary key (UUID).';
comment on column events.session_id
     is 'Foreign key to sessions — which session this event occurred in.';
comment on column events.folder_id
     is 'Foreign key to folders — which folder context. Denormalized for fast queries.';
comment on column events.event_type
     is 'Event category. Determines the shape of the data payload.';
comment on column events.turn_number
     is 'Which turn in the session this event occurred at. Null for non-turn events.';
comment on column events.data
     is 'Event payload as JSON. Shape determined by event_type — see table comment.';
comment on column events.created_at
     is 'Timestamp when this event occurred.';
