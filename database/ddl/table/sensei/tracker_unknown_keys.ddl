set search_path to sensei, extensions;

-- Keys a tracker's board was asked for and does not have (R11 negative cache).
-- Spec: docs/spec/daemon/tracker-jira-linkage.md R2, R11.
create table if not exists tracker_unknown_keys (
  tracker_id  uuid         not null references sensei.trackers(id) on delete cascade
, key         text         not null
, checked_at  timestamptz  not null default now()
, primary key (tracker_id, key)
);

comment on table tracker_unknown_keys is
'Ticket keys named in AI sessions that the tracker''s board does not have, as of checked_at. On-demand discovery does not ask the provider again for a key here until its TTL (6 h) passes; a full sync deletes the row of any key it now has.';

comment on column tracker_unknown_keys.tracker_id
     is 'The tracker whose board was asked.';
comment on column tracker_unknown_keys.key
     is 'The key as the adapter shaped it (Jira: upper-cased).';
comment on column tracker_unknown_keys.checked_at
     is 'When the board last answered that it does not have this key.';
