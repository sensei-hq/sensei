set search_path to activity, sensei, extensions;

create table if not exists assistant_events (
  id               bigserial         primary key
, session_id       text              not null default ''
, family           assistant_family  not null default 'claude'
, event_type       text              not null
, tool_name        text
, cwd              text
, ts               bigint            not null
, success          boolean
, payload          jsonb             not null default '{}'
, created_at       timestamptz       not null default now()
);

create index if not exists assistant_events_session_id_idx
    on assistant_events(session_id, created_at desc);

create index if not exists assistant_events_event_type_idx
    on assistant_events(event_type, created_at desc);

create index if not exists assistant_events_created_at_idx
    on assistant_events(created_at desc);

create index if not exists assistant_events_family_idx
    on assistant_events(family, created_at desc);

comment on table assistant_events is
'Raw assistant event log — one row per hook/event invocation, from any
assistant family. Captures events emitted by Claude Code, Cursor, Zed, etc.
session_id is the assistant string session ID, not a DB UUID.
family identifies the source assistant (claude, cursor, zed, …).
payload stores the full JSON payload received from stdin.';

comment on column assistant_events.id
     is 'Surrogate primary key (bigserial — high write volume).';
comment on column assistant_events.session_id
     is 'Assistant session ID string. Not a FK — not a DB UUID.';
comment on column assistant_events.family
     is 'Which assistant emitted this event: claude, cursor, zed, codex, aider, etc.';
comment on column assistant_events.event_type
     is 'hook_event_name from payload: SessionStart, PreToolUse, PostToolUse, Stop, etc.';
comment on column assistant_events.tool_name
     is 'tool_name from payload. Populated for PreToolUse and PostToolUse events only.';
comment on column assistant_events.cwd
     is 'Working directory at the time of the event.';
comment on column assistant_events.ts
     is 'Unix epoch milliseconds when the event fired (client clock).';
comment on column assistant_events.success
     is 'For PostToolUse: true if exit_code == 0. Null for all other event types.';
comment on column assistant_events.payload
     is 'Full JSON payload received from stdin — complete event data.';
comment on column assistant_events.created_at
     is 'Server-side timestamp when this row was inserted.';
