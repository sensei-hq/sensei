set search_path to dojo, extensions;

-- Lifecycle of a client engagement. `active` = in force, its work routes to
-- this engagement (source-dereferenced on publish, the always-on invariant);
-- `ended` = closed (retained for the compliance audit trail).
create type dojo.engagement_status
    as enum ('active', 'ended');
