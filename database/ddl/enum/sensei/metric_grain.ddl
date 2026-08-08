set search_path to sensei, extensions;

-- Grain a project_metrics row is stored at. session -> session_id set;
-- daily -> session_id null (the aggregate the roll-up views read).
create type metric_grain
    as enum ('session', 'daily');
