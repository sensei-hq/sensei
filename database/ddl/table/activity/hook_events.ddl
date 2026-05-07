set search_path to activity, sensei, extensions;

create table if not exists hook_events (
  id               bigserial         primary key
, session_id       text              not null default ''
, assistant_family text              not null default 'claude'
, event_type       text              not null
, tool_name        text
, cwd              text
, ts               bigint            not null
, success          boolean
, payload          jsonb             not null default '{}'
, created_at       timestamptz       not null default now()
);

create index if not exists hook_events_session_id_idx
    on hook_events(session_id, created_at desc);

create index if not exists hook_events_event_type_idx
    on hook_events(event_type, created_at desc);

create index if not exists hook_events_created_at_idx
    on hook_events(created_at desc);

create index if not exists hook_events_family_idx
    on hook_events(assistant_family, created_at desc);

comment on table hook_events is
'Raw hook event log — one row per hook invocation, from any assistant family.
Captures event types emitted by hooks from Claude Code, Cursor, Zed, etc.
session_id is the assistant string session ID, not a DB UUID.
assistant_family identifies the source assistant (claude, cursor, zed, …).
payload stores the full JSON payload received from stdin.';

comment on column hook_events.id
     is 'Surrogate primary key (bigserial — high write volume).';
comment on column hook_events.session_id
     is 'Assistant session ID string. Not a FK — not a DB UUID.';
comment on column hook_events.assistant_family
     is 'Which assistant emitted this event: claude, cursor, zed, codex, aider, etc. Stored as text for easy import; matches sensei.assistant_family enum values.';
comment on column hook_events.event_type
     is 'hook_event_name from payload: SessionStart, PreToolUse, PostToolUse, Stop, etc.';
comment on column hook_events.tool_name
     is 'tool_name from payload. Populated for PreToolUse and PostToolUse events only.';
comment on column hook_events.cwd
     is 'Working directory at the time of the hook event.';
comment on column hook_events.ts
     is 'Unix epoch milliseconds when the hook fired (client clock).';
comment on column hook_events.success
     is 'For PostToolUse: true if exit_code == 0. Null for all other event types.';
comment on column hook_events.payload
     is 'Full JSON payload received from stdin — complete hook event data.';
comment on column hook_events.created_at
     is 'Server-side timestamp when this row was inserted.';
