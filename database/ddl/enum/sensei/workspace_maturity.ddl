set search_path to sensei, extensions;

create type workspace_maturity
    as enum ('discovery', 'active', 'maintenance', 'archived');
