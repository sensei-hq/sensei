set search_path to relay, extensions;

-- Where a sprint run stands: drafted with its items and no crew yet, Task 0
-- under way, the plan unpacked and tasks flowing, every task done, or stopped
-- by the person. Labels append, never reorder.
create type relay.run_state as enum (
  'planning',
  'active',
  'completed',
  'cancelled',
  'draft'
);

comment on type relay.run_state is
'draft until started (#419: no crew, no Task 0); planning until the peer-approved plan unpacks; active until every task is done or cancelled; completed/cancelled are terminal.';
