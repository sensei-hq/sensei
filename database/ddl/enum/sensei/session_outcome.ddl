set search_path to sensei, extensions;

create type session_outcome
    as enum ('completed', 'corrected', 'blocked', 'partial', 'abandoned');
