set search_path to gateway, extensions;

create type model_capability
    as enum ('chat', 'reasoning', 'embed', 'classify', 'summarize', 'vision', 'audio');
