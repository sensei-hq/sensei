set search_path to relay, extensions;

-- What happened, one row per event. The kind decides which props keys the
-- daemon validates and which fold of the task view reads them. The last four
-- are the self-service queue's: a task or review claimed, a claim released
-- back to the pool, an in-turn checkpoint saved.
create type relay.action_kind as enum (
  'assigned',
  'brief_updated',
  'started',
  'reported',
  'reviewed',
  'asked',
  'answered',
  'message',
  'delivered',
  'read',
  'merged',
  'interrupted',
  'reoffered',
  'compacted',
  'handed_over',
  'claimed',
  'review_claimed',
  'released',
  'checkpointed'
);

comment on type relay.action_kind is
'delivered/read are receipts: they fold into their message, never entries of their own. claimed/review_claimed/released/checkpointed are the queue''s hand-offs.';
