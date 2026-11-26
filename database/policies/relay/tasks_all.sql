-- Read grant for relay.tasks_all (#393). No policy: it is security-invoker, so the
-- caller's RLS applies.
grant select on relay.tasks_all to authenticated;
