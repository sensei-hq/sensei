set search_path to inference, extensions;

create type recommendation_status
    as enum ('pending', 'accepted', 'dismissed', 'superseded');
