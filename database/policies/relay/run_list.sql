-- Read grant for relay.run_list. No policy: it is security-invoker, so the
-- caller's RLS on relay.runs, relay.tasks and relay.actions applies.
grant select on relay.run_list to authenticated;
