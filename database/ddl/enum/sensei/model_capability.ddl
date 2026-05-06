set search_path to sensei, extensions;

create type model_capability
    as enum ('chat', 'reasoning', 'embed', 'classify', 'summarize', 'vision', 'audio');
