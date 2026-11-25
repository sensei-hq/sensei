-- Read grant for relay.lanes_all (#393). No policy: it is security-invoker, so the
-- caller's RLS applies.
grant select on relay.lanes_all to authenticated;
