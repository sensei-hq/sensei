set search_path to sensei, extensions;

create type assistant_family
    as enum (
        'claude'
      , 'cursor'
      , 'zed'
      , 'continue'
      , 'codex'
      , 'aider'
      , 'opencode'
      , 'kiro'
    );
