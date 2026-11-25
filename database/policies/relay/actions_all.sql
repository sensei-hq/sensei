-- Read grant for relay.actions_all (#393). No policy: it is security-invoker, so the
-- caller's RLS applies.
grant select on relay.actions_all to authenticated;
