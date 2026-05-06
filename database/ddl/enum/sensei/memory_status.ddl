set search_path to sensei, extensions;

create type memory_status
    as enum ('active', 'reinforced', 'challenged', 'battle_tested', 'archived');
