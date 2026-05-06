set search_path to sensei, extensions;

create type library_ecosystem
    as enum ('npm', 'pypi', 'cargo', 'go', 'docs');
