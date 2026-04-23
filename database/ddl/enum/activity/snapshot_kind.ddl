set search_path to activity, extensions;

create type snapshot_kind
    as enum ('manual', 'checkpoint');
