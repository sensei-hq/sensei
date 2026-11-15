set search_path to relay, extensions;

-- Where a sprint run stands: Task 0 under way, the plan unpacked and tasks
-- flowing, every task done, or stopped by the person.
create type relay.run_state as enum (
  'planning',
  'active',
  'completed',
  'cancelled'
);

comment on type relay.run_state is
'planning until the peer-approved plan unpacks; active until every task is done or cancelled; completed/cancelled are terminal.';
