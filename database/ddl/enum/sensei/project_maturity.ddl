set search_path to sensei, extensions;

create type project_maturity
    as enum ('discovery', 'active', 'maintenance', 'archived');
