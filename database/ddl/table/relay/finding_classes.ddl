set search_path to relay, extensions;

-- The shared vocabulary of review findings. One table, no owner: seeded
-- identically on both planes from the README checklist, extended by the
-- learning loop (which signs its rows). The daemon rejects unknown classes.
create table if not exists relay.finding_classes (
  code     text not null,
  label    text not null,
  added_by uuid,
  constraint finding_classes_pkey primary key (code)
);

alter table relay.finding_classes enable row level security;

comment on table relay.finding_classes is
'Finding classes. Shared lookup: no user_id, readable by all, writable by the daemon owner only.';
comment on column relay.finding_classes.added_by is 'Who added the class. Null for the seeded ten.';
