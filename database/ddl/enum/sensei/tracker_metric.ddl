set search_path to sensei, extensions;

-- A usage or SDLC metric a tracker field can be mapped to (specs tracker-metrics-writeback W3,
-- tracker-sdlc-metrics S9).
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
-- SDLC figures (spec tracker-sdlc-metrics S9), appended in its order.
, 'compile_defects'
, 'test_defects'
, 'review_fixes'
, 'escaped_task_defects'
, 'review_defects'
, 'total_defects'
, 'build_runs'
, 'rework_cycles'
, 'prompts'
, 'testing_efficiency'
, 'review_efficiency'
, 'build_failure_rate'
, 'on_hold_ratio'
, 'post_review_rework_ratio'
, 'mean_build_recovery_minutes'
, 'on_hold_minutes'
, 'rework_minutes'
, 'worked_minutes'
, 'execution_hours'
, 'lead_hours'
, 'cost_per_prompt_cycle'
);

comment on type tracker_metric is
'A usage (W3) or SDLC (S9) metric of sensei.tracker_fields. Mirrors crate::tracker::writeback::TrackerMetric.';
