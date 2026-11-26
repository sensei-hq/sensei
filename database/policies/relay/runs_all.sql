-- Read grant for relay.runs_all (#393). No policy: it is security-invoker, so the
-- caller's RLS applies.
grant select on relay.runs_all to authenticated;
