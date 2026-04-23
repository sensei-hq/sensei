set search_path to inference, extensions;

create type recommendation_urgency
    as enum ('low', 'medium', 'high');
