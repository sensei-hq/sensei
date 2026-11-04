set search_path to activity, sensei, extensions;

-- How far each session's signals are derived (spec tracker-sdlc-metrics S2): the
-- events and catalogue they reflect, or why derivation failed.
create table if not exists session_signal_marks (
  session_id        uuid         primary key references activity.sessions(id) on delete cascade
, through_event_id  bigint       not null
, event_count       integer      not null
, catalogue_hash    text         not null
, error             text
, derived_at        timestamptz  not null default now()
);

comment on table session_signal_marks is
'Derivation watermark per session (spec tracker-sdlc-metrics S2). A session is derived again when its events'' max(id) or count, or the catalogue hash, differ from the mark.';

comment on column session_signal_marks.session_id
     is 'The session.';
comment on column session_signal_marks.through_event_id
     is 'Highest assistant_events.id read; 0 for a session with no events.';
comment on column session_signal_marks.event_count
     is 'Events read. A lower id committed after a higher one raises the count, not the max.';
comment on column session_signal_marks.catalogue_hash
     is 'The build/test command catalogue the signals were derived with.';
comment on column session_signal_marks.error
     is 'NULL when derived. Otherwise why derivation failed; the session then has no signals and is not retried until its inputs change.';
comment on column session_signal_marks.derived_at
     is 'When the mark was written.';
