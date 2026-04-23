set search_path to inference, extensions;

create type preference_confidence
    as enum ('low', 'medium', 'high');
