set search_path to sensei, extensions;

-- A workspace's SDLC metric settings (spec tracker-sdlc-metrics S1, D3). One row per
-- workspace, created on the first PUT; no row reads as the defaults. Project-level
-- by design: never a per-person rate.
create table if not exists workspace_metric_settings (
  workspace_id            uuid         primary key references sensei.workspaces(id) on delete cascade
, hourly_rate             numeric      check (hourly_rate > 0 and hourly_rate <= 10000)
, minutes_per_point       numeric      check (minutes_per_point > 0 and minutes_per_point <= 10080)
, idle_threshold_minutes  integer      not null default 15 check (idle_threshold_minutes between 1 and 240)
, modified_at             timestamptz  not null default now()
);

comment on table workspace_metric_settings is
'SDLC metric settings per workspace (spec tracker-sdlc-metrics S1). No row = defaults (no rate, no calibration, 15-minute idle gap). Never per person, and never written to a tracker.';

comment on column workspace_metric_settings.workspace_id
     is 'The workspace these settings apply to.';
comment on column workspace_metric_settings.hourly_rate
     is 'Blended cost of an hour of work, for effort and rework cost. NULL: those costs are null with reason no_rate. The upper bound also rejects NaN, which Postgres orders above every number.';
comment on column workspace_metric_settings.minutes_per_point
     is 'Active minutes a story point is expected to take, for expected time and time variance. NULL: those are null with reason no_calibration.';
comment on column workspace_metric_settings.idle_threshold_minutes
     is 'A gap between hook events longer than this ends a work slice (spec S2) and counts as a hold. Changing it re-derives the workspace''s sessions.';
comment on column workspace_metric_settings.modified_at
     is 'When the settings last changed.';
