set search_path to sensei, extensions;

create type recommendation_verdict
    as enum ('pending', 'positive', 'negative', 'neutral');
