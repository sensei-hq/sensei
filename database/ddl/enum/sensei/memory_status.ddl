set search_path to sensei, extensions;

create type memory_status
    as enum ('proposed', 'active', 'reinforced', 'challenged',
             'battle_tested', 'archived', 'rejected');
