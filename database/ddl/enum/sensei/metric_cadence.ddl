set search_path to sensei, extensions;

-- How a metric's group advances against its per-(repository, group) watermark.
--   commit -> immutable, keyed on new commits after the last sampled sha
--             (quality, churn-rate/concentration).
--   day    -> a calendar-day value that reopens the trailing/open day as late
--             data lands (session outcomes, autonomy, snapshots, rework_density).
-- ORTHOGONAL to capture authorization: a day-cadence metric may still be forbidden
-- from authorizing activity reclaim (see metric_capture).
create type metric_cadence
    as enum ('commit', 'day');
