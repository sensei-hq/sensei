set search_path to sensei, extensions;

-- Why a work slice ended (spec tracker-sdlc-metrics S2): the reason a hold that
-- follows it is attributed to.
create type hold_reason as enum (
  'build_failure'
, 'test_failure'
, 'permission_wait'
, 'interrupted'
, 'session_ended'
, 'unknown'
);

comment on type hold_reason is
'Why a work slice ended, read from its last meaningful event (spec tracker-sdlc-metrics S2).';
