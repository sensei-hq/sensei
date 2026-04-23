set search_path to activity, extensions;

create type session_outcome
    as enum ('completed', 'corrected', 'blocked', 'partial', 'abandoned');
