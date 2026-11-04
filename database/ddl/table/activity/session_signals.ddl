set search_path to activity, sensei, extensions;

-- A session's SDLC timeline (spec tracker-sdlc-metrics S2), derived once from its
-- hook events and replaced whole when they change. Ticket metrics are computed
-- from these at read.
create table if not exists session_signals (
  session_id  uuid                        not null references activity.sessions(id) on delete cascade
, seq         integer                     not null
, kind        sensei.session_signal_kind  not null
, at          timestamptz                 not null
, until       timestamptz
, category    text                        check (category in ('build', 'typecheck', 'test', 'e2e'))
, reason      sensei.hold_reason
, ref         text
, primary key (session_id, seq)
);

comment on table session_signals is
'Derived SDLC signals per session (spec tracker-sdlc-metrics S2). Replaced whole per session by the derive_session_signals pass.';

comment on column session_signals.session_id
     is 'The session.';
comment on column session_signals.seq
     is 'Order within the session, by event time then event id.';
comment on column session_signals.kind
     is 'What happened (sensei.session_signal_kind).';
comment on column session_signals.at
     is 'When: the hook event''s own time (assistant_events.ts), never its insert time.';
comment on column session_signals.until
     is 'slice only: its last event''s time.';
comment on column session_signals.category
     is 'Run signals and unmatched calls: build, typecheck, test or e2e.';
comment on column session_signals.reason
     is 'slice only: why it ended.';
comment on column session_signals.ref
     is 'review: the reviewer. review_fix: the path token the review named. task_tick: the task.';
