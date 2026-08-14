set search_path to sensei, extensions;

create type session_outcome
    as enum ('empty', 'incomplete', 'completed', 'corrected', 'blocked', 'partial', 'abandoned');
