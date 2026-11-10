set search_path to relay, extensions;

-- The quota window a limit-paused lane ran out of: a daily cap, a weekly
-- cap, or anything else (rate window, org policy, unknown).
create type relay.limit_scope as enum (
  'daily',
  'weekly',
  'other'
);

comment on type relay.limit_scope is
'Which quota a paused lane hit. daily/weekly mirror the assistant warnings; other is everything else.';
