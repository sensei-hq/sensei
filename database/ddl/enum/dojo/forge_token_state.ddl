set search_path to dojo, extensions;

-- Whether a connection's stored forge token still works (#308 R5b, D35). `ok`
-- after a token is set or replaced; `needs_new_token` once the forge rejects it
-- (expired or revoked) — reads stop until an admin replaces it, and a failed read
-- never ends a membership (#309). No token at all is NULL on the connection.
create type dojo.forge_token_state
    as enum ('ok', 'needs_new_token');
