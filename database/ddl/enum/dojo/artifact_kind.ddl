set search_path to dojo, extensions;

-- The six primitives that ride the Dōjō loop. Each has a canonical shape in
-- dojo.artifacts.payload (jsonb keyed by kind); upstream and downstream shapes
-- are identical so the round-trip preserves the payload.
--   principle 理 durable engineering value
--   pattern   紋 constructive shape (or anti-pattern) promoted to a rule
--   prompt    問 vetted prompt template or persona
--   guard     守 lint / check / safety rail
--   skill     技 packaged capability an assistant can pick up
--   agent     使 configured agent with tools and scope
create type dojo.artifact_kind
    as enum ('principle', 'pattern', 'prompt', 'guard', 'skill', 'agent');
