set search_path to dojo, extensions;

-- The consumer's local decision on a distributed artifact, fed back for
-- landing telemetry. `pending` = arrived, not yet acted on; `applied` =
-- accepted into the local rule/skill surface; `muted` = suppressed locally;
-- `pinned` = accepted and outranks ambiguous local alternatives on conflict.
create type dojo.inbox_state
    as enum ('pending', 'applied', 'muted', 'pinned');
