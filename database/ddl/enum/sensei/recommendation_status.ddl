set search_path to sensei, extensions;

create type recommendation_status
    as enum ('pending', 'accepted', 'dismissed', 'superseded');
