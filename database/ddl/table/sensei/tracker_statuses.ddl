set search_path to sensei, extensions;

-- A tracker's statuses and their categories (spec tracker-sdlc-metrics S4), so the
-- sprint replay can tell when a status in a ticket's history meant done.
create table if not exists tracker_statuses (
  tracker_id  uuid  not null references sensei.trackers(id) on delete cascade
, status_id   text  not null
, name        text  not null
, category    text  not null check (category in ('new', 'indeterminate', 'done', 'undefined'))
, primary key (tracker_id, status_id)
);

comment on table tracker_statuses is
'Statuses of a tracker and their category, replaced by each history run (spec tracker-sdlc-metrics S4).';

comment on column tracker_statuses.tracker_id
     is 'The tracker.';
comment on column tracker_statuses.status_id
     is 'The provider''s status id, as the changelog records it.';
comment on column tracker_statuses.name
     is 'The status name.';
comment on column tracker_statuses.category
     is 'statusCategory.key: new, indeterminate, done (or undefined).';
