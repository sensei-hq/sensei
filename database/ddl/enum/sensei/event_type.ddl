set search_path to sensei, extensions;

create type event_type
    as enum (
        'tool_call'
      , 'api_request'
      , 'correction'
      , 'turn'
      , 'phase_transition'
      , 'checkpoint'
      , 'task_start'
      , 'task_end'
      , 'context_loaded'
      , 'edit'
      , 'test'
      , 'error'
    );
