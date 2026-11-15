set search_path to sensei, extensions;

-- What a session-signal row records (spec tracker-sdlc-metrics S2).
create type session_signal_kind as enum (
  'slice'              -- a base work slice: events no more than a minute apart
, 'prompt'             -- a user prompt
, 'code_edit'          -- an edit of a code file (not docs/spec)
, 'build_ok'
, 'build_fail'
, 'test_ok'
, 'test_fail'
, 'run_unknown'        -- a build or test whose outcome can't be attributed
, 'unclassified_fail'  -- a failed command that ran no classified build or test
, 'unmatched_call'     -- a call that never reported a result (a lost failure, or interrupted)
, 'pr_created'
, 'pushed'
, 'task_tick'          -- a spec-driven task marked done
, 'review'             -- a review's text arrived
, 'review_fix'         -- the first edit, after a review, of a file it named
);

comment on type session_signal_kind is
'Kinds of activity.session_signals rows (spec tracker-sdlc-metrics S2).';
