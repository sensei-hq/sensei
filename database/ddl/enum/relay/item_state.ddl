set search_path to relay, extensions;

-- Where a queue item stands, in lifecycle order: unjudged, queued, promised, gone.
create type relay.item_state as enum (
  'triage',
  'backlog',
  'planned',
  'closed'
);

comment on type relay.item_state is
'triage leads: it precedes the backlog in the flow (lanes-data''s "one extra state").';
