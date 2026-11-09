set search_path to relay, extensions;

-- What a queue item is: a tracker's kind, or a review finding raised by hand.
create type relay.item_kind as enum (
  'bug',
  'feature',
  'gap',
  'issue',
  'finding'
);

comment on type relay.item_kind is
'Ticketed items snapshot kind at + time; the screen reads the live value from the tracker.';
