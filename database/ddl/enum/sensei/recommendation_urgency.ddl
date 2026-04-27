set search_path to sensei, extensions;

create type recommendation_urgency
    as enum ('low', 'medium', 'high');
