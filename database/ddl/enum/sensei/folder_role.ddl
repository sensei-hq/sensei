set search_path to sensei, extensions;

create type folder_role
    as enum ('backend', 'frontend', 'library', 'docs', 'infra');
