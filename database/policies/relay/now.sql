-- Read grant for relay.now. No policy: it is security-invoker, so the
-- caller's RLS on the base tables applies.
grant select on relay.now to authenticated;
