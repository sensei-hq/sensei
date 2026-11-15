-- Read grant for relay.task_history. No policy: it is security-invoker, so
-- the caller's RLS on the base tables applies.
grant select on relay.task_history to authenticated;
