set search_path to sensei, extensions;

-- A usage metric a tracker field can be mapped to (spec tracker-metrics-writeback W3).
-- Order and labels are pinned to `crate::tracker::writeback::TrackerMetric::ALL` by a test;
-- the order is the order mapped fields are written and hashed in.
create type tracker_metric as enum (
  'sessions'
, 'active_hours'
, 'claude_hours'
, 'user_hours'
, 'wall_hours'
, 'cycle_hours'
, 'tokens_fresh'
, 'cache_read'
, 'cache_write'
, 'tokens_out'
, 'tokens_reasoning'
, 'metered_cost'
, 'api_cost'
);

comment on type tracker_metric is
'A usage metric of sensei.tracker_fields (W3). Mirrors crate::tracker::writeback::TrackerMetric.';
