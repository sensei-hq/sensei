set search_path to sensei, extensions;

create type inference_role
    as enum ('inference', 'consolidation', 'embedding', 'voice', 'default_fallback');
