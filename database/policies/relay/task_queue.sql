-- Read grant for relay.task_queue. No policy: it is security-invoker, so the
-- caller's RLS on relay.tasks applies.
grant select on relay.task_queue to authenticated;
