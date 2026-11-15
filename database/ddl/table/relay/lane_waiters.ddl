set search_path to relay, extensions;

-- Who is holding tasks/wait open right now (F06). One row per waiting lane,
-- inserted when the hold begins and deleted when it answers; the sweeper
-- reads waiting_since to nudge idle lanes and evicts rows whose lane lease
-- lapsed. Never synced: a wait is a local fact.
create table if not exists relay.lane_waiters (
  user_id       uuid        not null,
  lane_id       uuid        not null,
  crew_id       uuid        not null,
  waiting_since timestamptz not null default current_timestamp,
  last_poll_at  timestamptz not null default current_timestamp,
  constraint lane_waiters_pkey primary key (user_id, lane_id),
  constraint lane_waiters_lane_fk foreign key (user_id, lane_id) references relay.lanes (user_id, id) on delete cascade,
  constraint lane_waiters_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id)
);

alter table relay.lane_waiters enable row level security;

comment on table relay.lane_waiters is
'Active long-poll waiters, one per lane. Inserted on hold, deleted on answer; the sweeper nudges by waiting_since and evicts dead lanes.';
comment on column relay.lane_waiters.waiting_since is 'When the lane first found nothing to do (not reset by a re-poll that still finds nothing).';
comment on column relay.lane_waiters.last_poll_at is 'The most recent hold the lane opened.';
