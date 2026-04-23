set search_path to inference, extensions;

create table if not exists tool_calls (
  id                       uuid        primary key default gen_random_uuid()
, session_id               uuid        not null references sensei.sessions(id) on delete cascade
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, tool_name                text        not null
, service_name             text        not null default 'sensei'
, input_params             jsonb       not null default '{}'
, response                 jsonb
, duration_ms              integer
, used_in_response         boolean
, turn_number              integer
, created_at               timestamptz not null default now()
);

create index if not exists tool_calls_session_id_idx
    on tool_calls(session_id, created_at);

create index if not exists tool_calls_folder_id_idx
    on tool_calls(folder_id);

create index if not exists tool_calls_tool_name_idx
    on tool_calls(tool_name);

comment on table tool_calls is
'Every MCP tool call made during sessions. Used for replay and usage analytics.
- service_name: which MCP server (sensei, postgres-mcp, stripe-mcp, etc.)
- used_in_response: did the assistant use this result in its next action? Null if unknown.
- turn_number: which turn in the session this call occurred at';

comment on column tool_calls.id
     is 'Surrogate primary key (UUID).';
comment on column tool_calls.session_id
     is 'Foreign key to sessions — which session this call was made in.';
comment on column tool_calls.folder_id
     is 'Foreign key to folders — which folder context this call was made in.';
comment on column tool_calls.tool_name
     is 'MCP tool name (e.g. "search", "get_callers", "get_patterns").';
comment on column tool_calls.service_name
     is 'Which MCP server: sensei, postgres-mcp, stripe-mcp, etc.';
comment on column tool_calls.input_params
     is 'What the assistant sent as input parameters.';
comment on column tool_calls.response
     is 'What the tool returned. Nullable if response was not captured.';
comment on column tool_calls.duration_ms
     is 'Wall-clock duration of the tool call in milliseconds.';
comment on column tool_calls.used_in_response
     is 'Whether the assistant used this result. Null if unknown.';
comment on column tool_calls.turn_number
     is 'Which turn in the session this call occurred at.';
comment on column tool_calls.created_at
     is 'Timestamp when this tool call was made.';
