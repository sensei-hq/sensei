set search_path to inference, extensions;

create type drift_status
    as enum ('current', 'drifted', 'broken');
