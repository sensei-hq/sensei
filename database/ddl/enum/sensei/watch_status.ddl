set search_path to sensei, extensions;

create type watch_status
    as enum ('scanning', 'watching', 'paused');
