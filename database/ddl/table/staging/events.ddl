set search_path to staging;

create table if not exists events (
  id           uuid
, user_uuid    text
, session_id   text
, repo_id      uuid
, phase        text
, tool         text
, project_path text
, input        jsonb
, ts           timestamptz
, created_at   timestamptz default now()
, modified_at  timestamptz default now()
, modified_by  text
);

create unique index if not exists events_id_ukey on events(id);

comment on table events is 'Intermediate import buffer for bulk-loading rows into sensei.events.';

comment on column events.id           is 'UUID that will become the surrogate primary key in sensei.events.';
comment on column events.user_uuid    is 'Identifier of the user who generated the event.';
comment on column events.session_id   is 'Logical session grouping related events together.';
comment on column events.repo_id      is 'UUID of the repo associated with this event; resolved against sensei.repos on import.';
comment on column events.phase        is 'Lifecycle phase of the tool invocation: pre (before) or post (after).';
comment on column events.tool         is 'Name of the tool or action that generated this event.';
comment on column events.project_path is 'Filesystem path of the project active when the event was recorded.';
comment on column events.input        is 'JSON payload of inputs captured at the time of the event.';
comment on column events.ts           is 'Source-side timestamp when the event occurred.';
comment on column events.created_at   is 'Timestamp when the staging row was inserted.';
comment on column events.modified_at  is 'Source-side modification timestamp; used as freshness gate during import.';
comment on column events.modified_by  is 'Source-side modifier identity; passed through to sensei.events on upsert.';
