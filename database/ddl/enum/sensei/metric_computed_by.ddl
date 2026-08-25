set search_path to sensei, extensions;

-- Who computed a metric row: this machine, or dōjō handing it down.
--
-- The loop-breaker for pull-else-compute. Without it, local pulls a shared value,
-- cannot tell it apart from its own, recomputes it, and pushes it back — forever.
--
-- Named `computed_by` rather than `origin` deliberately. `origin` already means
-- authored|learned|imported|builtin on the content entities, and this table
-- already carries `source` (measured|estimated). A third near-synonym beside two
-- existing ones is exactly the confusion the entity vocabulary was introduced to
-- end.
create type metric_computed_by as enum ('local', 'dojo');
