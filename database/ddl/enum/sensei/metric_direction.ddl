set search_path to sensei, extensions;

-- Which way is "good" for a metric — normalizes to [0,1] in the health score
-- (higher_better -> v; lower_better -> 1-v; neutral is read only with its companion).
create type metric_direction
    as enum ('higher_better', 'lower_better', 'neutral');
