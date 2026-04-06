set search_path to sensei, extensions;

create table if not exists events (
  id           uuid primary key default gen_random_uuid()
, user_uuid    text not null
, session_id   text
, repo_id      uuid references repos(id) on delete set null
, phase        text not null check (phase in ('pre', 'post'))
, tool         text not null
, project_path text
, input        jsonb
, seq          int4
, duration_ms  int4
, success      boolean
, error        text
, ts           timestamptz not null
, created_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
);

create index if not exists idx_events_user_uuid on events(user_uuid);
create index if not exists idx_events_ts on events(ts desc);
create index if not exists idx_events_tool on events(tool);

comment on column events.id is 'Surrogate primary key (UUID).';
comment on column events.user_uuid is 'Identifier of the user who triggered this event (not a FK; may be an external auth ID).';
comment on column events.session_id is 'Opaque session identifier supplied by the caller; not a FK to sensei.sessions.';
comment on column events.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column events.phase is 'Hook phase when the event was recorded: pre (before tool execution) or post (after).';
comment on column events.tool is 'Name of the MCP tool that generated this event.';
comment on column events.project_path is 'Filesystem path of the project directory at the time of the event.';
comment on column events.input is 'JSON-serialized input arguments passed to the tool for this event.';
comment on column events.seq is 'Monotonically increasing sequence number within the session for ordering events.';
comment on column events.duration_ms is 'Wall-clock duration of the tool call in milliseconds (null for pre-phase events).';
comment on column events.success is 'Whether the tool call succeeded; null for pre-phase events, false if the tool threw.';
comment on column events.error is 'Error message if the tool call failed; null on success.';
comment on column events.ts is 'Business timestamp of when the event logically occurred (set by the caller).';
comment on column events.created_at is 'Timestamp when the row was first created.';
comment on column events.modified_at is 'Timestamp of the last modification to this row.';
comment on column events.modified_by is 'Identity (user, role, or service) that last modified this row.';
