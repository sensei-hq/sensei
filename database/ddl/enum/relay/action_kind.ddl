set search_path to relay, extensions;

-- What happened, one row per event. The kind decides which props keys the
-- daemon validates and which fold of the task view reads them.
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
  'compacted'
);

comment on type relay.action_kind is
'delivered/read are receipts: they fold into their message, never entries of their own.';
