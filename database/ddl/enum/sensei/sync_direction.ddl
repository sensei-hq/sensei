set search_path to sensei, extensions;

-- Which way a sync moves.
--
-- Both directions are tracked per entity because they fail independently: a push
-- can be rejected by governance while a pull is perfectly healthy, and collapsing
-- them into one row would hide whichever failed second.
create type sync_direction as enum ('push', 'pull');
