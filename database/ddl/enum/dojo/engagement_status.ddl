set search_path to dojo, extensions;

-- Lifecycle of a client engagement. `active` = in force, its work routes to
-- this engagement and is dereferenced; `ended` = closed (retained for the
-- compliance audit trail).
create type dojo.engagement_status
    as enum ('active', 'ended');
