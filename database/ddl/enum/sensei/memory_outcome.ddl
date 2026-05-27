set search_path to sensei, extensions;

create type memory_outcome
    as enum ('applied', 'consulted', 'violated', 'ignored');
