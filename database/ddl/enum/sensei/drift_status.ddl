set search_path to sensei, extensions;

create type drift_status
    as enum ('current', 'drifted', 'broken');
