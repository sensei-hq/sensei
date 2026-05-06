set search_path to sensei, extensions;

create type hyperedge_kind
    as enum ('flow', 'group', 'co_change', 'addressed_by');
