set search_path to sensei, extensions;

-- A tracker's sprints, mirrored from the provider and never written back. Spec: tracker-jira-linkage.md R2, R5.
create table if not exists sprints (
  id                     uuid         primary key default gen_random_uuid()
, tracker_id             uuid         not null references sensei.trackers(id) on delete cascade
, external_id            text         not null
, seq                    bigint       not null
, name                   text         not null
, state                  text         not null
, start_at               timestamptz
, end_at                 timestamptz
, complete_at            timestamptz
, members_fetched_state  text
, removed_at             timestamptz
, last_seen_at           timestamptz  not null default now()
, unique (tracker_id, external_id)
);

comment on table sprints is
'Sprints of a tracker board, mirrored from the provider and never written back (spec tracker-metrics-writeback writes usage to tickets only). A sprint missing from a full sync gets removed_at (never deleted), so history and links survive.';

comment on column sprints.tracker_id
     is 'The board this sprint belongs to.';
comment on column sprints.external_id
     is 'The provider''s sprint id, as text.';
comment on column sprints.seq
     is 'Adapter-supplied creation order (Jira: the numeric sprint id). The tie-break for a ticket''s current sprint; never compare external_id text.';
comment on column sprints.name
     is 'Sprint name as the provider shows it.';
comment on column sprints.state
     is 'Common lifecycle state: future, active or closed (adapters map theirs onto these).';
comment on column sprints.start_at
     is 'Planned or actual start; NULL for a future sprint without dates.';
comment on column sprints.end_at
     is 'Planned end.';
comment on column sprints.complete_at
     is 'When the sprint was actually closed; NULL while open.';
comment on column sprints.members_fetched_state
     is 'The state at the last membership fetch. A sprint closed when fetched is never fetched again (closed sprints do not change).';
comment on column sprints.removed_at
     is 'When a full sync stopped seeing it. Set once; never advanced by later syncs.';
comment on column sprints.last_seen_at
     is 'When a sync last saw it.';
