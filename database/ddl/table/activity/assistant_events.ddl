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
-- Derived attributes (populated by the EnrichAssistantEvents worker from
-- tool_name + payload->tool_input + cwd — a base-insert + post-update split so the
-- hot capture path is never blocked and the derivation is re-runnable/backfillable).
-- All nullable; `enriched_at` NULL marks a row the worker has not processed yet.
, repository_id    uuid              references sensei.repositories(id) on delete set null
, plugin           text
, method           text
, tool_kind        text
, call_info        text
, enriched_at      timestamptz
);

create index if not exists assistant_events_session_id_idx
    on assistant_events(session_id, created_at desc);

create index if not exists assistant_events_event_type_idx
    on assistant_events(event_type, created_at desc);

create index if not exists assistant_events_created_at_idx
    on assistant_events(created_at desc);

create index if not exists assistant_events_family_idx
    on assistant_events(family, created_at desc);

-- The EnrichAssistantEvents worker scans un-enriched rows (enriched_at IS NULL) in
-- id order — a partial index keeps that scan cheap as the enriched backlog drains.
create index if not exists assistant_events_unenriched_idx
    on assistant_events(id) where enriched_at is null;

-- Repo-grain tool-usage reads (tool_usage_by_repository, once repointed to the stored
-- column) filter by repository + day.
create index if not exists assistant_events_repository_id_idx
    on assistant_events(repository_id, created_at desc) where repository_id is not null;

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
