set search_path to inference, extensions;

create type hyperedge_kind
    as enum ('flow', 'group', 'co_change', 'addressed_by');
