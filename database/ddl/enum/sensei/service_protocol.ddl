set search_path to sensei, extensions;

create type service_protocol
    as enum ('mcp', 'ollama', 'anthropic', 'openai');
