set search_path to inference, extensions;

create type recommendation_verdict
    as enum ('pending', 'positive', 'negative', 'neutral');
