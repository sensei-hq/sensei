set search_path to sensei, extensions;

-- The tracker fields usage is written into (spec tracker-metrics-writeback W6, W7).
create table if not exists tracker_fields (
  tracker_id  uuid            not null references sensei.trackers(id) on delete cascade
, metric      tracker_metric
, field_ref   text            not null
, retired_at  timestamptz
, unique (tracker_id, metric)
, unique (tracker_id, field_ref)
);

comment on table tracker_fields is
'A tracker''s custom fields for usage write-back. A row with a metric is mapped: that metric is written into the field. A row without one is retired: sensei wrote the field before, so it clears it on each ticket''s next write. Replaced by PUT …/trackers/{tracker}/fields.';

comment on column tracker_fields.tracker_id
     is 'The board the field belongs to.';
comment on column tracker_fields.metric
     is 'The metric written into the field; NULL once the field is no longer mapped (retired).';
comment on column tracker_fields.field_ref
     is 'The tracker''s field id (Jira: customfield_<n>).';
comment on column tracker_fields.retired_at
     is 'When the field stopped being mapped: a ticket last written before it gets null in the field. NULL while mapped.';
